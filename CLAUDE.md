# memex — Agent Instructions

## What this project is

`memex` is a CLI tool (Rust) that organizes LLM-assisted development into a versioned DAG of conversation nodes. It is a **storage and navigation layer**.

## Development workflow

This project tracks its own development using `memex`. Follow this pattern for every feature or fix:

1. **Find a parent node** — Identify the most relevant resolved node to attach to before branching:
   - `memex graph view` — see the full DAG; the deepest resolved leaf on the relevant branch is usually the right parent (marked `[*]` if it's the current active node)
   - `memex node list` — shows all nodes with IDs, statuses, git refs, and one-line goals; scan for the most recent resolved node whose scope contains yours
   - `memex search <keyword>` — full-text search across node summaries; use domain terms (e.g. `config`, `search`, `rename`) to surface the closest prior work
   - When work is genuinely new, attach to the current active node (`[*]` in graph view or `*` in node list)
   - Prefer the most specific ancestor: if a node for `feat/search` exists and you're extending search, use it rather than the root

2. **Branch** — `git checkout -b <type>/<name>` from `main`
3. **Node** — `memex node create --parent <parent-id> --goal "<placeholder goal>"` before writing any code. You'll fill in the real summary with `memex node edit` later.
4. **Implement** the feature
5. **Summarize** — `memex node edit` to write decisions, rejected approaches, and key artifacts. Do this yourself; do not add LLM API calls to automate it.
6. **Resolve** — `memex node resolve`
7. **Commit** source changes and the updated `.memex/` files together
8. **Push** and open a PR

## What to commit

Always commit `.memex/graph.json` and `.memex/nodes/` alongside source changes — the conversation history is part of the project record. Never commit `.memex/state.json`.

## Architecture

```
src/
  main.rs               — CLI entrypoint, clap derive command tree
  models.rs             — ConversationNode, NodeSummary, NodeStatus, Graph, State, Config
  store.rs              — GraphStore: all file I/O abstracted here
  editor.rs             — $EDITOR integration via temp TOML file
  git.rs                — git detection via subprocess (no libgit2)
  commands/
    init.rs             — memex init
    node.rs             — create, edit, show, list, resolve/abandon/reopen
    graph.rs            — ASCII tree view
    context.rs          — context payload generation (markdown/xml/plain)
    search.rs           — full-text search across node summaries
```

Key design decisions already made — don't revisit without good reason:
- One JSON file per node (`nodes/<uuid>.json`) for human-readable git diffs
- `graph.json` holds edges + root pointer; `state.json` (untracked) holds active node
- Git integration uses `git` subprocess, not libgit2, to keep the dependency footprint small

## Stack

- Rust 2021, clap v4 (derive), serde/serde_json, uuid, chrono, toml, anyhow, tempfile
