use std::{fs, io, path::Path};

use assert_cmd::Command;
use serde_json::Value;

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.drift.v1.schema.json"))?;
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

#[test]
fn mark_uses_default_name_and_writes_jsonl_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("input.txt"), "hello")?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "mark"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["schema"], "axt.drift.v1");
    assert_eq!(value["data"]["operation"], "mark");
    assert_eq!(value["data"]["name"], "default");
    assert!(temp.path().join(".axt/drift/default.jsonl").exists());
    let temp_files = fs::read_dir(temp.path().join(".axt/drift"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
        .count();
    assert_eq!(temp_files, 0);
    Ok(())
}

#[test]
fn diff_reports_created_modified_deleted_sorted_by_size_delta(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("deleted.txt"), "gone")?;
    fs::write(temp.path().join("modified.txt"), "small")?;
    mark(temp.path(), "base")?;

    fs::remove_file(temp.path().join("deleted.txt"))?;
    fs::write(temp.path().join("modified.txt"), "small plus more bytes")?;
    fs::write(temp.path().join("created.txt"), "new")?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "diff", "--since", "base"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    let changes = value["data"]["changes"]
        .as_array()
        .ok_or_else(|| io::Error::other("changes is not an array"))?;

    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0]["path"], "modified.txt");
    assert_eq!(changes[0]["action"], "modified");
    assert_eq!(changes[1]["path"], "deleted.txt");
    assert_eq!(changes[1]["action"], "deleted");
    assert_eq!(changes[2]["path"], "created.txt");
    assert_eq!(changes[2]["action"], "created");
    Ok(())
}

#[test]
fn diff_streaming_merge_handles_global_path_order() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::create_dir(temp.path().join("a"))?;
    fs::write(temp.path().join("a.txt"), "root before\n")?;
    fs::write(temp.path().join("a").join("b.txt"), "nested before\n")?;
    mark(temp.path(), "ordered")?;

    fs::write(temp.path().join("a.txt"), "root after with more bytes\n")?;
    fs::remove_file(temp.path().join("a").join("b.txt"))?;
    fs::write(temp.path().join("a-new.txt"), "created\n")?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "diff", "--since", "ordered"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    let changes = value["data"]["changes"]
        .as_array()
        .ok_or_else(|| io::Error::other("changes is not an array"))?;

    assert!(changes
        .iter()
        .any(|change| change["path"] == "a.txt" && change["action"] == "modified"));
    assert!(changes
        .iter()
        .any(|change| change["path"] == "a/b.txt" && change["action"] == "deleted"));
    assert!(changes
        .iter()
        .any(|change| change["path"] == "a-new.txt" && change["action"] == "created"));
    Ok(())
}

#[test]
fn hash_mode_includes_hashes_for_changed_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("file.txt"), "before")?;
    mark_hash(temp.path(), "hashy")?;
    fs::write(temp.path().join("file.txt"), "after")?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "diff", "--since", "hashy", "--hash"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert!(value["data"]["changes"][0]["hash"].as_str().is_some());
    Ok(())
}

#[test]
fn hash_mode_skips_files_above_configured_limit() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("large.txt"), "before")?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "mark", "--hash", "--hash-max-bytes", "1"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["data"]["hash_skipped_size"], 1);
    let snapshot = fs::read_to_string(temp.path().join(".axt/drift/default.jsonl"))?;
    assert!(snapshot.contains(r#""hash_skipped_size":true"#));
    Ok(())
}

#[test]
fn diff_hash_mode_does_not_modify_unchanged_metadata_mark() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("file.txt"), "same")?;
    mark(temp.path(), "mixed")?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "diff", "--since", "mixed", "--hash"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["data"]["changes"].as_array().map(Vec::len), Some(0));
    Ok(())
}

#[test]
fn run_executes_command_and_reports_drift() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "run", "--name", "build", "--"])
        .args(file_create_command())
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["data"]["operation"], "run");
    assert_eq!(value["data"]["exit"], 0);
    assert_eq!(value["data"]["changes"][0]["path"], "fixture.out");
    assert!(temp.path().join(".axt/drift/build.jsonl").exists());
    Ok(())
}

#[test]
fn run_reports_non_zero_command_exit() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;

    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "run", "--"])
        .args(non_zero_command())
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["ok"], false);
    assert_eq!(value["data"]["exit"], 7);
    assert_eq!(value["errors"][0]["code"], "command_failed");
    Ok(())
}

#[test]
fn list_and_reset_manage_marks() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("input.txt"), "hello")?;
    mark(temp.path(), "one")?;
    mark(temp.path(), "two")?;

    let list = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(list.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["marks"].as_array().map(Vec::len), Some(2));

    let reset = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--json", "reset"])
        .assert()
        .success();
    let stdout = String::from_utf8(reset.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["removed"], 2);
    Ok(())
}

#[test]
fn invalid_mark_name_and_missing_mark_fail() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;

    let invalid = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["mark", "--name", "../outside"])
        .assert()
        .failure();
    let stderr = String::from_utf8(invalid.get_output().stderr.clone())?;
    assert!(stderr.contains("invalid mark name"));

    let missing = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["diff", "--since", "missing"])
        .assert()
        .failure();
    let stderr = String::from_utf8(missing.get_output().stderr.clone())?;
    assert!(stderr.contains("mark not found"));
    Ok(())
}

#[test]
fn jsonl_and_agent_start_with_schema_records() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("input.txt"), "hello")?;

    let jsonl = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--agent", "mark"])
        .assert()
        .success();
    let stdout = String::from_utf8(jsonl.get_output().stdout.clone())?;
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("jsonl output was empty"))?;
    let value: Value = serde_json::from_str(first)?;
    assert_eq!(value["schema"], "axt.drift.summary.v1");

    let agent = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--agent", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(agent.get_output().stdout.clone())?;
    let first_line = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("agent output was empty"))?;
    let value: Value = serde_json::from_str(first_line)?;
    assert_eq!(value["schema"], "axt.drift.summary.v1");
    Ok(())
}

#[test]
fn jsonl_and_agent_summaries_report_truncation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    mark(temp.path(), "base")?;
    fs::write(temp.path().join("one.txt"), "one")?;

    let jsonl = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(temp.path())
        .args(["--agent", "--limit", "1", "diff", "--since", "base"])
        .assert()
        .success();
    let stdout = String::from_utf8(jsonl.get_output().stdout.clone())?;
    let mut lines = stdout.lines();
    let first = lines
        .next()
        .ok_or_else(|| io::Error::other("jsonl output was empty"))?;
    let value: Value = serde_json::from_str(first)?;
    assert_eq!(value["truncated"], true);
    let warning = lines
        .next()
        .ok_or_else(|| io::Error::other("jsonl warning was missing"))?;
    let value: Value = serde_json::from_str(warning)?;
    assert_eq!(value["type"], "warn");

    Ok(())
}

#[test]
fn print_schema_outputs_json_schema() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .args(["--print-schema"])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout)?;
    assert_eq!(value["properties"]["schema"]["const"], "axt.drift.v1");
    Ok(())
}

fn mark(dir: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(dir)
        .args(["mark", "--name", name])
        .assert()
        .success();
    Ok(())
}

fn mark_hash(dir: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-drift")?
        .env("AXT_OUTPUT", "human")
        .current_dir(dir)
        .args(["mark", "--name", name, "--hash"])
        .assert()
        .success();
    Ok(())
}

#[cfg(unix)]
fn file_create_command() -> [&'static str; 3] {
    ["sh", "-c", "printf created > fixture.out"]
}

#[cfg(windows)]
fn file_create_command() -> [&'static str; 3] {
    ["cmd", "/C", "echo created>fixture.out"]
}

#[cfg(unix)]
fn non_zero_command() -> [&'static str; 3] {
    ["sh", "-c", "exit 7"]
}

#[cfg(windows)]
fn non_zero_command() -> [&'static str; 3] {
    ["cmd", "/C", "exit /B 7"]
}
