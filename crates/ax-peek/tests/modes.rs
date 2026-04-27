use assert_cmd::Command;
use serde_json::Value;

fn run_ax_peek(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("ax-peek")?.args(args).assert().success();
    Ok(String::from_utf8(assert.get_output().stdout.clone())?)
}

#[test]
fn human_mode_matches_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_ax_peek(&[])?;
    insta::assert_snapshot!(stdout, @"ax-peek stub: Milestone 0 scaffolding only
");
    Ok(())
}

#[test]
fn json_mode_matches_snapshot_and_schema() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_ax_peek(&["--json"])?;
    insta::assert_snapshot!(stdout, @r###"{"schema":"ax.peek.v1","ok":true,"data":{"status":"stub"},"warnings":[],"errors":[]}
"###);

    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/ax.peek.v1.schema.json"))?;
    let instance: Value = serde_json::from_str(&stdout)?;
    let compiled = match jsonschema::JSONSchema::compile(&schema) {
        Ok(compiled) => compiled,
        Err(error) => panic!("schema compile failed: {error}"),
    };
    let validation = compiled.validate(&instance);
    if let Err(errors) = validation {
        let messages = errors
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        panic!("schema validation failed:\n{messages}");
    }

    Ok(())
}

#[test]
fn json_data_mode_preserves_stub_payload() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_ax_peek(&["--json-data"])?;
    insta::assert_snapshot!(stdout, @r###"{"status":"stub"}
"###);
    Ok(())
}

#[test]
fn jsonl_mode_matches_snapshot_and_contract() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_ax_peek(&["--jsonl"])?;
    insta::assert_snapshot!(stdout, @r###"{"schema":"ax.peek.summary.v1","type":"summary","ok":true,"stub":true}
"###);

    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    let first: Value = serde_json::from_str(lines[0])?;
    assert_eq!(
        first.get("schema"),
        Some(&Value::String("ax.peek.summary.v1".to_owned()))
    );
    assert_eq!(
        first.get("type"),
        Some(&Value::String("summary".to_owned()))
    );

    Ok(())
}

#[test]
fn agent_mode_matches_snapshot_and_acf_contract() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_ax_peek(&["--agent"])?;
    insta::assert_snapshot!(stdout, @"schema=ax.peek.agent.v1 ok=true mode=records stub=true truncated=false
");

    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("schema=ax.peek.agent.v1 "));
    assert!(lines[0].contains(" ok=true "));
    assert!(lines[0].contains(" mode=records "));
    assert!(lines[0].contains(" truncated=false"));

    Ok(())
}

#[test]
fn agent_mode_preserves_summary_first_when_truncated() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_ax_peek(&["--agent", "--limit", "0"])?;
    insta::assert_snapshot!(stdout, @r###"schema=ax.peek.agent.v1 ok=true mode=records stub=true truncated=true
W code=truncated reason=max_records truncated=true
"###);

    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("schema=ax.peek.agent.v1 "));
    assert!(lines[0].contains(" truncated=true"));
    assert_eq!(
        lines[1],
        "W code=truncated reason=max_records truncated=true"
    );

    Ok(())
}

#[test]
fn list_errors_outputs_full_catalog_as_jsonl() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_ax_peek(&["--list-errors"])?;
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 15);

    let first: Value = serde_json::from_str(lines[0])?;
    assert_eq!(first.get("code"), Some(&Value::String("ok".to_owned())));
    assert_eq!(first.get("exit"), Some(&Value::Number(0.into())));
    assert!(first.get("meaning").is_some());
    assert_eq!(
        first.get("retryable"),
        Some(&Value::String("n/a".to_owned()))
    );

    let last: Value = serde_json::from_str(lines[14])?;
    assert_eq!(
        last.get("code"),
        Some(&Value::String("network_disabled".to_owned()))
    );

    Ok(())
}

#[test]
fn conflicting_modes_are_rejected_by_clap() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("ax-peek")?
        .args(["--json", "--agent"])
        .assert()
        .failure()
        .code(2);
    Command::cargo_bin("ax-peek")?
        .args(["--jsonl", "--agent"])
        .assert()
        .failure()
        .code(2);
    Ok(())
}
