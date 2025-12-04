# to

Persistent, keyword-based directory shortcuts with longest-prefix matching, expirations, recents, bulk add, and shell completions — reimplemented in Rust.

> Note: As a standalone binary, `goto` cannot change the parent shell’s directory. Wrap it in a shell function to `cd` into the printed path (see “Shell integration”).

## Installation

Install directly from the repository (latest main):

```bash
cargo install --git https://github.com/kgruiz/goto.git to
```

Or build from a local checkout (handy while iterating):

```bash
git clone https://github.com/kgruiz/goto.git
cd goto
cargo build --release
```

If you already have the repo locally, you can run the helper script which wraps `cargo install --locked --force` against the current checkout:

```bash
./install
```

Any extra flags are forwarded to `cargo install` (e.g., `./install --features foo`). Requires Rust **1.85+** (edition 2024).

## Quick start

```bash
to --add proj ~/code/my-project          # save keyword
to proj/src/lib                          # jump (creates missing dirs)
to --print-path proj/docs                # print resolved path
to --list proj                           # search keyword + path
to --list                                # list all shortcuts
to --rm proj                             # remove
```

## Features

- Keyword shortcuts stored in `~/.goto/to_dirs`; expirations in `~/.goto/to_dirs_meta`; recents in `~/.goto/to_dirs_recent`; sort preference in `~/.goto/to_zsh_config`.
- Longest-prefix resolution for `keyword/any/depth`.
- Automatic directory creation (opt out with `--no-create`).
- Recents tracking for `recent` sort mode.
- Expiring shortcuts via `--expire <epoch>`.
- Bulk add via glob patterns; copy keywords or retarget paths.
- Search shortcuts by keyword and/or path with substring, glob, or regex matching.
- Colorful, zsh-like output (disable with `--no-color` or `NO_COLOR=1`).
- Shell completions with dynamic keyword/path suggestions.

## Options (summary)

- `-a, --add [<keyword>] <path> [-x|--expire <ts>]`
- `-b, --bulk-add <pattern>`
- `-c, --copy <existing> <new>`
- `-f, --force` (with add/copy/bulk-add) to replace an existing keyword or skip duplicate-path confirmation
- `-r, --remove <keyword>`
- `-l, --list[=QUERY] [-g|--glob] [-e|--regex] [-k|--keyword-only] [-y|--path-only] [-B|--both] [-w|--within <path> | -H|--here] [-d|--max-depth N] [-j|--json] [-n|--limit N]`
- `-p, --print-path <target>`
- `-u, --cursor` (open in Cursor) or `-C, --code` (open in VS Code) — mutually exclusive
- `-N, --no-create`
- `-s, --sort added|alpha|recent` (and `--show-sort`)
- `--completions <shell>` (alias: `--generate-completions`)
- `--write-default-completions` (with `--completions zsh`; aliases: `--write-completions`, `--install-completions`)
- `--no-color`

## Shell integration (cd)

`to` prints the resolved path; to change your shell directory, wrap it:

```zsh
function to() {
  local dest
  dest="$(command to --print-path "$@")" || return
  [ -z "$dest" ] && return
  mkdir -p "$dest"
  cd "$dest"
}
```

For cursor support, keep passing `-u` to the binary; the wrapper simply handles `cd`.

## Completions

Generate completion scripts:

```bash
to --completions zsh   > _to
to --completions bash  > to.bash
to --completions fish  > to.fish

# write zsh completions to the default location (~/.config/zsh/completions/_to)
to --completions zsh --write-completions
```

Zsh uses dynamic completion hooks for path-aware keyword + subpath behavior.

## Notes

- Implemented in Rust; uses the same config files for compatibility.
- Dynamic completion handled by the binary; no sourcing a large shell script.
- Requires a wrapper to `cd` (common for standalone binaries).

## Configuration details

- Files: `~/.goto/to_dirs`, `~/.goto/to_dirs_meta`, `~/.goto/to_dirs_recent`, `~/.goto/to_zsh_config`.
- Env overrides (useful for testing or custom setups): `TO_CONFIG_FILE`, `TO_CONFIG_META_FILE`, `TO_USER_CONFIG_FILE`, `TO_RECENT_FILE`.
- Env overrides (useful for tests): `TO_CONFIG_FILE`, `TO_CONFIG_META_FILE`, `TO_USER_CONFIG_FILE`, `TO_RECENT_FILE`.
- Sorting: `alpha` (natural), `added` (file order), `recent` (uses recents file).

## Search

- `to --list QUERY` searches keywords and paths with case-insensitive substring matching by default; omit QUERY to list everything.
- Scope fields with `-k/--keyword-only` or `-y/--path-only`; combine with `-B/--both` to require matches in both.
- Scope results to a root with `-w/--within <path>` or `-H/--here`, and limit depth with `-d/--max-depth N` (0 = root only).
- Pattern modes: substring (default), `-g/--glob`, or `-e/--regex` (case-insensitive). Quote patterns to avoid shell expansion.
- Output as JSON with `-j/--json`; limit rows with `-n/--limit`.

## MSRV

Minimum supported Rust version: **1.85** (edition 2024). Update `Cargo.toml` and release notes if this changes.

## License

GPL-3.0-only. See `LICENSE`.

## Changelog

See `CHANGELOG.md`.
