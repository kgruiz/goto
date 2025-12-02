use anyhow::Result;
use clap::{ArgAction, Parser};
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

    #[arg(
        short = 'l',
        long = "list",
        num_args = 0..=1,
        value_name = "QUERY",
        default_missing_value = "",
        help = "List or search shortcuts. Optional QUERY filters by keyword/path."
    )]
    pub list: Option<String>,

    #[arg(short = 'k', long = "keyword", action = ArgAction::SetTrue, help = "Search keywords only (with --list).")]
    pub listKeyword: bool,

    #[arg(short = 'P', long = "path", action = ArgAction::SetTrue, help = "Search paths only (with --list).")]
    pub listPath: bool,

    #[arg(short = 'A', long = "and", action = ArgAction::SetTrue, help = "Require matches on both keyword and path when both are searched.")]
    pub listRequireBoth: bool,

    #[arg(short = 'g', long = "glob", action = ArgAction::SetTrue, conflicts_with = "listRegex", help = "Treat list query as a glob pattern.")]
    pub listGlob: bool,

    #[arg(short = 'e', long = "regex", action = ArgAction::SetTrue, help = "Treat list query as a regular expression (case-insensitive).")]
    pub listRegex: bool,

    #[arg(short = 'j', long = "json", action = ArgAction::SetTrue, help = "Return list/search results as JSON.")]
    pub listJson: bool,

    #[arg(
        short = 'n',
        long = "limit",
        value_name = "N",
        help = "Limit number of list/search results."
    )]
    pub listLimit: Option<usize>,

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
}

pub fn ParseArgs() -> Result<CliArgs> {
    let args = CliArgs::parse();

    Ok(args)
}
