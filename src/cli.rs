use anyhow::Result;
use clap::{ArgAction, Args, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(
    name = "to",
    about = "Persistent directory shortcuts (Rust CLI)",
    version,
    disable_help_subcommand = true
)]
pub struct CliArgs {
    #[arg(
        short = 'a',
        long = "add",
        num_args = 1..=2,
        value_names = ["KEYWORD", "PATH"],
        help = "Save a shortcut (keyword + path). With one arg, keyword is derived from the path basename."
    )]
    pub add: Option<Vec<String>>,

    #[arg(
        long = "add-bulk",
        value_name = "PATTERN",
        help = "Add shortcuts for each directory matching the glob PATTERN."
    )]
    pub addBulk: Option<String>,

    #[arg(long = "copy", num_args = 2, value_names = ["EXISTING", "NEW"], help = "Duplicate an existing shortcut under a new keyword or path.")]
    pub copy: Option<Vec<String>>,

    #[arg(
        short = 'r',
        long = "rm",
        value_name = "KEYWORD",
        help = "Remove a saved shortcut."
    )]
    pub remove: Option<String>,

    #[arg(short = 'l', long = "list", action = ArgAction::SetTrue, help = "List all shortcuts.")]
    pub list: bool,

    #[arg(short = 'p', long = "print-path", action = ArgAction::SetTrue, help = "Print the resolved path for TARGET without changing directory.")]
    pub printPath: bool,

    #[arg(short = 'c', long = "cursor", action = ArgAction::SetTrue, help = "Open the target in Cursor after jumping.")]
    pub cursor: bool,

    #[arg(long = "no-create", action = ArgAction::SetTrue, help = "Fail instead of creating missing directories on jump.")]
    pub noCreate: bool,

    #[arg(
        short = 's',
        long = "sort",
        value_name = "MODE",
        help = "Set sorting mode: added | alpha | recent."
    )]
    pub sortMode: Option<String>,

    #[arg(long = "show-sort", action = ArgAction::SetTrue, help = "Print the current sorting mode.")]
    pub showSortMode: bool,

    #[arg(
        long = "expire",
        value_name = "TIMESTAMP",
        help = "Expiration timestamp (seconds since epoch) for --add."
    )]
    pub expire: Option<u64>,

    #[arg(
        long = "generate-completions",
        value_enum,
        value_name = "SHELL",
        help = "Generate shell completions to stdout."
    )]
    pub generateCompletions: Option<Shell>,

    #[arg(long = "__classify", hide = true, action = ArgAction::SetTrue)]
    pub classifyInvocation: bool,

    #[arg(long = "__complete-mode", hide = true)]
    pub completeMode: Option<String>,

    #[arg(long = "__complete-input", hide = true)]
    pub completeInput: Option<String>,

    #[arg(long = "no-color", action = ArgAction::SetTrue, help = "Disable colored output.")]
    pub noColor: bool,

    #[arg(value_name = "TARGET")]
    pub target: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(alias = "s", about = "Search saved shortcuts (alias: s)")]
    Search(SearchArgs),

    #[command(about = "List all shortcuts (alias of search with no query)")]
    List,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    #[arg(short = 'k', long = "keyword", action = ArgAction::SetTrue, help = "Search keywords only.")]
    pub keyword: bool,

    #[arg(short = 'p', long = "path", action = ArgAction::SetTrue, help = "Search paths only.")]
    pub path: bool,

    #[arg(short = 'A', long = "and", action = ArgAction::SetTrue, help = "Require matches on both keyword and path when both are searched.")]
    pub requireBoth: bool,

    #[arg(short = 'g', long = "glob", action = ArgAction::SetTrue, conflicts_with = "regex", help = "Treat query as a glob pattern." )]
    pub glob: bool,

    #[arg(short = 'r', long = "regex", action = ArgAction::SetTrue, help = "Treat query as a regular expression.")]
    pub regex: bool,

    #[arg(short = 'j', long = "json", action = ArgAction::SetTrue, help = "Return results as JSON.")]
    pub json: bool,

    #[arg(
        short = 'n',
        long = "limit",
        value_name = "N",
        help = "Limit number of results."
    )]
    pub limit: Option<usize>,
}

pub fn ParseArgs() -> Result<CliArgs> {
    let args = CliArgs::parse();

    Ok(args)
}
