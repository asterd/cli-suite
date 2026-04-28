use std::{
    io::{self, BufRead, BufReader},
    net::TcpListener,
    process::{Child, Command as StdCommand, Stdio},
    time::Duration,
};

use assert_cmd::Command;
use insta::assert_snapshot;
use serde_json::Value;

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.port.v1.schema.json"))?;
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
            "axt.port.summary.v1" => {
                include_str!("../../../schemas/axt.port.summary.v1.schema.json")
            }
            "axt.port.socket.v1" => {
                include_str!("../../../schemas/axt.port.socket.v1.schema.json")
            }
            "axt.port.holder.v1" => {
                include_str!("../../../schemas/axt.port.holder.v1.schema.json")
            }
            "axt.port.action.v1" => {
                include_str!("../../../schemas/axt.port.action.v1.schema.json")
            }
            "axt.port.warn.v1" => include_str!("../../../schemas/axt.port.warn.v1.schema.json"),
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

#[test]
fn who_reports_known_listener() -> Result<(), Box<dyn std::error::Error>> {
    let listener = ListenerChild::spawn()?;
    let port = listener.port.to_string();

    let assert = Command::cargo_bin("axt-port")?
        .args(["--json", "who", &port])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["schema"], "axt.port.v1");
    assert_eq!(value["data"]["action"], "who");
    assert_eq!(value["data"]["held"], true);
    assert_eq!(value["data"]["holders"][0]["port"], listener.port);
    Ok(())
}

#[test]
fn list_jsonl_and_agent_have_schema_first() -> Result<(), Box<dyn std::error::Error>> {
    let jsonl = Command::cargo_bin("axt-port")?
        .args(["--jsonl", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(jsonl.get_output().stdout.clone())?;
    validate_jsonl_schemas(&stdout)?;
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("jsonl output was empty"))?;
    let value: Value = serde_json::from_str(first)?;
    assert_eq!(value["schema"], "axt.port.summary.v1");

    let agent = Command::cargo_bin("axt-port")?
        .args(["--agent", "list"])
        .assert()
        .success();
    let stdout = String::from_utf8(agent.get_output().stdout.clone())?;
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("agent output was empty"))?;
    assert!(first.starts_with("schema=axt.port.agent.v1 "));
    Ok(())
}

#[test]
fn fixture_jsonl_and_agent_match_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let listener = ListenerChild::spawn()?;
    let port = listener.port.to_string();

    let jsonl = Command::cargo_bin("axt-port")?
        .args(["--jsonl", "free", &port, "--dry-run"])
        .assert()
        .success();
    let jsonl_stdout = String::from_utf8(jsonl.get_output().stdout.clone())?;
    validate_jsonl_schemas(&jsonl_stdout)?;
    assert_snapshot!(
        "port_jsonl_free_dry_run",
        &normalize_jsonl_fixture_output(&jsonl_stdout)?,
    );

    let agent = Command::cargo_bin("axt-port")?
        .args(["--agent", "free", &port, "--dry-run"])
        .assert()
        .success();
    let agent_stdout = String::from_utf8(agent.get_output().stdout.clone())?;
    assert_snapshot!(
        "port_agent_free_dry_run",
        &normalize_agent_fixture_output(&agent_stdout)
    );
    Ok(())
}

#[test]
fn dry_run_reports_simulated_attempt_without_killing() -> Result<(), Box<dyn std::error::Error>> {
    let listener = ListenerChild::spawn()?;
    let port = listener.port.to_string();

    let assert = Command::cargo_bin("axt-port")?
        .args(["--json", "free", &port, "--dry-run"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["data"]["freed"], false);
    assert_eq!(value["data"]["attempts"][0]["action"], "simulated");
    assert_eq!(value["data"]["attempts"][0]["result"], "skipped");
    assert!(listener.child_id() > 0);
    Ok(())
}

#[test]
fn free_kills_known_listener() -> Result<(), Box<dyn std::error::Error>> {
    let mut listener = ListenerChild::spawn()?;
    let port = listener.port.to_string();

    let assert = Command::cargo_bin("axt-port")?
        .args(["--json", "free", &port, "--signal", "kill"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["data"]["freed"], true);
    assert_eq!(value["data"]["attempts"][0]["result"], "freed");
    let _ = listener.child.wait();
    Ok(())
}

#[test]
fn watch_reports_free_port_without_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let port = free_port()?.to_string();
    let assert = Command::cargo_bin("axt-port")?
        .args(["--json", "watch", &port, "--timeout", "100ms"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["held"], false);
    assert_eq!(value["data"]["freed"], true);
    assert_eq!(value["data"]["timed_out"], false);
    Ok(())
}

#[test]
fn watch_times_out_while_port_stays_held() -> Result<(), Box<dyn std::error::Error>> {
    let listener = ListenerChild::spawn()?;
    let port = listener.port.to_string();
    let assert = Command::cargo_bin("axt-port")?
        .args(["--json", "watch", &port, "--timeout", "100ms"])
        .assert()
        .failure();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;

    assert_eq!(value["ok"], false);
    assert_eq!(value["data"]["held"], true);
    assert_eq!(value["data"]["timed_out"], true);
    assert_eq!(value["errors"][0]["code"], "timeout");
    Ok(())
}

#[test]
fn refuses_remote_host_port_syntax() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-port")?
        .args(["who", "example.com:443"])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn print_schema_outputs_json_schema() -> Result<(), Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-port")?
        .args(["--print-schema"])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&assert.get_output().stdout)?;
    assert_eq!(value["properties"]["schema"]["const"], "axt.port.v1");
    Ok(())
}

struct ListenerChild {
    port: u16,
    child: Child,
}

impl ListenerChild {
    fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let exe = Command::cargo_bin("axt-port")?.get_program().to_owned();
        let mut child = StdCommand::new(exe)
            .env("AXT_PORT_LISTENER_FIXTURE", "1")
            .stdout(Stdio::piped())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("fixture stdout missing"))?;
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line)?;
        let port = line.trim().parse::<u16>()?;
        std::thread::sleep(Duration::from_millis(200));
        Ok(Self { port, child })
    }

    fn child_id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for ListenerChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn normalize_jsonl_fixture_output(stdout: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut lines = Vec::new();
    for line in stdout.lines() {
        let mut value: Value = serde_json::from_str(line)?;
        normalize_json_value(&mut value);
        lines.push(serde_json::to_string(&value)?);
    }
    Ok(lines.join("\n"))
}

fn normalize_json_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(ports)) = map.get_mut("ports") {
                for port in ports {
                    *port = Value::String("<port>".to_owned());
                }
            }
            for key in ["port", "pid", "duration_ms", "ms", "mem"] {
                if map.contains_key(key) {
                    map.insert(key.to_owned(), Value::String(format!("<{key}>")));
                }
            }
            for key in ["process", "name", "cmd", "cwd", "owner"] {
                if map.contains_key(key) {
                    map.insert(key.to_owned(), Value::String(format!("<{key}>")));
                }
            }
            if let Some(Value::Array(bound)) = map.get_mut("bound") {
                for item in bound {
                    *item = Value::String("<bound>".to_owned());
                }
            } else if map.contains_key("bound") {
                map.insert("bound".to_owned(), Value::String("<bound>".to_owned()));
            }
            for child in map.values_mut() {
                normalize_json_value(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_json_value(item);
            }
        }
        _ => {}
    }
}

fn normalize_agent_fixture_output(stdout: &str) -> String {
    stdout
        .split_whitespace()
        .map(|field| {
            if field.starts_with("port=") {
                "port=<port>".to_owned()
            } else if field.starts_with("pid=") {
                "pid=<pid>".to_owned()
            } else if field.starts_with("name=") {
                "name=<name>".to_owned()
            } else if field.starts_with("cmd=") {
                "cmd=<cmd>".to_owned()
            } else if field.starts_with("cwd=") {
                "cwd=<cwd>".to_owned()
            } else if field.starts_with("bound=") {
                "bound=<bound>".to_owned()
            } else if field.starts_with("owner=") {
                "owner=<owner>".to_owned()
            } else if field.starts_with("mem=") {
                "mem=<mem>".to_owned()
            } else if field.starts_with("ms=") {
                "ms=<ms>".to_owned()
            } else {
                field.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
