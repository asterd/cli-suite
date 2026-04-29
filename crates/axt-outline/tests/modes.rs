use std::{fs, io};

use assert_cmd::Command;
use insta::assert_snapshot;
use serde_json::Value;
use tempfile::TempDir;

fn fixture(name: &str) -> String {
    format!(
        "{}/../../fixtures/axt-outline/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.outline.v1.schema.json"))?;
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

#[test]
fn all_output_modes_work() -> Result<(), Box<dyn std::error::Error>> {
    for mode in ["--plain", "--json", "--json-data", "--jsonl", "--agent"] {
        let assert = Command::cargo_bin("axt-outline")?
            .args([mode, &fixture("src/lib.rs"), "--sort", "source"])
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
    let human = Command::cargo_bin("axt-outline")?
        .args([&fixture("src/lib.rs"), "--sort", "source"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_human",
        String::from_utf8(human.get_output().stdout.clone())?
    );

    let plain = Command::cargo_bin("axt-outline")?
        .args(["--plain", &fixture("src/lib.rs"), "--sort", "source"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_plain",
        String::from_utf8(plain.get_output().stdout.clone())?
    );

    let envelope = Command::cargo_bin("axt-outline")?
        .args(["--json", &fixture("src/lib.rs"), "--sort", "source"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_json",
        String::from_utf8(envelope.get_output().stdout.clone())?
    );

    let json = Command::cargo_bin("axt-outline")?
        .args(["--json-data", &fixture("src/lib.rs"), "--sort", "source"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_json_data",
        String::from_utf8(json.get_output().stdout.clone())?
    );

    let jsonl = Command::cargo_bin("axt-outline")?
        .args(["--jsonl", &fixture("src/lib.rs"), "--sort", "source"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_jsonl",
        String::from_utf8(jsonl.get_output().stdout.clone())?
    );

    let agent = Command::cargo_bin("axt-outline")?
        .args(["--agent", &fixture("src/lib.rs"), "--sort", "source"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_agent",
        String::from_utf8(agent.get_output().stdout.clone())?
    );
    Ok(())
}

#[test]
fn extracts_rust_visibility_docs_ranges_and_parents() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-outline")?
        .args(["--json-data", &fixture("src/lib.rs"), "--sort", "source"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    let source_bytes = value["summary"]["source_bytes"].as_u64().unwrap_or(0);
    let signature_bytes = value["summary"]["signature_bytes"].as_u64().unwrap_or(0);
    assert!(source_bytes > 0);
    assert!(signature_bytes > 0);
    assert!(signature_bytes < source_bytes);
    let symbols = value["symbols"]
        .as_array()
        .ok_or_else(|| io::Error::other("symbols must be an array"))?;
    assert!(symbols.iter().any(|symbol| {
        symbol["kind"] == "struct"
            && symbol["name"] == "Widget"
            && symbol["visibility"] == "pub"
            && symbol["docs"] == "Public widget data."
            && symbol["range"]["start_line"] == 6
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["kind"] == "fn" && symbol["name"] == "new" && symbol["parent"] == "Widget"
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["kind"] == "enum" && symbol["name"] == "Mode" && symbol["visibility"] == "crate"
    }));
    Ok(())
}

#[test]
fn extracts_supported_language_outlines() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-outline")?
        .args(["--json-data", &fixture("langs"), "--sort", "path"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["summary"]["files"], 6);
    let symbols = value["symbols"]
        .as_array()
        .ok_or_else(|| io::Error::other("symbols must be an array"))?;
    for (language, name) in [
        ("typescript", "ApiUserService"),
        ("javascript", "Runner"),
        ("python", "UserRepository"),
        ("go", "NewStore"),
        ("java", "Service"),
        ("php", "Service"),
    ] {
        assert!(
            symbols
                .iter()
                .any(|symbol| symbol["language"] == language && symbol["name"] == name),
            "{language}:{name}"
        );
    }
    assert!(symbols.iter().any(|symbol| {
        symbol["language"] == "php" && symbol["name"] == "load" && symbol["parent"] == "Service"
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["language"] == "java"
            && symbol["name"] == "cacheKey"
            && symbol["visibility"] == "private"
    }));
    assert!(symbols.iter().any(|symbol| {
        symbol["language"] == "python"
            && symbol["name"] == "_cache_key"
            && symbol["visibility"] == "private"
    }));
    Ok(())
}

#[test]
fn tree_sitter_traversal_prunes_function_bodies() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let file = temp.path().join("body.rs");
    fs::write(
        &file,
        r#"
pub fn outer() {
    fn body_local() {}
}

pub struct Exposed;
"#,
    )?;
    let assert = Command::cargo_bin("axt-outline")?
        .args(["--json-data", file.to_string_lossy().as_ref()])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(stdout.contains("\"name\":\"outer\""));
    assert!(stdout.contains("\"name\":\"Exposed\""));
    assert!(!stdout.contains("body_local"));
    Ok(())
}

#[test]
fn supported_language_outputs_match_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let json = Command::cargo_bin("axt-outline")?
        .args(["--json-data", &fixture("langs"), "--sort", "path"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_languages_json_data",
        String::from_utf8(json.get_output().stdout.clone())?
    );

    let agent = Command::cargo_bin("axt-outline")?
        .args(["--agent", &fixture("langs"), "--sort", "path"])
        .assert()
        .success();
    assert_snapshot!(
        "outline_languages_agent",
        String::from_utf8(agent.get_output().stdout.clone())?
    );
    Ok(())
}

#[test]
fn language_filter_selects_only_requested_language() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-outline")?
        .args(["--json-data", "--lang", "python", &fixture("langs")])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["summary"]["files"], 1);
    assert!(stdout.contains("\"language\":\"python\""));
    assert!(!stdout.contains("\"language\":\"typescript\""));
    Ok(())
}

#[test]
fn unsupported_files_are_warnings_in_mixed_input() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-outline")?
        .args(["--json-data", &fixture("")])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert!(value["warnings"].as_array().is_some_and(|warnings| warnings
        .iter()
        .any(|warning| warning["code"] == "unsupported_language")));
    Ok(())
}

#[test]
fn unsupported_only_exits_feature_unsupported() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-outline")?
        .args([&fixture("README.md")])
        .assert()
        .code(9);
    Ok(())
}

#[test]
fn parse_errors_are_reported_as_warnings() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-outline")?
        .args(["--json-data", &fixture("src/lib.rs"), &fixture("broken.rs")])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert!(value["warnings"].as_array().is_some_and(|warnings| warnings
        .iter()
        .any(|warning| warning["code"] == "parse_error")));
    Ok(())
}

#[test]
fn truncation_and_strict_are_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let truncated = Command::cargo_bin("axt-outline")?
        .args([
            "--agent",
            "--limit",
            "2",
            &fixture("src/lib.rs"),
            "--sort",
            "source",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(truncated.get_output().stdout.clone())?;
    assert!(stdout.contains("W code=truncated"));

    Command::cargo_bin("axt-outline")?
        .args([
            "--agent",
            "--limit",
            "2",
            "--strict",
            &fixture("src/lib.rs"),
            "--sort",
            "source",
        ])
        .assert()
        .code(6);
    Ok(())
}

#[test]
fn max_bytes_truncation_is_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-outline")?
        .args([
            "--jsonl",
            "--max-bytes",
            "80",
            &fixture("src/lib.rs"),
            "--sort",
            "source",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(stdout.contains("\"code\":\"truncated\""));
    Ok(())
}

#[test]
fn print_schema_and_list_errors_work() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-outline")?
        .args(["--print-schema", "json"])
        .assert()
        .success();
    Command::cargo_bin("axt-outline")?
        .arg("--list-errors")
        .assert()
        .success();
    Ok(())
}

#[test]
fn public_only_filters_private_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-outline")?
        .args([
            "--json-data",
            "--public-only",
            &fixture("src/lib.rs"),
            "--sort",
            "source",
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(!stdout.contains("\"name\":\"id\""));
    assert!(stdout.contains("\"name\":\"new\""));
    Ok(())
}

#[test]
fn walks_directories_deterministically() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("src/nested"))?;
    fs::write(temp.path().join("src/a.rs"), "pub fn a() {}\n")?;
    fs::write(temp.path().join("src/nested/b.rs"), "pub fn b() {}\n")?;
    let assert = Command::cargo_bin("axt-outline")?
        .args(["--json-data", temp.path().to_string_lossy().as_ref()])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["summary"]["files"], 2);
    assert_eq!(value["summary"]["symbols"], 2);
    Ok(())
}
