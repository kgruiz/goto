#![allow(non_snake_case)]

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn BuildCommand(temp: &TempDir) -> Command {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("to"));

    let home = temp.path().to_path_buf();
    let goto_root = home.join(".goto");
    std::fs::create_dir_all(&goto_root).unwrap();

    let configFile = goto_root.join("to_dirs");
    let metaFile = goto_root.join("to_dirs_meta");
    let userConfigFile = goto_root.join("to_zsh_config");
    let recentFile = goto_root.join("to_dirs_recent");

    cmd.env("HOME", &home);
    cmd.env("TO_CONFIG_FILE", &configFile);
    cmd.env("TO_CONFIG_META_FILE", &metaFile);
    cmd.env("TO_USER_CONFIG_FILE", &userConfigFile);
    cmd.env("TO_RECENT_FILE", &recentFile);
    cmd.env("NO_COLOR", "1");
    cmd.env("GOTO_SKIP_LEGACY_CHECK", "1");
    cmd.env("GOTO_ASSUME_YES", "1");

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

    let configPath = temp.path().join(".goto/to_dirs");

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

    let recents = fs::read_to_string(temp.path().join(".goto/to_dirs_recent")).unwrap();

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
fn CodeAndCursorAreMutuallyExclusive() {
    let temp = TempDir::new().unwrap();

    let base = MakeDir(&temp, "base");

    BuildCommand(&temp)
        .args(["--add", "base", base.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["base", "--cursor", "--code"])
        .assert()
        .failure()
        .stderr(contains("mutually exclusive"));
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
        .args(["--bulk-add", &pattern])
        .assert()
        .success()
        .stdout(contains("Added"));

    let config = fs::read_to_string(temp.path().join(".goto/to_dirs")).unwrap();

    assert!(config.contains("one="));
    assert!(config.contains("two="));
}

#[test]
fn CompletionsIncludeOptions() {
    let temp = TempDir::new().unwrap();

    BuildCommand(&temp)
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(contains("--bulk-add"))
        .stdout(contains("--copy"))
        .stdout(contains("--no-create"))
        .stdout(contains("--sort"))
        .stdout(contains("--show-sort"))
        .stdout(contains("--force"))
        .stdout(contains("--within"))
        .stdout(contains("--here"))
        .stdout(contains("--path-only"));
}

#[test]
fn WriteDefaultCompletionsWritesFile() {
    let temp = TempDir::new().unwrap();

    let xdg_root = temp.path().join("xdg");

    BuildCommand(&temp)
        .env("XDG_CONFIG_HOME", &xdg_root)
        .args(["--completions", "zsh", "--write-completions"])
        .assert()
        .success();

    let target = xdg_root.join("zsh/completions/_to");

    let contents = fs::read_to_string(&target).expect("completion file exists");

    assert!(contents.contains("--list"));
    assert!(contents.contains("--bulk-add"));
}

#[test]
fn ShowSortModePrintsCurrent() {
    let temp = TempDir::new().unwrap();

    BuildCommand(&temp)
        .arg("--show-sort")
        .assert()
        .success()
        .stdout(contains("alpha"));

    BuildCommand(&temp)
        .args(["--sort", "recent"])
        .assert()
        .success();

    BuildCommand(&temp)
        .arg("--show-sort")
        .assert()
        .success()
        .stdout(contains("recent"));
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

#[test]
fn SearchFiltersByKeywordAndPath() {
    let temp = TempDir::new().unwrap();

    let alpha = MakeDir(&temp, "alpha");

    let nested = MakeDir(&temp, "projects/client-a");

    BuildCommand(&temp)
        .args(["--add", "alpha", alpha.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "proj", nested.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--list", "proj", "--path-only"])
        .assert()
        .success()
        .stdout(contains("proj"))
        .stdout(contains("alpha").not());
}

#[test]
fn ListHereScopesToCurrentDir() {
    let temp = TempDir::new().unwrap();

    let workspace = MakeDir(&temp, "workspace");
    let inside = workspace.join("inside");
    fs::create_dir_all(&inside).unwrap();
    let outside = MakeDir(&temp, "outside");

    BuildCommand(&temp)
        .args(["--add", "in", inside.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "out", outside.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .current_dir(&workspace)
        .args(["--list", "--here"])
        .assert()
        .success()
        .stdout(contains("in"))
        .stdout(contains("out").not());
}

#[test]
fn ListWithinScopesToProvidedRoot() {
    let temp = TempDir::new().unwrap();

    let workspace = MakeDir(&temp, "workspace");
    let inside = workspace.join("nested");
    fs::create_dir_all(&inside).unwrap();
    let outside = MakeDir(&temp, "elsewhere");

    BuildCommand(&temp)
        .args(["--add", "nested", inside.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "elsewhere", outside.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--list", "--within", workspace.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("nested"))
        .stdout(contains("elsewhere").not());
}

#[test]
fn ListMaxDepthLimitsResults() {
    let temp = TempDir::new().unwrap();

    let workspace = MakeDir(&temp, "workspace");
    let nested = workspace.join("deep/nested");
    fs::create_dir_all(&nested).unwrap();

    BuildCommand(&temp)
        .args(["--add", "root", workspace.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "deep", nested.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .current_dir(&workspace)
        .args(["--list", "--here", "--max-depth", "0"])
        .assert()
        .success()
        .stdout(contains("root"))
        .stdout(contains("deep").not());
}

#[test]
fn SearchAliasListsAllWhenEmpty() {
    let temp = TempDir::new().unwrap();

    let first = MakeDir(&temp, "first");

    let second = MakeDir(&temp, "second");

    BuildCommand(&temp)
        .args(["--add", "one", first.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "two", second.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .arg("--list")
        .assert()
        .success()
        .stdout(contains("one"))
        .stdout(contains("two"));
}

#[test]
fn SearchJsonOutputsValid() {
    let temp = TempDir::new().unwrap();

    let dir = MakeDir(&temp, "json-dir");

    BuildCommand(&temp)
        .args(["--add", "json", dir.to_str().unwrap()])
        .assert()
        .success();

    let output = BuildCommand(&temp)
        .args(["--list", "json", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let as_str = String::from_utf8(output).unwrap();

    let parsed: Value = serde_json::from_str(&as_str).unwrap();

    assert!(parsed.is_array());
}

#[test]
fn AddSameKeywordSamePathIsNoOp() {
    let temp = TempDir::new().unwrap();

    let dir = MakeDir(&temp, "alpha");

    BuildCommand(&temp)
        .args(["--add", "alpha", dir.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "alpha", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("already points"));
}

#[test]
fn AddSameKeywordDifferentPathRequiresForce() {
    let temp = TempDir::new().unwrap();

    let first = MakeDir(&temp, "one");
    let second = MakeDir(&temp, "two");

    BuildCommand(&temp)
        .args(["--add", "proj", first.to_str().unwrap()])
        .assert()
        .success();

    BuildCommand(&temp)
        .args(["--add", "proj", second.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(contains("already exists"));

    BuildCommand(&temp)
        .args(["--add", "proj", second.to_str().unwrap(), "--force"])
        .assert()
        .success()
        .stdout(contains("Replaced"));

    let config = fs::read_to_string(temp.path().join(".goto/to_dirs")).unwrap();

    assert!(config.contains(second.to_str().unwrap()));
}

#[test]
fn AddDifferentKeywordSamePathHonorsAssumeYes() {
    let temp = TempDir::new().unwrap();

    let dir = MakeDir(&temp, "shared");

    BuildCommand(&temp)
        .args(["--add", "one", dir.to_str().unwrap()])
        .assert()
        .success();

    let mut without_assume = BuildCommand(&temp);
    without_assume.env_remove("GOTO_ASSUME_YES");

    without_assume
        .args(["--add", "two", dir.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(contains("Aborted adding"));

    BuildCommand(&temp)
        .args(["--add", "two", dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("also saved under"));
}

#[test]
fn CheckWrapperFlagDetectsPresence() {
    let temp = TempDir::new().unwrap();

    let rc_path = temp.path().join(".zshrc");

    fs::write(
        &rc_path,
        "# >>> goto init >>>\nGOTO_FUNC_PATH=\"${XDG_CONFIG_HOME:-$HOME/.config}/zsh/plugins/goto/goto.zsh\"\n# <<< goto init <<<\n",
    )
    .unwrap();

    BuildCommand(&temp)
        .args(["--__check-wrapper", rc_path.to_str().unwrap()])
        .assert()
        .success();

    fs::write(&rc_path, "# empty\n").unwrap();

    BuildCommand(&temp)
        .args(["--__check-wrapper", rc_path.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn InstallWrapperAddsWhenMissing() {
    let temp = TempDir::new().unwrap();

    let rc_dir = temp.path().join("zdot");
    fs::create_dir_all(&rc_dir).unwrap();
    let rc_path = rc_dir.join(".zshrc");

    let mut cmd = BuildCommand(&temp);
    cmd.env("SHELL", "/bin/zsh");
    cmd.env("ZDOTDIR", rc_dir.to_str().unwrap());

    cmd.arg("--install-wrapper")
        .assert()
        .success()
        .stdout(contains("Wrapper added"));

    let contents = fs::read_to_string(&rc_path).unwrap();
    assert!(contents.contains("# >>> goto init >>>"));

    let mut second = BuildCommand(&temp);
    second.env("SHELL", "/bin/zsh");
    second.env("ZDOTDIR", rc_dir.to_str().unwrap());

    second
        .arg("--install-wrapper")
        .assert()
        .success()
        .stdout(contains("Wrapper already present"));
}
