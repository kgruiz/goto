use crate::paths::ConfigPaths;
use anyhow::{anyhow, bail, Context, Result};
use glob::glob;
use natord::compare;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
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

    pub fn AddShortcut(&mut self, keyword: &str, targetPath: &Path, expire: Option<u64>) -> Result<()> {

        if self.index.contains_key(keyword) {
            bail!("Error: Keyword '{keyword}' already exists.");
        }

        if !targetPath.exists() {
            bail!("Error: Path '{}' does not exist.", targetPath.display());
        }

        if !targetPath.is_dir() {
            bail!("Error: Path '{}' exists but is not a directory.", targetPath.display());
        }

        let absPath = targetPath
            .canonicalize()
            .with_context(|| format!("Failed to resolve '{}'", targetPath.display()))?;

        let entry = ShortcutEntry {
            keyword: keyword.to_string(),
            path: absPath.clone(),
        };

        self.index.insert(keyword.to_string(), self.entries.len());

        self.entries.push(entry);

        match expire {
            Some(ts) => {
                self.expiries.insert(keyword.to_string(), ts);
            }
            None => {
                self.expiries.remove(keyword);
            }
        }

        WriteConfig(&self.paths.configFile, &self.entries)?;

        WriteMeta(&self.paths.metaFile, &self.expiries)?;

        Ok(())
    }

    pub fn AddBulk(&mut self, pattern: &str) -> Result<Vec<String>> {

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

                self.AddShortcut(keyword, &path, None)?;

                added.push(keyword.to_string());
            }
        }

        Ok(added)
    }

    pub fn CopyShortcut(&mut self, existing: &str, newValue: &str) -> Result<()> {

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

        self.AddShortcut(&destKeyword, &destPath, None)
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
                let remainder = input.strip_prefix(&prefix).unwrap_or("").trim_start_matches('/');

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

    fn FetchEntry(&self, keyword: &str) -> Result<ShortcutEntry> {

        let index = self
            .index
            .get(keyword)
            .copied()
            .ok_or_else(|| anyhow!("Error: Keyword '{}' not found.", keyword))?;

        let entry = self.entries.get(index).ok_or_else(|| anyhow!("Internal error resolving '{}'", keyword))?;

        Ok(entry.clone())
    }

    fn RebuildIndex(&mut self) {

        self.index.clear();

        for (idx, entry) in self.entries.iter().enumerate() {
            self.index.insert(entry.keyword.clone(), idx);
        }
    }
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

    let mut file = File::create(path)?;

    for entry in entries {
        writeln!(file, "{}={}", entry.keyword, entry.path.display())?;
    }

    Ok(())
}

fn WriteMeta(path: &Path, expiries: &HashMap<String, u64>) -> Result<()> {

    if expiries.is_empty() {
        File::create(path)?;
        return Ok(());
    }

    let mut file = File::create(path)?;

    for (key, value) in expiries {
        writeln!(file, "{}={}", key, value)?;
    }

    Ok(())
}

fn WriteRecents(path: &Path, recents: &HashMap<String, u64>) -> Result<()> {

    let mut file = File::create(path)?;

    for (key, value) in recents {
        writeln!(file, "{}={}", key, value)?;
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

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;

    for line in lines {
        writeln!(file, "{line}")?;
    }

    let value = match mode {
        SortMode::Added => "added",
        SortMode::Alpha => "alpha",
        SortMode::Recent => "recent",
    };

    writeln!(file, "sort_order={value}")?;

    Ok(())
}
