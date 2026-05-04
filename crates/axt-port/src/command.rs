use std::time::Instant;

use axt_core::CommandContext;

use crate::{
    cli::{Args, Command, FreeArgs, WatchArgs},
    discovery,
    error::{PortError, Result},
    model::{PortAction, PortData},
    output::PortOutput,
    signal,
};

pub async fn run(args: &Args, ctx: &CommandContext) -> Result<PortOutput> {
    match args.command.as_ref().ok_or(PortError::MissingSubcommand)? {
        Command::List => inspect_action(args, PortAction::List, &[]).map(PortOutput::List),
        Command::Who(port_args) => {
            inspect_action(args, PortAction::Who, &port_args.ports).map(PortOutput::Who)
        }
        Command::Free(free_args) => free(args, free_args).await.map(PortOutput::Free),
        Command::Watch(watch_args) => watch(args, watch_args, ctx).await.map(PortOutput::Watch),
    }
}

fn inspect_action(args: &Args, action: PortAction, ports: &[u16]) -> Result<PortData> {
    let started = Instant::now();
    let (sockets, holders) = discovery::inspect(&args.filters, ports)?;
    Ok(PortData {
        action,
        ports: ports.to_vec(),
        held: !holders.is_empty(),
        sockets,
        holders,
        attempts: Vec::new(),
        freed: false,
        timed_out: false,
        duration_ms: elapsed_ms(started),
        truncated: false,
    })
}

async fn free(args: &Args, free_args: &FreeArgs) -> Result<PortData> {
    let started = Instant::now();
    let (sockets, holders) = discovery::inspect(&args.filters, &free_args.ports)?;
    let mut attempts = Vec::new();
    for holder in &holders {
        attempts.push(
            signal::free_holder(
                holder,
                free_args.signal,
                free_args.grace,
                free_args.kill_grace,
                free_args.dry_run,
                free_args.confirm,
                free_args.tree,
                free_args.force_self,
            )
            .await?,
        );
    }
    let freed =
        !free_args.dry_run && !holders.is_empty() && attempts.iter().all(|attempt| attempt.ok);
    Ok(PortData {
        action: PortAction::Free,
        ports: free_args.ports.clone(),
        held: !holders.is_empty(),
        sockets,
        holders,
        attempts,
        freed,
        timed_out: false,
        duration_ms: elapsed_ms(started),
        truncated: false,
    })
}

async fn watch(args: &Args, watch_args: &WatchArgs, ctx: &CommandContext) -> Result<PortData> {
    let started = Instant::now();
    let timeout = ctx.max_duration.map_or(watch_args.timeout, |duration| {
        duration.min(watch_args.timeout)
    });
    loop {
        let (sockets, holders) = discovery::inspect(&args.filters, &[watch_args.port])?;
        if holders.is_empty() || started.elapsed() >= timeout {
            let freed = holders.is_empty();
            let timed_out = !freed && started.elapsed() >= timeout;
            return Ok(PortData {
                action: PortAction::Watch,
                ports: vec![watch_args.port],
                held: !holders.is_empty(),
                sockets,
                holders,
                attempts: Vec::new(),
                freed,
                timed_out,
                duration_ms: elapsed_ms(started),
                truncated: false,
            });
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}
