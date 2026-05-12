# memex - Agent Instructions

## What this project is

`memex` is a CLI tool that organizes LLM-assisted development into a versioned DAG of conversation nodes. It is a **storage and navigation layer** — the graph lives on disk under `.memex/`, and git is the transport that carries it between machines and collaborators.

## Stack

| Layer       | Choice                                 |
| ----------- | -------------------------------------- |
| Language    | Rust 2021 edition                      |
| CLI parsing | `clap` v4 (derive)                     |
| Serde       | `serde` + `serde_json` + `toml`        |
| IDs / time  | `uuid` v4, `chrono`                    |
| Errors      | `anyhow`                               |
| Editor I/O  | `tempfile` + `$EDITOR` subprocess      |
| Git         | `git` subprocess (no libgit2)          |
| Testing     | built-in `cargo test` + `assert_cmd`   |

## Key design decisions

**One JSON file per node.** `.memex/nodes/<uuid>.json` keeps diffs human-readable and merge-friendly; avoid bundling multiple nodes into a single file.

**`state.json` is per-developer; everything else under `.memex/` is committed.** `state.json` holds the active-node pointer for the current developer and is `.gitignore`'d — committing it creates guaranteed merge conflicts. The DAG structure has no shared file: the root is the unique node with empty `parent_ids`, and edges are derived at read time by inverting each node's `parent_ids`.

**Git via subprocess.** Integration shells out to `git` rather than linking libgit2. Keeps the dependency footprint small and matches user expectations (same git binary, same config, same auth).

**All file I/O routes through `GraphStore`.** Commands in `src/commands/` should not read or write files directly — go through `store.rs` so tests can substitute a temp dir and behavior stays consistent.

**TOML for editor input, JSON for storage.** `memex node edit` opens a TOML temp file (friendlier to hand-edit); the store persists JSON. The asymmetry is intentional.

## Project structure

```
src/
  main.rs               - CLI entrypoint, clap derive command tree
  models.rs             - ConversationNode, NodeSummary, NodeStatus, State, Config
  store.rs              - GraphStore: all file I/O abstracted here
  editor.rs             - $EDITOR integration via temp TOML file
  git.rs                - git detection via subprocess (no libgit2)
  commands/
    init.rs             - memex init
    node.rs             - create, edit, show, list, resolve/abandon/reopen
    graph.rs            - ASCII tree view
    context.rs          - context payload generation (markdown/xml/plain)
    search.rs           - full-text search across node summaries
tests/
  integration_tests.rs  - end-to-end CLI tests via assert_cmd
```

## Where the work is tracked

memex tracks its own development as memex nodes. There is no external issue tracker — the frontier lives in `.memex/`. Run `memex node list` to see current state; use `memex graph view` to see branching. Old plans or design notes do not supersede the live graph.

## Scripts

| Command                                       | When to run                                              |
| --------------------------------------------- | -------------------------------------------------------- |
| `cargo build`                                 | Local build                                              |
| `cargo run -- <args>`                         | Manual verification of CLI behavior                      |
| `cargo test --all-targets`                    | Run unit + integration tests; what CI runs               |
| `cargo fmt --all -- --check`                  | Formatting check (CI-gated)                              |
| `cargo fmt --all`                             | Apply formatting                                         |
| `cargo clippy --all-targets -- -D warnings`   | Lint (CI-gated, warnings are errors); run before PR      |

CI (`.github/workflows/ci.yml`) runs fmt-check, clippy-with-deny-warnings, and the full test suite on every push and PR to `main`. Resolve a memex node only after all three pass locally.

---

## Development workflow

The full workflow — finding a parent node, branching, creating a node before writing code, recording decisions / rejected / open-threads as you go, resolving, committing — lives in the **`memex` Claude plugin skill** at [`claude-plugin/skills/memex/SKILL.md`](claude-plugin/skills/memex/SKILL.md). The skill triggers automatically in this repo because `.memex/` is present; treat it as the source of truth.

Conventions specific to *developing memex itself* (not in the skill):

- **Tests.** Non-trivial changes to `src/store.rs`, `src/models.rs`, or any `src/commands/*.rs` should come with tests — prefer integration tests in `tests/integration_tests.rs` for CLI-visible behavior, inline `#[cfg(test)]` modules for pure logic.
- **Verification.** Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets` locally before resolving a node; all three must pass and CI gates on the same.
- **Branch names.** `<type>/<name>` (e.g. `feat/export-command`, `fix/node-edit-crash`, `chore/audit-graph-parent-relationships`).
- **PR template:**

  ```markdown
  ## Summary

  - 2–5 bullets: what changed and why

  ## Test plan

  - Checklist of verification steps (commands to run, expected output)
  ```

  Title ends with `(closes #N)` when the PR resolves an issue.

## Documentation hygiene

After implementing any change, check whether it affects user-visible behavior, CLI output, or workflow guidance:

- If **AGENTS.md** describes the changed convention or repo-specific rule, update it.
- If **README.md** documents the changed command or output, update it.
- If **`claude-plugin/skills/memex/SKILL.md`** references a subcommand or flag whose surface changed, update it in the same diff — the skill drifts silently otherwise.

Always make documentation updates a **separate commit** from the source change.
