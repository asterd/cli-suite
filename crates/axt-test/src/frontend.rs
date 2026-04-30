use std::{path::Path, process::Command};

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

use crate::{
    cli::{FrameworkArg, RunArgs},
    model::{NormalizedEvent, TestCase, TestFailure, TestStatus, TestSuite},
};

#[derive(Debug, Clone)]
pub struct TestOptions {
    pub root: Utf8PathBuf,
    pub filter: Option<String>,
    pub files: Vec<Utf8PathBuf>,
    pub bail: bool,
    pub workers: Option<usize>,
    pub pass_through: bool,
    pub framework_flags: Vec<String>,
}

impl TestOptions {
    #[must_use]
    pub fn from_run(root: Utf8PathBuf, run: &RunArgs, files: Vec<Utf8PathBuf>) -> Self {
        Self {
            root,
            filter: run.filter.clone(),
            files,
            bail: run.bail,
            workers: run.workers,
            pass_through: run.pass_through,
            framework_flags: run.framework_flags.clone(),
        }
    }
}

pub trait TestFrontend {
    fn name(&self) -> &'static str;
    fn command_name(&self) -> &'static str;
    fn build_command(&self, executable: &Path, opts: &TestOptions) -> Command;
}

#[derive(Debug, Clone, Copy)]
pub struct Frontend {
    framework: FrameworkArg,
}

impl Frontend {
    #[must_use]
    pub const fn new(framework: FrameworkArg) -> Self {
        Self { framework }
    }

    #[must_use]
    pub const fn framework(self) -> FrameworkArg {
        self.framework
    }
}

impl TestFrontend for Frontend {
    fn name(&self) -> &'static str {
        self.framework.as_str()
    }

    fn command_name(&self) -> &'static str {
        match self.framework {
            FrameworkArg::Jest | FrameworkArg::Vitest => "npm",
            FrameworkArg::Pytest => "python",
            FrameworkArg::Cargo => "cargo",
            FrameworkArg::Go => "go",
            FrameworkArg::Bun => "bun",
            FrameworkArg::Deno => "deno",
        }
    }

    fn build_command(&self, executable: &Path, opts: &TestOptions) -> Command {
        let mut command = Command::new(executable);
        command.current_dir(opts.root.as_std_path());
        match self.framework {
            FrameworkArg::Jest | FrameworkArg::Vitest => {
                command.arg("test").arg("--");
                if let Some(filter) = &opts.filter {
                    command.arg(filter);
                }
                if opts.bail {
                    command.arg("--bail");
                }
                if let Some(workers) = opts.workers {
                    command.arg("--maxWorkers").arg(workers.to_string());
                }
            }
            FrameworkArg::Pytest => {
                command.arg("-m").arg("pytest").arg("-q");
                if let Some(filter) = &opts.filter {
                    command.arg("-k").arg(filter);
                }
                if opts.bail {
                    command.arg("-x");
                }
            }
            FrameworkArg::Cargo => {
                command.arg("test");
                if let Some(filter) = &opts.filter {
                    command.arg(filter);
                }
                command.arg("--").arg("--nocapture");
            }
            FrameworkArg::Go => {
                command.arg("test").arg("-json");
                if let Some(filter) = &opts.filter {
                    command.arg("-run").arg(filter);
                }
                command.arg("./...");
            }
            FrameworkArg::Bun => {
                command.arg("test");
                if let Some(filter) = &opts.filter {
                    command.arg("--test-name-pattern").arg(filter);
                }
                if opts.bail {
                    command.arg("--bail");
                }
            }
            FrameworkArg::Deno => {
                command.arg("test").arg("--reporter=json");
                if let Some(filter) = &opts.filter {
                    command.arg("--filter").arg(filter);
                }
            }
        }
        for file in &opts.files {
            command.arg(file.as_str());
        }
        if opts.pass_through {
            command.args(&opts.framework_flags);
        }
        command
    }

}

pub fn parse_output(framework: FrameworkArg, stdout: &str, stderr: &str) -> Vec<NormalizedEvent> {
    let mut events = Vec::new();
    let parsed_document = serde_json::from_str::<Value>(stdout).is_ok_and(|value| {
        parse_json_document(framework, &value, &mut events);
        true
    });
    if !parsed_document {
        for line in stdout.lines() {
            parse_json_line(framework, line, &mut events);
            if framework == FrameworkArg::Go {
                parse_go_line(line, &mut events);
            }
            if framework == FrameworkArg::Cargo {
                parse_cargo_text_line(line, &mut events);
            }
        }
    }
    if events.is_empty() && !stderr.trim().is_empty() {
        events.push(NormalizedEvent::Case(TestCase {
            framework: framework.as_str().to_owned(),
            status: TestStatus::Failed,
            name: "framework stderr".to_owned(),
            suite: None,
            file: None,
            line: None,
            duration_ms: 0,
            failure: Some(failure(first_line(stderr))),
            stdout: None,
            stderr: Some(stderr.to_owned()),
        }));
    }
    events
}

pub fn parse_stdout_line(framework: FrameworkArg, line: &str) -> Vec<NormalizedEvent> {
    let mut events = Vec::new();
    parse_json_line(framework, line, &mut events);
    if framework == FrameworkArg::Go {
        parse_go_line(line, &mut events);
    }
    if framework == FrameworkArg::Cargo {
        parse_cargo_text_line(line, &mut events);
    }
    events
}

fn parse_json_document(framework: FrameworkArg, value: &Value, events: &mut Vec<NormalizedEvent>) {
    if let Some(items) = value.as_array() {
        for item in items {
            parse_json_document(framework, item, events);
        }
        return;
    }
    let mut emitted_children = false;
    if let Some(results) = value.get("testResults").and_then(Value::as_array) {
        emitted_children = true;
        for suite in results {
            parse_jest_suite(framework, suite, events);
        }
    }
    if let Some(suites) = value.get("suites").and_then(Value::as_array) {
        emitted_children = true;
        for suite in suites {
            parse_generic_suite(framework, suite, events);
        }
    }
    if let Some(tests) = value.get("tests").and_then(Value::as_array) {
        emitted_children = true;
        for test in tests {
            events.push(NormalizedEvent::Case(case_from_json(framework, test)));
        }
    }
    if let Some(events_json) = value.get("events").and_then(Value::as_array) {
        emitted_children = true;
        for event in events_json {
            parse_json_document(framework, event, events);
        }
    }
    if !emitted_children && looks_like_case(value) {
        events.push(NormalizedEvent::Case(case_from_json(framework, value)));
    }
}

fn parse_jest_suite(framework: FrameworkArg, value: &Value, events: &mut Vec<NormalizedEvent>) {
    let file = value.get("name").and_then(Value::as_str).map(relative_path);
    let suite_name = file
        .as_ref()
        .map_or_else(|| framework.as_str().to_owned(), ToString::to_string);
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut todo = 0;
    if let Some(assertions) = value.get("assertionResults").and_then(Value::as_array) {
        for assertion in assertions {
            let mut case = case_from_json(framework, assertion);
            case.file = case.file.or_else(|| file.clone());
            case.suite = case.suite.or_else(|| Some(suite_name.clone()));
            count_status(
                case.status,
                &mut passed,
                &mut failed,
                &mut skipped,
                &mut todo,
            );
            events.push(NormalizedEvent::Case(case));
        }
    }
    events.push(NormalizedEvent::Suite(TestSuite {
        framework: framework.as_str().to_owned(),
        name: suite_name,
        file,
        passed,
        failed,
        skipped,
        todo,
        duration_ms: value
            .get("perfStats")
            .and_then(|perf| perf.get("runtime"))
            .and_then(Value::as_u64)
            .unwrap_or(0),
    }));
}

fn parse_json_line(framework: FrameworkArg, line: &str, events: &mut Vec<NormalizedEvent>) {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return;
    };
    if value.get("schema").and_then(Value::as_str) == Some("axt.test.fixture.v1")
        || value.get("type").and_then(Value::as_str) == Some("case")
        || (framework != FrameworkArg::Go && looks_like_case(&value))
    {
        events.push(NormalizedEvent::Case(case_from_json(framework, &value)));
    }
}

fn parse_generic_suite(framework: FrameworkArg, value: &Value, events: &mut Vec<NormalizedEvent>) {
    let suite_name = value
        .get("name")
        .or_else(|| value.get("title"))
        .and_then(Value::as_str)
        .unwrap_or_else(|| framework.as_str())
        .to_owned();
    let file = value.get("file").and_then(Value::as_str).map(relative_path);
    let before = events.len();

    for key in ["tests", "cases", "specs"] {
        if let Some(tests) = value.get(key).and_then(Value::as_array) {
            for test in tests {
                let mut case = case_from_json(framework, test);
                case.suite = case.suite.or_else(|| Some(suite_name.clone()));
                case.file = case.file.or_else(|| file.clone());
                events.push(NormalizedEvent::Case(case));
            }
        }
    }
    if let Some(children) = value.get("suites").and_then(Value::as_array) {
        for child in children {
            parse_generic_suite(framework, child, events);
        }
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut todo = 0;
    for event in &events[before..] {
        if let NormalizedEvent::Case(case) = event {
            count_status(
                case.status,
                &mut passed,
                &mut failed,
                &mut skipped,
                &mut todo,
            );
        }
    }
    if passed + failed + skipped + todo > 0 {
        events.push(NormalizedEvent::Suite(TestSuite {
            framework: framework.as_str().to_owned(),
            name: suite_name,
            file,
            passed,
            failed,
            skipped,
            todo,
            duration_ms: duration_ms_from_value(value),
        }));
    }
}

fn parse_go_line(line: &str, events: &mut Vec<NormalizedEvent>) {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return;
    };
    let Some(test) = value.get("Test").and_then(Value::as_str) else {
        return;
    };
    let action = value.get("Action").and_then(Value::as_str).unwrap_or("");
    let status = match action {
        "pass" => TestStatus::Passed,
        "fail" => TestStatus::Failed,
        "skip" => TestStatus::Skipped,
        _ => return,
    };
    let package = value.get("Package").and_then(Value::as_str);
    events.push(NormalizedEvent::Case(TestCase {
        framework: "go".to_owned(),
        status,
        name: test.to_owned(),
        suite: package.map(ToOwned::to_owned),
        file: None,
        line: None,
        duration_ms: seconds_to_ms(value.get("Elapsed").and_then(Value::as_f64)),
        failure: (status == TestStatus::Failed).then(|| failure(test)),
        stdout: None,
        stderr: None,
    }));
}

fn parse_cargo_text_line(line: &str, events: &mut Vec<NormalizedEvent>) {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix("test ") else {
        return;
    };
    let Some((name, status_text)) = rest.rsplit_once(" ... ") else {
        return;
    };
    let status = match status_text {
        "ok" => TestStatus::Passed,
        "FAILED" => TestStatus::Failed,
        "ignored" => TestStatus::Skipped,
        _ => return,
    };
    events.push(NormalizedEvent::Case(TestCase {
        framework: "cargo".to_owned(),
        status,
        name: name.to_owned(),
        suite: None,
        file: None,
        line: None,
        duration_ms: 0,
        failure: (status == TestStatus::Failed).then(|| failure(name)),
        stdout: None,
        stderr: None,
    }));
}

fn case_from_json(framework: FrameworkArg, value: &Value) -> TestCase {
    let status = status_from_value(value);
    let message = value
        .get("message")
        .or_else(|| {
            value
                .get("failureMessages")
                .and_then(|messages| messages.get(0))
        })
        .and_then(Value::as_str)
        .map(first_line);
    TestCase {
        framework: framework.as_str().to_owned(),
        status,
        name: value
            .get("fullName")
            .or_else(|| value.get("title"))
            .or_else(|| value.get("name"))
            .or_else(|| value.get("nodeid"))
            .or_else(|| value.get("test"))
            .or_else(|| value.get("Test"))
            .and_then(Value::as_str)
            .unwrap_or("unnamed test")
            .to_owned(),
        suite: value
            .get("suite")
            .or_else(|| value.get("ancestorTitles").and_then(|items| items.get(0)))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        file: value
            .get("file")
            .or_else(|| value.get("name"))
            .and_then(Value::as_str)
            .map(relative_path),
        line: value
            .get("line")
            .or_else(|| {
                value
                    .get("location")
                    .and_then(|location| location.get("line"))
            })
            .and_then(Value::as_u64),
        duration_ms: value
            .get("duration_ms")
            .or_else(|| value.get("durationMs"))
            .or_else(|| value.get("duration"))
            .and_then(Value::as_u64)
            .unwrap_or_else(|| duration_ms_from_value(value)),
        failure: (status == TestStatus::Failed).then(|| TestFailure {
            message: message.unwrap_or_else(|| "test failed".to_owned()),
            stack: value
                .get("stack")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            actual: value.get("actual").and_then(value_to_lossless_string),
            expected: value.get("expected").and_then(value_to_lossless_string),
            diff: value
                .get("diff")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        }),
        stdout: value
            .get("stdout")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        stderr: value
            .get("stderr")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
    }
}

fn looks_like_case(value: &Value) -> bool {
    let has_status = value.get("status").is_some()
        || value.get("outcome").is_some()
        || value.get("Action").is_some();
    let has_name = value.get("fullName").is_some()
        || value.get("title").is_some()
        || value.get("name").is_some()
        || value.get("nodeid").is_some()
        || value.get("test").is_some()
        || value.get("Test").is_some();
    has_status && has_name
}

fn duration_ms_from_value(value: &Value) -> u64 {
    value
        .get("duration_ms")
        .or_else(|| value.get("durationMs"))
        .or_else(|| value.get("duration"))
        .and_then(Value::as_u64)
        .unwrap_or_else(|| {
            seconds_to_ms(
                value
                    .get("duration_seconds")
                    .or_else(|| value.get("duration"))
                    .or_else(|| value.get("elapsed"))
                    .or_else(|| value.get("Elapsed"))
                    .and_then(Value::as_f64),
            )
        })
}

fn status_from_value(value: &Value) -> TestStatus {
    let raw = value
        .get("status")
        .or_else(|| value.get("outcome"))
        .and_then(Value::as_str)
        .unwrap_or("failed");
    match raw {
        "passed" | "pass" | "ok" | "success" => TestStatus::Passed,
        "skipped" | "skip" | "ignored" => TestStatus::Skipped,
        "todo" | "pending" => TestStatus::Todo,
        _ => TestStatus::Failed,
    }
}

fn count_status(
    status: TestStatus,
    passed: &mut usize,
    failed: &mut usize,
    skipped: &mut usize,
    todo: &mut usize,
) {
    match status {
        TestStatus::Passed => *passed += 1,
        TestStatus::Failed => *failed += 1,
        TestStatus::Skipped => *skipped += 1,
        TestStatus::Todo => *todo += 1,
    }
}

fn relative_path(path: &str) -> Utf8PathBuf {
    let candidate = Utf8Path::new(path);
    candidate
        .file_name()
        .map_or_else(|| Utf8PathBuf::from(path), Utf8PathBuf::from)
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or(text).trim().to_owned()
}

fn failure(message: impl Into<String>) -> TestFailure {
    TestFailure {
        message: first_line(&message.into()),
        stack: None,
        actual: None,
        expected: None,
        diff: None,
    }
}

fn seconds_to_ms(value: Option<f64>) -> u64 {
    const MAX_SAFE_SECONDS: f64 = 18_446_744_073_709_551.0;

    value.map_or(0, |seconds| {
        if seconds.is_sign_negative() || !seconds.is_finite() {
            0
        } else if seconds > MAX_SAFE_SECONDS {
            u64::MAX
        } else {
            u64::try_from(std::time::Duration::from_secs_f64(seconds).as_millis())
                .unwrap_or(u64::MAX)
        }
    })
}

fn value_to_lossless_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(inner) => Some(inner.clone()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jest_json_document() {
        let stdout = r#"{
          "testResults": [{
            "name": "/repo/tests/checkout.test.ts",
            "perfStats": {"runtime": 42},
            "assertionResults": [
              {"status":"passed","fullName":"passes","duration":3},
              {"status":"failed","fullName":"fails","failureMessages":["expected 200"],"actual":500,"expected":200}
            ]
          }]
        }"#;

        let events = parse_output(FrameworkArg::Jest, stdout, "");

        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], NormalizedEvent::Case(case) if case.status == TestStatus::Passed));
        assert!(matches!(&events[1], NormalizedEvent::Case(case) if case.failure.as_ref().is_some_and(|failure| failure.message == "expected 200")));
        assert!(matches!(&events[2], NormalizedEvent::Suite(suite) if suite.failed == 1 && suite.duration_ms == 42));
    }

    #[test]
    fn parses_pytest_json_report_document() {
        let stdout = r#"{
          "tests": [
            {"nodeid":"tests/test_checkout.py::test_passes","outcome":"passed","duration":0.012},
            {"nodeid":"tests/test_checkout.py::test_fails","outcome":"failed","call":{"crash":{"message":"boom"}},"message":"boom"}
          ]
        }"#;

        let events = parse_output(FrameworkArg::Pytest, stdout, "");

        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], NormalizedEvent::Case(case) if case.duration_ms == 12));
        assert!(matches!(&events[1], NormalizedEvent::Case(case) if case.status == TestStatus::Failed));
    }

    #[test]
    fn parses_generic_suite_json_document() {
        let stdout = r#"{
          "suites": [{
            "name": "math",
            "file": "tests/math.test.ts",
            "tests": [
              {"name":"adds","status":"pass","durationMs":7},
              {"name":"subtracts","status":"skip"}
            ]
          }]
        }"#;

        let events = parse_output(FrameworkArg::Vitest, stdout, "");

        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], NormalizedEvent::Case(case) if case.suite.as_deref() == Some("math") && case.duration_ms == 7));
        assert!(matches!(&events[2], NormalizedEvent::Suite(suite) if suite.passed == 1 && suite.skipped == 1));
    }

    #[test]
    fn parses_generic_json_line_case() {
        let events = parse_stdout_line(
            FrameworkArg::Deno,
            r#"{"name":"deno test","status":"ok","duration_seconds":0.004}"#,
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], NormalizedEvent::Case(case) if case.framework == "deno" && case.duration_ms == 4));
    }

    #[test]
    fn ignores_malformed_json_and_reports_stderr() {
        let events = parse_output(FrameworkArg::Bun, "{not-json", "fatal error\nsecond line");

        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], NormalizedEvent::Case(case) if case.status == TestStatus::Failed && case.stderr.is_some()));
    }

    #[test]
    fn hostile_durations_saturate_or_zero() {
        assert_eq!(seconds_to_ms(Some(-1.0)), 0);
        assert_eq!(seconds_to_ms(Some(f64::NAN)), 0);
        assert_eq!(seconds_to_ms(Some(f64::INFINITY)), 0);
        assert_eq!(seconds_to_ms(Some(f64::MAX)), u64::MAX);
    }
}
