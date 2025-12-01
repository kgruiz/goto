# Changelog

## 0.1.0

- Initial Rust implementation of `to` with keyword shortcuts, longest-prefix jumps, expirations, recents, bulk add, copy, remove, list, print-path, and Cursor flag.
- Dynamic Zsh/Bash/Fish completion generation with keyword + subpath suggestions.
- Config compatibility with `~/.to_dirs` family; env overrides for testing.
- Colorized output with `--no-color` / `NO_COLOR`.
- File locking for config/meta/recent writes to reduce race risk.
