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

memex tracks its own development as memex nodes. There is no external issue tracker — the frontier lives in `.memex/`. Run `memex node list` to see current state; use `memex graph` to see branching. Old plans or design notes do not supersede the live graph.

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

> **IMPORTANT:** For any code change, bug fix, or new feature — always execute the steps below without being explicitly asked. Do not write any code before completing steps 1–3 (find parent node, branch, create node).

This project tracks its own development using `memex`. Follow this pattern for every feature or fix:

1. **Find a parent node** - Choose the parent based on what your work _depends on_, not just what is most recent:
   - `memex graph` - visualize the current DAG to see branching structure
   - `memex node list` - shows all nodes with IDs, parent IDs, statuses, git refs, and one-line goals
   - `memex search <keyword>` - full-text search across node summaries; use domain terms (e.g. `config`, `search`, `rename`) to surface related prior work
   - If your work extends a specific prior feature, attach to that feature's node even if it isn't the tip
   - If your work is independent of recent changes, find the most recent resolved node whose scope your work builds on
   - If your work depends directly on what just landed, attach to the active node (the linear case, correct when the dependency is real)
   - e.g. adding tests for the search command → parent is the node that implemented search, not documentation or cleanup nodes that landed after it
   - e.g. fixing a crash in `node edit` → parent is the node that introduced `node edit`, not whatever resolved most recently
   - e.g. adding a new `memex export` command → find the most recent node that touched the CLI structure; skip unrelated docs or refactors that followed it

2. **Branch** - `git checkout -b <type>/<name>` from `main` (e.g. `fix/node-edit-crash`, `feat/export-command`, `test/store-roundtrip`, `chore/audit-graph-parent-relationships`).

3. **Node** - `memex node create --parent <parent-id> --goal "<your goal here>"` before writing any code. Use the real goal if you already know it; a short placeholder is fine when the scope is still uncertain.

4. **Implement** the feature. Non-trivial changes to `src/store.rs`, `src/models.rs`, or any `src/commands/*.rs` should come with tests — prefer integration tests in `tests/integration_tests.rs` for CLI-visible behavior, inline `#[cfg(test)]` modules for pure logic.

5. **Summarize** - Record as you go, not after. For every `--decision` you record, ask: _what alternative did I deliberately not take, and why?_ If there's an answer, that's a `--rejected`. Before resolving, ask: _what question did I defer, what caveat did I notice, what did I leave for later?_ Each one is an `--open-thread`. These two fields are under-used across the repo; using them gives future agents (and future-you) the context that decisions alone don't capture.

   ```
   memex node edit --decision "chose X over Y because Z"
   memex node edit --artifact "path/to/key/file.rs"
   memex node edit --open-thread "question to revisit later"
   memex node edit --rejected $'description = "Alternative approach"\nreason = "Why rejected"'
   memex node edit --goal "Updated goal if scope changed"
   ```

   Each flag appends to (or overwrites for `--goal`) the current node without touching other fields.
   Use `--summary` only for a full bulk replacement (e.g. bootstrapping from a plan).

6. **Resolve or abandon** - Run `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets` locally; all three must pass. Then `memex node resolve` when the work is complete. If the task is superseded or turns out to be the wrong approach, use `memex node abandon` with a note in the summary explaining why.

7. **Commit** - Commit source changes and `.memex/nodes/` together — they describe the same unit of work. Never commit `.memex/state.json`: it's per-developer working-node state and will create merge conflicts. Documentation updates (CLAUDE.md, README.md) go in a separate commit (same PR) so the source diff and doc diff are independently reviewable.

8. **Push** and open a PR.

   PR description template — match what recent PRs already do:

   ```markdown
   ## Summary

   - 2–5 bullets: what changed and why

   ## Test plan

   - Checklist of verification steps (commands to run, expected output)
   ```

   Title ends with `(closes #N)` when the PR resolves an issue. Branch names follow `<type>/<name>` (e.g. `fix/node-edit-crash`, `chore/audit-graph-parent-relationships`).

## Documentation hygiene

After implementing any change, check whether it affects user-visible behavior, CLI output, or workflow guidance:

- If **CLAUDE.md** describes the changed behavior (commands, output format, workflow steps), update it.
- If **README.md** documents the changed command or output, update it.

Always make documentation updates a **separate commit** from the source change.
