use clap::{Args as ClapArgs, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "axt-drift")]
#[command(about = "Mark filesystem state and report changes since the mark.")]
#[command(version)]
pub struct Args {
    #[command(flatten)]
    pub common: axt_core::CommonArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Mark(MarkArgs),
    Diff(DiffArgs),
    Run(RunArgs),
    List,
    Reset,
}

#[derive(Debug, ClapArgs)]
pub struct MarkArgs {
    #[arg(long, default_value = "default", value_name = "NAME")]
    pub name: String,

    #[arg(long)]
    pub hash: bool,
}

#[derive(Debug, ClapArgs)]
pub struct DiffArgs {
    #[arg(long, default_value = "default", value_name = "NAME")]
    pub since: String,

    #[arg(long)]
    pub hash: bool,
}

#[derive(Debug, ClapArgs)]
pub struct RunArgs {
    #[arg(long, default_value = "default", value_name = "NAME")]
    pub name: String,

    #[arg(long)]
    pub hash: bool,

    #[arg(last = true, value_name = "COMMAND")]
    pub command: Vec<String>,
}
