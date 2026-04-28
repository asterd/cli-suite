use std::{
    collections::VecDeque,
    io::IsTerminal,
    process::Stdio,
    time::{Duration, Instant},
};

use camino::{Utf8Path, Utf8PathBuf};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command as TokioCommand},
};

use crate::{
    cli::{CaptureMode, RunArgs},
    error::{Result, RunError},
    fswatch::{Snapshot, WatchOptions},
    model::{RunCommand, RunData, SavedRun, StreamSummary},
    storage,
};

#[derive(Debug)]
pub struct RunRequest<'a> {
    pub args: &'a RunArgs,
    pub base_cwd: &'a Utf8Path,
}

pub async fn run_command(request: RunRequest<'_>) -> Result<RunData> {
    if request.args.command.is_empty() {
        return Err(RunError::MissingCommand);
    }

    let cwd = resolve_cwd(request.base_cwd, request.args.cwd.as_ref())?;
    let env = collect_env(request.args, request.base_cwd)?;
    let command = command_model(&request.args.command, request.args.shell);
    let save_paths = if request.args.no_save {
        None
    } else {
        Some(storage::prepare(
            &cwd,
            request.args.save.as_deref(),
            &request.args.command,
        )?)
    };
    let before = if request.args.watch_files_enabled() {
        Some(Snapshot::capture(&cwd, &watch_options(request.args))?)
    } else {
        None
    };

    let resolved = resolved_capture(request.args.capture);
    let mut child = spawn_child(request.args, &cwd, &env, resolved)?;
    let stdout = child.child.stdout.take();
    let stderr = child.child.stderr.take();
    let stdout_path = save_paths.as_ref().map(|paths| paths.stdout.clone());
    let stderr_path = save_paths.as_ref().map(|paths| paths.stderr.clone());
    let capture = matches!(resolved, ResolvedCapture::Pipe);
    let max_log_bytes = request.args.max_log_bytes.0;
    let tail_bytes = request.args.tail_bytes.unwrap_or(4096);

    let stdout_task = tokio::spawn(capture_stream(
        stdout,
        stdout_path.clone(),
        capture,
        max_log_bytes,
        tail_bytes,
    ));
    let stderr_task = tokio::spawn(capture_stream(
        stderr,
        stderr_path.clone(),
        capture,
        max_log_bytes,
        tail_bytes,
    ));

    let started = Instant::now();
    let (exit, timed_out) =
        wait_with_timeout(&mut child, request.args.timeout.map(|arg| arg.0)).await?;
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    let mut stdout_summary = stdout_task
        .await
        .map_err(|err| RunError::Execute(std::io::Error::other(err.to_string())))??;
    let mut stderr_summary = stderr_task
        .await
        .map_err(|err| RunError::Execute(std::io::Error::other(err.to_string())))??;
    stdout_summary.log = stdout_path.map(|path| path.to_string());
    stderr_summary.log = stderr_path.map(|path| path.to_string());

    let mut changed = if let Some(before) = before {
        let after = Snapshot::capture(&cwd, &watch_options(request.args))?;
        before.diff(&after)
    } else {
        Vec::new()
    };
    let changed_count = changed.len();
    if request.args.summary_only {
        stdout_summary.tail.clear();
        stderr_summary.tail.clear();
        changed.clear();
    }

    let saved = save_paths.as_ref().map(|paths| SavedRun {
        name: paths.name.clone(),
        path: paths.dir.to_string(),
    });
    let truncated = stdout_summary.truncated || stderr_summary.truncated;
    let data = RunData {
        command,
        cwd: cwd.to_string(),
        exit,
        timed_out,
        duration_ms,
        stdout: stdout_summary,
        stderr: stderr_summary,
        changed_count,
        changed,
        saved,
        truncated,
    };
    if let Some(paths) = save_paths {
        let summary = crate::render::agent_summary_line(&data)?;
        storage::write_artifacts(&paths, &data, &format!("{summary}\n"))?;
        suggest_gitignore(&cwd);
    }
    Ok(data)
}

fn resolve_cwd(base: &Utf8Path, requested: Option<&Utf8PathBuf>) -> Result<Utf8PathBuf> {
    let cwd = requested.as_ref().map_or_else(
        || base.to_owned(),
        |path| {
            if path.is_absolute() {
                (*path).clone()
            } else {
                base.join(path)
            }
        },
    );
    if !cwd.exists() {
        return Err(RunError::PathNotFound(cwd));
    }
    Ok(cwd)
}

fn collect_env(args: &RunArgs, base_cwd: &Utf8Path) -> Result<Vec<(String, String)>> {
    let mut values = Vec::new();
    if let Some(path) = &args.env_file {
        let path = if path.is_absolute() {
            path.clone()
        } else {
            base_cwd.join(path)
        };
        let text = std::fs::read_to_string(&path).map_err(|source| RunError::EnvFileRead {
            path: path.clone(),
            source,
        })?;
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(RunError::EnvFileParse {
                    path: path.clone(),
                    line: index + 1,
                });
            };
            values.push((
                key.trim().to_owned(),
                strip_env_quotes(value.trim()).to_owned(),
            ));
        }
    }
    for item in &args.env {
        let Some((key, value)) = item.split_once('=') else {
            return Err(RunError::InvalidEnv(item.clone()));
        };
        values.push((key.to_owned(), value.to_owned()));
    }
    Ok(values)
}

fn strip_env_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|stripped| stripped.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|stripped| stripped.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn command_model(command: &[String], shell: bool) -> RunCommand {
    RunCommand {
        program: command[0].clone(),
        args: command.iter().skip(1).cloned().collect(),
        shell,
    }
}

struct ManagedChild {
    child: Child,
    #[cfg(windows)]
    job: Option<windows_job::Job>,
}

fn spawn_child(
    args: &RunArgs,
    cwd: &Utf8Path,
    env: &[(String, String)],
    resolved: ResolvedCapture,
) -> Result<ManagedChild> {
    let mut command = if args.shell {
        shell_command(&args.command)
    } else {
        let mut command = TokioCommand::new(&args.command[0]);
        command.args(args.command.iter().skip(1));
        command
    };
    command.current_dir(cwd);
    command.envs(env.iter().map(|(key, value)| (key, value)));
    match resolved {
        ResolvedCapture::Inherit => {
            command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }
        ResolvedCapture::Pipe => {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
    }
    configure_process_group(&mut command);
    let child = command.spawn().map_err(RunError::Execute)?;
    #[cfg(windows)]
    let job = windows_job::assign(&child)?;
    Ok(ManagedChild {
        child,
        #[cfg(windows)]
        job,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedCapture {
    Pipe,
    Inherit,
}

fn resolved_capture(mode: CaptureMode) -> ResolvedCapture {
    match mode {
        CaptureMode::Always => ResolvedCapture::Pipe,
        CaptureMode::Never => ResolvedCapture::Inherit,
        CaptureMode::Auto => {
            if std::io::stdout().is_terminal() {
                ResolvedCapture::Inherit
            } else {
                ResolvedCapture::Pipe
            }
        }
    }
}

fn shell_command(command_parts: &[String]) -> TokioCommand {
    let joined = command_parts.join(" ");
    #[cfg(windows)]
    {
        let mut command = TokioCommand::new("cmd");
        command.arg("/C").arg(joined);
        command
    }
    #[cfg(not(windows))]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
        let mut command = TokioCommand::new(shell);
        command.arg("-lc").arg(joined);
        command
    }
}

#[cfg(unix)]
fn configure_process_group(command: &mut TokioCommand) {
    // SAFETY: `pre_exec` runs after fork and before exec. The closure only
    // calls async-signal-safe `setpgid` and returns the OS error directly.
    unsafe {
        command.pre_exec(|| {
            if libc::setpgid(0, 0) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut TokioCommand) {}

async fn wait_with_timeout(
    child: &mut ManagedChild,
    timeout: Option<Duration>,
) -> Result<(Option<i32>, bool)> {
    if let Some(timeout) = timeout {
        match tokio::time::timeout(timeout, child.child.wait()).await {
            Ok(status) => Ok((status.map_err(RunError::Execute)?.code(), false)),
            Err(_) => {
                terminate_child(child).await?;
                Ok((None, true))
            }
        }
    } else {
        Ok((
            child.child.wait().await.map_err(RunError::Execute)?.code(),
            false,
        ))
    }
}

async fn terminate_child(child: &mut ManagedChild) -> Result<()> {
    #[cfg(windows)]
    if let Some(job) = &child.job {
        job.terminate();
        return child
            .child
            .wait()
            .await
            .map(|_| ())
            .map_err(RunError::Execute);
    }
    #[cfg(unix)]
    if let Some(id) = child.child.id() {
        let pgid = -(i32::try_from(id).unwrap_or(i32::MAX));
        // SAFETY: sending a signal to the process group created in
        // `configure_process_group`; errors are non-fatal because the child may
        // have exited between timeout detection and signal delivery.
        unsafe {
            libc::kill(pgid, libc::SIGTERM);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
        if let Ok(Some(_status)) = child.child.try_wait() {
            return Ok(());
        }
        // SAFETY: same process-group signal as above, escalated after grace.
        unsafe {
            libc::kill(pgid, libc::SIGKILL);
        }
    }
    child.child.kill().await.map_err(RunError::Execute)
}

async fn capture_stream(
    stream: Option<impl tokio::io::AsyncRead + Unpin>,
    path: Option<Utf8PathBuf>,
    capture: bool,
    max_log_bytes: u64,
    tail_bytes: usize,
) -> Result<StreamSummary> {
    let Some(mut stream) = stream else {
        return Ok(StreamSummary {
            bytes: 0,
            lines: 0,
            truncated: false,
            log: None,
            tail: Vec::new(),
        });
    };
    let mut file = if capture {
        match path {
            Some(path) => Some(
                tokio::fs::File::create(&path)
                    .await
                    .map_err(|source| RunError::Io { path, source })?,
            ),
            None => None,
        }
    } else {
        None
    };
    let mut bytes = 0_u64;
    let mut written = 0_u64;
    let mut lines = 0_usize;
    let mut truncated = false;
    let mut tail = TailBuffer::new(tail_bytes);
    let mut buffer = [0; 8192];
    loop {
        let read = stream.read(&mut buffer).await.map_err(RunError::Execute)?;
        if read == 0 {
            break;
        }
        bytes = bytes.saturating_add(u64::try_from(read).unwrap_or(u64::MAX));
        lines += buffer[..read].iter().filter(|byte| **byte == b'\n').count();
        tail.push(&buffer[..read]);
        if let Some(file) = &mut file {
            let remaining = max_log_bytes.saturating_sub(written);
            if remaining > 0 {
                let to_write = read.min(usize::try_from(remaining).unwrap_or(usize::MAX));
                tokio::io::AsyncWriteExt::write_all(file, &buffer[..to_write])
                    .await
                    .map_err(RunError::Execute)?;
                written = written.saturating_add(u64::try_from(to_write).unwrap_or(u64::MAX));
            }
            if u64::try_from(read).unwrap_or(u64::MAX) > remaining {
                truncated = true;
            }
        }
    }
    Ok(StreamSummary {
        bytes,
        lines,
        truncated,
        log: None,
        tail: tail.lines(),
    })
}

#[derive(Debug)]
struct TailBuffer {
    max: usize,
    bytes: VecDeque<u8>,
}

impl TailBuffer {
    fn new(max: usize) -> Self {
        Self {
            max,
            bytes: VecDeque::new(),
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        if self.max == 0 {
            return;
        }
        for byte in chunk {
            self.bytes.push_back(*byte);
            while self.bytes.len() > self.max {
                self.bytes.pop_front();
            }
        }
    }

    fn lines(self) -> Vec<String> {
        let bytes = self.bytes.into_iter().collect::<Vec<_>>();
        String::from_utf8_lossy(&bytes)
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    }
}

fn watch_options(args: &RunArgs) -> WatchOptions {
    WatchOptions {
        include: args.include.clone(),
        exclude: args.exclude.clone(),
        hash: args.hash,
    }
}

fn suggest_gitignore(cwd: &Utf8Path) {
    let gitignore = cwd.join(".gitignore");
    let ignored = std::fs::read_to_string(&gitignore)
        .map(|text| text.lines().any(|line| line.trim() == ".axt/"))
        .unwrap_or(false);
    if !ignored {
        eprintln!("Suggestion: add .axt/ to .gitignore to ignore saved run artifacts.");
    }
}

#[cfg(windows)]
mod windows_job {
    use tokio::process::Child;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
            SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        },
    };

    use crate::error::{Result, RunError};

    pub struct Job(HANDLE);

    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    impl Job {
        pub fn terminate(&self) {
            unsafe {
                TerminateJobObject(self.0, 1);
            }
        }
    }

    pub fn assign(child: &Child) -> Result<Option<Job>> {
        // SAFETY: null security attributes and an unnamed job object are valid
        // CreateJobObjectW inputs. The returned handle is checked for null.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(RunError::Execute(std::io::Error::last_os_error()));
        }
        let job = Job(job);
        // SAFETY: JOBOBJECT_EXTENDED_LIMIT_INFORMATION is a repr(C), Copy Win32
        // data structure. Zero-initialization matches the documented C pattern
        // before setting the specific limit flags we need.
        let mut info = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let info_size = u32::try_from(std::mem::size_of_val(&info)).unwrap_or(u32::MAX);
        // SAFETY: `info` points to a valid structure of `info_size` bytes for
        // the JobObjectExtendedLimitInformation class.
        let configured = unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                std::ptr::addr_of!(info).cast(),
                info_size,
            )
        };
        if configured == 0 {
            return Err(RunError::Execute(std::io::Error::last_os_error()));
        }
        let handle = child.raw_handle().ok_or_else(|| {
            RunError::Execute(std::io::Error::other("child process handle is unavailable"))
        })?;
        // SAFETY: both handles are live OS handles owned by the job wrapper and
        // Tokio child process. The call only associates the process with the job.
        let assigned = unsafe { AssignProcessToJobObject(job.0, handle) };
        if assigned == 0 {
            return Err(RunError::Execute(std::io::Error::last_os_error()));
        }
        Ok(Some(job))
    }
}
