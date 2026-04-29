use std::{fs, io};

use assert_cmd::Command;
use insta::assert_snapshot;
use serde_json::Value;
use tempfile::TempDir;

fn fixture(name: &str) -> String {
    format!(
        "{}/../../fixtures/axt-ctxpack/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.ctxpack.v1.schema.json"))?;
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
fn all_output_modes_work() -> Result<(), Box<dyn std::error::Error>> {
    for mode in ["--json", "--agent"] {
        let assert = Command::cargo_bin("axt-ctxpack")?
            .env("AXT_OUTPUT", "human")
            .args([
                mode,
                "--pattern",
                "todo=TODO",
                "--include",
                "**/*.rs",
                &fixture("src"),
            ])
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
    let human = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--pattern",
            "todo=TODO",
            "--pattern",
            "unwrap=unwrap_or",
            "--include",
            "**/*.rs",
            "--context",
            "1",
            &fixture("src"),
        ])
        .assert()
        .success();
    assert_snapshot!(
        "ctxpack_human",
        String::from_utf8(human.get_output().stdout.clone())?
    );

    let envelope = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--pattern",
            "todo=TODO",
            "--include",
            "**/*.rs",
            &fixture("src"),
        ])
        .assert()
        .success();
    assert_snapshot!(
        "ctxpack_json",
        String::from_utf8(envelope.get_output().stdout.clone())?
    );

    let data = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--pattern",
            "todo=TODO",
            "--include",
            "**/*.rs",
            &fixture("src"),
        ])
        .assert()
        .success();
    assert_snapshot!(
        "ctxpack_json_data",
        json_data_string(&String::from_utf8(data.get_output().stdout.clone())?)?
    );

    let jsonl = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--agent",
            "--pattern",
            "todo=TODO",
            "--include",
            "**/*.rs",
            &fixture("src"),
        ])
        .assert()
        .success();
    assert_snapshot!(
        "ctxpack_jsonl",
        String::from_utf8(jsonl.get_output().stdout.clone())?
    );

    let agent = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--agent",
            "--pattern",
            "todo=TODO",
            "--include",
            "**/*.rs",
            &fixture("src"),
        ])
        .assert()
        .success();
    assert_snapshot!(
        "ctxpack_agent",
        String::from_utf8(agent.get_output().stdout.clone())?
    );
    Ok(())
}

#[test]
fn named_patterns_and_overlapping_hits_are_reported() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--pattern",
            "alpha=alpha",
            "--pattern",
            "word=alpha beta",
            &fixture("src/notes.txt"),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["patterns"].as_array().map_or(0, Vec::len), 2);
    let hits = value["hits"]
        .as_array()
        .ok_or_else(|| io::Error::other("hits must be an array"))?;
    assert!(hits.iter().any(|hit| hit["pattern"] == "alpha"));
    assert!(hits.iter().any(|hit| hit["pattern"] == "word"));
    Ok(())
}

#[test]
fn rust_hits_are_classified_with_tree_sitter_ast() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--pattern",
            "todo=TODO",
            "--pattern",
            "unwrap=unwrap_or",
            &fixture("src/lib.rs"),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    let hits = value["hits"]
        .as_array()
        .ok_or_else(|| io::Error::other("hits must be an array"))?;

    assert!(hits.iter().any(|hit| {
        hit["pattern"] == "todo"
            && hit["kind"] == "comment"
            && hit["classification_source"] == "ast"
            && hit["node_kind"] == "line_comment"
            && hit["language"] == "rust"
    }));
    assert!(hits.iter().any(|hit| {
        hit["pattern"] == "todo"
            && hit["kind"] == "test"
            && hit["classification_source"] == "ast"
            && hit["enclosing_symbol"] == "todo_test"
            && hit["ast_path"]
                .as_array()
                .is_some_and(|path| path.iter().any(|kind| kind == "function_item"))
    }));
    assert!(hits.iter().any(|hit| {
        hit["pattern"] == "unwrap"
            && hit["kind"] == "code"
            && hit["node_kind"] == "field_identifier"
            && hit["enclosing_symbol"] == "unwrap_value"
    }));
    Ok(())
}

#[test]
fn no_hits_is_successful() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--pattern",
            "missing=DOES_NOT_EXIST",
            &fixture("src/empty.txt"),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["summary"]["hits"], 0);
    assert_eq!(value["hits"].as_array().map_or(usize::MAX, Vec::len), 0);
    Ok(())
}

#[test]
fn hidden_and_ignored_files_are_skipped_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    fs::write(root.join(".ignore"), "ignored.txt\n")?;
    fs::write(root.join("visible.txt"), "TODO visible\n")?;
    fs::write(root.join(".hidden.txt"), "TODO hidden\n")?;
    fs::write(root.join("ignored.txt"), "TODO ignored\n")?;
    let root = root.to_string_lossy();

    let assert = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args(["--json", "--pattern", "todo=TODO", root.as_ref()])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    assert!(stdout.contains("visible.txt"));
    assert!(!stdout.contains(".hidden.txt"));
    assert!(!stdout.contains("ignored.txt"));

    let with_hidden = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--hidden",
            "--no-ignore",
            "--pattern",
            "todo=TODO",
            root.as_ref(),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(with_hidden.get_output().stdout.clone())?;
    assert!(stdout.contains(".hidden.txt"));
    assert!(stdout.contains("ignored.txt"));
    Ok(())
}

#[test]
fn binary_files_are_skipped_with_warning() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args(["--json", "--pattern", "abc=abc", &fixture("binary.bin")])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["summary"]["hits"], 0);
    assert!(value["warnings"].as_array().is_some_and(|warnings| warnings
        .iter()
        .any(|warning| warning["code"] == "binary_skipped")));
    Ok(())
}

#[test]
fn snippets_include_requested_context() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--pattern",
            "unwrap=unwrap_or",
            "--context",
            "1",
            &fixture("src/lib.rs"),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    let snippet = value["hits"][0]["snippet"]
        .as_str()
        .ok_or_else(|| io::Error::other("snippet must be string"))?;
    assert!(snippet.contains("pub fn unwrap_value"));
    assert!(snippet.contains("value.unwrap_or"));
    assert!(snippet.contains("}"));
    Ok(())
}

#[test]
fn truncates_by_limit() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--limit",
            "1",
            "--pattern",
            "todo=TODO",
            &fixture("src/lib.rs"),
        ])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["summary"]["hits"], 1);
    assert_eq!(value["summary"]["truncated"], true);
    Ok(())
}

#[test]
fn strict_jsonl_truncation_exits_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--agent",
            "--limit",
            "1",
            "--strict",
            "--pattern",
            "todo=TODO",
            &fixture("src/lib.rs"),
        ])
        .assert()
        .code(6);
    Ok(())
}

#[test]
fn print_schema_and_list_errors_work() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args(["--print-schema", "json"])
        .assert()
        .success();
    Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .arg("--list-errors")
        .assert()
        .success();
    Ok(())
}

#[test]
fn invalid_pattern_is_usage_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let file = temp.path().join("sample.txt");
    fs::write(&file, "TODO")?;
    Command::cargo_bin("axt-ctxpack")?
        .env("AXT_OUTPUT", "human")
        .args([
            "--json",
            "--pattern",
            "broken",
            file.to_string_lossy().as_ref(),
        ])
        .assert()
        .code(2);
    Ok(())
}
