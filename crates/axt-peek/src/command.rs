use axt_core::CommandContext;

use crate::{
    cli::Args,
    collect::collect,
    error::Result,
    model::{PeekData, PeekWarning, WarningCode},
};

pub fn run(args: &Args, ctx: &CommandContext) -> Result<PeekData> {
    let mut data = collect(args, &ctx.cwd)?;
    if args.git_enabled() && data.entries.len() + 1 > ctx.limits.max_records {
        data.warnings.push(PeekWarning {
            code: WarningCode::GitCapped,
            path: None,
            reason: "--limit will truncate output before every git status can be emitted"
                .to_owned(),
        });
    }
    if args.summary_only {
        data.entries.clear();
    }
    Ok(data)
}
