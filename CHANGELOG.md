# Changelog

## 0.1.0

- Initial Rust implementation of `to` with keyword shortcuts, longest-prefix jumps, expirations, recents, bulk add, copy, remove, list, print-path, and Cursor flag.
- Dynamic Zsh/Bash/Fish completion generation with keyword + subpath suggestions.
- Config compatibility with `~/.goto/to_dirs` family (env overrides still supported).
- Colorized output with `--no-color` / `NO_COLOR`.
- File locking for config/meta/recent writes to reduce race risk.
