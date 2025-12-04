use crate::paths::ConfigPaths;
use anyhow::{Context, Result, anyhow, bail};
use fd_lock::RwLock;
use glob::{Pattern, glob};
use natord::compare;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::io::IsTerminal;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortMode {
    Added,
    Alpha,
    Recent,
}

#[derive(Debug, Clone)]
pub struct ShortcutEntry {
    pub keyword: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub keyword: String,
    pub path: PathBuf,
    pub expiry: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum SearchMode {
    Substring(String),
    Glob(Pattern),
    Regex(Regex),
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub matchKeyword: bool,
    pub matchPath: bool,
    pub requireBoth: bool,
    pub mode: SearchMode,
    pub limit: Option<usize>,
    pub within: Option<PathBuf>,
    pub maxDepth: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct AddBehavior {
    pub force: bool,
    pub assumeYes: bool,
}

#[derive(Debug, Clone)]
pub enum AddOutcome {
    Added {
        path: PathBuf,
        expiry: Option<u64>,
        duplicateKeywords: Vec<String>,
    },
    AlreadyPresent {
        path: PathBuf,
        expiry: Option<u64>,
        expiryChanged: bool,
    },
    Replaced {
        previousPath: PathBuf,
        newPath: PathBuf,
        expiry: Option<u64>,
    },
}

impl SearchMode {
    pub fn matches(&self, value: &str) -> bool {
        match self {
            SearchMode::Substring(query) => {
                let haystack = value.to_lowercase();
                let needle = query.to_lowercase();

                haystack.contains(&needle)
            }
            SearchMode::Glob(pattern) => pattern.matches(value),
            SearchMode::Regex(regex) => regex.is_match(value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedJump {
    pub keyword: String,
    pub basePath: PathBuf,
    pub targetPath: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Store {
    pub entries: Vec<ShortcutEntry>,
    pub expiries: HashMap<String, u64>,
    pub recents: HashMap<String, u64>,
    pub paths: ConfigPaths,
    pub sortMode: SortMode,
    index: HashMap<String, usize>,
}

impl Store {
    pub fn Load(paths: ConfigPaths) -> Result<Self> {
        EnsureFilesExist(&paths)?;

        let mut expiries = LoadNumberMap(&paths.metaFile)?;

        let recents = LoadNumberMap(&paths.recentFile)?;

        let rawEntries = LoadConfigEntries(&paths.configFile)?;

        let now = CurrentEpoch();

        let mut entries = Vec::new();

        let mut index = HashMap::new();

        let mut removedExpired = false;

        for entry in rawEntries {
            let maybeExpiry = expiries.get(&entry.keyword).copied();

            if let Some(expiry) = maybeExpiry {
                if expiry <= now {
                    expiries.remove(&entry.keyword);
                    removedExpired = true;
                    continue;
                }
            }

            index.insert(entry.keyword.clone(), entries.len());

            entries.push(entry);
        }

        if removedExpired {
            WriteConfig(&paths.configFile, &entries)?;

            WriteMeta(&paths.metaFile, &expiries)?;
        }

        let sortMode = LoadSortMode(&paths.userConfigFile)?;

        Ok(Self {
            entries,
            expiries,
            recents,
            paths,
            sortMode,
            index,
        })
    }

    pub fn SetSortMode(&mut self, mode: &str) -> Result<()> {
        let parsed = ParseSortMode(mode)?;

        WriteSortMode(&self.paths.userConfigFile, &parsed)?;

        self.sortMode = parsed;

        Ok(())
    }

    pub fn SortedKeywords(&self) -> Vec<String> {
        let mut keywords: Vec<String> = self.entries.iter().map(|e| e.keyword.clone()).collect();

        match self.sortMode {
            SortMode::Added => keywords,
            SortMode::Alpha => {
                keywords.sort_by(|a, b| compare(a, b));
                keywords
            }
            SortMode::Recent => {
                keywords.sort_by(|a, b| {
                    let aTs = self.recents.get(a).copied().unwrap_or(0);
                    let bTs = self.recents.get(b).copied().unwrap_or(0);
                    bTs.cmp(&aTs)
                });
                keywords
            }
        }
    }

    pub fn Search(&self, options: &SearchOptions) -> Vec<SearchResult> {
        let mut results = Vec::new();

        let keywords = self.SortedKeywords();

        let matchKeyword = if options.matchKeyword || options.matchPath {
            options.matchKeyword
        } else {
            true
        };

        let matchPath = if options.matchKeyword || options.matchPath {
            options.matchPath
        } else {
            true
        };

        let within = options.within.as_ref();

        for keyword in keywords {
            let entry = match self.entries.iter().find(|e| e.keyword == keyword) {
                Some(entry) => entry,
                None => continue,
            };

            if let Some(root) = within {
                let canonical = match entry.path.canonicalize() {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                if !canonical.starts_with(root) {
                    continue;
                }

                if let Some(maxDepth) = options.maxDepth {
                    let depth = match canonical.strip_prefix(root) {
                        Ok(remainder) => remainder.components().count(),
                        Err(_) => continue,
                    };

                    if depth > maxDepth {
                        continue;
                    }
                }
            }

            let keywordMatches = if matchKeyword {
                options.mode.matches(&entry.keyword)
            } else {
                false
            };

            let pathMatches = if matchPath {
                let pathStr = entry.path.to_string_lossy().to_string();

                options.mode.matches(&pathStr)
            } else {
                false
            };

            let include = if options.requireBoth && matchKeyword && matchPath {
                keywordMatches && pathMatches
            } else {
                (matchKeyword && keywordMatches) || (matchPath && pathMatches)
            };

            if include {
                results.push(SearchResult {
                    keyword: entry.keyword.clone(),
                    path: entry.path.clone(),
                    expiry: self.expiries.get(&entry.keyword).copied(),
                });

                if let Some(limit) = options.limit {
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }

        results
    }

    pub fn AddShortcut(
        &mut self,
        keyword: &str,
        targetPath: &Path,
        expire: Option<u64>,
        behavior: &AddBehavior,
    ) -> Result<AddOutcome> {
        if !targetPath.exists() {
            bail!("Error: Path '{}' does not exist.", targetPath.display());
        }

        if !targetPath.is_dir() {
            bail!(
                "Error: Path '{}' exists but is not a directory.",
                targetPath.display()
            );
        }

        let absPath = targetPath
            .canonicalize()
            .with_context(|| format!("Failed to resolve '{}'", targetPath.display()))?;

        let duplicateKeywords: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| entry.path == absPath && entry.keyword != keyword)
            .map(|entry| entry.keyword.clone())
            .collect();

        if let Some(position) = self.index.get(keyword).copied() {
            let (existingPath, samePath) = {
                let existing = self
                    .entries
                    .get(position)
                    .ok_or_else(|| anyhow!("Internal error resolving '{keyword}'"))?;

                (existing.path.clone(), existing.path == absPath)
            };

            if samePath {
                let (expiry, expiryChanged) = self.ApplyExpiry(keyword, expire);

                if expiryChanged {
                    WriteMeta(&self.paths.metaFile, &self.expiries)?;
                }

                return Ok(AddOutcome::AlreadyPresent {
                    path: existingPath,
                    expiry,
                    expiryChanged,
                });
            }

            if !behavior.force {
                bail!(
                    "Error: Keyword '{keyword}' already exists for '{}'. Re-run with --force to replace it with '{}'.",
                    existingPath.display(),
                    absPath.display()
                );
            }

            let previousPath = existingPath;

            self.entries[position].path = absPath.clone();

            let (expiry, _) = self.ApplyExpiry(keyword, expire);

            WriteConfig(&self.paths.configFile, &self.entries)?;

            WriteMeta(&self.paths.metaFile, &self.expiries)?;

            return Ok(AddOutcome::Replaced {
                previousPath,
                newPath: absPath,
                expiry,
            });
        }

        if !duplicateKeywords.is_empty() && !behavior.force {
            if behavior.assumeYes {
                // proceed
            } else {
                let confirmed = ConfirmDuplicatePath(&absPath, keyword, &duplicateKeywords)?;

                if !confirmed {
                    bail!(
                        "Aborted adding '{keyword}'. Use --force or set GOTO_ASSUME_YES=1 to proceed."
                    );
                }
            }
        }

        let entry = ShortcutEntry {
            keyword: keyword.to_string(),
            path: absPath.clone(),
        };

        self.index.insert(keyword.to_string(), self.entries.len());

        self.entries.push(entry);

        let (expiry, _) = self.ApplyExpiry(keyword, expire);

        WriteConfig(&self.paths.configFile, &self.entries)?;

        WriteMeta(&self.paths.metaFile, &self.expiries)?;

        Ok(AddOutcome::Added {
            path: absPath,
            expiry,
            duplicateKeywords,
        })
    }

    pub fn AddBulk(&mut self, pattern: &str, behavior: &AddBehavior) -> Result<Vec<String>> {
        let mut added = Vec::new();

        for entry in glob(pattern)? {
            let path = entry?;

            if path.is_dir() {
                let keyword = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or_else(|| anyhow!("Unable to derive keyword from '{}'", path.display()))?;

                if self.index.contains_key(keyword) {
                    continue;
                }

                self.AddShortcut(keyword, &path, None, behavior)?;

                added.push(keyword.to_string());
            }
        }

        Ok(added)
    }

    pub fn CopyShortcut(
        &mut self,
        existing: &str,
        newValue: &str,
        behavior: &AddBehavior,
    ) -> Result<()> {
        let existingEntry = self.FetchEntry(existing)?;

        let targetIsPath = Path::new(newValue).is_absolute() || Path::new(newValue).is_dir();

        let (destKeyword, destPath) = if targetIsPath {
            let destPath = PathBuf::from(newValue);

            let destKeyword = destPath
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow!("Unable to derive keyword from '{}'", newValue))?;

            (destKeyword.to_string(), destPath)
        } else {
            (newValue.to_string(), existingEntry.path.clone())
        };

        self.AddShortcut(&destKeyword, &destPath, None, behavior)
            .map(|_| ())
    }

    pub fn RemoveShortcut(&mut self, keyword: &str) -> Result<()> {
        let position = self
            .index
            .get(keyword)
            .copied()
            .ok_or_else(|| anyhow!("Error: Keyword '{}' not found.", keyword))?;

        self.entries.remove(position);

        self.RebuildIndex();

        self.expiries.remove(keyword);

        self.recents.remove(keyword);

        WriteConfig(&self.paths.configFile, &self.entries)?;

        WriteMeta(&self.paths.metaFile, &self.expiries)?;

        WriteRecents(&self.paths.recentFile, &self.recents)?;

        Ok(())
    }

    pub fn ResolveJump(&self, input: &str) -> Result<ResolvedJump> {
        let parts: Vec<&str> = input.split('/').collect();

        let mut prefixes = Vec::new();

        let mut current = String::new();

        for (idx, part) in parts.iter().enumerate() {
            if idx == 0 {
                current.push_str(part);
            } else {
                current.push('/');
                current.push_str(part);
            }

            prefixes.push(current.clone());
        }

        prefixes.reverse();

        for prefix in prefixes {
            if let Some(entry) = self.index.get(&prefix).and_then(|i| self.entries.get(*i)) {
                let remainder = input
                    .strip_prefix(&prefix)
                    .unwrap_or("")
                    .trim_start_matches('/');

                let mut targetPath = entry.path.clone();

                if !remainder.is_empty() {
                    targetPath.push(remainder);
                }

                return Ok(ResolvedJump {
                    keyword: entry.keyword.clone(),
                    basePath: entry.path.clone(),
                    targetPath,
                });
            }
        }

        bail!("Error: Shortcut or path '{}' not found.", input);
    }

    pub fn UpdateRecentUsage(&mut self, keyword: &str) -> Result<()> {
        let timestamp = CurrentEpoch();

        self.recents.insert(keyword.to_string(), timestamp);

        WriteRecents(&self.paths.recentFile, &self.recents)?;

        Ok(())
    }

    pub fn SaveRecents(&self) -> Result<()> {
        WriteRecents(&self.paths.recentFile, &self.recents)
    }

    pub fn ExpiryFor(&self, keyword: &str) -> Option<u64> {
        self.expiries.get(keyword).copied()
    }

    fn ApplyExpiry(&mut self, keyword: &str, expire: Option<u64>) -> (Option<u64>, bool) {
        let previous = self.expiries.get(keyword).copied();

        match expire {
            Some(ts) => {
                self.expiries.insert(keyword.to_string(), ts);
            }
            None => {
                self.expiries.remove(keyword);
            }
        }

        let current = self.expiries.get(keyword).copied();

        let changed = previous != current;

        (current, changed)
    }

    fn FetchEntry(&self, keyword: &str) -> Result<ShortcutEntry> {
        let index = self
            .index
            .get(keyword)
            .copied()
            .ok_or_else(|| anyhow!("Error: Keyword '{}' not found.", keyword))?;

        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| anyhow!("Internal error resolving '{}'", keyword))?;

        Ok(entry.clone())
    }

    fn RebuildIndex(&mut self) {
        self.index.clear();

        for (idx, entry) in self.entries.iter().enumerate() {
            self.index.insert(entry.keyword.clone(), idx);
        }
    }
}

fn ConfirmDuplicatePath(path: &Path, keyword: &str, existingKeywords: &[String]) -> Result<bool> {
    let joined = existingKeywords.join(", ");

    println!(
        "Path '{}' is already saved under keyword(s): {}.",
        path.display(),
        joined
    );

    print!("Add keyword '{}' for the same path? [y/N]: ", keyword);

    io::stdout().flush()?;

    if !io::stdin().is_terminal() {
        return Ok(false);
    }

    let mut input = String::new();

    io::stdin().read_line(&mut input)?;

    let normalized = input.trim().to_lowercase();

    Ok(normalized == "y" || normalized == "yes")
}

pub fn ParseSortMode(raw: &str) -> Result<SortMode> {
    match raw {
        "added" => Ok(SortMode::Added),
        "alpha" => Ok(SortMode::Alpha),
        "recent" => Ok(SortMode::Recent),
        _ => bail!("Invalid sort mode '{}'. Use added, alpha, or recent.", raw),
    }
}

fn EnsureFilesExist(paths: &ConfigPaths) -> Result<()> {
    EnsureParent(paths.configFile.parent())?;
    EnsureParent(paths.metaFile.parent())?;
    EnsureParent(paths.userConfigFile.parent())?;
    EnsureParent(paths.recentFile.parent())?;

    TouchIfMissing(&paths.configFile)?;
    TouchIfMissing(&paths.metaFile)?;
    TouchIfMissing(&paths.recentFile)?;

    Ok(())
}

fn EnsureParent(parent: Option<&Path>) -> Result<()> {
    if let Some(dir) = parent {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }

    Ok(())
}

fn TouchIfMissing(path: &Path) -> Result<()> {
    if !path.exists() {
        File::create(path)?;
    }

    Ok(())
}

fn LoadNumberMap(path: &Path) -> Result<HashMap<String, u64>> {
    let mut map = HashMap::new();

    if !path.exists() {
        return Ok(map);
    }

    let file = File::open(path)?;

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        if let Some((key, value)) = line.split_once('=') {
            if let Ok(number) = value.trim().parse::<u64>() {
                map.insert(key.to_string(), number);
            }
        }
    }

    Ok(map)
}

fn LoadConfigEntries(path: &Path) -> Result<Vec<ShortcutEntry>> {
    let mut entries = Vec::new();

    if !path.exists() {
        return Ok(entries);
    }

    let file = File::open(path)?;

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        if let Some((key, value)) = line.split_once('=') {
            if key.trim().is_empty() || value.trim().is_empty() {
                continue;
            }

            entries.push(ShortcutEntry {
                keyword: key.to_string(),
                path: PathBuf::from(value),
            });
        }
    }

    Ok(entries)
}

fn CurrentEpoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn WriteConfig(path: &Path, entries: &[ShortcutEntry]) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;

    let mut lock = RwLock::new(file);

    let mut guard = lock.write()?;

    guard.set_len(0)?;
    guard.seek(SeekFrom::Start(0))?;

    for entry in entries {
        writeln!(&mut *guard, "{}={}", entry.keyword, entry.path.display())?;
    }

    Ok(())
}

fn WriteMeta(path: &Path, expiries: &HashMap<String, u64>) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;

    let mut lock = RwLock::new(file);

    let mut guard = lock.write()?;

    guard.set_len(0)?;
    guard.seek(SeekFrom::Start(0))?;

    for (key, value) in expiries {
        writeln!(&mut *guard, "{}={}", key, value)?;
    }

    Ok(())
}

fn WriteRecents(path: &Path, recents: &HashMap<String, u64>) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;

    let mut lock = RwLock::new(file);

    let mut guard = lock.write()?;

    guard.set_len(0)?;
    guard.seek(SeekFrom::Start(0))?;

    for (key, value) in recents {
        writeln!(&mut *guard, "{}={}", key, value)?;
    }

    Ok(())
}

fn LoadSortMode(path: &Path) -> Result<SortMode> {
    if !path.exists() {
        return Ok(SortMode::Alpha);
    }

    let file = File::open(path)?;

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "sort_order" {
                return ParseSortMode(value.trim()).or(Ok(SortMode::Alpha));
            }
        }
    }

    Ok(SortMode::Alpha)
}

fn WriteSortMode(path: &Path, mode: &SortMode) -> Result<()> {
    let mut lines = Vec::new();

    if path.exists() {
        let file = File::open(path)?;

        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;

            if line.starts_with("sort_order=") {
                continue;
            }

            lines.push(line);
        }
    }

    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(path)?;

    let mut lock = RwLock::new(file);

    let mut guard = lock.write()?;

    guard.set_len(0)?;
    guard.seek(SeekFrom::Start(0))?;

    for line in lines {
        writeln!(&mut *guard, "{line}")?;
    }

    let value = match mode {
        SortMode::Added => "added",
        SortMode::Alpha => "alpha",
        SortMode::Recent => "recent",
    };

    writeln!(&mut *guard, "sort_order={value}")?;

    Ok(())
}
