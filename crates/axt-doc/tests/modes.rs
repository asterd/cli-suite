use std::{fs, io, path::PathBuf};

use assert_cmd::Command;
use serde_json::Value;

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.doc.v1.schema.json"))?;
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

fn bin_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(
        Command::cargo_bin("axt-doc")?
            .env("AXT_OUTPUT", "human")
            .get_program(),
    ))
}

#[test]
fn which_json_validates_schema() -> Result<(), Box<dyn std::error::Error>> {
    let bin = bin_path()?;
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .args(["--json", "which"])
        .arg(bin)
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["schema"], "axt.doc.v1");
    assert_eq!(value["data"]["which"]["found"], true);
    assert_eq!(value["data"]["path"], Value::Null);
    assert_eq!(value["data"]["env"], Value::Null);
    Ok(())
}

#[test]
fn path_reports_duplicates_and_missing_entries() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    fs::create_dir(&bin)?;
    let missing = temp.path().join("missing");
    let path = std::env::join_paths([bin.as_path(), bin.as_path(), missing.as_path()])?;

    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .env("PATH", path)
        .args(["--json", "path"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(
        value["data"]["path"]["duplicates"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(
        value["data"]["path"]["missing"].as_array().map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[test]
fn env_redacts_secret_like_values() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .env("AXT_DOC_TOKEN", "super-secret")
        .args(["--json", "env"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    let secrets = value["data"]["env"]["secret_like"]
        .as_array()
        .ok_or_else(|| io::Error::other("secret_like is not an array"))?;
    let token = secrets
        .iter()
        .find(|item| item["name"] == "AXT_DOC_TOKEN")
        .ok_or_else(|| io::Error::other("AXT_DOC_TOKEN not found"))?;
    assert_eq!(token["value"], "<redacted>");
    Ok(())
}

#[test]
fn show_secrets_warns_on_stderr() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .env("AXT_DOC_TOKEN", "super-secret")
        .args(["--show-secrets", "--json", "env"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
    validate_json_schema(&stdout)?;
    assert!(stdout.contains("super-secret"));
    assert!(stderr.contains("--show-secrets"));
    Ok(())
}

#[test]
fn all_jsonl_starts_with_summary() -> Result<(), Box<dyn std::error::Error>> {
    let bin = bin_path()?;
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .args(["--agent", "all"])
        .arg(bin)
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("jsonl output was empty"))?;
    let value: Value = serde_json::from_str(first)?;
    assert_eq!(value["schema"], "axt.doc.summary.v1");
    assert_eq!(value["type"], "summary");
    Ok(())
}

#[test]
fn all_json_runs_which_path_and_env() -> Result<(), Box<dyn std::error::Error>> {
    let bin = bin_path()?;
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .args(["--json", "all"])
        .arg(bin)
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["which"]["found"], true);
    assert!(value["data"]["path"].is_object());
    assert!(value["data"]["env"].is_object());
    Ok(())
}

#[test]
fn bare_command_is_all_shortcut() -> Result<(), Box<dyn std::error::Error>> {
    let bin = bin_path()?;
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .arg("--json")
        .arg(bin)
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["which"]["found"], true);
    assert!(value["data"]["path"].is_object());
    assert!(value["data"]["env"].is_object());
    Ok(())
}

#[test]
fn agent_mode_has_schema_first() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .args(["--agent", "env"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let first_line = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("agent output was empty"))?;
    let first: Value = serde_json::from_str(first_line)?;
    assert_eq!(first["schema"], "axt.doc.summary.v1");
    assert_eq!(first["type"], "summary");
    Ok(())
}

#[test]
fn print_schema_outputs_json_schema() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-doc")?
        .env("AXT_OUTPUT", "human")
        .args(["--print-schema"])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout)?;
    assert_eq!(value["properties"]["schema"]["const"], "axt.doc.v1");
    Ok(())
}
