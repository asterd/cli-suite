use std::io::Write;

use axt_core::ErrorCode;
use axt_output::{
    format_agent_fields, AgentCompactWriter, AgentField, JsonEnvelope, JsonlWriter,
    OutputDiagnostic, RenderContext, Renderable, Result as RenderResult,
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    command::{FrameworkInfo, TestOutput},
    model::{TestCase, TestData, TestStatus, TestSummary},
};

impl TestOutput {
    pub fn render_json_data(&self, w: &mut dyn Write) -> RenderResult<()> {
        match self {
            Self::Run { data, .. } => serde_json::to_writer(&mut *w, data)?,
            Self::Frameworks { frameworks } => serde_json::to_writer(&mut *w, frameworks)?,
        }
        writeln!(w)?;
        Ok(())
    }
}

impl Renderable for TestOutput {
    fn render_human(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        match self {
            Self::Frameworks { frameworks } => render_frameworks_human(w, frameworks),
            Self::Run {
                data,
                top_failures,
                include_output,
            } => render_run_human(w, data, *top_failures, *include_output),
        }
    }

    fn render_json(&self, w: &mut dyn Write, _ctx: &RenderContext<'_>) -> RenderResult<()> {
        let envelope =
            JsonEnvelope::with_status("axt.test.v1", self.ok(), self, Vec::new(), errors(self));
        serde_json::to_writer(&mut *w, &envelope)?;
        writeln!(w)?;
        Ok(())
    }

    fn render_jsonl(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = JsonlWriter::new(w, ctx.limits);
        match self {
            Self::Frameworks { frameworks } => {
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
            } => {
                writer.write_record(&jsonl_summary(data, false))?;
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
                for case in selected_cases(data, *top_failures) {
                    writer.write_record(&case_record(case, *include_output))?;
                }
            }
        }
        let _summary = writer.finish("axt.test.warn.v1")?;
        Ok(())
    }

    fn render_agent(&self, w: &mut dyn Write, ctx: &RenderContext<'_>) -> RenderResult<()> {
        let mut writer = AgentCompactWriter::new(w, ctx.limits);
        match self {
            Self::Frameworks { frameworks } => {
                writer.write_line("schema=axt.test.agent.v1 ok=true mode=frameworks")?;
                for framework in frameworks {
                    writer.write_line(&prefixed_line(
                        "F",
                        &[
                            AgentField::str("name", framework.name.as_str()),
                            AgentField::str("marker", framework.marker.as_str()),
                            AgentField::str("detection", framework.detection.as_str()),
                        ],
                    )?)?;
                }
            }
            Self::Run {
                data,
                top_failures,
                include_output: _,
            } => {
                writer.write_line(&agent_summary_line(data, false)?)?;
                for suite in &data.suites {
                    writer.write_line(&prefixed_line(
                        "U",
                        &[
                            AgentField::str("name", &suite.name),
                            AgentField::str("framework", &suite.framework),
                            AgentField::str(
                                "file",
                                suite.file.as_ref().map_or("-", |path| path.as_str()),
                            ),
                            AgentField::usize("passed", suite.passed),
                            AgentField::usize("failed", suite.failed),
                            AgentField::usize("skipped", suite.skipped),
                            AgentField::u64("ms", suite.duration_ms),
                        ],
                    )?)?;
                }
                for case in selected_cases(data, *top_failures)
                    .into_iter()
                    .filter(|case| case.status != TestStatus::Passed)
                {
                    writer.write_line(&agent_case_line(case)?)?;
                }
                writer.write_line("S run=\"axt-test\"")?;
            }
        }
        let _summary = writer.finish()?;
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
    for case in selected_cases(data, top_failures)
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

fn jsonl_summary(data: &TestData, truncated: bool) -> Value {
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
        "truncated": summary.truncated
    })
}

fn selected_cases(data: &TestData, top_failures: usize) -> Vec<&TestCase> {
    let mut failures_left = top_failures;
    data.cases
        .iter()
        .filter(|case| {
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

fn agent_summary_line(data: &TestData, truncated: bool) -> RenderResult<String> {
    format_agent_fields(&[
        AgentField::str("schema", "axt.test.agent.v1"),
        AgentField::bool("ok", data.ok()),
        AgentField::str("mode", "records"),
        AgentField::str("frameworks", &data.frameworks.join(",")),
        AgentField::usize("total", data.total),
        AgentField::usize("passed", data.passed),
        AgentField::usize("failed", data.failed),
        AgentField::usize("skipped", data.skipped),
        AgentField::usize("todo", data.todo),
        AgentField::u64("ms", data.duration_ms),
        AgentField::str("started", &data.started),
        AgentField::bool("truncated", truncated),
    ])
}

fn agent_case_line(case: &TestCase) -> RenderResult<String> {
    let mut fields = vec![
        AgentField::str("status", case.status.as_str()),
        AgentField::str("name", &case.name),
        AgentField::str("framework", &case.framework),
        AgentField::str("file", case.file.as_ref().map_or("-", |path| path.as_str())),
        AgentField::u64("line", case.line.unwrap_or(0)),
        AgentField::u64("ms", case.duration_ms),
    ];
    if let Some(suite) = &case.suite {
        fields.push(AgentField::str("suite", suite));
    }
    if let Some(failure) = &case.failure {
        fields.push(AgentField::str("message", &failure.message));
    }
    prefixed_line("C", &fields)
}

fn prefixed_line(prefix: &str, fields: &[AgentField<'_>]) -> RenderResult<String> {
    let mut line = String::with_capacity(prefix.len() + 1);
    line.push_str(prefix);
    if !fields.is_empty() {
        line.push(' ');
        line.push_str(&format_agent_fields(fields)?);
    }
    Ok(line)
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
