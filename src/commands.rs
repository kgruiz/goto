use crate::cli::CliArgs;
use crate::output;
use crate::paths::ConfigPaths;
use crate::store::Store;
use anyhow::{bail, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub enum Action {
    Help,
    List,
    Add { keyword: String, path: PathBuf, expire: Option<u64> },
    AddBulk { pattern: String },
    Copy { existing: String, newValue: String },
    Remove { keyword: String },
    PrintPath { target: String },
    Jump { target: String, runCursor: bool, create: bool },
    Complete { mode: String, input: String },
}

pub fn Execute(args: CliArgs) -> Result<()> {

    if let Some(shell) = args.generateCompletions {
        GenerateCompletions(shell)?;
        return Ok(());
    }

    if args.noColor || env::var("NO_COLOR").is_ok() {
        owo_colors::set_override(false);
    }

    let paths = ConfigPaths::Resolve()?;

    let skipLegacyCheck = matches!(env::var("GOTO_SKIP_LEGACY_CHECK"), Ok(val) if val == "1");

    if !skipLegacyCheck && LegacyToDetected()? {
        eprintln!("Detected a legacy Zsh `to` function (likely from to-zsh). Disable it before running goto.");
        std::process::exit(1);
    }

    let mut store = Store::Load(paths)?;

    if let Some(mode) = args.sortMode.as_deref() {
        store.SetSortMode(mode)?;
        output::PrintSortMode(mode);
    }

    let action = DetermineAction(&args)?;

    match action {
        Action::Help => {
            output::PrintHelp();
            output::PrintSavedShortcuts(&store);
        }
        Action::List => {
            output::PrintList(&store);
        }
        Action::Add { keyword, path, expire } => {
            store.AddShortcut(&keyword, &path, expire)?;
            output::PrintAdded(&keyword, &path, expire);
        }
        Action::AddBulk { pattern } => {
            let added = store.AddBulk(&pattern)?;
            output::PrintBulkAdded(&added);
        }
        Action::Copy { existing, newValue } => {
            store.CopyShortcut(&existing, &newValue)?;
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
        Action::Jump { target, runCursor, create } => {
            JumpAndMaybeCreate(&mut store, &target, runCursor, create)?;
        }
        Action::Complete { mode, input } => {
            Complete(&store, &mode, &input)?;
        }
    }

    Ok(())
}

fn DetermineAction(args: &CliArgs) -> Result<Action> {

    if let Some(mode) = args.completeMode.as_ref() {
        let input = args.completeInput.clone().unwrap_or_default();

        return Ok(Action::Complete {
            mode: mode.to_string(),
            input,
        });
    }

    let mut actions = 0;

    if args.list {
        actions += 1;
    }

    if args.add.is_some() {
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

    if let Some(keyword) = args.remove.as_ref() {
        return Ok(Action::Remove {
            keyword: keyword.to_string(),
        });
    }

    if args.list {
        return Ok(Action::List);
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

fn JumpAndMaybeCreate(store: &mut Store, target: &str, runCursor: bool, create: bool) -> Result<()> {

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

    let status = Command::new("cursor")
        .arg(".")
        .current_dir(path)
        .status();

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

fn ZshCompletionScript() -> &'static str {

    r#"#compdef to

_to() {
    local state
    _arguments -s -C \
      '(-h --help)'{-h,--help}'[show help]' \
      '(-l --list)'{-l,--list}'[list shortcuts]' \
      '(-c --cursor)'{-c,--cursor}'[open in Cursor]' \
      '(-p --print-path)'{-p,--print-path}'[print stored path]:target:->targets' \
      '(-a --add)'{-a,--add}'[add shortcut]:keyword:->keywords :path:_files -/' \
      '--add-bulk[add shortcuts from pattern]:pattern:_files -/' \
      '--copy[copy existing shortcut]:existing keyword:->keywords :new:' \
      '--expire[expiration timestamp]:timestamp:' \
      '--no-create[do not create missing directories]' \
      '(-s --sort)'{-s,--sort}'[set sorting mode]:mode:(added alpha recent)' \
      '(-r --rm)'{-r,--rm}'[remove shortcut]:keyword:->keywords' \
      '--generate-completions[generate completions for shell]:shell:(bash zsh fish)' \
      '*:target:->targets' && return

    case $state in
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
fn GenerateCompletions(shell: Shell) -> Result<()> {

    if shell == Shell::Zsh {
        print!("{}", ZshCompletionScript());
        return Ok(());
    }

    let mut cmd = CliArgs::command();

    generate(shell, &mut cmd, "to", &mut std::io::stdout());

    Ok(())
}

fn LegacyToDetected() -> Result<bool> {

    let output = Command::new("zsh")
        .arg("-lc")
        .arg("whence -w to")
        .output();

    let Ok(out) = output else {
        return Ok(false);
    };

    let stdout = String::from_utf8_lossy(&out.stdout);

    let detected = stdout.contains("function") || stdout.contains("to ()");

    Ok(detected)
}
