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
        short = 'b',
        long = "bulk-add",
        value_name = "PATTERN",
        help = "Add shortcuts for each directory matching the glob PATTERN."
    )]
    pub bulkAdd: Option<String>,

    #[arg(short = 'f', long = "force", action = ArgAction::SetTrue, help = "Replace an existing keyword or overwrite duplicate paths without prompting.")]
    pub addForce: bool,

    #[arg(
        short = 'c',
        long = "copy",
        num_args = 2,
        value_names = ["EXISTING", "NEW"],
        help = "Duplicate an existing shortcut under a new keyword or path."
    )]
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

    #[arg(short = 'g', long = "glob", action = ArgAction::SetTrue, conflicts_with = "listRegex", help = "Treat list query as a glob pattern.")]
    pub listGlob: bool,

    #[arg(short = 'e', long = "regex", action = ArgAction::SetTrue, help = "Treat list query as a regular expression (case-insensitive).")]
    pub listRegex: bool,

    #[arg(short = 'k', long = "keyword-only", action = ArgAction::SetTrue, help = "Search keywords only (with --list).")]
    pub listKeywordOnly: bool,

    #[arg(short = 'y', long = "path-only", action = ArgAction::SetTrue, help = "Search paths only (with --list).")]
    pub listPathOnly: bool,

    #[arg(short = 'B', long = "both", action = ArgAction::SetTrue, help = "Require matches on both keyword and path when both are searched.")]
    pub listRequireBoth: bool,

    #[arg(
        short = 'w',
        long = "within",
        value_name = "PATH",
        help = "Limit list/search results to shortcuts under PATH."
    )]
    pub listWithin: Option<String>,

    #[arg(short = 'H', long = "here", action = ArgAction::SetTrue, help = "Limit list/search results to shortcuts under the current directory.")]
    pub listHere: bool,

    #[arg(
        short = 'd',
        long = "max-depth",
        value_name = "N",
        help = "Limit list/search results to a maximum depth under the scoped root (0 = root only)."
    )]
    pub listMaxDepth: Option<usize>,

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

    #[arg(short = 'u', long = "cursor", action = ArgAction::SetTrue, help = "Open the target in Cursor after jumping.")]
    pub cursor: bool,

    #[arg(short = 'C', long = "code", action = ArgAction::SetTrue, help = "Open the target in VS Code after jumping.")]
    pub code: bool,

    #[arg(short = 'N', long = "no-create", action = ArgAction::SetTrue, help = "Fail instead of creating missing directories on jump.")]
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
        short = 'x',
        long = "expire",
        value_name = "TIMESTAMP",
        help = "Expiration timestamp (seconds since epoch) for --add."
    )]
    pub expire: Option<u64>,

    #[arg(
        long = "completions",
        visible_alias = "generate-completions",
        value_enum,
        value_name = "SHELL",
        help = "Generate shell completions to stdout."
    )]
    pub generateCompletions: Option<Shell>,

    #[arg(
        long = "write-default-completions",
        visible_aliases = ["write-completions", "install-completions"],
        action = ArgAction::SetTrue,
        requires = "generateCompletions",
        help = "Write completions to the default location for the shell instead of stdout (zsh only)."
    )]
    pub writeDefaultCompletions: bool,

    #[arg(long = "install-wrapper", action = ArgAction::SetTrue, help = "Add the goto shell wrapper to your rc file (detects rc automatically unless overridden).")]
    pub installWrapper: bool,

    #[arg(
        long = "install-wrapper-rc",
        value_name = "RC_PATH",
        requires = "installWrapper",
        help = "Override rc file path used by --install-wrapper."
    )]
    pub installWrapperRc: Option<String>,

    #[arg(long = "install-wrapper-force", action = ArgAction::SetTrue, help = "Overwrite existing goto wrapper when using --install-wrapper.")]
    pub installWrapperForce: bool,

    #[arg(long = "__check-wrapper", hide = true)]
    pub checkWrapper: Option<String>,

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
