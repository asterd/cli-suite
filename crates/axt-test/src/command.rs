use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{ExitStatus, Stdio},
    thread,
    time::Instant,
};

use axt_core::CommandContext;
use axt_output::{JsonlWriter, RenderContext};
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

use crate::{
    cli::{Args, Command},
    discovery::{self, Project},
    error::{Result, TestError},
    frontend::{parse_output, parse_stdout_line, Frontend, TestFrontend, TestOptions},
    model::{NormalizedEvent, TestCase, TestData, TestStatus, TestSuite, TestSummary},
};

pub fn run(args: &Args, ctx: &CommandContext) -> Result<TestOutput> {
    match args.command {
        Some(Command::ListFrameworks) => Ok(TestOutput::Frameworks {
            frameworks: discovery::framework_rows()
                .into_iter()
                .map(|(name, marker, detection)| FrameworkInfo {
                    name: name.to_owned(),
                    marker: marker.to_owned(),
                    detection: detection.to_owned(),
                })
                .collect(),
        }),
        None => run_tests(args, ctx).map(|data| TestOutput::Run {
            data,
            top_failures: args.run.top_failures,
            include_output: args.run.include_output,
        }),
    }
}

fn run_tests(args: &Args, ctx: &CommandContext) -> Result<TestData> {
    let started = Instant::now();
    let started_text = ctx
        .clock
        .now_utc()
        .format(&Rfc3339)
        .map_err(|err| TestError::Io(err.to_string()))?;
    let projects = discovery::detect_projects(&ctx.cwd, args.run.framework);
    if projects.is_empty() {
        return Err(TestError::NoFramework);
    }
    if args.run.single && projects.len() > 1 {
        return Err(TestError::MultipleFrameworks);
    }

    let changed_files = changed_files(args, &ctx.cwd)?;
    let mut suites = Vec::new();
    let mut cases = Vec::new();
    let mut frameworks = BTreeSet::new();
    for project in projects {
        let files = selected_files(&project, &args.run.files, &changed_files);
        if (args.run.changed || args.run.changed_since.is_some()) && files.is_empty() {
            continue;
        }
        frameworks.insert(project.framework.as_str().to_owned());
        let frontend = Frontend::new(project.framework);
        let executable = resolve_tool(frontend.command_name())?;
        let opts = TestOptions::from_run(project.root.clone(), &args.run, files);
        let (status, events) = invoke_frontend(frontend, &executable, &opts)?;
        for event in events {
            match event {
                NormalizedEvent::Suite(suite) => suites.push(prefix_suite(&ctx.cwd, suite)),
                NormalizedEvent::Case(case) => cases.push(prefix_case(&ctx.cwd, case)),
                NormalizedEvent::Summary(_) => {}
            }
        }
        if !status.success() && !cases.iter().any(|case| case.status == TestStatus::Failed) {
            cases.push(TestCase {
                framework: frontend.name().to_owned(),
                status: TestStatus::Failed,
                name: "framework command failed".to_owned(),
                suite: None,
                file: None,
                line: None,
                duration_ms: 0,
                failure: Some(crate::model::TestFailure {
                    message: format!(
                        "{} exited with status {}",
                        frontend.command_name(),
                        status.code().unwrap_or(1)
                    ),
                    stack: None,
                    actual: None,
                    expected: None,
                    diff: None,
                }),
                stdout: None,
                stderr: None,
            });
        }
    }

    if suites.is_empty() {
        suites = suites_from_cases(&cases);
    }
    let (passed, failed, skipped, todo) = counts(&cases);
    let total = cases.len();
    Ok(TestData {
        frameworks: frameworks.into_iter().collect(),
        suites,
        cases,
        total,
        passed,
        failed,
        skipped,
        todo,
        duration_ms: elapsed_ms(started),
        started: started_text,
        truncated: false,
    })
}

pub fn run_jsonl_streaming(
    args: &Args,
    ctx: &CommandContext,
    w: &mut dyn Write,
    render_ctx: &RenderContext<'_>,
) -> Result<bool> {
    let started = Instant::now();
    let started_text = ctx
        .clock
        .now_utc()
        .format(&Rfc3339)
        .map_err(|err| TestError::Io(err.to_string()))?;
    let projects = discovery::detect_projects(&ctx.cwd, args.run.framework);
    if projects.is_empty() {
        return Err(TestError::NoFramework);
    }
    if args.run.single && projects.len() > 1 {
        return Err(TestError::MultipleFrameworks);
    }

    let changed_files = changed_files(args, &ctx.cwd)?;
    let mut writer = JsonlWriter::new(w, render_ctx.limits);
    let mut suites = Vec::new();
    let mut cases = Vec::new();
    let mut frameworks = BTreeSet::new();
    let mut failures_left = args.run.top_failures;

    for project in projects {
        let files = selected_files(&project, &args.run.files, &changed_files);
        if (args.run.changed || args.run.changed_since.is_some()) && files.is_empty() {
            continue;
        }
        frameworks.insert(project.framework.as_str().to_owned());
        let frontend = Frontend::new(project.framework);
        let executable = resolve_tool(frontend.command_name())?;
        let opts = TestOptions::from_run(project.root.clone(), &args.run, files);
        let status = invoke_frontend_streaming(
            frontend,
            &executable,
            &opts,
            ctx,
            &mut writer,
            args.run.include_output,
            &mut failures_left,
            &mut suites,
            &mut cases,
        )?;
        let project_failed = cases
            .iter()
            .any(|case| case.framework == frontend.name() && case.status == TestStatus::Failed);
        if !status.success() && !project_failed {
            let case = TestCase {
                framework: frontend.name().to_owned(),
                status: TestStatus::Failed,
                name: "framework command failed".to_owned(),
                suite: None,
                file: None,
                line: None,
                duration_ms: 0,
                failure: Some(crate::model::TestFailure {
                    message: format!(
                        "{} exited with status {}",
                        frontend.command_name(),
                        status.code().unwrap_or(1)
                    ),
                    stack: None,
                    actual: None,
                    expected: None,
                    diff: None,
                }),
                stdout: None,
                stderr: None,
            };
            write_case(
                &mut writer,
                &case,
                args.run.include_output,
                &mut failures_left,
            )?;
            cases.push(case);
        }
    }

    if suites.is_empty() {
        suites = suites_from_cases(&cases);
    }
    for suite in &suites {
        writer.write_record(&suite_record(suite))?;
    }
    let (passed, failed, skipped, todo) = counts(&cases);
    let total = cases.len();
    let data = TestData {
        frameworks: frameworks.into_iter().collect(),
        suites,
        cases,
        total,
        passed,
        failed,
        skipped,
        todo,
        duration_ms: elapsed_ms(started),
        started: started_text,
        truncated: false,
    };
    writer.write_record(&summary_record(&data))?;
    let _summary = writer.finish("axt.test.warn.v1")?;
    Ok(data.ok())
}

fn invoke_frontend(
    frontend: Frontend,
    executable: &Path,
    opts: &TestOptions,
) -> Result<(ExitStatus, Vec<NormalizedEvent>)> {
    let mut child = frontend
        .build_command(executable, opts)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| TestError::Command {
            command: frontend.command_name().to_owned(),
            source,
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TestError::Io("framework stdout pipe missing".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TestError::Io("framework stderr pipe missing".to_owned()))?;
    let mut stdout = BufReader::new(stdout);
    let mut stderr = BufReader::new(stderr);
    let events = frontend.parse_reader(&mut stdout, &mut stderr);
    let status = child.wait().map_err(|source| TestError::Command {
        command: frontend.command_name().to_owned(),
        source,
    })?;
    Ok((status, events))
}

#[allow(clippy::too_many_arguments)]
fn invoke_frontend_streaming<W: Write + ?Sized>(
    frontend: Frontend,
    executable: &Path,
    opts: &TestOptions,
    ctx: &CommandContext,
    writer: &mut JsonlWriter<'_, W>,
    include_output: bool,
    failures_left: &mut usize,
    suites: &mut Vec<TestSuite>,
    cases: &mut Vec<TestCase>,
) -> Result<ExitStatus> {
    let mut child = frontend
        .build_command(executable, opts)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| TestError::Command {
            command: frontend.command_name().to_owned(),
            source,
        })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TestError::Io("framework stdout pipe missing".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TestError::Io("framework stderr pipe missing".to_owned()))?;
    let stderr_handle = thread::spawn(move || {
        let mut text = String::new();
        let mut reader = BufReader::new(stderr);
        reader.read_to_string(&mut text).map(|_bytes| text)
    });

    let mut stdout_text = String::new();
    let mut saw_events = false;
    let mut reader = BufReader::new(stdout);
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|err| TestError::Io(err.to_string()))?;
        if bytes == 0 {
            break;
        }
        stdout_text.push_str(&line);
        for event in parse_stdout_line(frontend.framework(), &line) {
            saw_events = true;
            handle_streamed_event(
                event,
                ctx,
                writer,
                include_output,
                failures_left,
                suites,
                cases,
            )?;
        }
    }

    let status = child.wait().map_err(|source| TestError::Command {
        command: frontend.command_name().to_owned(),
        source,
    })?;
    let stderr_text = stderr_handle
        .join()
        .map_err(|_err| TestError::Io("framework stderr reader panicked".to_owned()))?
        .map_err(|err| TestError::Io(err.to_string()))?;

    if !saw_events {
        for event in parse_output(frontend.framework(), &stdout_text, &stderr_text) {
            handle_streamed_event(
                event,
                ctx,
                writer,
                include_output,
                failures_left,
                suites,
                cases,
            )?;
        }
    }
    Ok(status)
}

fn handle_streamed_event<W: Write + ?Sized>(
    event: NormalizedEvent,
    ctx: &CommandContext,
    writer: &mut JsonlWriter<'_, W>,
    include_output: bool,
    failures_left: &mut usize,
    suites: &mut Vec<TestSuite>,
    cases: &mut Vec<TestCase>,
) -> Result<()> {
    match event {
        NormalizedEvent::Suite(suite) => {
            suites.push(prefix_suite(&ctx.cwd, suite));
        }
        NormalizedEvent::Case(case) => {
            let case = prefix_case(&ctx.cwd, case);
            write_case(writer, &case, include_output, failures_left)?;
            cases.push(case);
        }
        NormalizedEvent::Summary(_) => {}
    }
    Ok(())
}

fn write_case<W: Write + ?Sized>(
    writer: &mut JsonlWriter<'_, W>,
    case: &TestCase,
    include_output: bool,
    failures_left: &mut usize,
) -> Result<()> {
    if case.status == TestStatus::Failed {
        if *failures_left == 0 {
            return Ok(());
        }
        *failures_left -= 1;
    }
    writer.write_record(&case_record(case, include_output))?;
    writer.flush()?;
    Ok(())
}

fn resolve_tool(command: &str) -> Result<std::path::PathBuf> {
    which::which(command)
        .map_err(|_err| TestError::MissingTool {
            command: command.to_owned(),
        })
}

fn changed_files(args: &Args, cwd: &Utf8Path) -> Result<Vec<Utf8PathBuf>> {
    if let Some(reference) = &args.run.changed_since {
        let repo = axt_git::repo_root_for(cwd)
            .map_err(|err| TestError::Git(err.to_string()))?
            .ok_or(TestError::GitUnavailable)?;
        return axt_git::diff_paths(&repo, reference, "HEAD")
            .map(|paths| absolute_changed_paths(repo.root(), paths))
            .map_err(|err| TestError::Git(err.to_string()));
    }
    if args.run.changed {
        let repo = axt_git::repo_root_for(cwd)
            .map_err(|err| TestError::Git(err.to_string()))?
            .ok_or(TestError::GitUnavailable)?;
        return axt_git::StatusCache::from_repo(&repo)
            .map(|cache| absolute_changed_paths(repo.root(), cache.changed_paths()))
            .map_err(|err| TestError::Git(err.to_string()));
    }
    Ok(Vec::new())
}

fn absolute_changed_paths(repo_root: &Utf8Path, paths: Vec<Utf8PathBuf>) -> Vec<Utf8PathBuf> {
    paths
        .into_iter()
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                repo_root.join(path)
            }
        })
        .collect()
}

fn selected_files(
    project: &Project,
    explicit: &[Utf8PathBuf],
    changed: &[Utf8PathBuf],
) -> Vec<Utf8PathBuf> {
    if !explicit.is_empty() {
        return explicit.to_vec();
    }
    changed
        .iter()
        .filter_map(|path| {
            let absolute = if path.is_absolute() {
                path.clone()
            } else {
                project.root.join(path)
            };
            absolute
                .strip_prefix(&project.root)
                .ok()
                .map(Utf8Path::to_path_buf)
        })
        .filter(|path| looks_like_test_file(path))
        .collect()
}

fn looks_like_test_file(path: &Utf8Path) -> bool {
    let text = path.as_str();
    text.contains("test") || text.contains("spec") || text.ends_with("_test.go")
}

fn prefix_suite(cwd: &Utf8Path, mut suite: TestSuite) -> TestSuite {
    suite.file = suite.file.map(|path| repo_relative(cwd, path));
    suite
}

fn prefix_case(cwd: &Utf8Path, mut case: TestCase) -> TestCase {
    case.file = case.file.map(|path| repo_relative(cwd, path));
    case
}

fn repo_relative(cwd: &Utf8Path, path: Utf8PathBuf) -> Utf8PathBuf {
    if path.is_absolute() {
        path.strip_prefix(cwd)
            .map_or_else(|_err| path.clone(), Utf8Path::to_path_buf)
    } else {
        path
    }
}

fn suites_from_cases(cases: &[TestCase]) -> Vec<TestSuite> {
    let mut suites = Vec::new();
    for case in cases {
        let name = case
            .suite
            .clone()
            .or_else(|| case.file.as_ref().map(ToString::to_string))
            .unwrap_or_else(|| case.framework.clone());
        if let Some(suite) = suites
            .iter_mut()
            .find(|suite: &&mut TestSuite| suite.framework == case.framework && suite.name == name)
        {
            add_case_to_suite(suite, case);
        } else {
            let mut suite = TestSuite {
                framework: case.framework.clone(),
                name,
                file: case.file.clone(),
                passed: 0,
                failed: 0,
                skipped: 0,
                todo: 0,
                duration_ms: 0,
            };
            add_case_to_suite(&mut suite, case);
            suites.push(suite);
        }
    }
    suites
}

fn add_case_to_suite(suite: &mut TestSuite, case: &TestCase) {
    suite.duration_ms = suite.duration_ms.saturating_add(case.duration_ms);
    match case.status {
        TestStatus::Passed => suite.passed += 1,
        TestStatus::Failed => suite.failed += 1,
        TestStatus::Skipped => suite.skipped += 1,
        TestStatus::Todo => suite.todo += 1,
    }
}

fn counts(cases: &[TestCase]) -> (usize, usize, usize, usize) {
    cases.iter().fold((0, 0, 0, 0), |mut acc, case| {
        match case.status {
            TestStatus::Passed => acc.0 += 1,
            TestStatus::Failed => acc.1 += 1,
            TestStatus::Skipped => acc.2 += 1,
            TestStatus::Todo => acc.3 += 1,
        }
        acc
    })
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn summary_record(data: &TestData) -> Value {
    let summary = TestSummary::from(data);
    json!({
        "schema": "axt.test.summary.v1",
        "type": "summary",
        "frameworks": summary.frameworks,
        "total": summary.total,
        "passed": summary.passed,
        "failed": summary.failed,
        "skipped": summary.skipped,
        "todo": summary.todo,
        "duration_ms": summary.duration_ms,
        "started": summary.started,
        "truncated": summary.truncated
    })
}

fn suite_record(suite: &TestSuite) -> Value {
    json!({
        "schema": "axt.test.suite.v1",
        "type": "suite",
        "framework": suite.framework,
        "name": suite.name,
        "file": suite.file,
        "passed": suite.passed,
        "failed": suite.failed,
        "skipped": suite.skipped,
        "todo": suite.todo,
        "duration_ms": suite.duration_ms
    })
}

fn case_record(case: &TestCase, include_output: bool) -> Value {
    json!({
        "schema": "axt.test.case.v1",
        "type": "case",
        "framework": case.framework,
        "status": case.status,
        "name": case.name,
        "suite": case.suite,
        "file": case.file,
        "line": case.line,
        "duration_ms": case.duration_ms,
        "failure": case.failure,
        "stdout": if include_output || case.status == TestStatus::Failed { case.stdout.clone() } else { None },
        "stderr": if include_output || case.status == TestStatus::Failed { case.stderr.clone() } else { None }
    })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TestOutput {
    Run {
        data: TestData,
        top_failures: usize,
        include_output: bool,
    },
    Frameworks {
        frameworks: Vec<FrameworkInfo>,
    },
}

impl TestOutput {
    #[must_use]
    pub const fn ok(&self) -> bool {
        match self {
            Self::Run { data, .. } => data.ok(),
            Self::Frameworks { .. } => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FrameworkInfo {
    pub name: String,
    pub marker: String,
    pub detection: String,
}
