use std::{
    collections::BTreeSet,
    io::{self, IsTerminal, Write},
    time::{Duration, Instant},
};

use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

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
        signal_result = send_signal(*pid, signal);
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
        let _ignored = send_signal(*pid, SignalArg::Kill);
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
fn send_signal(pid: u32, signal: SignalArg) -> io::Result<()> {
    use nix::{
        sys::signal::{kill, Signal},
        unistd::Pid,
    };
    let raw = match i32::try_from(pid) {
        Ok(value) => value,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "pid out of range",
            ))
        }
    };
    let sig = match signal {
        SignalArg::Term => Signal::SIGTERM,
        SignalArg::Kill => Signal::SIGKILL,
        SignalArg::Int => Signal::SIGINT,
    };
    kill(Pid::from_raw(raw), sig).map_err(|errno| io::Error::from_raw_os_error(errno as i32))
}

#[cfg(windows)]
fn send_signal(pid: u32, signal: SignalArg) -> io::Result<()> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, FALSE},
        System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    };

    let exit_code: u32 = match signal {
        SignalArg::Kill | SignalArg::Term => 1,
        SignalArg::Int => 0xC000_013A,
    };

    // SAFETY: We open the process with the minimum rights needed to terminate it,
    // check the returned handle for null, and always close it before returning.
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, FALSE, pid);
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let result = TerminateProcess(handle, exit_code);
        CloseHandle(handle);
        if result == 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
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

fn parent_pid() -> u32 {
    let current = std::process::id();
    process_parents()
        .into_iter()
        .find_map(|(pid, parent)| (pid == current).then_some(parent))
        .unwrap_or(0)
}

fn process_parents() -> Vec<(u32, u32)> {
    let mut system =
        System::new_with_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));
    system.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::new(),
    );
    system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let parent = process.parent()?;
            Some((Pid::as_u32(*pid), parent.as_u32()))
        })
        .collect()
}
