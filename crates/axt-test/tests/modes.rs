use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

#[cfg(unix)]
use std::{
    io::{BufRead, BufReader},
    process::Stdio,
};

use assert_cmd::Command;
use insta::assert_snapshot;
use serde_json::Value;
use tempfile::TempDir;

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.test.v1.schema.json"))?;
    let instance: Value = serde_json::from_str(stdout)?;
    validate_against_schema(&schema, &instance)
}

fn validate_jsonl_schemas(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    for line in stdout.lines() {
        let record: Value = serde_json::from_str(line)?;
        let schema_name = record
            .get("schema")
            .and_then(Value::as_str)
            .ok_or_else(|| io::Error::other("jsonl record missing schema"))?;
        let schema_text = match schema_name {
            "axt.test.summary.v1" => {
                include_str!("../../../schemas/axt.test.summary.v1.schema.json")
            }
            "axt.test.suite.v1" => include_str!("../../../schemas/axt.test.suite.v1.schema.json"),
            "axt.test.case.v1" => include_str!("../../../schemas/axt.test.case.v1.schema.json"),
            "axt.test.framework.v1" => {
                include_str!("../../../schemas/axt.test.framework.v1.schema.json")
            }
            "axt.test.warn.v1" => include_str!("../../../schemas/axt.test.warn.v1.schema.json"),
            "axt.test.frameworks.summary.v1" => continue,
            other => return Err(io::Error::other(format!("unknown jsonl schema {other}")).into()),
        };
        let schema: Value = serde_json::from_str(schema_text)?;
        validate_against_schema(&schema, &record)?;
    }
    Ok(())
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

#[test]
fn list_frameworks_reports_supported_frontends() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-test")?
        .env("AXT_OUTPUT", "human")
        .args(["--agent", "list-frameworks"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_jsonl_schemas(&stdout)?;
    assert!(stdout.contains("\"name\":\"jest\""));
    assert!(stdout.contains("\"name\":\"deno\""));
    Ok(())
}

#[test]
fn json_output_validates_against_schema() -> Result<(), Box<dyn std::error::Error>> {
    let tools = FakeTools::new()?;
    let assert = Command::cargo_bin("axt-test")?
        .env("AXT_OUTPUT", "human")
        .current_dir(fixture_path("jest"))
        .env("PATH", tools.path_value()?)
        .args(["--json", "--framework", "jest"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["schema"], "axt.test.v1");
    assert_eq!(value["ok"], false);
    assert_eq!(value["data"]["data"]["failed"], 1);
    Ok(())
}

#[test]
fn fixture_jsonl_and_agent_match_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let tools = FakeTools::new()?;
    let jsonl = Command::cargo_bin("axt-test")?
        .env("AXT_OUTPUT", "human")
        .current_dir(fixture_path("jest"))
        .env("PATH", tools.path_value()?)
        .args(["--agent", "--framework", "jest"])
        .assert()
        .failure();
    let jsonl_stdout = String::from_utf8(jsonl.get_output().stdout.clone())?;
    validate_jsonl_schemas(&jsonl_stdout)?;
    assert_snapshot!("test_jsonl_jest", normalize_jsonl(&jsonl_stdout)?);

    let agent = Command::cargo_bin("axt-test")?
        .env("AXT_OUTPUT", "human")
        .current_dir(fixture_path("jest"))
        .env("PATH", tools.path_value()?)
        .args(["--agent", "--framework", "jest"])
        .assert()
        .failure();
    let agent_stdout = String::from_utf8(agent.get_output().stdout.clone())?;
    assert_snapshot!("test_agent_jest", normalize_agent(&agent_stdout));
    Ok(())
}

#[test]
fn all_supported_frameworks_emit_valid_jsonl() -> Result<(), Box<dyn std::error::Error>> {
    let tools = FakeTools::new()?;
    for framework in ["jest", "vitest", "pytest", "cargo", "go", "bun", "deno"] {
        let fixture = fixture_path(framework);
        let assert = Command::cargo_bin("axt-test")?
            .env("AXT_OUTPUT", "human")
            .current_dir(fixture)
            .env("PATH", tools.path_value()?)
            .args(["--agent", "--framework", framework])
            .assert()
            .failure();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
        validate_jsonl_schemas(&stdout)?;
        assert!(stdout.contains("\"status\":\"failed\""), "{framework}");
        assert!(!stdout.contains("\"status\":\"passed\""), "{framework}");
        assert!(!stdout.contains("\"status\":\"skipped\""), "{framework}");
    }
    Ok(())
}

#[test]
fn all_supported_frameworks_agent_output_matches_snapshots(
) -> Result<(), Box<dyn std::error::Error>> {
    let tools = FakeTools::new()?;
    for framework in ["jest", "vitest", "pytest", "cargo", "go", "bun", "deno"] {
        let fixture = fixture_path(framework);
        let assert = Command::cargo_bin("axt-test")?
            .env("AXT_OUTPUT", "human")
            .current_dir(fixture)
            .env("PATH", tools.path_value()?)
            .args(["--agent", "--framework", framework])
            .assert()
            .failure();
        let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
        assert_snapshot!(format!("test_agent_{framework}"), normalize_agent(&stdout));
    }
    Ok(())
}

#[test]
fn config_files_override_marker_detection() -> Result<(), Box<dyn std::error::Error>> {
    let tools = FakeTools::new()?;
    let temp = tempfile::tempdir()?;

    let package_dir = temp.path().join("package-config");
    fs::create_dir_all(&package_dir)?;
    fs::write(
        package_dir.join("package.json"),
        r#"{"scripts":{"test":"jest"},"axt-test":{"framework":"vitest"}}"#,
    )?;
    assert_detected_framework(&package_dir, &tools, "vitest")?;

    let toml_dir = temp.path().join("toml-config");
    fs::create_dir_all(&toml_dir)?;
    fs::write(toml_dir.join("axt-test.toml"), "framework = \"pytest\"\n")?;
    assert_detected_framework(&toml_dir, &tools, "pytest")?;

    let pyproject_dir = temp.path().join("pyproject-config");
    fs::create_dir_all(&pyproject_dir)?;
    fs::write(
        pyproject_dir.join("pyproject.toml"),
        "[tool.axt-test]\nframework = \"go\"\n",
    )?;
    assert_detected_framework(&pyproject_dir, &tools, "go")?;
    Ok(())
}

#[test]
fn changed_files_are_mapped_into_nested_project_roots() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repo");
    let app = root.join("app");
    let tests = app.join("tests");
    fs::create_dir_all(&tests)?;
    fs::write(
        app.join("package.json"),
        r#"{"scripts":{"test":"jest"},"devDependencies":{"jest":"30.0.0"}}"#,
    )?;
    fs::write(tests.join("checkout.test.ts"), "initial\n")?;
    run_git(&root, &["init"])?;
    run_git(&root, &["config", "user.name", "axt tests"])?;
    run_git(
        &root,
        &["config", "user.email", "axt-tests@example.invalid"],
    )?;
    run_git(&root, &["add", "."])?;
    run_git(&root, &["commit", "-m", "initial"])?;
    fs::write(tests.join("checkout.test.ts"), "changed\n")?;

    let tools = FakeTools::with_npm(npm_requires_nested_test_path())?;
    let assert = Command::cargo_bin("axt-test")?
        .env("AXT_OUTPUT", "human")
        .current_dir(&root)
        .env("PATH", tools.path_value()?)
        .args(["--json", "--changed"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["data"]["frameworks"][0], "jest");
    assert_eq!(value["data"]["total"], 3);
    Ok(())
}

#[cfg(unix)]
#[test]
fn jsonl_failures_are_flushed_before_process_exit() -> Result<(), Box<dyn std::error::Error>> {
    let tools = FakeTools::with_npm(npm_slow_after_first_failure())?;
    let mut child = ProcessCommand::new(assert_cmd::cargo::cargo_bin("axt-test"))
        .current_dir(fixture_path("jest"))
        .env("PATH", tools.path_value()?)
        .args(["--agent", "--framework", "jest"])
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("missing child stdout"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    line.clear();
    reader.read_line(&mut line)?;
    assert!(line.contains("\"status\":\"failed\""));
    assert!(
        child.try_wait()?.is_none(),
        "framework process exited before the streaming assertion"
    );
    child.kill()?;
    let _status = child.wait()?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn max_duration_terminates_hung_framework() -> Result<(), Box<dyn std::error::Error>> {
    let tools = FakeTools::with_npm(npm_hangs_without_output())?;
    let assert = Command::cargo_bin("axt-test")?
        .env("AXT_OUTPUT", "human")
        .current_dir(fixture_path("jest"))
        .env("PATH", tools.path_value()?)
        .args(["--json", "--framework", "jest", "--max-duration", "100ms"])
        .assert()
        .failure()
        .code(5);
    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
    assert!(stderr.contains("exceeded max duration"));
    Ok(())
}

struct FakeTools {
    _dir: TempDir,
    bin: PathBuf,
}

impl FakeTools {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_npm(npm_script())
    }

    fn with_npm(npm: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let bin = dir.path().to_path_buf();
        write_tool(&bin, "npm", npm)?;
        write_tool(&bin, "python", json_line_script("pytest"))?;
        write_tool(&bin, "cargo", cargo_script())?;
        write_tool(&bin, "go", go_script())?;
        write_tool(&bin, "bun", json_line_script("bun"))?;
        write_tool(&bin, "deno", json_line_script("deno"))?;
        Ok(Self { _dir: dir, bin })
    }

    fn path_value(&self) -> Result<String, Box<dyn std::error::Error>> {
        let old = env::var_os("PATH").unwrap_or_default();
        let paths = env::split_paths(&old);
        let joined = env::join_paths(std::iter::once(self.bin.clone()).chain(paths))?;
        joined
            .into_string()
            .map_err(|_| io::Error::other("PATH is not valid UTF-8").into())
    }
}

fn assert_detected_framework(
    dir: &Path,
    tools: &FakeTools,
    framework: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-test")?
        .env("AXT_OUTPUT", "human")
        .current_dir(dir)
        .env("PATH", tools.path_value()?)
        .args(["--json"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value = json_data(&stdout)?;
    assert_eq!(value["data"]["frameworks"][0], framework);
    Ok(())
}

fn run_git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = ProcessCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("git {} failed with {status}", args.join(" "))).into())
    }
}

fn write_tool(dir: &Path, name: &str, script: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join(name);
    fs::write(&path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions)?;
    }
    #[cfg(windows)]
    {
        fs::write(dir.join(format!("{name}.cmd")), windows_script(script))?;
    }
    Ok(())
}

fn npm_script() -> &'static str {
    r#"#!/bin/sh
case "$PWD" in
  *vitest*) FW=vitest ;;
  *) FW=jest ;;
esac
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"passed","name":"passes","suite":"checkout flow","file":"tests/checkout.test.ts","line":10,"duration_ms":11}\n'
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"failed","name":"fails","suite":"checkout flow","file":"tests/checkout.test.ts","line":20,"duration_ms":12,"message":"expected 200, got 500","actual":500,"expected":200}\n'
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"skipped","name":"skips","suite":"checkout flow","file":"tests/checkout.test.ts","line":30,"duration_ms":0}\n'
exit 1
"#
}

fn npm_requires_nested_test_path() -> &'static str {
    r#"#!/bin/sh
case "$*" in
  *"tests/checkout.test.ts"*) ;;
  *) echo "missing nested test path: $*" >&2; exit 2 ;;
esac
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"passed","name":"passes","suite":"checkout flow","file":"tests/checkout.test.ts","line":10,"duration_ms":11}\n'
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"failed","name":"fails","suite":"checkout flow","file":"tests/checkout.test.ts","line":20,"duration_ms":12,"message":"expected 200, got 500"}\n'
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"skipped","name":"skips","suite":"checkout flow","file":"tests/checkout.test.ts","line":30,"duration_ms":0}\n'
exit 1
"#
}

#[cfg(unix)]
fn npm_slow_after_first_failure() -> &'static str {
    r#"#!/bin/sh
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"failed","name":"fails early","suite":"checkout flow","file":"tests/checkout.test.ts","line":20,"duration_ms":12,"message":"expected 200, got 500"}\n'
sleep 5
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"passed","name":"passes late","suite":"checkout flow","file":"tests/checkout.test.ts","line":10,"duration_ms":11}\n'
exit 1
"#
}

#[cfg(unix)]
fn npm_hangs_without_output() -> &'static str {
    r#"#!/bin/sh
sleep 5
exit 0
"#
}

fn json_line_script(_framework: &str) -> &'static str {
    r#"#!/bin/sh
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"passed","name":"passes","suite":"suite","file":"tests/sample.test","line":10,"duration_ms":11}\n'
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"failed","name":"fails","suite":"suite","file":"tests/sample.test","line":20,"duration_ms":12,"message":"expected true"}\n'
printf '{"schema":"axt.test.fixture.v1","type":"case","status":"skipped","name":"skips","suite":"suite","file":"tests/sample.test","line":30,"duration_ms":0}\n'
exit 1
"#
}

fn cargo_script() -> &'static str {
    r#"#!/bin/sh
printf 'test tests::passes ... ok\n'
printf 'test tests::fails ... FAILED\n'
printf 'test tests::skips ... ignored\n'
exit 1
"#
}

fn go_script() -> &'static str {
    r#"#!/bin/sh
printf '{"Action":"pass","Package":"example.com/axt","Test":"TestPasses","Elapsed":0.011}\n'
printf '{"Action":"fail","Package":"example.com/axt","Test":"TestFails","Elapsed":0.012}\n'
printf '{"Action":"skip","Package":"example.com/axt","Test":"TestSkips","Elapsed":0}\n'
exit 1
"#
}

#[cfg(windows)]
fn windows_script(script: &str) -> String {
    let mut lines = vec!["@echo off".to_owned()];
    for line in script.lines() {
        if let Some(payload) = line
            .strip_prefix("printf '")
            .and_then(|line| line.strip_suffix("\\n'"))
        {
            lines.push(format!("echo {payload}"));
        }
    }
    lines.push("exit /b 1".to_owned());
    lines.join("\r\n")
}

fn normalize_jsonl(stdout: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut lines = Vec::new();
    for line in stdout.lines() {
        let mut value: Value = serde_json::from_str(line)?;
        normalize_value(&mut value);
        lines.push(serde_json::to_string(&value)?);
    }
    Ok(lines.join("\n"))
}

fn fixture_path(framework: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/axt-test")
        .join(framework)
}

fn normalize_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for key in ["duration_ms", "line"] {
                if map.contains_key(key) {
                    map.insert(key.to_owned(), Value::String(format!("<{key}>")));
                }
            }
            if map.contains_key("started") {
                map.insert("started".to_owned(), Value::String("<started>".to_owned()));
            }
            for child in map.values_mut() {
                normalize_value(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_value(item);
            }
        }
        _ => {}
    }
}

fn normalize_agent(stdout: &str) -> String {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .map(|mut value| {
            normalize_value(&mut value);
            serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_owned())
        })
        .collect::<Vec<_>>()
        .join(" ")
}
