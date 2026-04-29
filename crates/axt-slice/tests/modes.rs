use std::{fs, io};

use assert_cmd::Command;
use insta::assert_snapshot;
use serde_json::Value;
use tempfile::TempDir;

fn fixture(name: &str) -> String {
    format!(
        "{}/../../fixtures/axt-slice/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.slice.v1.schema.json"))?;
    let instance: Value = serde_json::from_str(stdout)?;
    validate_against_schema(&schema, &instance)
}

fn validate_against_schema(
    schema: &Value,
    instance: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    let compiled = jsonschema::JSONSchema::compile(schema)
        .map_err(|error| io::Error::other(format!("schema compile failed: {error}")))?;
    if let Err(errors) = compiled.validate(instance) {
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
fn all_output_modes_work() -> Result<(), Box<dyn std::error::Error>> {
    for mode in ["--json", "--agent"] {
        let assert = Command::cargo_bin("axt-slice")?
            .env("AXT_OUTPUT", "human")
            .args([mode, &fixture("src/lib.rs"), "--symbol", "process_request"])
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
        assert!(!stdout.is_empty(), "{mode}");
        if mode == "--json" {
            validate_json_schema(&stdout)?;
        }
    }
    Ok(())
}

#[test]
fn fixture_modes_match_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let human = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([&fixture("src/lib.rs"), "--symbol", "process_request"])
        .assert()
        .success();
    assert_snapshot!(
        "slice_human",
        String::from_utf8(human.get_output().stdout.clone())?
    );

    let envelope = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("src/lib.rs"),
            "--symbol",
            "process_request",
        ])
        .assert()
        .success();
    assert_snapshot!(
        "slice_json",
        String::from_utf8(envelope.get_output().stdout.clone())?
    );

    let json = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("src/lib.rs"),
            "--symbol",
            "process_request",
        ])
        .assert()
        .success();
    assert_snapshot!(
        "slice_json_data",
        json_data_string(&String::from_utf8(json.get_output().stdout.clone())?)?
    );

    let agent = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--agent",
            &fixture("src/lib.rs"),
            "--symbol",
            "process_request",
        ])
        .assert()
        .success();
    assert_snapshot!(
        "slice_agent",
        String::from_utf8(agent.get_output().stdout.clone())?
    );
    Ok(())
}

#[test]
fn extracts_exact_symbol_with_docs_and_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("src/lib.rs"),
            "--symbol",
            "process_request",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["status"], "selected");
    assert_eq!(value["symbol"]["name"], "process_request");
    assert_eq!(value["symbol"]["range"]["start_line"], 22);
    assert_eq!(value["range"]["start_line"], 20);
    assert!(value["source"].as_str().is_some_and(|source| {
        source.starts_with("/// Process one request.\n#[inline]\n")
            && source.contains("pub fn process_request")
            && !source.contains("use std::fmt")
    }));
    Ok(())
}

#[test]
fn symbol_queries_can_be_qualified() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("src/lib.rs"),
            "--symbol",
            "Parser::parse",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["status"], "selected");
    assert_eq!(value["symbol"]["qualified_name"], "Parser::parse");
    assert!(value["source"]
        .as_str()
        .is_some_and(|source| source.starts_with("    /// Parse with the parser type.")));
    Ok(())
}

#[test]
fn ambiguous_symbols_return_candidates() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args(["--json", &fixture("src/lib.rs"), "--symbol", "parse"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["status"], "ambiguous");
    assert!(value["source"].is_null());
    assert_eq!(value["candidates"].as_array().map_or(0, Vec::len), 2);
    assert!(stdout.contains("Parser::parse"));
    assert!(stdout.contains("OtherParser::parse"));
    Ok(())
}

#[test]
fn line_fallback_selects_enclosing_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args(["--json", &fixture("src/lib.rs"), "--line", "23"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["selection"]["kind"], "line");
    assert_eq!(value["symbol"]["name"], "process_request");
    Ok(())
}

#[test]
fn include_imports_tests_and_adjacent_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("src/lib.rs"),
            "--symbol",
            "process_request",
            "--include-imports=all",
            "--include-tests",
            "--before-symbol",
            "--after-symbol",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    let source = value["source"]
        .as_str()
        .ok_or_else(|| io::Error::other("source must be present"))?;
    assert!(source.contains("use std::fmt;"));
    assert!(source.contains("impl Response"));
    assert!(source.contains("pub struct Parser;"));
    assert!(source.contains("fn process_request_returns_body()"));
    Ok(())
}

#[test]
fn supported_language_symbols_extract() -> Result<(), Box<dyn std::error::Error>> {
    for (file, symbol, expected) in [
        (
            "langs/service.ts",
            "processRequest",
            "export function processRequest",
        ),
        ("langs/view.tsx", "View", "export const View"),
        ("langs/view.tsx", "loadLabel", "export function loadLabel"),
        ("langs/service.py", "process_request", "def process_request"),
        ("langs/service.go", "ProcessRequest", "func ProcessRequest"),
        (
            "langs/Service.java",
            "Service::process",
            "public String process",
        ),
        (
            "langs/service.php",
            "Service::process",
            "public function process",
        ),
    ] {
        let assert = Command::cargo_bin("axt-slice")?
            .env("AXT_OUTPUT", "human")
            .args(["--json", &fixture(file), "--symbol", symbol])
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
        let value = json_data(&stdout)?;
        assert_eq!(value["status"], "selected", "{file}");
        assert!(value["source"]
            .as_str()
            .is_some_and(|source| source.contains(expected)));
    }
    Ok(())
}

#[test]
fn kind_qualified_queries_accept_qualified_names() -> Result<(), Box<dyn std::error::Error>> {
    for (file, symbol, expected_name) in [
        ("src/lib.rs", "fn::Parser::parse", "parse"),
        ("langs/service.go", "method::Service::Process", "Process"),
        (
            "langs/Service.java",
            "constructor::Service::Service",
            "Service",
        ),
        ("langs/Service.java", "method::Service::process", "process"),
        ("langs/service.php", "method::Service::process", "process"),
    ] {
        let assert = Command::cargo_bin("axt-slice")?
            .env("AXT_OUTPUT", "human")
            .args(["--json", &fixture(file), "--symbol", symbol])
            .assert()
            .success();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
        let value = json_data(&stdout)?;
        assert_eq!(value["status"], "selected", "{file} {symbol}");
        assert_eq!(value["symbol"]["name"], expected_name);
    }
    Ok(())
}

#[test]
fn include_imports_supports_all_and_matched_modes() -> Result<(), Box<dyn std::error::Error>> {
    let matched = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("src/lib.rs"),
            "--symbol",
            "helper",
            "--include-imports=matched",
        ])
        .assert()
        .success();
    let matched_stdout = String::from_utf8(matched.get_output().stdout.clone())?;
    let matched_value = json_data(&matched_stdout)?;
    let matched_source = matched_value["source"]
        .as_str()
        .ok_or_else(|| io::Error::other("source must be present"))?;
    assert!(matched_source.contains("use std::path::Path;"));
    assert!(!matched_source.contains("use std::fmt;"));
    assert_eq!(matched_value["spans"].as_array().map_or(0, Vec::len), 2);

    let all = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            &fixture("src/lib.rs"),
            "--symbol",
            "helper",
            "--include-imports=all",
        ])
        .assert()
        .success();
    let all_stdout = String::from_utf8(all.get_output().stdout.clone())?;
    let all_value = json_data(&all_stdout)?;
    let all_source = all_value["source"]
        .as_str()
        .ok_or_else(|| io::Error::other("source must be present"))?;
    assert!(all_source.contains("use std::fmt;"));
    assert!(all_source.contains("use std::path::Path;"));
    Ok(())
}

#[test]
fn crlf_input_is_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let file = temp.path().join("crlf.rs");
    fs::write(
        &file,
        "/// CRLF docs\r\n#[inline]\r\npub fn crlf_target() -> u8 {\r\n    1\r\n}\r\n",
    )?;
    let assert = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            file.to_string_lossy().as_ref(),
            "--symbol",
            "crlf_target",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    let source = value["source"]
        .as_str()
        .ok_or_else(|| io::Error::other("source must be present"))?;
    assert!(source.contains("\r\n#[inline]\r\n"));
    Ok(())
}

#[test]
fn truncation_and_strict_are_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let truncated = Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--agent",
            "--limit",
            "1",
            &fixture("src/lib.rs"),
            "--symbol",
            "parse",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(truncated.get_output().stdout.clone())?;
    assert!(stdout.contains("\"code\":\"truncated\""));
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("agent summary must be present"))?;
    assert!(first.contains("\"truncated\":true"));

    Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--agent",
            "--limit",
            "1",
            "--strict",
            &fixture("src/lib.rs"),
            "--symbol",
            "parse",
        ])
        .assert()
        .code(6);
    Ok(())
}

#[test]
fn selector_is_required_by_cli() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .arg(fixture("src/lib.rs"))
        .assert()
        .code(2);
    Ok(())
}

#[test]
fn binary_and_non_utf8_inputs_are_refused() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let binary = temp.path().join("binary.rs");
    fs::write(&binary, b"pub fn bad() {}\0")?;
    Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([binary.to_string_lossy().as_ref(), "--symbol", "bad"])
        .assert()
        .code(9);

    let non_utf8 = temp.path().join("non_utf8.rs");
    fs::write(&non_utf8, [0xff, 0xfe, b'\n'])?;
    Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args([non_utf8.to_string_lossy().as_ref(), "--symbol", "bad"])
        .assert()
        .code(9);
    Ok(())
}

#[test]
fn print_schema_and_list_errors_work() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .args(["--print-schema", "json"])
        .assert()
        .success();
    Command::cargo_bin("axt-slice")?
        .env("AXT_OUTPUT", "human")
        .arg("--list-errors")
        .assert()
        .success();
    Ok(())
}
