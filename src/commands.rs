use crate::cli::CliArgs;
use crate::output;
use crate::paths::ConfigPaths;
use crate::store::{AddBehavior, SearchMode, SearchOptions, Store};
use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use glob::Pattern;
use regex::RegexBuilder;
use std::env;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

pub enum Action {
    Help,
    ShowSort,
    Add {
        keyword: String,
        path: PathBuf,
        expire: Option<u64>,
    },
    AddBulk {
        pattern: String,
    },
    Copy {
        existing: String,
        newValue: String,
    },
    Remove {
        keyword: String,
    },
    PrintPath {
        target: String,
    },
    InstallWrapper {
        rcPath: Option<String>,
        force: bool,
    },
    Jump {
        target: String,
        runCursor: bool,
        create: bool,
    },
    Complete {
        mode: String,
        input: String,
    },

    Search {
        query: String,
        matchKeyword: bool,
        matchPath: bool,
        requireBoth: bool,
        mode: SearchMode,
        outputJson: bool,
        limit: Option<usize>,
    },
    CheckWrapper {
        rcPath: String,
    },
}

pub fn Execute(args: CliArgs) -> Result<()> {
    if args.classifyInvocation {
        let action = DetermineAction(&args)?;

        match action {
            Action::Jump { .. } => println!("jump"),
            _ => println!("nojump"),
        }

        return Ok(());
    }

    if let Some(rcPath) = args.checkWrapper.as_ref() {
        let path = PathBuf::from(rcPath);

        let present = WrapperPresent(&path)?;

        if present {
            std::process::exit(0);
        }

        std::process::exit(1);
    }

    if let Some(shell) = args.generateCompletions {
        if args.writeDefaultCompletions {
            WriteDefaultCompletions(shell)?;
        } else {
            GenerateCompletions(shell)?;
        }

        return Ok(());
    }

    if args.noColor || env::var("NO_COLOR").is_ok() {
        owo_colors::set_override(false);
    }

    let paths = ConfigPaths::Resolve()?;

    let skipLegacyCheck = matches!(env::var("GOTO_SKIP_LEGACY_CHECK"), Ok(val) if val == "1");

    if !skipLegacyCheck && LegacyToDetected()? {
        eprintln!(
            "Detected a legacy Zsh `to` function (likely from to-zsh). Disable it before running goto."
        );
        std::process::exit(1);
    }

    let mut store = Store::Load(paths)?;

    let addBehavior = AddBehavior {
        force: args.addForce,
        assumeYes: matches!(std::env::var("GOTO_ASSUME_YES"), Ok(val) if val == "1"),
    };

    if let Some(mode) = args.sortMode.as_deref() {
        store.SetSortMode(mode)?;
        output::PrintSortMode(mode);
    }

    if args.showSortMode {
        output::PrintCurrentSortMode(&store.sortMode);
        return Ok(());
    }

    let action = DetermineAction(&args)?;

    match action {
        Action::Help => {
            let mut cmd = CliArgs::command();
            cmd.print_help()?;
            println!();

            output::PrintSavedShortcuts(&store);
        }
        Action::InstallWrapper { rcPath, force } => {
            let rcPath = rcPath.unwrap_or_else(|| DetectShellRc());
            let rcPath = PathBuf::from(rcPath);

            let result = InstallWrapper(&rcPath, force)?;

            match result {
                WrapperAction::Added => println!("Wrapper added to {}", rcPath.display()),
                WrapperAction::Updated => println!("Wrapper updated in {}", rcPath.display()),
                WrapperAction::Skipped => println!(
                    "Wrapper already present in {} (use --install-wrapper-force to overwrite)",
                    rcPath.display()
                ),
            }
        }
        Action::Search {
            query,
            matchKeyword,
            matchPath,
            requireBoth,
            mode,
            outputJson,
            limit,
        } => {
            let options = SearchOptions {
                query,
                matchKeyword,
                matchPath,
                requireBoth,
                mode,
                limit,
            };

            let results = store.Search(&options);

            if outputJson {
                output::PrintSearchJson(&results)?;
            } else {
                output::PrintSearchResults(&results, &options.query);
            }
        }
        Action::Add {
            keyword,
            path,
            expire,
        } => {
            let outcome = store.AddShortcut(&keyword, &path, expire, &addBehavior)?;
            let resolved = store.ResolveJump(&keyword)?;
            output::PrintAddOutcome(&keyword, &resolved.targetPath, expire, &outcome);
        }
        Action::AddBulk { pattern } => {
            let added = store.AddBulk(&pattern, &addBehavior)?;
            output::PrintBulkAdded(&added);
        }
        Action::Copy { existing, newValue } => {
            store.CopyShortcut(&existing, &newValue, &addBehavior)?;
            output::PrintCopy(&existing, &newValue);
        }
        Action::Remove { keyword } => {
            store.RemoveShortcut(&keyword)?;
            output::PrintRemoved(&keyword);
        }
        Action::PrintPath { target } => {
            let resolved = store.ResolveJump(&target)?;
            println!("{}", resolved.targetPath.display());
        }
        Action::ShowSort => unreachable!(),
        Action::Jump {
            target,
            runCursor,
            create,
        } => {
            WarnIfWrapperMissing();
            JumpAndMaybeCreate(&mut store, &target, runCursor, create)?;
        }
        Action::Complete { mode, input } => {
            Complete(&store, &mode, &input)?;
        }
        Action::CheckWrapper { .. } => unreachable!(),
    }

    Ok(())
}

fn DetermineAction(args: &CliArgs) -> Result<Action> {
    if args.installWrapper {
        return Ok(Action::InstallWrapper {
            rcPath: args.installWrapperRc.clone(),
            force: args.installWrapperForce,
        });
    }

    if let Some(mode) = args.completeMode.as_ref() {
        let input = args.completeInput.clone().unwrap_or_default();

        return Ok(Action::Complete {
            mode: mode.to_string(),
            input,
        });
    }

    let mut actions = 0;

    let listFlagsUsed = args.listKeyword
        || args.listPath
        || args.listRequireBoth
        || args.listGlob
        || args.listRegex
        || args.listJson
        || args.listLimit.is_some();

    if listFlagsUsed && args.list.is_none() {
        bail!("--keyword/--path/--and/--glob/--regex/--json/--limit require --list.");
    }

    if args.installWrapper {
        actions += 1;
    }

    if args.list.is_some() {
        actions += 1;
    }

    if args.add.is_some() {
        actions += 1;
    }

    if args.showSortMode {
        actions += 1;
    }

    if args.addBulk.is_some() {
        actions += 1;
    }

    if args.copy.is_some() {
        actions += 1;
    }

    if args.remove.is_some() {
        actions += 1;
    }

    if args.printPath {
        actions += 1;
    }

    if actions > 1 {
        bail!("Please run one primary action at a time.");
    }

    if args.expire.is_some() && args.add.is_none() {
        bail!("--expire can only be used with --add.");
    }

    if args.addForce && args.add.is_none() && args.copy.is_none() && args.addBulk.is_none() {
        bail!("--force can only be used with --add, --copy, or --add-bulk.");
    }

    if let Some(addArgs) = args.add.as_ref() {
        let (keyword, path) = ParseAddArgs(addArgs)?;

        return Ok(Action::Add {
            keyword,
            path,
            expire: args.expire,
        });
    }

    if let Some(pattern) = args.addBulk.as_ref() {
        return Ok(Action::AddBulk {
            pattern: pattern.to_string(),
        });
    }

    if let Some(copyArgs) = args.copy.as_ref() {
        return Ok(Action::Copy {
            existing: copyArgs[0].clone(),
            newValue: copyArgs[1].clone(),
        });
    }

    if args.showSortMode {
        return Ok(Action::ShowSort);
    }

    if let Some(keyword) = args.remove.as_ref() {
        return Ok(Action::Remove {
            keyword: keyword.to_string(),
        });
    }

    if let Some(query) = args.list.as_ref() {
        return BuildListAction(args, query);
    }

    if args.printPath {
        let target = args
            .target
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Usage: goto --print-path <keyword>[/subdir]"))?;

        return Ok(Action::PrintPath {
            target: target.to_string(),
        });
    }

    let target = match args.target.as_ref() {
        Some(value) => value.to_string(),
        None => {
            return Ok(Action::Help);
        }
    };

    Ok(Action::Jump {
        target,
        runCursor: args.cursor,
        create: !args.noCreate,
    })
}

fn BuildListAction(args: &CliArgs, query: &str) -> Result<Action> {
    if query.is_empty() && (args.listGlob || args.listRegex) {
        bail!("Provide a query when using --glob or --regex with --list.");
    }

    let mode = if args.listGlob {
        let pattern = Pattern::new(query)?;

        SearchMode::Glob(pattern)
    } else if args.listRegex {
        let regex = RegexBuilder::new(query).case_insensitive(true).build()?;

        SearchMode::Regex(regex)
    } else {
        SearchMode::Substring(query.to_string())
    };

    Ok(Action::Search {
        query: query.to_string(),
        matchKeyword: args.listKeyword,
        matchPath: args.listPath,
        requireBoth: args.listRequireBoth,
        mode,
        outputJson: args.listJson,
        limit: args.listLimit,
    })
}

fn ParseAddArgs(values: &[String]) -> Result<(String, PathBuf)> {
    if values.is_empty() {
        bail!("Usage: goto --add <keyword> <path>");
    }

    if values.len() == 1 {
        let path = PathBuf::from(&values[0]);

        let keyword = DeriveKeywordFromPath(&path)?;

        return Ok((keyword, path));
    }

    let keyword = values[0].clone();

    let path = PathBuf::from(&values[1]);

    Ok((keyword, path))
}

fn DeriveKeywordFromPath(path: &PathBuf) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Unable to derive keyword from '{}'", path.display()))?;

    Ok(name.to_string())
}

fn JumpAndMaybeCreate(
    store: &mut Store,
    target: &str,
    runCursor: bool,
    create: bool,
) -> Result<()> {
    let resolved = store.ResolveJump(target)?;

    if resolved.targetPath.exists() {
        std::env::set_current_dir(&resolved.targetPath)?;
        output::PrintJump(&resolved.targetPath);
        store.UpdateRecentUsage(&resolved.keyword)?;
        MaybeRunCursor(&resolved.targetPath, runCursor)?;
        return Ok(());
    }

    if create {
        std::fs::create_dir_all(&resolved.targetPath)?;
        std::env::set_current_dir(&resolved.targetPath)?;
        output::PrintCreatedAndJumped(&resolved.targetPath);
        store.UpdateRecentUsage(&resolved.keyword)?;
        MaybeRunCursor(&resolved.targetPath, runCursor)?;
        return Ok(());
    }

    bail!(
        "Error: Resolved path '{}' does not exist.",
        resolved.targetPath.display()
    );
}

fn MaybeRunCursor(path: &PathBuf, runCursor: bool) -> Result<()> {
    if !runCursor {
        return Ok(());
    }

    let status = Command::new("cursor").arg(".").current_dir(path).status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("cursor exited with status {}", status),
        Err(error) => bail!("failed to run cursor: {error}"),
    }
}

fn Complete(store: &Store, mode: &str, input: &str) -> Result<()> {
    match mode {
        "keywords" => {
            let mut suggestions = store.SortedKeywords();

            if !input.is_empty() {
                suggestions.retain(|k| k.starts_with(input));
            }

            for suggestion in suggestions {
                println!("{suggestion}");
            }
        }
        "targets" => {
            let trimmed = input;

            if let Some((keyword, remainder)) = trimmed.split_once('/') {
                if let Some(entry) = store.entries.iter().find(|e| e.keyword == keyword) {
                    let (parentPart, prefix) = match remainder.rsplit_once('/') {
                        Some((parent, leaf)) => (Some(parent.to_string()), leaf.to_string()),
                        None => (None, remainder.to_string()),
                    };

                    let mut searchRoot = entry.path.clone();

                    if let Some(parent) = parentPart.clone() {
                        if !parent.is_empty() {
                            searchRoot.push(parent);
                        }
                    }

                    if searchRoot.is_dir() {
                        for dirEntry in std::fs::read_dir(&searchRoot)? {
                            let dirEntry = dirEntry?;
                            let name = dirEntry.file_name();
                            let name = name.to_string_lossy();

                            if !name.starts_with(&prefix) {
                                continue;
                            }

                            let mut suggestion = String::new();
                            suggestion.push_str(keyword);
                            suggestion.push('/');

                            if let Some(parent) = parentPart.as_ref() {
                                if !parent.is_empty() {
                                    suggestion.push_str(parent);
                                    suggestion.push('/');
                                }
                            }

                            suggestion.push_str(&name);

                            if dirEntry.file_type()?.is_dir() {
                                suggestion.push('/');
                            }

                            println!("{suggestion}");
                        }

                        return Ok(());
                    }
                }
            }

            let mut keywords = store.SortedKeywords();

            if !input.is_empty() {
                keywords.retain(|k| k.starts_with(input));
            }

            for keyword in keywords {
                println!("{keyword}");
            }
        }
        _ => bail!("Invalid completion mode"),
    }

    Ok(())
}

const WRAPPER_START: &str = "# >>> goto init >>>";
const WRAPPER_END: &str = "# <<< goto init <<<";

fn WrapperSnippet() -> &'static str {
    r#"# >>> goto init >>>
GOTO_FUNC_PATH="${XDG_CONFIG_HOME:-$HOME/.config}/zsh/plugins/goto/goto.zsh"
GOTO_COMP_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/zsh/completions"
if [ -d "$GOTO_COMP_DIR" ]; then
  case " ${fpath[*]} " in
    *" $GOTO_COMP_DIR "*) ;;
    *) fpath=("$GOTO_COMP_DIR" $fpath) ;;
  esac
fi
if [ -f "$GOTO_FUNC_PATH" ]; then
  if ! . "$GOTO_FUNC_PATH" 2>&1; then
    echo "Error: Failed to source \"$(basename \"$GOTO_FUNC_PATH\")\"" >&2
  fi
else
  echo "Error: \"$(basename \"$GOTO_FUNC_PATH\")\" not found at:" >&2
  echo "  $GOTO_FUNC_PATH" >&2
fi
if (( ! $+_comps[to] )); then
  autoload -Uz compinit
  compinit -i
fi
unset GOTO_FUNC_PATH
unset GOTO_COMP_DIR
# <<< goto init <<<
"#
}

fn WrapperSnippetBody() -> &'static str {
    r#"GOTO_FUNC_PATH="${XDG_CONFIG_HOME:-$HOME/.config}/zsh/plugins/goto/goto.zsh"
GOTO_COMP_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/zsh/completions"
if [ -d "$GOTO_COMP_DIR" ]; then
  case " ${fpath[*]} " in
    *" $GOTO_COMP_DIR "*) ;;
    *) fpath=("$GOTO_COMP_DIR" $fpath) ;;
  esac
fi
if [ -f "$GOTO_FUNC_PATH" ]; then
  if ! . "$GOTO_FUNC_PATH" 2>&1; then
    echo "Error: Failed to source \"$(basename \"$GOTO_FUNC_PATH\")\"" >&2
  fi
else
  echo "Error: \"$(basename \"$GOTO_FUNC_PATH\")\" not found at:" >&2
  echo "  $GOTO_FUNC_PATH" >&2
fi
if (( ! $+_comps[to] )); then
  autoload -Uz compinit
  compinit -i
fi
unset GOTO_FUNC_PATH
unset GOTO_COMP_DIR"#
}

fn DetectShellRc() -> String {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let shell = env::var("SHELL").unwrap_or_default();

    if shell.ends_with("zsh") {
        let zdotdir = env::var("ZDOTDIR").unwrap_or_else(|_| home.clone());

        return format!("{}/.zshrc", zdotdir);
    }

    if shell.ends_with("bash") {
        return format!("{}/.bashrc", home);
    }

    format!("{}/.profile", home)
}

#[derive(Debug, PartialEq)]
enum WrapperAction {
    Added,
    Updated,
    Skipped,
}

fn ResolveRcTarget(path: &Path) -> PathBuf {
    if path.is_symlink() {
        if let Ok(link) = fs::read_link(path) {
            if link.is_absolute() {
                return link;
            }

            if let Some(parent) = path.parent() {
                return parent.join(link);
            }
        }
    }

    path.to_path_buf()
}

fn WrapperPresent(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read rc file at {}", path.display()))?;

    if text.contains(WRAPPER_START) && text.contains(WRAPPER_END) {
        return Ok(true);
    }

    let snippet = WrapperSnippet();
    let body = WrapperSnippetBody();

    Ok(text.contains(snippet) || text.contains(body))
}

fn ReplaceWrapperBlock(text: &str, snippet: &str) -> String {
    if let Some(start) = text.find(WRAPPER_START) {
        if let Some(end) = text[start..].find(WRAPPER_END) {
            let end_idx = start + end + WRAPPER_END.len();

            let mut newText = String::new();
            newText.push_str(&text[..start]);
            if !text[..start].is_empty() && !text[..start].ends_with('\n') {
                newText.push('\n');
            }
            newText.push_str(snippet);
            if !text[end_idx..].is_empty() {
                if !text[end_idx..].starts_with('\n') {
                    newText.push('\n');
                }
                newText.push_str(&text[end_idx..]);
            }

            return newText;
        }
    }

    text.to_string()
}

fn InstallWrapper(path: &Path, force: bool) -> Result<WrapperAction> {
    let target = ResolveRcTarget(path);

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create rc parent directory at {}",
                parent.display()
            )
        })?;
    }

    let mut content = if target.exists() {
        fs::read_to_string(&target)
            .with_context(|| format!("Failed to read rc file at {}", target.display()))?
    } else {
        String::new()
    };

    let snippet = WrapperSnippet();
    let body = WrapperSnippetBody();

    let already_present = content.contains(snippet)
        || content.contains(body)
        || (content.contains(WRAPPER_START) && content.contains(WRAPPER_END));

    if already_present && !force {
        return Ok(WrapperAction::Skipped);
    }

    let mut action = if already_present {
        WrapperAction::Updated
    } else {
        WrapperAction::Added
    };

    if content.contains(WRAPPER_START) && content.contains(WRAPPER_END) {
        let replaced = ReplaceWrapperBlock(&content, snippet);

        if replaced == content {
            action = WrapperAction::Skipped;
        }

        content = replaced;
    } else if content.contains(body) {
        if force {
            content = content.replace(body, "");
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(snippet);
            content.push('\n');
        } else {
            action = WrapperAction::Skipped;
        }
    } else if content.contains(snippet) {
        action = WrapperAction::Skipped;
    } else {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(snippet);
        content.push('\n');
    }

    fs::write(&target, content)
        .with_context(|| format!("Failed to write rc file at {}", target.display()))?;

    Ok(action)
}

fn WarnIfWrapperMissing() {
    static ONCE: OnceLock<()> = OnceLock::new();

    ONCE.get_or_init(|| {
        let wrapper_flag = env::var("GOTO_WRAPPER").unwrap_or_default();

        if wrapper_flag == "1" {
            return;
        }

        if !std::io::stdout().is_terminal() {
            return;
        }

        eprintln!(
            "warning: shell wrapper not active; jumps will not change your shell directory. Run to --install-wrapper to add it."
        );
    });
}

fn ZshCompletionScript() -> &'static str {
    r#"#compdef to

_to() {
    local state
    _arguments -s -C \
      '(-h --help)'{-h,--help}'[show help]' \
      '(-l --list)'{-l,--list}'[list or search shortcuts]::query:->listquery' \
      '(-c --cursor)'{-c,--cursor}'[open in Cursor]' \
      '(-p --print-path)'{-p,--print-path}'[print stored path]:target:->targets' \
      '(-a --add)'{-a,--add}'[add shortcut]:keyword:->keywords :path:_files -/' \
      '--add-bulk[add shortcuts from pattern]:pattern:_files -/' \
      '--copy[copy existing shortcut]:existing keyword:->keywords :new:' \
      '--expire[expiration timestamp]:timestamp:' \
      '--no-create[do not create missing directories]' \
      '(-s --sort)'{-s,--sort}'[set sorting mode]:mode:(added alpha recent)' \
      '--show-sort[print current sorting mode]' \
      '(-r --rm)'{-r,--rm}'[remove shortcut]:keyword:->keywords' \
      '--install-wrapper[add goto shell wrapper to your rc file]' \
      '--install-wrapper-rc[override rc file used by --install-wrapper]:rc file:_files' \
      '--install-wrapper-force[overwrite existing wrapper when installing]' \
      '--generate-completions[generate completions for shell]:shell:(bash zsh fish)' \
      '--keyword[search keywords only]' \
      '--path[search paths only]' \
      '--and[require match in keyword and path]' \
      '--glob[interpret list query as glob]' \
      '--regex[interpret list query as regex]' \
      '--json[output list/search as json]' \
      '--limit[limit list/search results]:N:' \
      '*:target:->targets' && return

    case $state in
      listquery)
        _message 'list or search query'
        ;;
      keywords)
        compadd -- $(to --__complete-mode keywords --__complete-input "$words[CURRENT]")
        ;;
      targets)
        compadd -- $(to --__complete-mode targets --__complete-input "$words[CURRENT]")
        ;;
    esac
}

compdef _to to
"#
}
fn EmitCompletions<W: Write>(shell: Shell, mut writer: W) -> Result<()> {
    if shell == Shell::Zsh {
        writer.write_all(ZshCompletionScript().as_bytes())?;

        return Ok(());
    }

    let mut cmd = CliArgs::command();

    generate(shell, &mut cmd, "to", &mut writer);

    Ok(())
}

fn GenerateCompletions(shell: Shell) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    EmitCompletions(shell, &mut handle)?;

    Ok(())
}

fn WriteDefaultCompletions(shell: Shell) -> Result<()> {
    if shell != Shell::Zsh {
        bail!("--write-default-completions is currently supported only for zsh");
    }

    let configHome = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        format!(
            "{}/.config",
            env::var("HOME").unwrap_or_else(|_| ".".to_string())
        )
    });

    let completionDir = PathBuf::from(configHome).join("zsh/completions");

    fs::create_dir_all(&completionDir).with_context(|| {
        format!(
            "Failed to create completion directory at {}",
            completionDir.display()
        )
    })?;

    let target = completionDir.join("_to");

    let mut file = fs::File::create(&target).with_context(|| {
        format!(
            "Failed to open completion file for writing at {}",
            target.display()
        )
    })?;

    EmitCompletions(shell, &mut file)?;

    file.flush()?;

    println!("Wrote zsh completions to {}", target.display());

    Ok(())
}

fn LegacyToDetected() -> Result<bool> {
    let output = Command::new("zsh").arg("-lc").arg("whence -w to").output();

    let Ok(out) = output else {
        return Ok(false);
    };

    let stdout = String::from_utf8_lossy(&out.stdout);

    let detected = stdout.contains("function") || stdout.contains("to ()");

    Ok(detected)
}
