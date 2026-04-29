use std::{collections::HashMap, env, fs, path::PathBuf, time::Duration};

use axt_core::CommandContext;
use camino::Utf8PathBuf;
use tokio::{process::Command as TokioCommand, time};

use crate::{
    cli::{Args, Command},
    error::{DocError, Result},
    model::{
        CommandMatch, DocData, EnvReport, EnvSuspicion, EnvVarReport, OrderingIssue, PathDuplicate,
        PathEntry, PathReport, VersionProbe, WhichReport,
    },
};

pub async fn run(args: &Args, _ctx: &CommandContext) -> Result<crate::output::DocOutput> {
    if args.show_secrets {
        eprintln!("Warning: --show-secrets prints secret-like environment variable values.");
    }

    if let Some(cmd) = &args.cmd {
        let manager_hints = manager_hints(Duration::from_millis(300)).await;
        let data = DocData {
            which: Some(which_report(cmd, Duration::from_millis(1_500), &manager_hints).await?),
            path: Some(path_report(&manager_hints)?),
            env: Some(env_report(args.show_secrets)),
        };
        return Ok(crate::output::DocOutput::All(data));
    }

    match args.command.as_ref().ok_or(DocError::MissingSubcommand)? {
        Command::Which(which) => {
            let manager_hints = manager_hints(Duration::from_millis(300)).await;
            let data = DocData {
                which: Some(which_report(&which.cmd, which.timeout.0, &manager_hints).await?),
                path: None,
                env: None,
            };
            Ok(crate::output::DocOutput::Which(data))
        }
        Command::Path => {
            let manager_hints = manager_hints(Duration::from_millis(300)).await;
            let data = DocData {
                which: None,
                path: Some(path_report(&manager_hints)?),
                env: None,
            };
            Ok(crate::output::DocOutput::Path(data))
        }
        Command::Env => {
            let data = DocData {
                which: None,
                path: None,
                env: Some(env_report(args.show_secrets)),
            };
            Ok(crate::output::DocOutput::Env(data))
        }
        Command::All(all) => {
            let manager_hints = manager_hints(Duration::from_millis(300)).await;
            let data = DocData {
                which: Some(which_report(&all.cmd, all.timeout.0, &manager_hints).await?),
                path: Some(path_report(&manager_hints)?),
                env: Some(env_report(args.show_secrets)),
            };
            Ok(crate::output::DocOutput::All(data))
        }
    }
}

async fn which_report(
    cmd: &str,
    timeout: Duration,
    manager_hints: &[ManagerHint],
) -> Result<WhichReport> {
    let matches = command_matches(cmd, manager_hints)?;
    let version = if matches.is_empty() {
        VersionProbe {
            attempted: false,
            ok: false,
            timed_out: false,
            command: None,
            output: None,
            error: Some("command not found in PATH".to_owned()),
        }
    } else {
        probe_version(&matches[0].path, timeout).await?
    };

    Ok(WhichReport {
        cmd: cmd.to_owned(),
        found: !matches.is_empty(),
        primary: matches.first().map(|item| item.path.clone()),
        matches,
        version,
    })
}

fn command_matches(cmd: &str, manager_hints: &[ManagerHint]) -> Result<Vec<CommandMatch>> {
    let paths = which::which_all(cmd)
        .map(|items| items.collect::<Vec<_>>())
        .unwrap_or_default();
    paths
        .into_iter()
        .map(|path| {
            let utf8 = utf8_path(path)?;
            let manager = manager_for_path(utf8.as_str(), manager_hints).map(str::to_owned);
            Ok(CommandMatch {
                source: source_for_manager(manager.as_deref()).to_owned(),
                manager,
                executable: true,
                path: utf8.to_string(),
            })
        })
        .collect()
}

async fn probe_version(path: &str, timeout: Duration) -> Result<VersionProbe> {
    let mut command = TokioCommand::new(path);
    command.arg("--version");
    command.kill_on_drop(true);
    let command_label = format!("{path} --version");
    match time::timeout(timeout, command.output()).await {
        Ok(Ok(output)) => {
            let text = first_non_empty_line(&output.stdout)
                .or_else(|| first_non_empty_line(&output.stderr))
                .unwrap_or_default();
            Ok(VersionProbe {
                attempted: true,
                ok: output.status.success(),
                timed_out: false,
                command: Some(command_label),
                output: if text.is_empty() { None } else { Some(text) },
                error: if output.status.success() {
                    None
                } else {
                    Some(format!("version probe exited with {}", output.status))
                },
            })
        }
        Ok(Err(source)) => Err(DocError::Probe {
            cmd: path.to_owned(),
            source,
        }),
        Err(_) => Ok(VersionProbe {
            attempted: true,
            ok: false,
            timed_out: true,
            command: Some(command_label),
            output: None,
            error: Some(format!("version probe exceeded {}ms", timeout.as_millis())),
        }),
    }
}

fn path_report(manager_hints: &[ManagerHint]) -> Result<PathReport> {
    let path_var = env::var_os("PATH").unwrap_or_default();
    let raw_entries = env::split_paths(&path_var).collect::<Vec<_>>();
    let mut seen = HashMap::<String, usize>::new();
    let mut duplicates = Vec::new();
    let mut missing = Vec::new();
    let mut broken_symlinks = Vec::new();
    let mut entries = Vec::with_capacity(raw_entries.len());

    for (index, raw) in raw_entries.into_iter().enumerate() {
        let utf8 = utf8_path(raw)?;
        let metadata = fs::symlink_metadata(&utf8);
        let exists = metadata.is_ok();
        let is_symlink = metadata
            .as_ref()
            .is_ok_and(|item| item.file_type().is_symlink());
        let is_dir = metadata.as_ref().is_ok_and(fs::Metadata::is_dir);
        let canonical = fs::canonicalize(&utf8).ok().and_then(path_to_string);
        let duplicate_key = canonical.clone().unwrap_or_else(|| utf8.to_string());

        if !exists {
            missing.push(utf8.to_string());
        }
        if is_symlink && canonical.is_none() {
            broken_symlinks.push(utf8.to_string());
        }
        if let Some(first_index) = seen.get(&duplicate_key).copied() {
            duplicates.push(PathDuplicate {
                path: utf8.to_string(),
                first_index,
                duplicate_index: index,
            });
        } else {
            seen.insert(duplicate_key, index);
        }

        entries.push(PathEntry {
            index,
            path: utf8.to_string(),
            exists,
            is_dir,
            is_symlink,
            canonical,
            manager: manager_for_path(utf8.as_str(), manager_hints).map(str::to_owned),
        });
    }

    let ordering_issues = ordering_issues(&entries, &duplicates);
    Ok(PathReport {
        entries,
        duplicates,
        missing,
        broken_symlinks,
        ordering_issues,
    })
}

fn ordering_issues(entries: &[PathEntry], duplicates: &[PathDuplicate]) -> Vec<OrderingIssue> {
    let duplicate_issues = duplicates.iter().map(|duplicate| OrderingIssue {
        kind: "duplicate".to_owned(),
        path: duplicate.path.clone(),
        index: duplicate.duplicate_index,
        earlier_index: duplicate.first_index,
        message: "duplicate PATH entry is shadowed by an earlier entry".to_owned(),
    });

    let manager_issues = entries.iter().filter_map(|entry| {
        let manager = entry.manager.as_ref()?;
        let earlier_system = entries.iter().find(|candidate| {
            candidate.index < entry.index
                && candidate.manager.is_none()
                && looks_like_system_bin(&candidate.path)
        })?;
        Some(OrderingIssue {
            kind: "manager_after_system".to_owned(),
            path: entry.path.clone(),
            index: entry.index,
            earlier_index: earlier_system.index,
            message: format!("{manager} appears after a system bin directory"),
        })
    });

    duplicate_issues.chain(manager_issues).collect()
}

fn env_report(show_secrets: bool) -> EnvReport {
    let mut vars = env::vars()
        .map(|(name, value)| {
            let secret_like = is_secret_like(&name);
            let empty = value.is_empty();
            EnvVarReport {
                value: if secret_like && !show_secrets {
                    "<redacted>".to_owned()
                } else {
                    value
                },
                name,
                secret_like,
                empty,
            }
        })
        .collect::<Vec<_>>();
    vars.sort_by(|left, right| left.name.cmp(&right.name));

    let secret_like = vars
        .iter()
        .filter(|item| item.secret_like)
        .cloned()
        .collect::<Vec<_>>();
    let suspicious = vars
        .iter()
        .filter(|item| item.empty || item.name.contains(' '))
        .map(|item| EnvSuspicion {
            name: item.name.clone(),
            reason: if item.empty {
                "empty value".to_owned()
            } else {
                "variable name contains whitespace".to_owned()
            },
        })
        .collect::<Vec<_>>();

    EnvReport {
        total: vars.len(),
        vars,
        secret_like,
        suspicious,
        show_secrets,
    }
}

fn first_non_empty_line(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn utf8_path(path: PathBuf) -> Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(path).map_err(DocError::PathNotUtf8)
}

fn path_to_string(path: PathBuf) -> Option<String> {
    Utf8PathBuf::from_path_buf(path)
        .ok()
        .map(|utf8| utf8.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagerHint {
    name: &'static str,
    prefix: String,
}

async fn manager_hints(timeout: Duration) -> Vec<ManagerHint> {
    let prefix_probes = [
        ("homebrew", "brew", &["--prefix"][..]),
        ("rustup", "rustup", &["show", "home"][..]),
        ("pyenv", "pyenv", &["root"][..]),
        ("rbenv", "rbenv", &["root"][..]),
        ("mise", "mise", &["data-dir"][..]),
    ];
    let mut hints = Vec::new();
    for (name, program, args) in prefix_probes {
        if which::which(program).is_err() {
            continue;
        }
        if let Some(prefix) = probe_manager_prefix(program, args, timeout).await {
            hints.push(ManagerHint { name, prefix });
        }
    }

    if which::which("asdf").is_ok() {
        if let Some(prefix) = probe_asdf_data_dir(timeout).await {
            hints.push(ManagerHint {
                name: "asdf",
                prefix,
            });
        }
    }

    if which::which("volta").is_ok() {
        if let Some(prefix) = env::var("VOLTA_HOME").ok().or_else(|| home_child(".volta")) {
            hints.push(ManagerHint {
                name: "volta",
                prefix,
            });
        }
    }

    if let Ok(prefix) = env::var("NVM_DIR") {
        hints.push(ManagerHint {
            name: "nvm-shim",
            prefix,
        });
    }

    if which::which("scoop").is_ok() {
        if let Some(prefix) = env::var("SCOOP").ok().or_else(|| home_child("scoop")) {
            hints.push(ManagerHint {
                name: "scoop",
                prefix,
            });
        }
    }

    if which::which("choco").is_ok() || which::which("chocolatey").is_ok() {
        if let Some(prefix) = env::var("ChocolateyInstall")
            .ok()
            .or_else(|| Some("C:\\ProgramData\\chocolatey".to_owned()))
        {
            hints.push(ManagerHint {
                name: "chocolatey",
                prefix,
            });
        }
    }

    if which::which("winget").is_ok() {
        if let Some(prefix) = env::var("LOCALAPPDATA")
            .ok()
            .map(|path| format!("{path}\\Microsoft\\WinGet"))
        {
            hints.push(ManagerHint {
                name: "winget",
                prefix,
            });
        }
    }

    hints
}

async fn probe_manager_prefix(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut command = TokioCommand::new(program);
    command.args(args);
    command.kill_on_drop(true);
    let output = time::timeout(timeout, command.output()).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    first_non_empty_line(&output.stdout)
}

async fn probe_asdf_data_dir(timeout: Duration) -> Option<String> {
    if let Ok(path) = env::var("ASDF_DATA_DIR") {
        return Some(path);
    }

    let mut command = TokioCommand::new("asdf");
    command.arg("info");
    command.kill_on_drop(true);
    let output = time::timeout(timeout, command.output()).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("ASDF_DATA_DIR=")
            .or_else(|| trimmed.strip_prefix("ASDF_DIR="))
            .map(str::to_owned)
    })
}

fn home_child(child: &str) -> Option<String> {
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).ok()?;
    Some(format!("{home}/{child}"))
}

fn manager_for_path<'a>(path: &str, manager_hints: &'a [ManagerHint]) -> Option<&'a str> {
    let lower = path.to_ascii_lowercase();
    for hint in manager_hints {
        if lower.starts_with(&hint.prefix.to_ascii_lowercase()) {
            return Some(hint.name);
        }
    }
    manager_for_path_pattern(path)
}

fn manager_for_path_pattern(path: &str) -> Option<&'static str> {
    let lower = path.to_ascii_lowercase();
    if lower.contains("/homebrew/") || lower.contains("/cellar/") || lower.contains("\\homebrew\\")
    {
        Some("homebrew")
    } else if lower.contains("/.local/share/mise/")
        || lower.contains("/mise/")
        || lower.contains("\\mise\\")
    {
        Some("mise")
    } else if lower.contains("/.asdf/") || lower.contains("\\.asdf\\") {
        Some("asdf")
    } else if lower.contains("/.rustup/") || lower.contains("\\.rustup\\") {
        Some("rustup")
    } else if lower.contains("/.cargo/bin") || lower.contains("\\.cargo\\bin") {
        Some("cargo")
    } else if lower.contains("/.pyenv/") || lower.contains("\\.pyenv\\") {
        Some("pyenv")
    } else if lower.contains("/.rbenv/") || lower.contains("\\.rbenv\\") {
        Some("rbenv")
    } else if lower.contains("/.volta/") || lower.contains("\\volta\\") {
        Some("volta")
    } else if lower.contains("/.nvm/") || lower.contains("\\nvm\\") {
        Some("nvm-shim")
    } else if lower.contains("\\scoop\\") {
        Some("scoop")
    } else if lower.contains("\\chocolatey\\") {
        Some("chocolatey")
    } else if lower.contains("\\winget\\") {
        Some("winget")
    } else {
        None
    }
}

fn source_for_manager(manager: Option<&str>) -> &'static str {
    manager.map_or("path", |_| "version_manager")
}

fn looks_like_system_bin(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "/bin" | "/usr/bin" | "/usr/local/bin" | "c:/windows/system32" | "c:/windows"
    ) || normalized.ends_with("/windows/system32")
}

fn is_secret_like(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper == "PASS"
        || upper.ends_with("_TOKEN")
        || upper.contains("_SECRET")
        || upper.ends_with("_KEY")
        || upper.ends_with("_PASSWORD")
        || upper.contains("_CREDENTIAL")
        || upper.contains("_PRIVATE")
        || upper.contains("_AUTH")
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_secret_like_names() {
        assert!(is_secret_like("GITHUB_TOKEN"));
        assert!(is_secret_like("DB_PASSWORD"));
        assert!(is_secret_like("PASS"));
        assert!(is_secret_like("AWS_SECRET_ACCESS_KEY"));
        assert!(!is_secret_like("PATH"));
    }

    #[test]
    fn attributes_manager_paths() {
        assert_eq!(
            manager_for_path("/Users/me/.cargo/bin/rustc", &[]),
            Some("cargo")
        );
        assert_eq!(
            manager_for_path("/Users/me/.pyenv/shims/python", &[]),
            Some("pyenv")
        );
    }

    #[test]
    fn attributes_queried_manager_prefixes() {
        let hints = vec![ManagerHint {
            name: "homebrew",
            prefix: "/opt/homebrew".to_owned(),
        }];
        assert_eq!(
            manager_for_path("/opt/homebrew/bin/git", &hints),
            Some("homebrew")
        );
    }
}
