use anyhow::{Result, anyhow};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub configFile: PathBuf,
    pub metaFile: PathBuf,
    pub userConfigFile: PathBuf,
    pub recentFile: PathBuf,
}

impl ConfigPaths {
    pub fn Resolve() -> Result<Self> {
        let home = env::var("HOME").map_err(|_| anyhow!("HOME is not set"))?;
        let root = Path::new(&home).join(".goto");

        let root_str = root.to_string_lossy().to_string();

        let configFile = ResolvePath("TO_CONFIG_FILE", &root_str, "to_dirs");
        let metaFile = ResolvePath("TO_CONFIG_META_FILE", &root_str, "to_dirs_meta");
        let userConfigFile = ResolvePath("TO_USER_CONFIG_FILE", &root_str, "to_zsh_config");
        let recentFile = ResolvePath("TO_RECENT_FILE", &root_str, "to_dirs_recent");

        Ok(Self {
            configFile,
            metaFile,
            userConfigFile,
            recentFile,
        })
    }
}

fn ResolvePath(envKey: &str, home: &str, defaultName: &str) -> PathBuf {
    let envValue = env::var(envKey).ok();

    match envValue {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => Path::new(home).join(defaultName),
    }
}
