use std::{fs, io};

use assert_cmd::Command;
use insta::assert_snapshot;
use proptest::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn fixture(name: &str) -> String {
    format!("../../fixtures/axt-logdx/{name}")
}

fn command() -> Result<Command, Box<dyn std::error::Error>> {
    let mut command = Command::cargo_bin("axt-logdx")?;
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    Ok(command)
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.logdx.v1.schema.json"))?;
    let instance: Value = serde_json::from_str(stdout)?;
    let compiled = jsonschema::JSONSchema::compile(&schema)
        .map_err(|error| io::Error::other(format!("schema compile failed: {error}")))?;
    if let Err(errors) = compiled.validate(&instance) {
        let messages = errors
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(io::Error::other(format!("schema validation failed:\n{messages}")).into());
    }
    Ok(())
}

fn json_data(stdout: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let value: Value = serde_json::from_str(stdout)?;
    Ok(value["data"].clone())
}

fn json_data_string(stdout: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(format!("{}\n", serde_json::to_string(&json_data(stdout)?)?))
}

#[test]
fn fixture_modes_match_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let human = command()?
        .env("AXT_OUTPUT", "human")
        .args([&fixture("plain.log"), "--severity", "warn", "--top", "10"])
        .assert()
        .success();
    assert_snapshot!(
        "logdx_human",
        String::from_utf8(human.get_output().stdout.clone())?
    );

    let envelope = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--json", &fixture("plain.log"), "--severity", "warn"])
        .assert()
        .success();
    let stdout = String::from_utf8(envelope.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    assert_snapshot!("logdx_json", stdout);

    let data = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--json", &fixture("plain.log"), "--severity", "warn"])
        .assert()
        .success();
    assert_snapshot!(
        "logdx_json_data",
        json_data_string(&String::from_utf8(data.get_output().stdout.clone())?)?
    );

    let agent = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--agent", &fixture("plain.log"), "--severity", "warn"])
        .assert()
        .success();
    assert_snapshot!(
        "logdx_agent",
        String::from_utf8(agent.get_output().stdout.clone())?
    );
    Ok(())
}

#[test]
fn jsonl_logs_are_grouped() -> Result<(), Box<dyn std::error::Error>> {
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--json", &fixture("jsonl.log"), "--severity", "error"])
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 1);
    assert_eq!(data["groups"][0]["count"], 2);
    assert!(data["groups"][0]["stack"]
        .as_array()
        .is_some_and(|stack| !stack.is_empty()));
    Ok(())
}

#[test]
fn syslog_ansi_and_crlf_logs_parse() -> Result<(), Box<dyn std::error::Error>> {
    for file in ["syslog.log", "ansi.log", "crlf.log"] {
        let assert = command()?
            .env("AXT_OUTPUT", "human")
            .args(["--json", &fixture(file), "--severity", "warn"])
            .assert()
            .success();
        let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
        assert!(
            data["summary"]["groups"].as_u64().unwrap_or(0) > 0,
            "{file}"
        );
    }
    Ok(())
}

#[test]
fn stack_traces_are_captured() -> Result<(), Box<dyn std::error::Error>> {
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--json", &fixture("stacks.log"), "--severity", "error"])
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    let groups = data["groups"]
        .as_array()
        .ok_or_else(|| io::Error::other("groups must be an array"))?;
    assert_eq!(groups.len(), 4);
    assert!(groups.iter().any(|group| {
        group["message"]
            .as_str()
            .is_some_and(|message| message.contains("python crash"))
            && group["stack"].as_array().is_some_and(|stack| {
                stack.iter().any(|line| {
                    line.as_str()
                        .is_some_and(|line| line.contains("RuntimeError"))
                })
            })
    }));
    assert!(groups.iter().any(|group| {
        group["message"]
            .as_str()
            .is_some_and(|message| message.contains("jvm crash"))
            && group["stack"].as_array().is_some_and(|stack| {
                stack.iter().any(|line| {
                    line.as_str()
                        .is_some_and(|line| line.contains("RuntimeException"))
                })
            })
    }));
    Ok(())
}

#[test]
fn modern_development_log_formats_parse() -> Result<(), Box<dyn std::error::Error>> {
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--json", &fixture("modern.log"), "--severity", "error"])
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 5);
    assert_eq!(data["summary"]["errors"], 5);
    assert_eq!(data["timeline"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[test]
fn nested_modern_json_logs_parse() -> Result<(), Box<dyn std::error::Error>> {
    let input = concat!(
        "{\"@timestamp\":\"2026-04-28T10:00:00Z\",\"severityText\":\"ERROR\",\"body\":\"otel body failure\",\"exception\":{\"stacktrace\":\"Error: boom\\n    at worker.js:1:2\"}}\n",
        "{\"timestamp\":\"2026-04-28T10:00:01Z\",\"severity\":\"ERROR\",\"jsonPayload\":{\"message\":\"cloud payload failure\",\"error\":{\"stack_trace\":[\"Error: cloud\",\"    at cloud.js:2:3\"]}}}\n",
        "{\"timestamp\":\"2026-04-28T10:00:02Z\",\"attributes\":{\"log.level\":\"error\",\"message\":\"ecs attribute failure\"}}\n"
    );
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--stdin", "--json", "--severity", "error"])
        .write_stdin(input)
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 3);
    assert!(data["groups"].as_array().is_some_and(|groups| {
        groups.iter().any(|group| {
            group["stack"].as_array().is_some_and(|stack| {
                stack
                    .iter()
                    .any(|line| line.as_str().is_some_and(|line| line.contains("cloud.js")))
            })
        })
    }));
    Ok(())
}

#[test]
fn epoch_timestamps_parse_for_filters_and_timeline() -> Result<(), Box<dyn std::error::Error>> {
    let input = concat!(
        "{\"time\":1777370400000,\"level\":\"error\",\"message\":\"epoch millis failure\"}\n",
        "ts=1777370400 level=error msg=\"epoch seconds failure\"\n"
    );
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args([
            "--stdin",
            "--json",
            "--severity",
            "error",
            "--since",
            "2026-04-28T09:59:00Z",
            "--until",
            "2026-04-28T10:01:00Z",
        ])
        .write_stdin(input)
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 2);
    assert_eq!(data["timeline"][0]["bucket"], "2026-04-28T10:00:00Z");
    assert_eq!(data["timeline"][0]["error"], 2);
    Ok(())
}

#[test]
fn severity_and_time_filters_apply() -> Result<(), Box<dyn std::error::Error>> {
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("plain.log"),
            "--severity",
            "fatal",
            "--since",
            "2026-04-28T10:00:30Z",
            "--until",
            "2026-04-28T10:01:30Z",
        ])
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 1);
    assert_eq!(data["groups"][0]["severity"], "fatal");
    Ok(())
}

#[test]
fn stdin_input_works() -> Result<(), Box<dyn std::error::Error>> {
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args(["--stdin", "--json", "--severity", "error"])
        .write_stdin("2026-04-28T10:00:00Z ERROR stdin failure\n")
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["sources"][0]["path"], "<stdin>");
    assert_eq!(data["summary"]["groups"], 1);
    Ok(())
}

#[test]
fn invalid_utf8_input_is_decoded_lossily() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let path = temp.path().join("invalid.log");
    fs::write(
        &path,
        b"2026-04-28T10:00:00Z ERROR invalid utf8 \xff payload\n",
    )?;
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            path.to_str()
                .ok_or_else(|| io::Error::other("temp path must be utf-8"))?,
            "--severity",
            "error",
        ])
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 1);
    assert!(data["warnings"].as_array().is_some_and(|warnings| {
        warnings
            .iter()
            .any(|warning| warning["code"].as_str() == Some("invalid_utf8"))
    }));
    Ok(())
}

#[test]
fn large_file_streaming_and_truncation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let path = temp.path().join("large.log");
    let mut contents = String::new();
    for index in 0..500 {
        let first = char::from(b'a' + u8::try_from(index % 26)?);
        let second = char::from(b'a' + u8::try_from((index / 26) % 26)?);
        contents.push_str(&format!(
            "2026-04-28T10:00:00Z ERROR unique failure {first}{second}\n"
        ));
    }
    fs::write(&path, contents)?;
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            path.to_str()
                .ok_or_else(|| io::Error::other("temp path must be utf-8"))?,
            "--severity",
            "error",
            "--top",
            "5",
        ])
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 5);
    assert_eq!(data["summary"]["truncated"], true);
    Ok(())
}

#[test]
fn high_cardinality_logs_use_bounded_aggregation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let path = temp.path().join("cardinality.log");
    let mut contents = String::new();
    for index in 0..2_000 {
        let first = char::from(b'a' + u8::try_from(index % 26)?);
        let second = char::from(b'a' + u8::try_from((index / 26) % 26)?);
        let third = char::from(b'a' + u8::try_from((index / 676) % 26)?);
        contents.push_str(&format!(
            "2026-04-28T10:00:00Z ERROR unique failure code-{first}{second}{third}\n"
        ));
    }
    fs::write(&path, contents)?;
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            path.to_str()
                .ok_or_else(|| io::Error::other("temp path must be utf-8"))?,
            "--severity",
            "error",
            "--top",
            "5",
        ])
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 5);
    assert_eq!(data["summary"]["truncated"], true);
    assert!(data["warnings"].as_array().is_some_and(|warnings| {
        warnings
            .iter()
            .any(|warning| warning["code"].as_str() == Some("input_truncated"))
    }));
    Ok(())
}

#[test]
fn time_filters_warn_when_matching_records_have_no_time() -> Result<(), Box<dyn std::error::Error>>
{
    let assert = command()?
        .env("AXT_OUTPUT", "human")
        .args([
            "--stdin",
            "--json",
            "--severity",
            "error",
            "--since",
            "2026-04-28T10:00:00Z",
        ])
        .write_stdin("ERROR undated failure\n")
        .assert()
        .success();
    let data = json_data(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    assert_eq!(data["summary"]["groups"], 0);
    assert!(data["warnings"].as_array().is_some_and(|warnings| {
        warnings
            .iter()
            .any(|warning| warning["code"].as_str() == Some("time_unparseable"))
    }));
    Ok(())
}

#[test]
fn strict_agent_truncation_exits_nonzero() -> Result<(), Box<dyn std::error::Error>> {
    command()?
        .env("AXT_OUTPUT", "human")
        .args([
            "--agent",
            &fixture("plain.log"),
            "--severity",
            "warn",
            "--max-bytes",
            "100",
            "--strict",
        ])
        .assert()
        .failure();
    Ok(())
}

proptest! {
    #[test]
    fn arbitrary_stdin_bytes_do_not_crash(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let mut command = command().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let assert = command
            .env("AXT_OUTPUT", "human")
            .args(["--stdin", "--json", "--severity", "trace"])
            .write_stdin(bytes)
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let value: Value = serde_json::from_str(&stdout)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(value["schema"].as_str(), Some("axt.logdx.v1"));
        prop_assert_eq!(value["ok"].as_bool(), Some(true));
    }

    #[test]
    fn arbitrary_log_like_lines_do_not_crash(line in ".{0,512}") {
        let input = format!("{line}\n");
        let mut command = command().map_err(|error| TestCaseError::fail(error.to_string()))?;
        let assert = command
            .env("AXT_OUTPUT", "human")
            .args(["--stdin", "--json", "--severity", "trace"])
            .write_stdin(input)
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let value: Value = serde_json::from_str(&stdout)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert!(value["data"]["summary"]["lines"].as_u64().unwrap_or(0) <= 1);
    }
}
