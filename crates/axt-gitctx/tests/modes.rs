use std::{fs, io, path::Path, process::Command as ProcessCommand};

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

fn run_axt_gitctx(root: &Utf8Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-gitctx")?
        .current_dir(root)
        .env("AXT_OUTPUT", "human")
        .args(args)
        .assert()
        .success();
    Ok(String::from_utf8(assert.get_output().stdout.clone())?)
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.gitctx.v1.schema.json"))?;
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
fn human_json_and_agent_match_snapshots() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    fs::write(root.join("tracked.txt"), "one\nchanged\n")?;
    fs::write(root.join("new.txt"), "new\nfile\n")?;

    let human = run_axt_gitctx(&root, &["--commits", "1", "--inline-diff-max-bytes", "0"])?;
    insta::assert_snapshot!(normalize_output(&human, &root), @r###"Repository .
Branch     main upstream=none ahead=0 behind=0
Summary    changed=2 staged=0 unstaged=2 untracked=1 +3 -1 dirty=true truncated=false

Changes
  modified   tracked.txt                      +1 -1 hunks=1 bytes=12
  untracked  new.txt                          +2 -0 hunks=0 bytes=9

Commits
  <hash> axt tests initial
"###);

    let json = run_axt_gitctx(
        &root,
        &["--json", "--commits", "1", "--inline-diff-max-bytes", "0"],
    )?;
    validate_json_schema(&json)?;
    insta::assert_snapshot!(normalize_output(&json, &root), @r###"{"schema":"axt.gitctx.v1","ok":true,"data":{"repo":".","root":"<root>","branch":{"name":"main","upstream":null,"ahead":0,"behind":0},"summary":{"changed":2,"staged":0,"unstaged":2,"untracked":1,"added":3,"deleted":1,"dirty":true,"truncated":false},"files":[{"path":"tracked.txt","previous_path":null,"status":"modified","index_status":null,"worktree_status":"modified","additions":1,"deletions":1,"hunks":1,"bytes":12,"diff_inline":false,"diff_truncated":true,"diff":null},{"path":"new.txt","previous_path":null,"status":"untracked","index_status":"untracked","worktree_status":"untracked","additions":2,"deletions":0,"hunks":0,"bytes":9,"diff_inline":false,"diff_truncated":true,"diff":null}],"commits":[{"hash":"<hash>","subject":"initial","author":"axt tests","timestamp":"<ts>","age":"<age>"}],"next":["axt-slice tracked.txt --agent","axt-slice new.txt --agent"]},"warnings":[],"errors":[]}
"###);

    let agent = run_axt_gitctx(
        &root,
        &["--agent", "--commits", "1", "--inline-diff-max-bytes", "0"],
    )?;
    insta::assert_snapshot!(normalize_output(&agent, &root), @r###"{"schema":"axt.gitctx.summary.v1","type":"summary","ok":true,"repo":".","branch":"main","upstream":null,"ahead":0,"behind":0,"changed":2,"staged":0,"unstaged":2,"untracked":1,"dirty":true,"truncated":false,"next":["axt-slice tracked.txt --agent","axt-slice new.txt --agent"]}
{"schema":"axt.gitctx.file.v1","type":"file","p":"tracked.txt","prev":null,"g":"modified","idx":null,"wt":"modified","add":1,"del":1,"hunks":1,"b":12,"diff_inline":false,"diff_truncated":true,"diff":null}
{"schema":"axt.gitctx.file.v1","type":"file","p":"new.txt","prev":null,"g":"untracked","idx":"untracked","wt":"untracked","add":2,"del":0,"hunks":0,"b":9,"diff_inline":false,"diff_truncated":true,"diff":null}
{"schema":"axt.gitctx.commit.v1","type":"commit","hash":"<hash>","subject":"initial","author":"axt tests","ts":"<ts>","age":"<age>"}
"###);
    Ok(())
}

#[test]
fn clean_repo_reports_not_dirty() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    let stdout = run_axt_gitctx(&root, &["--json", "--commits", "0"])?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["summary"]["dirty"], false);
    assert_eq!(value["data"]["summary"]["changed"], 0);
    assert_eq!(value["data"]["files"].as_array().map(Vec::len), Some(0));
    Ok(())
}

#[test]
fn staged_untracked_renamed_and_deleted_statuses_work() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    fs::write(root.join("tracked.txt"), "one\nchanged\n")?;
    run_git(&root, &["add", "tracked.txt"])?;
    fs::write(root.join("new.txt"), "new\n")?;
    run_git(&root, &["mv", "renamed.txt", "renamed-now.txt"])?;
    fs::remove_file(root.join("deleted.txt"))?;

    let stdout = run_axt_gitctx(&root, &["--json", "--commits", "0"])?;
    let value: Value = serde_json::from_str(&stdout)?;
    let files = value["data"]["files"]
        .as_array()
        .ok_or_else(|| io::Error::other("files was not an array"))?;
    let status = |path: &str| {
        files
            .iter()
            .find(|file| file["path"] == path)
            .and_then(|file| file["status"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(status("tracked.txt"), "modified");
    assert_eq!(status("new.txt"), "untracked");
    assert_eq!(status("renamed-now.txt"), "renamed");
    assert_eq!(status("deleted.txt"), "deleted");
    let renamed = files
        .iter()
        .find(|file| file["path"] == "renamed-now.txt")
        .ok_or_else(|| io::Error::other("missing renamed file"))?;
    assert_eq!(renamed["previous_path"], "renamed.txt");
    assert_eq!(value["data"]["summary"]["staged"], 2);
    assert_eq!(value["data"]["summary"]["untracked"], 1);
    Ok(())
}

#[test]
fn no_git_directory_exits_with_git_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    Command::cargo_bin("axt-gitctx")?
        .current_dir(temp.path())
        .arg("--json")
        .assert()
        .failure()
        .code(12);
    Ok(())
}

#[test]
fn ahead_and_behind_use_local_bare_remote() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let temp_root = utf8_path_io(temp.path())?;
    let bare = temp_root.join("remote.git");
    run_git_at(&temp_root, &["init", "--bare", bare.as_str()])?;

    let local = temp_root.join("local");
    fs::create_dir(&local)?;
    run_git(&local, &["init"])?;
    configure_git(&local)?;
    fs::write(local.join("file.txt"), "base\n")?;
    run_git(&local, &["add", "."])?;
    commit(&local, "base")?;
    run_git(&local, &["branch", "-M", "main"])?;
    run_git(&local, &["remote", "add", "origin", bare.as_str()])?;
    run_git(&local, &["push", "-u", "origin", "main"])?;

    fs::write(local.join("file.txt"), "base\nlocal\n")?;
    run_git(&local, &["add", "."])?;
    commit(&local, "local")?;

    let other = temp_root.join("other");
    run_git_at(&temp_root, &["clone", bare.as_str(), other.as_str()])?;
    configure_git(&other)?;
    fs::write(other.join("remote.txt"), "remote\n")?;
    run_git(&other, &["add", "."])?;
    commit(&other, "remote")?;
    run_git(&other, &["push"])?;
    run_git(&local, &["fetch", "origin"])?;

    let stdout = run_axt_gitctx(&local, &["--json", "--commits", "0"])?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["branch"]["upstream"], "origin/main");
    assert_eq!(value["data"]["branch"]["ahead"], 1);
    assert_eq!(value["data"]["branch"]["behind"], 1);
    Ok(())
}

#[test]
fn inline_diff_thresholds_are_enforced() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    fs::write(root.join("tracked.txt"), "one\nchanged\n")?;

    let inline = run_axt_gitctx(
        &root,
        &[
            "--json",
            "--commits",
            "0",
            "--inline-diff-max-bytes",
            "10000",
        ],
    )?;
    let inline_value: Value = serde_json::from_str(&inline)?;
    assert_eq!(inline_value["data"]["files"][0]["diff_inline"], true);
    assert!(inline_value["data"]["files"][0]["diff"]
        .as_str()
        .is_some_and(|diff| diff.contains("-two")));

    let capped = run_axt_gitctx(
        &root,
        &["--json", "--commits", "0", "--inline-diff-max-bytes", "1"],
    )?;
    let capped_value: Value = serde_json::from_str(&capped)?;
    assert_eq!(capped_value["data"]["files"][0]["diff_inline"], false);
    assert_eq!(capped_value["data"]["files"][0]["diff_truncated"], true);
    assert_eq!(capped_value["data"]["files"][0]["diff"], Value::Null);
    Ok(())
}

#[test]
fn agent_truncation_and_strict_mode_work() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    fs::write(root.join("tracked.txt"), "one\nchanged\n")?;
    fs::write(root.join("new.txt"), "new\n")?;

    let stdout = run_axt_gitctx(&root, &["--agent", "--commits", "0", "--limit", "1"])?;
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    let summary: Value = serde_json::from_str(lines[0])?;
    let warning: Value = serde_json::from_str(lines[1])?;
    assert_eq!(summary["schema"], "axt.gitctx.summary.v1");
    assert_eq!(summary["truncated"], true);
    assert_eq!(warning["schema"], "axt.gitctx.warn.v1");

    Command::cargo_bin("axt-gitctx")?
        .current_dir(&root)
        .args(["--agent", "--commits", "0", "--limit", "1", "--strict"])
        .assert()
        .failure()
        .code(6);
    Ok(())
}

#[test]
fn agent_summary_reports_commit_and_byte_truncation() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    fs::write(root.join("tracked.txt"), "one\ntwo\nthree\n")?;
    run_git(&root, &["add", "."])?;
    commit(&root, "second")?;
    fs::write(root.join("tracked.txt"), "one\ntwo\nthree\nfour\n")?;
    run_git(&root, &["add", "."])?;
    commit(&root, "third")?;

    let record_truncated = run_axt_gitctx(&root, &["--agent", "--commits", "3", "--limit", "2"])?;
    let first_line = record_truncated
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("missing summary record"))?;
    let summary: Value = serde_json::from_str(first_line)?;
    assert_eq!(summary["truncated"], true);

    let byte_truncated =
        run_axt_gitctx(&root, &["--agent", "--commits", "3", "--max-bytes", "10"])?;
    let first_line = byte_truncated
        .lines()
        .next()
        .ok_or_else(|| io::Error::other("missing summary record"))?;
    let summary: Value = serde_json::from_str(first_line)?;
    assert_eq!(summary["truncated"], true);
    Ok(())
}

#[test]
fn next_hints_quote_paths_for_shell_reuse() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    fs::write(root.join("space name.txt"), "new\n")?;
    fs::write(root.join("quote'name.txt"), "new\n")?;

    let stdout = run_axt_gitctx(&root, &["--json", "--commits", "0"])?;
    let value: Value = serde_json::from_str(&stdout)?;
    let next = value["data"]["next"]
        .as_array()
        .ok_or_else(|| io::Error::other("next was not an array"))?;
    assert!(next
        .iter()
        .any(|hint| hint == "axt-slice 'space name.txt' --agent"));
    assert!(next
        .iter()
        .any(|hint| hint == "axt-slice 'quote'\\''name.txt' --agent"));
    Ok(())
}

#[test]
fn print_schema_and_list_errors_work() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Command::cargo_bin("axt-gitctx")?
        .args(["--print-schema", "agent"])
        .assert()
        .success();
    let schema_stdout = String::from_utf8(schema.get_output().stdout.clone())?;
    assert!(schema_stdout.starts_with("schema=axt.gitctx.agent.v1 "));

    let errors = Command::cargo_bin("axt-gitctx")?
        .arg("--list-errors")
        .assert()
        .success();
    let errors_stdout = String::from_utf8(errors.get_output().stdout.clone())?;
    assert!(errors_stdout.contains("\"code\":\"git_unavailable\""));
    Ok(())
}

fn initialized_repo() -> Result<(tempfile::TempDir, Utf8PathBuf), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = utf8_path_io(temp.path())?;
    run_git(&root, &["init"])?;
    configure_git(&root)?;
    fs::write(root.join("tracked.txt"), "one\ntwo\n")?;
    fs::write(root.join("renamed.txt"), "rename me\n")?;
    fs::write(root.join("deleted.txt"), "delete me\n")?;
    run_git(&root, &["add", "."])?;
    commit(&root, "initial")?;
    run_git(&root, &["branch", "-M", "main"])?;
    Ok((temp, root))
}

fn configure_git(root: &Utf8Path) -> Result<(), Box<dyn std::error::Error>> {
    run_git(root, &["config", "user.name", "axt tests"])?;
    run_git(root, &["config", "user.email", "axt@example.test"])?;
    Ok(())
}

fn commit(root: &Utf8Path, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let status = ProcessCommand::new("git")
        .args(["commit", "-m", message])
        .current_dir(root)
        .env("GIT_AUTHOR_DATE", "2026-04-27T10:12:00Z")
        .env("GIT_COMMITTER_DATE", "2026-04-27T10:12:00Z")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("git commit failed with {status}")).into())
    }
}

fn run_git(root: &Utf8Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    run_git_at(root, args)
}

fn run_git_at(root: &Utf8Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = ProcessCommand::new("git")
        .args(args)
        .current_dir(root)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("git {} failed with {status}", args.join(" "))).into())
    }
}

fn utf8_path_io(path: &Path) -> Result<Utf8PathBuf, io::Error> {
    Utf8PathBuf::from_path_buf(path.to_path_buf())
        .map_err(|path| io::Error::other(format!("path is not UTF-8: {path:?}")))
}

fn normalize_output(output: &str, root: &Utf8Path) -> String {
    let root_text = root.as_str();
    let private_root = format!("/private{root_text}");
    output
        .replace(&private_root, "<root>")
        .replace(root_text, "<root>")
        .lines()
        .map(normalize_line)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn normalize_line(line: &str) -> String {
    let mut normalized = line.to_owned();
    normalized = normalize_hashes(&normalized);
    normalized = normalize_timestamps(&normalized);
    normalized = normalize_ages(&normalized);
    normalized
}

fn normalize_hashes(line: &str) -> String {
    let mut parts = Vec::new();
    for part in line.split('"') {
        if part.len() == 40 && part.chars().all(|ch| ch.is_ascii_hexdigit()) {
            parts.push("<hash>".to_owned());
        } else {
            parts.push(part.to_owned());
        }
    }
    let quoted = parts.join("\"");
    let mut words = Vec::new();
    for word in quoted.split(' ') {
        if word.len() == 7 && word.chars().all(|ch| ch.is_ascii_hexdigit()) {
            words.push("<hash>".to_owned());
        } else {
            words.push(word.to_owned());
        }
    }
    words.join(" ")
}

fn normalize_timestamps(line: &str) -> String {
    let Some(index) = line.find("2026-04-27T10:12:00") else {
        return line.to_owned();
    };
    let end = line[index..]
        .find('"')
        .map_or(line.len(), |offset| index + offset);
    format!("{}<ts>{}", &line[..index], &line[end..])
}

fn normalize_ages(line: &str) -> String {
    let Some(index) = line.find("\"age\":\"") else {
        return line.to_owned();
    };
    let value_start = index + "\"age\":\"".len();
    let Some(value_end) = line[value_start..]
        .find('"')
        .map(|offset| value_start + offset)
    else {
        return line.to_owned();
    };
    format!("{}<age>{}", &line[..value_start], &line[value_end..])
}
