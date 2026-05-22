# Changelog

All notable changes to memex are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-05-22

### Added

- `memex pr-context <ref>` — search the graph for nodes matching a git ref (branch name, commit SHA, or PR number) and emit a Markdown payload of the most-recent matches, intended to be posted as a PR comment. `--limit N` caps the output (default 5).
- GitHub Action workflow that posts a `memex pr-context` comment to each PR automatically.
- `memex node auto <json>` — non-interactive edit subcommand for agent-authored updates. Accepts a JSON payload with `decisions`, `artifacts`, `open_threads`, and `rejected` arrays and applies them to the active node in one call.

### Changed

- README lede reframed around how memex compares with ADRs and changelogs as a navigable DAG of conversation nodes.
- Release-cut workflow is now tracked as a memex node, so each release shipped from this repo carries its own graph context.

## [0.1.0] - 2026-05-11

The first tagged release of memex — a CLI for organizing AI-assisted development as a versioned, navigable DAG of conversation nodes.

### Added

- `memex init` — initialize a `.memex/` graph in the current project, with a sensible default `.memex/.gitignore` excluding the per-developer `state.json`.
- `memex node create` — create a new node with a `--parent` and `--goal`, optionally `--tag`.
- `memex node edit` — append `--decision`, `--artifact`, `--open-thread`, or `--rejected` entries to the active node; overwrite the `--goal` or full `--summary`. Opens `$EDITOR` over a TOML temp file when no flags are passed.
- `memex node show` — print a node by ID (or the active node by default).
- `memex node list` — list all nodes with status, parent, and goal.
- `memex node resolve` / `abandon` / `reopen` — transition node status. Destructive transitions require confirmation.
- `memex graph view` — ASCII tree view of the DAG with status markers.
- `memex context` — render the project root's goal/decisions, the trimmed ancestor chain, and the active node into a single payload. `--format markdown|xml|plain`, `--depth N`, `--id <node>` for targeting an arbitrary node.
- `memex search <keyword>` — full-text search across node goals, decisions, artifacts, open threads, and rejected approaches.
- Node IDs accept any unambiguous prefix (e.g. `abc1` for `abc12345-...`).
- GitHub Actions CI runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --all-targets`, and `cargo audit` on every push/PR to `main`, across Linux, macOS, and Windows.
- A `memex-check` workflow validates that each PR includes a corresponding `.memex/nodes/` file with a valid parent reference.
- Claude Code plugin under `claude-plugin/` shipping a `memex` skill that triggers in any repo with a `.memex/` directory.
- Release workflow (`.github/workflows/release.yml`) builds binaries for `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, and `x86_64-pc-windows-msvc`, attaches them with SHA-256 checksums to a draft GitHub release, and publishes to crates.io on tag push.

[Unreleased]: https://github.com/maxrios/memex/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/maxrios/memex/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/maxrios/memex/releases/tag/v0.1.0
