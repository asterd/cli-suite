use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    JsonEnvelope, AgentJsonlWriter, OutputDiagnostic, RenderContext, Renderable,
    Result as RenderResult,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    command::{FrameworkInfo, TestOutput},
    model::{TestCase, TestData, TestStatus, TestSummary},
};

impl Renderable for TestOutput {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        match self {
            Self::Frameworks { frameworks } => render_frameworks_human(w, frameworks),
            Self::Run {
                data,
                top_failures,
                include_output,
                failures_only,
            } => render_run_human(w, data, *top_failures, *include_output, *failures_only),
        }
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope =
            JsonEnvelope::with_status("axt.test.v1", self.ok(), self, Vec::new(), errors(self));
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentJsonlWriter::new(w, ctx.limits);
        match self {
            Self::Frameworks { frameworks } => {
                writer.write_record(&json!({
                    "schema": "axt.test.frameworks.summary.v1",
                    "type": "summary",
                    "frameworks": frameworks.len(),
                    "next": ["axt-test --json"]
                }))?;
                for framework in frameworks {
                    writer.write_record(&json!({
                        "schema": "axt.test.framework.v1",
                        "type": "framework",
                        "name": framework.name,
                        "marker": framework.marker,
                        "detection": framework.detection
                    }))?;
                }
            }
            Self::Run {
                data,
                top_failures,
                include_output,
                failures_only,
            } => {
                writer.write_record(&jsonl_summary(data, false, &next_hints(data)))?;
                for suite in &data.suites {
                    writer.write_record(&json!({
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
                    }))?;
                }
                for case in selected_cases(data, *top_failures, *failures_only) {
                    writer.write_record(&case_record(case, *include_output))?;
                }
            }
        }
        let _summary = writer.finish("axt.test.warn.v1")?;
        Ok(())
    }
}

fn render_frameworks_human(w: &mut dyn Write, frameworks: &[FrameworkInfo]) -> RenderResult<()> {
    writeln!(w, "Framework  Marker         Detection")?;
    for framework in frameworks {
        writeln!(
            w,
            "{:<10} {:<14} {}",
            framework.name, framework.marker, framework.detection
        )?;
    }
    Ok(())
}

fn render_run_human(
    w: &mut dyn Write,
    data: &TestData,
    top_failures: usize,
    include_output: bool,
    failures_only: bool,
) -> RenderResult<()> {
    writeln!(
        w,
        "frameworks={} total={} passed={} failed={} skipped={} todo={} ms={}",
        data.frameworks.join(","),
        data.total,
        data.passed,
        data.failed,
        data.skipped,
        data.todo,
        data.duration_ms
    )?;
    for case in selected_cases(data, top_failures, failures_only)
        .into_iter()
        .filter(|case| case.status == TestStatus::Failed)
    {
        writeln!(
            w,
            "FAILED {} {}",
            case.file
                .as_ref()
                .map_or_else(|| "-".to_owned(), ToString::to_string),
            case.name
        )?;
        if let Some(failure) = &case.failure {
            writeln!(w, "  {}", failure.message)?;
        }
        if include_output {
            if let Some(stdout) = &case.stdout {
                writeln!(w, "  stdout: {stdout}")?;
            }
            if let Some(stderr) = &case.stderr {
                writeln!(w, "  stderr: {stderr}")?;
            }
        }
    }
    Ok(())
}

fn jsonl_summary(data: &TestData, truncated: bool, next: &[String]) -> Value {
    let mut summary = TestSummary::from(data);
    summary.truncated = truncated;
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
        "truncated": summary.truncated,
        "next": next
    })
}

fn next_hints(data: &TestData) -> Vec<String> {
    let mut hints = Vec::new();
    if data.failed > 0 {
        hints.push("axt-test --rerun-failed --include-output --agent".to_owned());
        hints.push("axt-test --top-failures 5 --include-output --json".to_owned());
        if let Some(failed) = data
            .cases
            .iter()
            .find(|case| case.status == TestStatus::Failed)
        {
            if let (Some(file), Some(line)) = (failed.file.as_ref(), failed.line) {
                hints.push(format!("axt-outline {file} --agent  # near line {line}"));
            }
        }
    }
    hints
}

fn selected_cases(data: &TestData, top_failures: usize, failures_only: bool) -> Vec<&TestCase> {
    let mut failures_left = top_failures;
    data.cases
        .iter()
        .filter(|case| {
            if failures_only && case.status != TestStatus::Failed {
                return false;
            }
            if case.status == TestStatus::Failed {
                if failures_left == 0 {
                    false
                } else {
                    failures_left -= 1;
                    true
                }
            } else {
                true
            }
        })
        .collect()
}

fn case_record(case: &TestCase, include_output: bool) -> Value {
    let mut record = json!({
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
    });
    if !case.parser_defaulted_fields.is_empty() {
        record["parser_defaulted_fields"] = json!(case.parser_defaulted_fields);
    }
    record
}

fn errors(output: &TestOutput) -> Vec<OutputDiagnostic> {
    match output {
        TestOutput::Run { data, .. } if !data.ok() => vec![OutputDiagnostic {
            code: ErrorCode::CommandFailed,
            message: "one or more tests failed".to_owned(),
            context: json!({ "failed": data.failed }),
        }],
        _ => Vec::new(),
    }
}

#[allow(dead_code)]
fn serialized_jsonl_len<T: Serialize>(record: &T) -> RenderResult<usize> {
    Ok(serde_json::to_vec(record)?.len() + 1)
}
