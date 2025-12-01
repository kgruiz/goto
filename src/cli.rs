use anyhow::Result;
use clap::{ArgAction, Parser};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(name = "to", about = "Persistent directory shortcuts (Rust CLI)", version, disable_help_subcommand = true)]
pub struct CliArgs {
    #[arg(short = 'a', long = "add", num_args = 1..=2, value_names = ["KEYWORD", "PATH"])]
    pub add: Option<Vec<String>>,

    #[arg(long = "add-bulk", value_name = "PATTERN")]
    pub addBulk: Option<String>,

    #[arg(long = "copy", num_args = 2, value_names = ["EXISTING", "NEW"])]
    pub copy: Option<Vec<String>>,

    #[arg(short = 'r', long = "rm", value_name = "KEYWORD")]
    pub remove: Option<String>,

    #[arg(short = 'l', long = "list", action = ArgAction::SetTrue)]
    pub list: bool,

    #[arg(short = 'p', long = "print-path", action = ArgAction::SetTrue)]
    pub printPath: bool,

    #[arg(short = 'c', long = "cursor", action = ArgAction::SetTrue)]
    pub cursor: bool,

    #[arg(long = "no-create", action = ArgAction::SetTrue)]
    pub noCreate: bool,

    #[arg(short = 's', long = "sort", value_name = "MODE")]
    pub sortMode: Option<String>,

    #[arg(long = "expire", value_name = "TIMESTAMP")]
    pub expire: Option<u64>,

    #[arg(long = "generate-completions", value_enum, value_name = "SHELL")]
    pub generateCompletions: Option<Shell>,

    #[arg(long = "__complete-mode", hide = true)]
    pub completeMode: Option<String>,

    #[arg(long = "__complete-input", hide = true)]
    pub completeInput: Option<String>,

    #[arg(long = "no-color", action = ArgAction::SetTrue)]
    pub noColor: bool,

    #[arg(value_name = "TARGET")]
    pub target: Option<String>,
}

pub fn ParseArgs() -> Result<CliArgs> {

    let args = CliArgs::parse();

    Ok(args)
}
