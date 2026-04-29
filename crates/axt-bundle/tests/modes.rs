use std::{fs, io};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn agent_output_is_summary_first() -> Result<(), Box<dyn std::error::Error>> {
    let temp = fixture_project()?;
    let assert = Command::cargo_bin("axt-bundle")?
        .env("AXT_OUTPUT", "human")
        .args(["--agent", temp.path().to_string_lossy().as_ref()])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_jsonl_schemas(&stdout)?;
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("missing summary record"))?;
    let value: Value = serde_json::from_str(first)?;
    assert_eq!(value["schema"], "axt.bundle.summary.v1");
    assert_eq!(value["type"], "summary");
    assert_eq!(value["ok"], true);
    assert!(stdout.contains("\"schema\":\"axt.bundle.manifest.v1\""));
    Ok(())
}

#[test]
fn json_output_uses_envelope() -> Result<(), Box<dyn std::error::Error>> {
    let temp = fixture_project()?;
    let assert = Command::cargo_bin("axt-bundle")?
        .env("AXT_OUTPUT", "human")
        .args(["--json", temp.path().to_string_lossy().as_ref()])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["schema"], "axt.bundle.v1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["summary"]["manifests"], 1);
    Ok(())
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.bundle.v1.schema.json"))?;
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

fn validate_jsonl_schemas(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    for line in stdout.lines() {
        let record: Value = serde_json::from_str(line)?;
        let schema_name = record
            .get("schema")
            .and_then(Value::as_str)
            .ok_or_else(|| io::Error::other("jsonl record missing schema"))?;
        let schema_text = match schema_name {
            "axt.bundle.summary.v1" => {
                include_str!("../../../schemas/axt.bundle.summary.v1.schema.json")
            }
            "axt.bundle.manifest.v1" => {
                include_str!("../../../schemas/axt.bundle.manifest.v1.schema.json")
            }
            "axt.bundle.git.v1" => include_str!("../../../schemas/axt.bundle.git.v1.schema.json"),
            "axt.bundle.file.v1" => {
                include_str!("../../../schemas/axt.bundle.file.v1.schema.json")
            }
            "axt.bundle.warn.v1" => {
                include_str!("../../../schemas/axt.bundle.warn.v1.schema.json")
            }
            other => {
                return Err(io::Error::other(format!("unknown jsonl schema {other}")).into());
            }
        };
        let schema: Value = serde_json::from_str(schema_text)?;
        let compiled = jsonschema::JSONSchema::compile(&schema)
            .map_err(|error| io::Error::other(format!("schema compile failed: {error}")))?;
        let validation = compiled.validate(&record);
        if let Err(errors) = validation {
            let messages = errors
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(io::Error::other(format!("schema validation failed:\n{messages}")).into());
        }
    }
    Ok(())
}

fn fixture_project() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n",
    )?;
    fs::create_dir(temp.path().join("src"))?;
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn answer() -> u8 { 42 }\n",
    )?;
    Ok(temp)
}
