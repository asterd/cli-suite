use std::{
    collections::BTreeSet,
    io::{self, IsTerminal, Write},
    process::Command,
    time::{Duration, Instant},
};

use crate::{
    cli::SignalArg,
    discovery,
    error::Result,
    model::{FreeAction, FreeAttempt, FreeResult, PortHolder},
};

pub async fn free_holder(
    holder: &PortHolder,
    signal: SignalArg,
    grace: Duration,
    dry_run: bool,
    confirm: bool,
    tree: bool,
    force_self: bool,
) -> Result<FreeAttempt> {
    let started = Instant::now();
    if let Some(reason) = refusal_reason(holder, force_self) {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Refused,
                FreeResult::Refused,
                false,
                started,
            )
            .with_error("usage_error", reason),
        ));
    }
    if dry_run {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Simulated,
                FreeResult::Skipped,
                true,
                started,
            ),
        ));
    }
    if confirm && io::stdout().is_terminal() && !confirmed(holder)? {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Refused,
                FreeResult::Skipped,
                false,
                started,
            )
            .with_error("usage_error", "confirmation declined"),
        ));
    }

    let pids = if tree {
        tree_pids(holder.pid)
    } else {
        BTreeSet::from([holder.pid])
    };
    let mut signal_result = Ok(());
    for pid in &pids {
        signal_result = send_signal(*pid, signal, tree);
        if signal_result.is_err() {
            break;
        }
    }
    if let Err(err) = signal_result {
        let result = if err.kind() == io::ErrorKind::PermissionDenied {
            FreeResult::PermissionDenied
        } else {
            FreeResult::Failed
        };
        return Ok(attempt(
            holder,
            AttemptMeta::new(signal, FreeAction::Signaled, result, false, started)
                .with_error(result.as_str(), err.to_string()),
        ));
    }

    if signal == SignalArg::Kill {
        tokio::time::sleep(Duration::from_millis(100)).await;
    } else {
        tokio::time::sleep(grace).await;
    }
    let still_held = holder_still_present(holder)?;
    if !still_held || signal == SignalArg::Kill {
        return Ok(attempt(
            holder,
            AttemptMeta::new(
                signal,
                FreeAction::Signaled,
                if still_held {
                    FreeResult::Held
                } else {
                    FreeResult::Freed
                },
                !still_held,
                started,
            ),
        ));
    }

    for pid in &pids {
        let _ignored = send_signal(*pid, SignalArg::Kill, tree);
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let held_after_kill = holder_still_present(holder)?;
    Ok(attempt(
        holder,
        AttemptMeta::new(
            signal,
            FreeAction::Signaled,
            if held_after_kill {
                FreeResult::Held
            } else {
                FreeResult::Freed
            },
            !held_after_kill,
            started,
        )
        .escalated(),
    ))
}

fn refusal_reason(holder: &PortHolder, force_self: bool) -> Option<String> {
    if holder.pid == 1 {
        return Some("refusing to kill PID 1".to_owned());
    }
    let current = std::process::id();
    if holder.pid == current {
        return Some("refusing to kill the current process".to_owned());
    }
    if holder.pid == parent_pid() && !force_self {
        return Some("refusing to kill the parent process without --force-self".to_owned());
    }
    None
}

struct AttemptMeta {
    signal: SignalArg,
    action: FreeAction,
    result: FreeResult,
    ok: bool,
    escalated: bool,
    started: Instant,
    error_code: Option<String>,
    message: Option<String>,
}

impl AttemptMeta {
    const fn new(
        signal: SignalArg,
        action: FreeAction,
        result: FreeResult,
        ok: bool,
        started: Instant,
    ) -> Self {
        Self {
            signal,
            action,
            result,
            ok,
            escalated: false,
            started,
            error_code: None,
            message: None,
        }
    }

    fn with_error(mut self, code: impl Into<String>, message: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self.message = Some(message.into());
        self
    }

    const fn escalated(mut self) -> Self {
        self.escalated = true;
        self
    }
}

fn attempt(holder: &PortHolder, meta: AttemptMeta) -> FreeAttempt {
    FreeAttempt {
        port: holder.port,
        pid: holder.pid,
        name: holder.name.clone(),
        signal: meta.signal.as_str().to_owned(),
        action: meta.action,
        result: meta.result,
        ok: meta.ok,
        escalated: meta.escalated,
        ms: u64::try_from(meta.started.elapsed().as_millis()).unwrap_or(u64::MAX),
        error_code: meta.error_code,
        message: meta.message,
    }
}

fn confirmed(holder: &PortHolder) -> Result<bool> {
    eprint!(
        "Kill PID {} ({}) holding port {}? [y/N] ",
        holder.pid, holder.name, holder.port
    );
    let _ = io::stderr().flush();
    let mut answer = String::new();
    io::stdin()
        .read_line(&mut answer)
        .map_err(|source| crate::error::PortError::Command {
            command: "stdin",
            source,
        })?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}

fn holder_still_present(holder: &PortHolder) -> Result<bool> {
    let filters = crate::cli::FilterArgs {
        proto: match holder.proto {
            crate::model::Protocol::Tcp => crate::cli::ProtocolArg::Tcp,
            crate::model::Protocol::Udp => crate::cli::ProtocolArg::Udp,
        },
        include_loopback: true,
        listening_only: true,
        host: None,
        owner: None,
        pid: Some(holder.pid),
    };
    let (_sockets, holders) = discovery::inspect(&filters, &[holder.port])?;
    Ok(!holders.is_empty())
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: SignalArg, _tree: bool) -> io::Result<()> {
    let sig = match signal {
        SignalArg::Term => "-TERM",
        SignalArg::Kill => "-KILL",
        SignalArg::Int => "-INT",
    };
    let status = Command::new("kill")
        .args([sig, &pid.to_string()])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("kill exited with {status}")))
    }
}

#[cfg(windows)]
fn send_signal(pid: u32, signal: SignalArg, tree: bool) -> io::Result<()> {
    let mut command = Command::new("taskkill");
    command.args(["/PID", &pid.to_string()]);
    if tree {
        command.arg("/T");
    }
    if signal == SignalArg::Kill {
        command.arg("/F");
    }
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("taskkill exited with {status}")))
    }
}

fn tree_pids(root: u32) -> BTreeSet<u32> {
    let mut pids = BTreeSet::from([root]);
    for (pid, parent) in process_parents() {
        if parent == root {
            pids.insert(pid);
        }
    }
    pids
}

#[cfg(unix)]
fn parent_pid() -> u32 {
    process_parents()
        .into_iter()
        .find_map(|(pid, parent)| (pid == std::process::id()).then_some(parent))
        .unwrap_or(0)
}

#[cfg(windows)]
fn parent_pid() -> u32 {
    0
}

#[cfg(target_os = "linux")]
fn process_parents() -> Vec<(u32, u32)> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let pid = entry.file_name().to_string_lossy().parse::<u32>().ok()?;
            let status = std::fs::read_to_string(entry.path().join("status")).ok()?;
            let parent = status.lines().find_map(|line| {
                line.strip_prefix("PPid:").and_then(|value| {
                    value
                        .split_whitespace()
                        .next()
                        .and_then(|number| number.parse::<u32>().ok())
                })
            })?;
            Some((pid, parent))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn process_parents() -> Vec<(u32, u32)> {
    let Ok(output) = Command::new("ps").args(["-axo", "pid=,ppid="]).output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse::<u32>().ok()?;
            let ppid = fields.next()?.parse::<u32>().ok()?;
            Some((pid, ppid))
        })
        .collect()
}

#[cfg(windows)]
fn process_parents() -> Vec<(u32, u32)> {
    Vec::new()
}
