# memex - Agent Instructions

## What this project is

`memex` is a CLI tool that organizes LLM-assisted development into a versioned DAG of conversation nodes. It is a **storage and navigation layer**.

## Development workflow

> **IMPORTANT:** For any code change, bug fix, or new feature — always execute the steps below without being explicitly asked. Do not write any code before completing steps 1–3 (find parent node, branch, create node).

This project tracks its own development using `memex`. Follow this pattern for every feature or fix:

1. **Find a parent node** - Identify the most relevant resolved node to attach to before branching:
2. - `memex context` - get the a context payload for the active and root nodes.
   - `memex node list` - shows all nodes with IDs, parent IDs, statuses, git refs, and one-line goals; scan for the most recent resolved node whose scope contains yours
   - `memex search <keyword>` - full-text search across node summaries; use domain terms (e.g. `config`, `search`, `rename`) to surface the closest prior work
   - When work is genuinely new, attach to the current active node (`[*]` in graph view or `*` in node list)
   - Prefer the most specific ancestor: if a node for `feat/search` exists and you're extending search, use it rather than the root

2. **Branch** - `git checkout -b <type>/<name>` from `main`
3. **Node** - `memex node create --parent <parent-id> --goal "<your goal here>"` before writing any code. Use the real goal if you already know it; a short placeholder is fine when the scope is still uncertain.
4. **Implement** the feature
5. **Summarize** - Record decisions, artifacts, and rejected approaches incrementally as you work using additive flags. Do this yourself; do not add LLM API calls to automate it.

   Preferred: write incrementally during implementation so the record builds up as you go:
   ```
   memex node edit --decision "reason you made a key choice"
   memex node edit --artifact "path/to/key/file.rs"
   memex node edit --open-thread "question to revisit later"
   memex node edit --rejected $'description = "Alternative approach"\nreason = "Why rejected"'
   memex node edit --goal "Updated goal if scope changed"
   ```
   Each flag appends to (or overwrites for `--goal`) the current node without touching other fields.
   Use `--summary` only for a full bulk replacement (e.g. bootstrapping from a plan).
6. **Resolve** - `memex node resolve`
7. **Commit** source changes and the updated `.memex/` files together
8. **Push** and open a PR

## Documentation hygiene

After implementing any change, check whether it affects user-visible behavior, CLI output, or workflow guidance:

- If **CLAUDE.md** describes the changed behavior (commands, output format, workflow steps), update it.
- If **README.md** documents the changed command or output, update it.

Always make documentation updates a **separate commit** from the source change. This keeps the source diff and the doc diff independently reviewable.

## What to commit

Always commit `.memex/graph.json` and `.memex/nodes/` alongside source changes - the conversation history is part of the project record. Never commit `.memex/state.json`.

## Architecture

```
src/
  main.rs               - CLI entrypoint, clap derive command tree
  models.rs             - ConversationNode, NodeSummary, NodeStatus, Graph, State, Config
  store.rs              - GraphStore: all file I/O abstracted here
  editor.rs             - $EDITOR integration via temp TOML file
  git.rs                - git detection via subprocess (no libgit2)
  commands/
    init.rs             - memex init
    node.rs             - create, edit, show, list, resolve/abandon/reopen
    graph.rs            - ASCII tree view
    context.rs          - context payload generation (markdown/xml/plain)
    search.rs           - full-text search across node summaries
```

Key design decisions already made - don't revisit without good reason:
- One JSON file per node (`nodes/<uuid>.json`) for human-readable git diffs
- `graph.json` holds edges + root pointer; `state.json` (untracked) holds active node
- Git integration uses `git` subprocess, not libgit2, to keep the dependency footprint small
