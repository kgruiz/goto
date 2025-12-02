# Changelog

## Unreleased

- Added `search` subcommand (alias `s`) with keyword/path scoping, glob/regex modes, JSON output, and `to s` listing convenience.

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
