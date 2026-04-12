# memex - Agent Instructions

## What this project is

`memex` is a CLI tool that organizes LLM-assisted development into a versioned DAG of conversation nodes. It is a **storage and navigation layer**.

## Development workflow

> **IMPORTANT:** For any code change, bug fix, or new feature — always execute the steps below without being explicitly asked. Do not write any code before completing steps 1–3 (find parent node, branch, create node).

This project tracks its own development using `memex`. Follow this pattern for every feature or fix:

1. **Find a parent node** - Choose the parent based on what your work *depends on*, not just what is most recent:
   - `memex graph` - visualize the current DAG to see branching structure
   - `memex node list` - shows all nodes with IDs, parent IDs, statuses, git refs, and one-line goals
   - `memex search <keyword>` - full-text search across node summaries; use domain terms (e.g. `config`, `search`, `rename`) to surface related prior work
   - If your work extends a specific prior feature, attach to that feature's node even if it isn't the tip
   - If your work is independent of recent changes, find the most recent resolved node whose scope your work builds on
   - If your work depends directly on what just landed, attach to the active node (the linear case, correct when the dependency is real)
   - e.g. adding tests for the search command → parent is the node that implemented search, not documentation or cleanup nodes that landed after it
   - e.g. fixing a crash in `node edit` → parent is the node that introduced `node edit`, not whatever resolved most recently
   - e.g. adding a new `memex export` command → find the most recent node that touched the CLI structure; skip unrelated docs or refactors that followed it

2. **Branch** - `git checkout -b <type>/<name>` from `main`

3. **Node** - `memex node create --parent <parent-id> --goal "<your goal here>"` before writing any code. Use the real goal if you already know it; a short placeholder is fine when the scope is still uncertain.

4. **Implement** the feature

5. **Summarize** - Record decisions, artifacts, and rejected approaches incrementally as you work. Write these from observations during implementation, not from post-hoc reflection:
   ```
   memex node edit --decision "chose X over Y because Z"
   memex node edit --artifact "path/to/key/file.rs"
   memex node edit --open-thread "question to revisit later"
   memex node edit --rejected $'description = "Alternative approach"\nreason = "Why rejected"'
   memex node edit --goal "Updated goal if scope changed"
   ```
   Each flag appends to (or overwrites for `--goal`) the current node without touching other fields.
   Use `--summary` only for a full bulk replacement (e.g. bootstrapping from a plan).

6. **Resolve or abandon** - `memex node resolve` when the work is complete. If the task is superseded or turns out to be the wrong approach, use `memex node abandon` with a note in the summary explaining why.

7. **Commit** - Commit source changes and `.memex/graph.json` + `.memex/nodes/` together. Never commit `.memex/state.json`. Documentation updates (CLAUDE.md, README.md) go in a separate commit so the source diff and doc diff are independently reviewable.

8. **Push** and open a PR

## Documentation hygiene

After implementing any change, check whether it affects user-visible behavior, CLI output, or workflow guidance:

- If **CLAUDE.md** describes the changed behavior (commands, output format, workflow steps), update it.
- If **README.md** documents the changed command or output, update it.

Always make documentation updates a **separate commit** from the source change.

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

Key design decisions:
- One JSON file per node (`nodes/<uuid>.json`) for human-readable git diffs
- `graph.json` holds edges + root pointer; `state.json` (untracked) holds active node
- Git integration uses `git` subprocess, not libgit2, to keep the dependency footprint small
