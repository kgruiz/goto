use crate::store::Store;
use owo_colors::OwoColorize;
use std::path::PathBuf;

pub fn PrintHelp() {
    println!("{}", "to - Persistent Directory Shortcuts (Rust)".yellow());
    println!();

    println!("{}", "Usage:".magenta());
    println!(
        "  {:<55}{}",
        "goto <keyword>",
        "Navigate to saved shortcut".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --add, -a <keyword> <path> [--expire <timestamp>]",
        "Save new shortcut".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --add <path> [--expire <timestamp>]",
        "Save shortcut using directory name as keyword".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --add-bulk <pattern>",
        "Add shortcuts for each matching directory".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --copy <existing> <new>",
        "Duplicate shortcut".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --rm, -r <keyword>",
        "Remove existing shortcut".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --list, -l",
        "List all shortcuts".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --print-path, -p <keyword>",
        "Print stored path only".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --cursor, -c <keyword>",
        "Open in Cursor after navigation".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --no-create",
        "Do not create nested path on jump".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --sort, -s <mode>",
        "Set sorting mode (added | alpha | recent)".dimmed()
    );
    println!(
        "  {:<55}{}",
        "goto --show-sort",
        "Print current sorting mode".dimmed()
    );
    println!("  {:<55}{}", "goto --help, -h", "Show this help".dimmed());
}

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
