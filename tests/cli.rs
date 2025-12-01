#![allow(non_snake_case)]

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn BuildCommand(temp: &TempDir) -> Command {

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("goto"));

    let home = temp.path().to_path_buf();

    let configFile = home.join(".to_dirs");
    let metaFile = home.join(".to_dirs_meta");
    let userConfigFile = home.join(".to_zsh_config");
    let recentFile = home.join(".to_dirs_recent");

    cmd.env("HOME", &home);
    cmd.env("TO_CONFIG_FILE", &configFile);
    cmd.env("TO_CONFIG_META_FILE", &metaFile);
    cmd.env("TO_USER_CONFIG_FILE", &userConfigFile);
    cmd.env("TO_RECENT_FILE", &recentFile);

    cmd
}

fn MakeDir(base: &TempDir, name: &str) -> PathBuf {

    let path = base.path().join(name);

    fs::create_dir_all(&path).expect("create dir");

    path
}

#[test]
fn HelpDisplaysWhenNoArgs() {

    let temp = TempDir::new().unwrap();

    BuildCommand(&temp)
        .assert()
        .success()
        .stdout(contains("Usage:"))
        .stdout(contains("No shortcuts saved."));
}

#[test]
fn AddAndListShortcut() {

    let temp = TempDir::new().unwrap();

    let projectDir = MakeDir(&temp, "project");

    BuildCommand(&temp)
        .args(["--add", "proj", projectDir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("Added"));

    BuildCommand(&temp)
        .arg("--list")
        .assert()
        .success()
        .stdout(contains("proj"));

    BuildCommand(&temp)
        .args(["--print-path", "proj"])
        .assert()
        .success()
        .stdout(contains(projectDir.to_str().unwrap()));
}

#[test]
fn AddWithoutKeywordUsesBasename() {

    let temp = TempDir::new().unwrap();

    let dir = MakeDir(&temp, "alpha");

    BuildCommand(&temp)
        .args(["--add", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("alpha"));

    let configPath = temp.path().join(".to_dirs");

    let contents = fs::read_to_string(configPath).unwrap();

    assert!(contents.contains("alpha="));
}

#[test]
fn CopyWithNewKeywordKeepsPath() {

    let temp = TempDir::new().unwrap();

    let dir = MakeDir(&temp, "source");

    BuildCommand(&temp)
        .args(["--add", "src", dir.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--copy", "src", "clone"])
        .assert()
        .success()
        .stdout(contains("Copied"));

    BuildCommand(&temp)
        .arg("--list")
        .assert()
        .success()
        .stdout(contains("clone"));
}

#[test]
fn JumpCreatesWhenAllowed() {

    let temp = TempDir::new().unwrap();

    let base = MakeDir(&temp, "base");

    BuildCommand(&temp)
        .args(["--add", "base", base.to_str().unwrap()])
        .assert()
        .success();

    let target = base.join("nested/deeper");

    BuildCommand(&temp)
        .arg("base/nested/deeper")
        .assert()
        .success()
        .stdout(contains("Created and changed directory"));

    assert!(target.exists());

    let recents = fs::read_to_string(temp.path().join(".to_dirs_recent")).unwrap();

    assert!(recents.contains("base="));
}

#[test]
fn JumpWithoutCreateFailsWhenFlagSet() {

    let temp = TempDir::new().unwrap();

    let base = MakeDir(&temp, "base");

    BuildCommand(&temp)
        .args(["--add", "base", base.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--no-create", "base/nested"])
        .assert()
        .failure()
        .stderr(contains("does not exist"));
}

#[test]
fn AddBulkAddsAllDirectories() {

    let temp = TempDir::new().unwrap();

    let roots = MakeDir(&temp, "roots");

    let first = roots.join("one");
    let second = roots.join("two");

    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();

    let pattern = format!("{}/roots/*", temp.path().display());

    BuildCommand(&temp)
        .args(["--add-bulk", &pattern])
        .assert()
        .success()
        .stdout(contains("Added"));

    let config = fs::read_to_string(temp.path().join(".to_dirs")).unwrap();

    assert!(config.contains("one="));
    assert!(config.contains("two="));
}

#[test]
fn CompletionsIncludeOptions() {

    let temp = TempDir::new().unwrap();

    BuildCommand(&temp)
        .args(["--generate-completions", "zsh"])
        .assert()
        .success()
        .stdout(contains("--add-bulk"))
        .stdout(contains("--copy"))
        .stdout(contains("--no-create"))
        .stdout(contains("--sort"));
}

#[test]
fn CompleteKeywordsFiltersByPrefix() {

    let temp = TempDir::new().unwrap();

    let dirA = MakeDir(&temp, "apple");
    let dirB = MakeDir(&temp, "banana");

    BuildCommand(&temp)
        .args(["--add", "app", dirA.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "ban", dirB.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--__complete-mode", "keywords", "--__complete-input", "a"])
        .assert()
        .success()
        .stdout(contains("app"))
        .stdout(contains("ban").not());
}

#[test]
fn CompleteTargetsAddsSubpaths() {

    let temp = TempDir::new().unwrap();

    let base = MakeDir(&temp, "base");
    let nested = base.join("src");
    fs::create_dir_all(&nested).unwrap();

    BuildCommand(&temp)
        .args(["--add", "base", base.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args([
            "--__complete-mode",
            "targets",
            "--__complete-input",
            "base/s",
        ])
        .assert()
        .success()
        .stdout(contains("base/src/"));
}
