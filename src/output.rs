use crate::store::{AddOutcome, SearchResult, Store};
use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;

pub fn PrintSavedShortcuts(store: &Store) {
    let sorted = store.SortedKeywords();

    if sorted.is_empty() {
        println!("{}", "No shortcuts saved.".red().bold());
        return;
    }

    let total = sorted.len();

    let shown = if total < 30 { total } else { 30 };

    let mut maxLen = 0;

    for key in sorted.iter().take(shown) {
        if key.len() > maxLen {
            maxLen = key.len();
        }
    }

    let width = maxLen + 2;

    if total <= 30 {
        println!("\n{}", "Saved shortcuts:".magenta());
    } else {
        println!(
            "\n{}",
            format!("Saved shortcuts (showing {shown} of {total}):").magenta()
        );
    }

    let cols = 3;

    let rows = (shown + cols - 1) / cols;

    for row in 0..rows {
        for col in 0..cols {
            let idx = col * rows + row;

            if idx < shown {
                let key = &sorted[idx];
                print!(
                    "  {:>2}. {:<width$}",
                    idx + 1,
                    key.bold().cyan(),
                    width = width
                );
            }
        }
        println!();
    }

    if total > shown {
        println!("  … and {} more", total - shown);
    }

    println!(
        "\nCurrent sorting mode: {}",
        store.sortMode.clone().ToLabel()
    );
}

pub fn PrintList(store: &Store) {
    let keywords = store.SortedKeywords();

    if keywords.is_empty() {
        println!("{}", "No shortcuts saved.".red().bold());
        return;
    }

    for keyword in keywords {
        let entry = store.entries.iter().find(|e| e.keyword == keyword);

        if let Some(entry) = entry {
            println!(
                "{} → {}",
                entry.keyword.bold().cyan(),
                entry.path.display().to_string().dimmed()
            );
        }
    }
}

pub fn PrintSearchResults(results: &[SearchResult], query: &str) {
    if results.is_empty() {
        if query.is_empty() {
            println!("{}", "No shortcuts saved.".red().bold());

            return;
        }

        println!(
            "{}",
            format!("No shortcuts matched '{}'.", query).red().bold()
        );

        return;
    }

    for result in results {
        match result.expiry {
            Some(ts) => println!(
                "{} → {} (expires {})",
                result.keyword.bold().cyan(),
                result.path.display().to_string().dimmed(),
                ts
            ),
            None => println!(
                "{} → {}",
                result.keyword.bold().cyan(),
                result.path.display().to_string().dimmed()
            ),
        }
    }
}

pub fn PrintSearchJson(results: &[SearchResult]) -> Result<()> {
    let payload: Vec<_> = results
        .iter()
        .map(|result| {
            serde_json::json!({
                "keyword": result.keyword,
                "path": result.path,
                "expiry": result.expiry,
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&payload)?);

    Ok(())
}

pub fn PrintAdded(keyword: &str, path: &PathBuf, expire: Option<u64>) {
    match expire {
        Some(ts) => println!(
            "{} {} → {} (expires {})",
            "Added".green(),
            keyword.bold().cyan(),
            path.display().to_string().dimmed(),
            ts
        ),
        None => println!(
            "{} {} → {}",
            "Added".green(),
            keyword.bold().cyan(),
            path.display().to_string().dimmed()
        ),
    }
}

pub fn PrintAlreadyPresent(
    keyword: &str,
    path: &PathBuf,
    expire: Option<u64>,
    expiryChanged: bool,
) {
    let base = format!(
        "Keyword '{}' already points to {}",
        keyword.bold().cyan(),
        path.display().to_string().dimmed()
    );

    if expiryChanged {
        match expire {
            Some(ts) => println!("{} (expiry updated to {})", base, ts),
            None => println!("{} (expiry cleared)", base),
        }
    } else {
        println!("{} (unchanged)", base);
    }
}

pub fn PrintReplaced(keyword: &str, previous: &PathBuf, newPath: &PathBuf, expire: Option<u64>) {
    match expire {
        Some(ts) => println!(
            "{} {}: {} → {} (expires {})",
            "Replaced".yellow(),
            keyword.bold().cyan(),
            previous.display().to_string().dimmed(),
            newPath.display().to_string().dimmed(),
            ts
        ),
        None => println!(
            "{} {}: {} → {}",
            "Replaced".yellow(),
            keyword.bold().cyan(),
            previous.display().to_string().dimmed(),
            newPath.display().to_string().dimmed()
        ),
    }
}

pub fn PrintDuplicateNote(keywords: &[String]) {
    if keywords.is_empty() {
        return;
    }

    let joined = keywords.join(", ");

    println!(
        "Note: this path is also saved under keyword(s): {}",
        joined.bold().cyan()
    );
}

pub fn PrintAddOutcome(
    keyword: &str,
    resolvedPath: &PathBuf,
    expire: Option<u64>,
    outcome: &AddOutcome,
) {
    match outcome {
        AddOutcome::Added {
            path,
            expiry,
            duplicateKeywords,
        } => {
            PrintAdded(keyword, path, expire.or(*expiry));
            PrintDuplicateNote(duplicateKeywords);
        }
        AddOutcome::AlreadyPresent {
            path,
            expiry,
            expiryChanged,
        } => {
            PrintAlreadyPresent(keyword, path, expire.or(*expiry), *expiryChanged);
        }
        AddOutcome::Replaced {
            previousPath,
            newPath,
            expiry,
        } => {
            PrintReplaced(keyword, previousPath, newPath, expire.or(*expiry));
        }
    }

    // Provide resolved target for caller visibility when it differs by canonicalization.
    if resolvedPath
        != match outcome {
            AddOutcome::Added { path, .. } => path,
            AddOutcome::AlreadyPresent { path, .. } => path,
            AddOutcome::Replaced { newPath, .. } => newPath,
        }
    {
        println!(
            "Resolved path: {}",
            resolvedPath.display().to_string().dimmed()
        );
    }
}

pub fn PrintBulkAdded(keywords: &[String]) {
    if keywords.is_empty() {
        println!("{}", "No directories matched.".yellow());
        return;
    }

    for keyword in keywords {
        println!("{} {}", "Added".green(), keyword.bold().cyan());
    }
}

pub fn PrintCopy(existing: &str, newValue: &str) {
    println!(
        "{} {} → {}",
        "Copied".green(),
        existing.bold().cyan(),
        newValue.bold().cyan()
    );
}

pub fn PrintRemoved(keyword: &str) {
    println!("{} {}", "Removed".green(), keyword.bold().cyan());
}

pub fn PrintJump(path: &PathBuf) {
    println!(
        "{} {}",
        "Changed directory to".green(),
        path.display().to_string().dimmed()
    );
}

pub fn PrintCreatedAndJumped(path: &PathBuf) {
    println!(
        "{} {}",
        "Created and changed directory to".green(),
        path.display().to_string().dimmed()
    );
}

pub fn PrintSortMode(mode: &str) {
    println!("Sorting mode set to {}", mode.bold().cyan());
}

pub fn PrintCurrentSortMode(mode: &crate::store::SortMode) {
    println!("Current sorting mode: {}", mode.ToLabel().bold().cyan());
}

trait SortModeLabel {
    fn ToLabel(&self) -> String;
}

impl SortModeLabel for crate::store::SortMode {
    fn ToLabel(&self) -> String {
        match self {
            crate::store::SortMode::Added => "added".to_string(),
            crate::store::SortMode::Alpha => "alpha".to_string(),
            crate::store::SortMode::Recent => "recent".to_string(),
        }
    }
}
