use std::{
    collections::BTreeSet,
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};

use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

use crate::{
    cli::SignalArg,
    discovery,
    error::Result,
    model::{FreeAction, FreeAttempt, FreeResult, PortHolder},
};

#[allow(clippy::too_many_arguments)]
pub async fn free_holder(
    holder: &PortHolder,
    signal: SignalArg,
    grace: Duration,
    kill_grace: Duration,
    dry_run: bool,
    confirm: bool,
    tree: bool,
    force_self: bool,
) -> Result<FreeAttempt> {
    let started = Instant::now();
    if let Some(reason) = refusal_reason(holder, force_self) {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Refused,
                FreeResult::Refused,
                false,
                started,
            )
            .with_error("usage_error", reason),
        ));
    }
    if dry_run {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Simulated,
                FreeResult::Skipped,
                true,
                started,
            ),
        ));
    }
    let pids = if tree {
        tree_pids(holder.pid)
    } else {
        BTreeSet::from([holder.pid])
    };
    let targets = match open_signal_targets(&pids) {
        Ok(targets) => targets,
        Err(err) => {
            let result = if err.kind() == io::ErrorKind::PermissionDenied {
                FreeResult::PermissionDenied
            } else {
                FreeResult::Failed
            };
            return Ok(attempt(
                holder,
                AttemptMeta::new(signal, FreeAction::Refused, result, false, started)
                    .with_error(result.as_str(), err.to_string()),
            ));
        }
    };

    if confirm && io::stdout().is_terminal() && !confirmed(holder)? {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Refused,
                FreeResult::Skipped,
                false,
                started,
            )
            .with_error("usage_error", "confirmation declined"),
        ));
    }

    let mut signal_result = Ok(());
    for target in &targets {
        signal_result = target.send(signal);
        if signal_result.is_err() {
            break;
        }
    }
    if let Err(err) = signal_result {
        let result = if err.kind() == io::ErrorKind::PermissionDenied {
            FreeResult::PermissionDenied
        } else {
            FreeResult::Failed
        };
        return Ok(attempt(
            holder,
            AttemptMeta::new(signal, FreeAction::Signaled, result, false, started)
                .with_error(result.as_str(), err.to_string()),
        ));
    }

    if signal == SignalArg::Kill {
        tokio::time::sleep(kill_grace).await;
    } else {
        tokio::time::sleep(grace).await;
    }
    let still_held = holder_still_present(holder)?;
    if !still_held || signal == SignalArg::Kill {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Signaled,
                if still_held {
                    FreeResult::Held
                } else {
                    FreeResult::Freed
                },
                !still_held,
                started,
            ),
        ));
    }

    for target in &targets {
        let _ignored = target.send(SignalArg::Kill);
    }
    tokio::time::sleep(kill_grace).await;
    let held_after_kill = holder_still_present(holder)?;
    Ok(attempt(
        holder,
        AttemptMeta::new(
            signal,
            FreeAction::Signaled,
            if held_after_kill {
                FreeResult::Held
            } else {
                FreeResult::Freed
            },
            !held_after_kill,
            started,
        )
        .escalated(),
    ))
}

fn refusal_reason(holder: &PortHolder, force_self: bool) -> Option<String> {
    if holder.pid == 1 {
        return Some("refusing to kill PID 1".to_owned());
    }
    let current = std::process::id();
    if holder.pid == current {
        return Some("refusing to kill the current process".to_owned());
    }
    match parent_pid() {
        Some(parent) if holder.pid == parent && !force_self => {
            return Some("refusing to kill the parent process without --force-self".to_owned());
        }
        None if !force_self => {
            return Some(
                "refusing to kill because the parent process could not be resolved without --force-self"
                    .to_owned(),
            );
        }
        _ => {}
    }
    None
}

struct AttemptMeta {
    signal: SignalArg,
    action: FreeAction,
    result: FreeResult,
    ok: bool,
    escalated: bool,
    started: Instant,
    error_code: Option<String>,
    message: Option<String>,
}

impl AttemptMeta {
    const fn new(
        signal: SignalArg,
        action: FreeAction,
        result: FreeResult,
        ok: bool,
        started: Instant,
    ) -> Self {
        Self {
            signal,
            action,
            result,
            ok,
            escalated: false,
            started,
            error_code: None,
            message: None,
        }
    }

    fn with_error(mut self, code: impl Into<String>, message: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self.message = Some(message.into());
        self
    }

    const fn escalated(mut self) -> Self {
        self.escalated = true;
        self
    }
}

fn attempt(holder: &PortHolder, meta: AttemptMeta) -> FreeAttempt {
    FreeAttempt {
        port: holder.port,
        pid: holder.pid,
        name: holder.name.clone(),
        signal: meta.signal.as_str().to_owned(),
        action: meta.action,
        result: meta.result,
        ok: meta.ok,
        escalated: meta.escalated,
        ms: meta.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        error_code: meta.error_code,
        message: meta.message,
    }
}

fn confirmed(holder: &PortHolder) -> Result<bool> {
    eprint!(
        "Kill PID {} ({}) holding port {}? [y/N] ",
        holder.pid, holder.name, holder.port
    );
    let _ = io::stderr().flush();
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|source| crate::error::PortError::Command {
            command: "stdin",
            source,
        })?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}

fn holder_still_present(holder: &PortHolder) -> Result<bool> {
    let filters = crate::cli::FilterArgs {
        proto: match holder.proto {
            crate::model::Protocol::Tcp => crate::cli::ProtocolArg::Tcp,
            crate::model::Protocol::Udp => crate::cli::ProtocolArg::Udp,
        },
        include_loopback: true,
        listening_only: true,
        host: None,
        owner: None,
        pid: Some(holder.pid),
    };
    let (_sockets, holders) = discovery::inspect(&filters, &[holder.port])?;
    Ok(!holders.is_empty())
}

fn open_signal_targets(pids: &BTreeSet<u32>) -> io::Result<Vec<SignalTarget>> {
    pids.iter().map(|pid| SignalTarget::open(*pid)).collect()
}

struct SignalTarget {
    pid: u32,
    platform: PlatformSignalTarget,
}

impl SignalTarget {
    fn open(pid: u32) -> io::Result<Self> {
        Ok(Self {
            pid,
            platform: PlatformSignalTarget::open(pid)?,
        })
    }

    fn send(&self, signal: SignalArg) -> io::Result<()> {
        self.platform.send(self.pid, signal)
    }
}

#[cfg(target_os = "linux")]
enum PlatformSignalTarget {
    PidFd(std::os::fd::OwnedFd),
    Pid,
}

#[cfg(target_os = "linux")]
impl PlatformSignalTarget {
    fn open(pid: u32) -> io::Result<Self> {
        use std::os::fd::{FromRawFd, OwnedFd};

        let raw_pid = i32::try_from(pid)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid out of range"))?;
        // SAFETY: `pidfd_open` is called with a numeric PID and flags=0. A
        // non-negative return value is a new owned file descriptor.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, raw_pid, 0) };
        if fd >= 0 {
            let fd_i32 =
                i32::try_from(fd).map_err(|_| io::Error::other("pidfd value out of range"))?;
            // SAFETY: `fd_i32` is the live descriptor returned by pidfd_open
            // above and ownership is transferred to `OwnedFd`.
            return Ok(Self::PidFd(unsafe { OwnedFd::from_raw_fd(fd_i32) }));
        }
        let err = io::Error::last_os_error();
        if matches!(err.raw_os_error(), Some(libc::ENOSYS | libc::EINVAL)) {
            return Ok(Self::Pid);
        }
        Err(err)
    }

    fn send(&self, pid: u32, signal: SignalArg) -> io::Result<()> {
        use std::os::fd::AsRawFd;

        match self {
            Self::PidFd(fd) => {
                let sig = signal_number(signal);
                // SAFETY: `fd` is a live pidfd owned by this target. Null
                // siginfo and flags=0 match the pidfd_send_signal contract.
                let result = unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        fd.as_raw_fd(),
                        sig,
                        std::ptr::null::<libc::siginfo_t>(),
                        0,
                    )
                };
                if result == 0 {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            }
            Self::Pid => send_signal_pid(pid, signal),
        }
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
struct PlatformSignalTarget;

#[cfg(all(unix, not(target_os = "linux")))]
impl PlatformSignalTarget {
    fn open(_pid: u32) -> io::Result<Self> {
        Ok(Self)
    }

    fn send(&self, pid: u32, signal: SignalArg) -> io::Result<()> {
        send_signal_pid(pid, signal)
    }
}

#[cfg(target_os = "linux")]
fn signal_number(signal: SignalArg) -> i32 {
    match signal {
        SignalArg::Term => libc::SIGTERM,
        SignalArg::Kill => libc::SIGKILL,
        SignalArg::Int => libc::SIGINT,
    }
}

#[cfg(unix)]
fn send_signal_pid(pid: u32, signal: SignalArg) -> io::Result<()> {
    use nix::{
        sys::signal::{kill, Signal},
        unistd::Pid,
    };
    let raw = match i32::try_from(pid) {
        Ok(value) => value,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "pid out of range",
            ))
        }
    };
    let sig = match signal {
        SignalArg::Term => Signal::SIGTERM,
        SignalArg::Kill => Signal::SIGKILL,
        SignalArg::Int => Signal::SIGINT,
    };
    kill(Pid::from_raw(raw), sig).map_err(|errno| io::Error::from_raw_os_error(errno as i32))
}

#[cfg(windows)]
struct PlatformSignalTarget(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl Drop for PlatformSignalTarget {
    fn drop(&mut self) {
        // SAFETY: the handle is returned by OpenProcess and owned by this type.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
impl PlatformSignalTarget {
    fn open(pid: u32) -> io::Result<Self> {
        use windows_sys::Win32::{
            Foundation::FALSE,
            System::Threading::{OpenProcess, PROCESS_TERMINATE},
        };

        // SAFETY: We request only PROCESS_TERMINATE, pass no inheritable handle,
        // and check for a null return value before storing the handle.
        let handle = unsafe { OpenProcess(PROCESS_TERMINATE, FALSE, pid) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(handle))
    }

    fn send(&self, _pid: u32, signal: SignalArg) -> io::Result<()> {
        use windows_sys::Win32::System::Threading::TerminateProcess;

        let exit_code: u32 = match signal {
            SignalArg::Kill | SignalArg::Term => 1,
            SignalArg::Int => 0xC000_013A,
        };

        // SAFETY: the stored handle is live for the original process object and
        // was opened with PROCESS_TERMINATE.
        let result = unsafe { TerminateProcess(self.0, exit_code) };
        if result == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

fn tree_pids(root: u32) -> BTreeSet<u32> {
    tree_pids_from(root, &process_parents())
}

fn tree_pids_from(root: u32, parents: &[(u32, u32)]) -> BTreeSet<u32> {
    let mut pids = BTreeSet::from([root]);
    let mut changed = true;
    while changed {
        changed = false;
        for (pid, parent) in parents {
            if pids.contains(parent) && pids.insert(*pid) {
                changed = true;
            }
        }
    }
    pids
}

fn parent_pid() -> Option<u32> {
    let current = std::process::id();
    process_parents()
        .into_iter()
        .find_map(|(pid, parent)| (pid == current).then_some(parent))
}

fn process_parents() -> Vec<(u32, u32)> {
    let mut system =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::new(),
    );
    system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let parent = process.parent()?;
            Some((Pid::as_u32(*pid), parent.as_u32()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::tree_pids_from;

    #[test]
    fn tree_pids_include_nested_descendants() {
        let parents = vec![(20, 10), (30, 20), (40, 30), (50, 99)];
        let pids = tree_pids_from(10, &parents);
        assert!(pids.contains(&10));
        assert!(pids.contains(&20));
        assert!(pids.contains(&30));
        assert!(pids.contains(&40));
        assert!(!pids.contains(&50));
    }
}
