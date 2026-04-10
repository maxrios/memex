# llmgraph — Agent Instructions

## What this project is

`llmgraph` is a CLI tool (Rust) that organizes LLM-assisted development into a versioned DAG of conversation nodes. It is a **storage and navigation layer**.

## Development workflow

This project tracks its own development using `llmgraph`. Follow this pattern for every feature or fix:

1. **Branch** — `git checkout -b <type>/<name>` from `main`
2. **Node** — `llmgraph node create --parent <parent-id>` before writing any code. Use a placeholder goal; you'll fill in the real summary later.
3. **Implement** the feature
4. **Summarize** — `llmgraph node edit` to write decisions, rejected approaches, and key artifacts. Do this yourself; do not add LLM API calls to automate it.
5. **Resolve** — `llmgraph node resolve`
6. **Commit** source changes and the updated `.llmgraph/` files together
7. **Push** and open a PR

## What to commit

Always commit `.llmgraph/graph.json` and `.llmgraph/nodes/` alongside source changes — the conversation history is part of the project record. Never commit `.llmgraph/state.json`.

## Architecture

```
src/
  main.rs               — CLI entrypoint, clap derive command tree
  models.rs             — ConversationNode, NodeSummary, NodeStatus, Graph, State, Config
  store.rs              — GraphStore: all file I/O abstracted here
  editor.rs             — $EDITOR integration via temp TOML file
  git.rs                — git detection via subprocess (no libgit2)
  commands/
    init.rs             — llmgraph init
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
