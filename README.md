# goto

Persistent, keyword-based directory shortcuts with longest-prefix matching, expirations, recents, bulk add, and shell completions — reimplemented in Rust.

> Note: As a standalone binary, `goto` cannot change the parent shell’s directory. Wrap it in a shell function to `cd` into the printed path (see “Shell integration”).

## Install

```bash
cargo install --path .
```

Requires Rust **1.85+** (edition 2024).

## Quick start

```bash
goto --add proj ~/code/my-project          # save keyword
goto proj/src/lib                          # jump (creates missing dirs)
goto --print-path proj/docs                # print resolved path
goto --list                                # list saved shortcuts
goto --rm proj                             # remove
```

## Features

- Keyword shortcuts stored in `~/.to_dirs`; expirations in `~/.to_dirs_meta`; recents in `~/.to_dirs_recent`; sort preference in `~/.to_zsh_config`.
- Longest-prefix resolution for `keyword/any/depth`.
- Automatic directory creation (opt out with `--no-create`).
- Recents tracking for `recent` sort mode.
- Expiring shortcuts via `--expire <epoch>`.
- Bulk add via glob patterns; copy keywords or retarget paths.
- Colorful, zsh-like output (disable with `--no-color` or `NO_COLOR=1`).
- Shell completions with dynamic keyword/path suggestions.

## Options (summary)

- `--add, -a [<keyword>] <path> [--expire <ts>]`
- `--add-bulk <pattern>`
- `--copy <existing> <new>`
- `--rm, -r <keyword>`
- `--list, -l`
- `--print-path, -p <target>`
- `--cursor, -c`
- `--no-create`
- `--sort, -s added|alpha|recent`
- `--generate-completions <shell>`
- `--no-color`

## Shell integration (cd)

`goto` prints the resolved path; to change your shell directory, wrap it:

```zsh
function goto() {
  local dest
  dest="$(command goto "$@")" || return
  cd "$dest"
}
```

For cursor support, keep passing `-c` to the binary; the wrapper simply handles `cd`.

## Completions

Generate completion scripts:

```bash
goto --generate-completions zsh   > _goto
goto --generate-completions bash  > goto.bash
goto --generate-completions fish  > goto.fish
```

Zsh uses dynamic completion hooks for path-aware keyword + subpath behavior.

## Notes

- Implemented in Rust; uses the same config files for compatibility.
- Dynamic completion handled by the binary; no sourcing a large shell script.
- Requires a wrapper to `cd` (common for standalone binaries).

## Configuration details

- Files: `~/.to_dirs`, `~/.to_dirs_meta`, `~/.to_dirs_recent`, `~/.to_zsh_config`.
- Env overrides (useful for tests): `TO_CONFIG_FILE`, `TO_CONFIG_META_FILE`, `TO_USER_CONFIG_FILE`, `TO_RECENT_FILE`.
- Sorting: `alpha` (natural), `added` (file order), `recent` (uses recents file).

## MSRV

Minimum supported Rust version: **1.85** (edition 2024). Update `Cargo.toml` and release notes if this changes.

## License

GPL-3.0-only. See `LICENSE`.

## Changelog

See `CHANGELOG.md`.
