use std::{fs, io, path::Path, process::Command as StdCommand};

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;

fn fixture(name: &str) -> String {
    format!("fixtures/{name}")
}

fn run_axt_peek(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let assert = Command::cargo_bin("axt-peek")?
        .current_dir(workspace_root())
        .env("AXT_OUTPUT", "human")
        .args(args)
        .assert()
        .success();
    Ok(String::from_utf8(assert.get_output().stdout.clone())?)
}

fn normalize_timestamps(output: &str) -> String {
    let mut normalized = String::with_capacity(output.len());
    for line in output.lines() {
        normalized.push_str(&normalize_line_timestamps(line));
        normalized.push('\n');
    }
    normalized
}

fn normalize_line_timestamps(line: &str) -> String {
    let Some(t_index) = line.find('T') else {
        return line.to_owned();
    };
    let start = line[..t_index]
        .rfind(['"', ',', ' '])
        .map_or(0, |index| index + 1);
    let Some(end) = line[t_index..].find('Z').map(|index| t_index + index + 1) else {
        return line.to_owned();
    };
    let mut replaced = String::new();
    replaced.push_str(&line[..start]);
    replaced.push_str("<ts>");
    replaced.push_str(&line[end..]);
    normalize_line_timestamps(&replaced)
}

fn validate_json_schema(stdout: &str) -> Result<(), Box<dyn std::error::Error>> {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/axt.peek.v1.schema.json"))?;
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
fn human_mode_matches_small_tree_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_axt_peek(&[&fixture("fs-small")])?;
    insta::assert_snapshot!(normalize_timestamps(&stdout), @r###"fixtures/fs-small/
  README.md                            56 B  markdown   clean
  dist/                                 0 B             clean
  dist/app.min.js                     561 B  javascript clean
  generated.txt                        56 B  text       clean
  src/                                  0 B             clean
  src/main.rs                          45 B  rust       clean

Summary
  files     4        modified   0
  dirs      2        untracked  0
  bytes     718 B    ignored    1
  git       clean    truncated  no
"###);
    Ok(())
}

#[test]
fn json_mode_matches_schema_and_contract() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_axt_peek(&[&fixture("fs-small"), "--json"])?;
    validate_json_schema(&stdout)?;
    insta::assert_snapshot!(normalize_timestamps(&stdout), @r###"{"schema":"axt.peek.v1","ok":true,"data":{"root":"fixtures/fs-small","summary":{"files":4,"dirs":2,"bytes":718,"git_state":"clean","modified":0,"untracked":0,"ignored":1,"truncated":false},"entries":[{"path":"README.md","kind":"file","bytes":56,"language":"markdown","mime":"text/markdown","encoding":"utf-8","newline":"lf","is_generated":false,"git":"clean","mtime":"<ts>","hash":null},{"path":"dist","kind":"dir","bytes":0,"language":null,"mime":null,"encoding":null,"newline":null,"is_generated":true,"git":"clean","mtime":"<ts>","hash":null},{"path":"dist/app.min.js","kind":"file","bytes":561,"language":"javascript","mime":"text/javascript","encoding":"utf-8","newline":"lf","is_generated":true,"git":"clean","mtime":"<ts>","hash":null},{"path":"generated.txt","kind":"file","bytes":56,"language":"text","mime":"text/plain","encoding":"utf-8","newline":"lf","is_generated":true,"git":"clean","mtime":"<ts>","hash":null},{"path":"src","kind":"dir","bytes":0,"language":null,"mime":null,"encoding":null,"newline":null,"is_generated":false,"git":"clean","mtime":"<ts>","hash":null},{"path":"src/main.rs","kind":"file","bytes":45,"language":"rust","mime":"text/x-rust","encoding":"utf-8","newline":"lf","is_generated":false,"git":"clean","mtime":"<ts>","hash":null}]},"warnings":[],"errors":[]}
"###);
    Ok(())
}

#[test]
fn json_mode_contains_payload() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_axt_peek(&[&fixture("fs-small"), "--json", "--summary-only"])?;
    let value: Value = serde_json::from_str(&stdout)?;
    let data = &value["data"];
    assert_eq!(data["root"], "fixtures/fs-small");
    assert_eq!(data["summary"]["files"], 4);
    assert_eq!(data["entries"].as_array().map(Vec::len), Some(0));
    Ok(())
}

#[test]
fn print_schema_agent_outputs_agent_schema() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_axt_peek(&["--print-schema", "agent"])?;
    assert!(stdout.starts_with("schema=axt.peek.agent.v1 "));
    assert!(stdout.contains("first=summary"));
    Ok(())
}

#[test]
fn agent_mode_matches_jsonl_contract() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_axt_peek(&[&fixture("fs-small"), "--agent"])?;
    let lines = stdout.lines().collect::<Vec<_>>();
    let first: Value = serde_json::from_str(lines[0])?;
    assert_eq!(first["schema"], "axt.peek.summary.v1");
    assert_eq!(first["type"], "summary");
    assert_eq!(first["ok"], true);
    assert_eq!(first["truncated"], false);
    assert!(first["next"].is_array(), "summary must include next hints");
    for line in &lines[1..] {
        let value: Value = serde_json::from_str(line)?;
        assert!(value.get("schema").is_some());
    }
    Ok(())
}

#[test]
fn agent_mode_preserves_summary_first_when_truncated() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_axt_peek(&[&fixture("fs-small"), "--agent", "--limit", "3"])?;
    let mut lines = stdout.lines();
    let first: Value = serde_json::from_str(lines.next().unwrap())?;
    assert_eq!(first["schema"], "axt.peek.summary.v1");
    assert_eq!(first["truncated"], true);
    Ok(())
}

#[test]
fn filters_depth_hash_and_summary_only_work() -> Result<(), Box<dyn std::error::Error>> {
    let depth = run_axt_peek(&[&fixture("fs-small"), "--json", "--depth", "1"])?;
    validate_json_schema(&depth)?;
    let depth_value: Value = serde_json::from_str(&depth)?;
    assert_eq!(depth_value["data"]["summary"]["files"], 2);
    assert_eq!(depth_value["data"]["summary"]["dirs"], 2);

    let depth_zero = run_axt_peek(&[&fixture("fs-small"), "--json", "--depth", "0"])?;
    validate_json_schema(&depth_zero)?;
    let depth_zero_value: Value = serde_json::from_str(&depth_zero)?;
    assert_eq!(
        depth_zero_value["data"]["entries"].as_array().map(Vec::len),
        Some(0)
    );

    let depth_ten = run_axt_peek(&[&fixture("fs-small"), "--json", "--depth", "10"])?;
    validate_json_schema(&depth_ten)?;
    let depth_ten_value: Value = serde_json::from_str(&depth_ten)?;
    assert_eq!(depth_ten_value["data"]["summary"]["files"], 4);

    let hash = run_axt_peek(&[
        &fixture("fs-small"),
        "--json",
        "--hash",
        "blake3",
        "--lang",
        "rust",
    ])?;
    validate_json_schema(&hash)?;
    let hash_value: Value = serde_json::from_str(&hash)?;
    assert_eq!(hash_value["data"]["entries"][0]["path"], "src/main.rs");
    assert!(hash_value["data"]["entries"][0]["hash"].as_str().is_some());

    let summary = run_axt_peek(&[&fixture("fs-small"), "--agent", "--summary-only"])?;
    assert_eq!(summary.lines().count(), 1);
    Ok(())
}

#[test]
fn all_modes_emit_for_empty_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = utf8_path_io(temp.path())?;
    let root_arg = root.to_string();
    for mode in [None, Some("--json"), Some("--agent")] {
        let mut args = vec![root_arg.as_str()];
        if let Some(flag) = mode {
            args.push(flag);
        }
        let stdout = run_axt_peek(&args)?;
        assert!(!stdout.is_empty());
    }
    Ok(())
}

#[test]
fn changed_and_changed_since_work_in_git_repo() -> Result<(), Box<dyn std::error::Error>> {
    let (_temp, root) = initialized_repo()?;
    fs::write(root.join("tracked.txt"), "tracked modified\n")?;
    fs::write(root.join("untracked.txt"), "new file\n")?;

    let root_arg = root.to_string();
    let changed = run_axt_peek(&[&root_arg, "--json", "--changed"])?;
    validate_json_schema(&changed)?;
    let changed_value: Value = serde_json::from_str(&changed)?;
    let paths = changed_value["data"]["entries"]
        .as_array()
        .ok_or_else(|| io::Error::other("entries was not an array"))?
        .iter()
        .filter_map(|entry| entry["path"].as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"tracked.txt"));
    assert!(paths.contains(&"untracked.txt"));

    fs::write(root.join("changed.txt"), "second version\n")?;
    run_git(&root, &["add", "changed.txt"])?;
    run_git(&root, &["commit", "-m", "second"])?;
    let since = run_axt_peek(&[&root_arg, "--json", "--changed-since", "HEAD~1"])?;
    validate_json_schema(&since)?;
    let since_value: Value = serde_json::from_str(&since)?;
    assert_eq!(since_value["data"]["entries"][0]["path"], "changed.txt");
    Ok(())
}

#[test]
fn submodule_like_directory_is_reported_mixed() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = utf8_path_io(temp.path())?;
    fs::create_dir_all(root.join("sub"))?;
    fs::write(
        root.join("sub").join(".git"),
        "gitdir: ../.git/modules/sub\n",
    )?;

    let stdout = run_axt_peek(&[root.as_str(), "--json"])?;
    validate_json_schema(&stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["data"]["entries"][0]["path"], "sub");
    assert_eq!(value["data"]["entries"][0]["git"], "mixed");
    Ok(())
}

#[test]
fn byte_limit_marks_agent_truncated() -> Result<(), Box<dyn std::error::Error>> {
    let agent = run_axt_peek(&[&fixture("fs-small"), "--agent", "--max-bytes", "1"])?;
    let first: Value = serde_json::from_str(
        agent
            .lines()
            .next()
            .ok_or_else(|| io::Error::other("missing agent summary"))?,
    )?;
    assert_eq!(first["truncated"], true);
    Ok(())
}

#[cfg(unix)]
#[test]
fn permission_denied_subtree_warns_and_continues() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir()?;
    let root = utf8_path_io(temp.path())?;
    fs::write(root.join("ok.txt"), "ok\n")?;
    fs::create_dir(root.join("private"))?;
    fs::write(root.join("private").join("hidden.txt"), "hidden\n")?;
    fs::set_permissions(root.join("private"), fs::Permissions::from_mode(0o000))?;

    let stdout = run_axt_peek(&[root.as_str(), "--agent"])?;
    fs::set_permissions(root.join("private"), fs::Permissions::from_mode(0o700))?;

    assert!(stdout.contains("\"p\":\"ok.txt\""));
    assert!(stdout.contains("\"code\":\"permission_denied\""));
    for line in stdout.lines() {
        let _value: Value = serde_json::from_str(line)?;
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_loop_warns_when_following_links() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = utf8_path_io(temp.path())?;
    std::os::unix::fs::symlink(root.as_std_path(), root.join("loop"))?;

    let stdout = run_axt_peek(&[root.as_str(), "--agent", "--follow-symlinks"])?;
    assert!(stdout.contains("\"code\":\"symlink_loop\""));
    Ok(())
}

#[test]
fn missing_path_exits_non_zero() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-peek")?
        .current_dir(workspace_root())
        .args(["fixtures/does-not-exist"])
        .assert()
        .failure();
    Ok(())
}

#[test]
fn list_errors_outputs_full_catalog_as_jsonl() -> Result<(), Box<dyn std::error::Error>> {
    let stdout = run_axt_peek(&["--list-errors"])?;
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 15);

    let first: Value = serde_json::from_str(lines[0])?;
    assert_eq!(first["code"], "ok");
    assert_eq!(first["exit"], 0);

    let last: Value = serde_json::from_str(lines[14])?;
    assert_eq!(last["code"], "network_disabled");
    Ok(())
}

#[test]
fn conflicting_modes_are_rejected_by_clap() -> Result<(), Box<dyn std::error::Error>> {
    Command::cargo_bin("axt-peek")?
        .current_dir(workspace_root())
        .args(["--json", "--agent"])
        .assert()
        .failure()
        .code(2);
    Ok(())
}

fn initialized_repo() -> Result<(tempfile::TempDir, Utf8PathBuf), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = Utf8PathBuf::from_path_buf(temp.path().join("repo"))
        .map_err(|path| io::Error::other(format!("non-utf8 temp path: {path:?}")))?;
    copy_dir(&workspace_root().join("fixtures/fs-with-git"), &root)?;
    run_git(&root, &["init"])?;
    run_git(&root, &["config", "user.name", "axt tests"])?;
    run_git(
        &root,
        &["config", "user.email", "axt-tests@example.invalid"],
    )?;
    run_git(&root, &["add", "."])?;
    run_git(&root, &["commit", "-m", "initial"])?;
    run_git(&root, &["branch", "-M", "main"])?;
    Ok((temp, root))
}

fn copy_dir(from: &Utf8Path, to: &Utf8Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let source = entry.path();
        let target = to.join(entry.file_name().to_string_lossy().as_ref());
        if source.is_dir() {
            copy_dir(&utf8_path_io(&source)?, &target)?;
        } else {
            fs::copy(source, target)?;
        }
    }
    Ok(())
}

fn run_git(root: &Utf8Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = StdCommand::new("git")
        .arg("-C")
        .arg(root.as_std_path())
        .args(args)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("git {} failed with {status}", args.join(" "))).into())
    }
}

fn utf8_path_io(path: &Path) -> io::Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path.to_path_buf())
        .map_err(|path| io::Error::other(format!("path is not valid UTF-8: {path:?}")))
}

fn workspace_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
