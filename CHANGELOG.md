# Changelog

## 0.8.1 - Dec 4, 2025

- Patch release to ship corrected zsh completion spec for `--list/-l` in the generated `_to` file.

## 0.8.0 - Dec 4, 2025

- Added explicit editor selection: `-u/--cursor` or `-C/--code` after jump; mutually exclusive. Errors if both are provided.
- Updated zsh completions, README, and tests for the editor flags.
- Fixed zsh completion spec for `--list/-l` to avoid `_arguments` parse errors.

## 0.7.0 - Dec 4, 2025

- **Breaking:** Reworked flag layout for a more Unix-like CLI (new shorts: `-b` bulk-add, `-c` copy, `-f` force, `-u` cursor, `-N` no-create, `-y` path-only, `-H` here; list field scope renamed to `--keyword-only/--path-only`, `--both` replaces `--and`).
- Added scoped listing: `--within/-w` and `--here/-H` to show shortcuts under a root, plus `--max-depth/-d` to limit recursion (default remains non-recursive).
- Renamed completions command to `--completions` (alias: `--generate-completions`); zsh completion script updated accordingly.
- Updated docs, tests, and installer to reflect the new flags and completion command.

## 0.6.3 - Dec 4, 2025

- Added shorter aliases `--write-completions` / `--install-completions` for writing completions to the default location.

## 0.6.2 - Dec 4, 2025

- Added `--write-default-completions` to write zsh completions directly to the default path when desired.

## 0.6.1 - Dec 4, 2025

- Fixed Zsh completion: the `--list/-l` option spec is now valid, so `_arguments` no longer errors when completing commands like `to -a keyword path`.

## 0.6.0 - Dec 4, 2025

- `--add/--copy/--add-bulk` now support `--force` plus duplicate-path prompts: keeps noop for same path, allows replace with force, and warns when the path is already saved under other keywords (respecting `GOTO_ASSUME_YES`).
- Added clearer output for replaced/duplicate shortcuts and expiry updates.

## 0.5.2 - Dec 4, 2025

- Added installer + CLI wrapper audit: hidden wrapper check, user-facing `--install-wrapper`, wrapper env flag, and post-cd verification.
- Binary now warns when the wrapper is missing for jump attempts; installer keeps rc symlinks intact.

## 0.5.1 - Dec 4, 2025

- Installer now preserves symlinked shell rc files when injecting the wrapper block.

## 0.5.0 - Dec 2, 2025

- `--list` now doubles as search with an optional query plus keyword/path scoping, glob/regex modes, JSON output, and result limiting.

## 0.3.0

- `to --add` now prints the canonicalized path in its confirmation message, so casing matches the actual filesystem path on macOS.

## 0.2.2

- Fix wrapper: use the correct hidden flag (`--__classify`) so jump detection works and shells actually `cd` when using the installed wrapper.

## 0.2.1

- Wrapper no longer hardcodes non-jump flags; it asks `to --classify` to decide when to `cd`, so new flags are handled automatically.
- Added hidden CLI flag `--classify` (internal; reports `jump`/`nojump`).

## 0.2.0

- Installer: detect the shell wrapper even if marker comments are removed; avoid non-zero exit when the wrapper already exists without markers.
- CLI: new `--show-sort` flag to display the current sorting mode; zsh completion updated accordingly.
- Help consistency: bare `to` now prints Clap-generated help (same as `to -h`) before the saved-shortcuts list.
- UX: added descriptive help text for all user-facing flags in Clap help output.

## 0.1.0

- Initial Rust implementation of `to` with keyword shortcuts, longest-prefix jumps, expirations, recents, bulk add, copy, remove, list, print-path, and Cursor flag.
- Dynamic Zsh/Bash/Fish completion generation with keyword + subpath suggestions.
- Config compatibility with `~/.goto/to_dirs` family (env overrides still supported).
- Colorized output with `--no-color` / `NO_COLOR`.
- File locking for config/meta/recent writes to reduce race risk.
