use std::{fs, io, path::PathBuf, process::Command as StdCommand};

use assert_cmd::Command;
use insta::assert_snapshot;
use serde_json::Value;

fn run_bin_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(Command::cargo_bin("axt-run")?.get_program()))
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.run.v1.schema.json"))?;
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

fn validate_jsonl_records(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    for line in stdout.lines() {
        let value: Value = serde_json::from_str(line)?;
        if value.get("schema").and_then(Value::as_str).is_none() {
            return Err(io::Error::other("jsonl record is missing schema").into());
        }
    }
    Ok(())
}

fn normalize_ms(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes.get(index..index + 3) == Some(b"ms=") {
            normalized.push_str("ms=0");
            index += 3;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            continue;
        }
        if bytes.get(index..index + 5) == Some(b"\"ms\":") {
            normalized.push_str("\"ms\":0");
            index += 5;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            continue;
        }
        if bytes[index].is_ascii_digit() {
            let start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if bytes.get(index..index + 2) == Some(b"ms") {
                normalized.push_str("<ms>");
            } else {
                normalized.push_str(&text[start..index]);
            }
        } else {
            normalized.push(char::from(bytes[index]));
            index += 1;
        }
    }
    normalized
}

fn normalize_run_text(text: &str, helper: &std::path::Path) -> String {
    normalize_ms(&normalize_helper_path(text, helper))
}

fn normalize_helper_path(text: &str, helper: &std::path::Path) -> String {
    let direct = text.replace(&helper.to_string_lossy().to_string(), "<helper>");
    let helper_names = ["axt_run_helper.exe", "axt_run_helper"];
    let mut normalized = direct;
    for helper_name in helper_names {
        normalized = replace_helper_name_paths(&normalized, helper_name);
    }
    normalized
}

fn replace_helper_name_paths(text: &str, helper_name: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut search_from = 0;
    while let Some(relative_index) = text[search_from..].find(helper_name) {
        let helper_start = search_from + relative_index;
        let helper_end = helper_start + helper_name.len();
        let path_start = text[..helper_start]
            .rfind(['"', '[', ',', ' ', '\n'])
            .map_or(0, |index| index + 1);
        if path_start >= helper_start {
            search_from = helper_end;
            continue;
        }
        output.push_str(&text[cursor..path_start]);
        output.push_str("<helper>");
        cursor = helper_end;
        search_from = helper_end;
    }
    output.push_str(&text[cursor..]);
    output
}

#[test]
fn json_mode_runs_command_and_validates_schema() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = run_bin_path()?;
    let assert = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json", "--no-save", "--"])
        .arg(bin)
        .args(["--print-schema", "human"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["schema"], "axt.run.v1");
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["exit"], 0);
    assert_eq!(value["data"]["stdout"]["lines"], 1);
    Ok(())
}

#[test]
fn non_zero_command_exits_with_command_failed() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = run_bin_path()?;
    let assert = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json", "--no-save", "--"])
        .arg(bin)
        .assert()
        .code(11);
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["ok"], false);
    assert_eq!(value["errors"][0]["code"], "command_failed");
    Ok(())
}

#[test]
fn env_flag_is_passed_to_child() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let helper = compile_helper(temp.path())?;
    let assert = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json", "--no-save", "--env", "AXT_RUN_TEST=ok", "--"])
        .arg(helper)
        .arg("env")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["stdout"]["lines"], 1);
    Ok(())
}

#[test]
fn no_watch_files_disables_file_change_detection() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("modified.txt"), "before\n")?;
    fs::write(temp.path().join("deleted.txt"), "delete me\n")?;
    let helper = compile_helper(temp.path())?;
    let assert = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json", "--no-save", "--no-watch-files", "--"])
        .arg(helper)
        .arg("change")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["changed_count"], 0);
    Ok(())
}

#[test]
fn saved_runs_can_be_listed_and_shown() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let helper = compile_helper(temp.path())?;
    Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--save", "named", "--"])
        .arg(&helper)
        .args(["echo", "hello"])
        .assert()
        .success();

    let list = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json-data", "list"])
        .assert()
        .success();
    let list_value: Value = serde_json::from_slice(&list.get_output().stdout)?;
    assert_eq!(list_value["runs"][0]["name"], "named");

    let show = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["show", "named", "--stdout"])
        .assert()
        .success();
    assert_eq!(
        String::from_utf8(show.get_output().stdout.clone())?,
        "hello\n"
    );
    Ok(())
}

#[test]
fn json_subcommands_validate_schema() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let helper = compile_helper(temp.path())?;
    Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--save", "named", "--"])
        .arg(&helper)
        .args(["echo", "hello"])
        .assert()
        .success();

    for args in [
        vec!["--json", "list"],
        vec!["--json", "show", "named"],
        vec!["--json", "show", "named", "--stdout"],
        vec!["--json", "clean", "--older-than", "999d"],
    ] {
        let assert = Command::cargo_bin("axt-run")?
            .current_dir(temp.path())
            .args(args)
            .assert()
            .success();
        validate_json_schema(&String::from_utf8(assert.get_output().stdout.clone())?)?;
    }
    Ok(())
}

#[test]
fn file_watching_reports_created_modified_and_deleted() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("modified.txt"), "before\n")?;
    fs::write(temp.path().join("deleted.txt"), "delete me\n")?;
    let helper = compile_helper(temp.path())?;
    let assert = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json", "--no-save", "--"])
        .arg(helper)
        .arg("change")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    let value: Value = serde_json::from_str(&stdout)?;
    let actions = value["data"]["changed"]
        .as_array()
        .ok_or_else(|| io::Error::other("changed was not an array"))?
        .iter()
        .map(|item| {
            (
                item["path"].as_str().unwrap_or_default().to_owned(),
                item["action"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert!(actions.contains(&("created.txt".to_owned(), "created".to_owned())));
    assert!(actions.contains(&("modified.txt".to_owned(), "modified".to_owned())));
    assert!(actions.contains(&("deleted.txt".to_owned(), "deleted".to_owned())));
    Ok(())
}

#[cfg(unix)]
#[test]
fn fixture_script_reports_created_modified_and_deleted() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    fs::write(temp.path().join("modified.txt"), "before\n")?;
    fs::write(temp.path().join("deleted.txt"), "delete me\n")?;
    let script = format!(
        "{}/../../fixtures/runs/change-files.sh",
        env!("CARGO_MANIFEST_DIR")
    );
    let assert = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json", "--no-save", "--shell", "--"])
        .arg(format!("sh {script}"))
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["changed_count"], 3);
    Ok(())
}

#[test]
fn timeout_exits_with_timeout_code() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let helper = compile_helper(temp.path())?;
    let assert = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json", "--no-save", "--timeout", "10ms", "--"])
        .arg(helper)
        .arg("sleep")
        .assert()
        .code(5);
    let stdout = String::from_utf8(assert.get_output().stdout.clone())?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["errors"][0]["code"], "timeout");
    Ok(())
}

#[test]
fn deterministic_fixture_modes_match_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let helper = compile_helper(temp.path())?;

    let human = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--no-save", "--no-watch-files", "--"])
        .arg(&helper)
        .args(["echo", "hello"])
        .assert()
        .success();
    assert_snapshot!(
        "run_human_echo",
        normalize_run_text(
            &String::from_utf8(human.get_output().stdout.clone())?,
            &helper
        )
    );

    let agent = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--agent", "--no-save", "--no-watch-files", "--"])
        .arg(&helper)
        .args(["false"])
        .assert()
        .code(11);
    assert_snapshot!(
        "run_agent_false",
        normalize_run_text(
            &String::from_utf8(agent.get_output().stdout.clone())?,
            &helper
        )
    );

    let jsonl = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--jsonl", "--no-save", "--no-watch-files", "--"])
        .arg(&helper)
        .args(["echo", "hello"])
        .assert()
        .success();
    let stdout = String::from_utf8(jsonl.get_output().stdout.clone())?;
    validate_jsonl_records(&stdout)?;
    assert_snapshot!("run_jsonl_echo", normalize_run_text(&stdout, &helper));
    Ok(())
}

#[test]
fn clean_removes_old_runs() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let helper = compile_helper(temp.path())?;
    Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--save", "old", "--"])
        .arg(helper)
        .args(["echo", "old"])
        .assert()
        .success();
    let clean = Command::cargo_bin("axt-run")?
        .current_dir(temp.path())
        .args(["--json-data", "clean", "--older-than", "0s"])
        .assert()
        .success();
    let value: Value = serde_json::from_slice(&clean.get_output().stdout)?;
    assert_eq!(value["removed"], 1);
    Ok(())
}

fn compile_helper(dir: &std::path::Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source = dir.join("helper.rs");
    let binary = dir.join(if cfg!(windows) {
        "axt_run_helper.exe"
    } else {
        "axt_run_helper"
    });
    fs::write(
        &source,
        r#"
use std::{env, fs, thread, time::Duration};

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("env") => println!("{}", env::var("AXT_RUN_TEST").unwrap_or_default()),
        Some("echo") => println!("{}", args.next().unwrap_or_default()),
        Some("false") => std::process::exit(1),
        Some("change") => {
            let _ = fs::write("created.txt", "created\n");
            let _ = fs::write("modified.txt", "after\n");
            let _ = fs::remove_file("deleted.txt");
        }
        Some("sleep") => thread::sleep(Duration::from_secs(1)),
        _ => std::process::exit(2),
    }
}
"#,
    )?;
    let status = StdCommand::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .status()?;
    if !status.success() {
        return Err(io::Error::other("failed to compile helper").into());
    }
    Ok(binary)
}

#[test]
fn shell_fixture_script_is_valid_on_unix() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(unix) {
        let status = StdCommand::new("sh")
            .arg("-n")
            .arg(format!(
                "{}/../../fixtures/runs/change-files.sh",
                env!("CARGO_MANIFEST_DIR")
            ))
            .status()?;
        assert!(status.success());
    }
    Ok(())
}
