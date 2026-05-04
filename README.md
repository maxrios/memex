# memex

A CLI tool for organizing AI-assisted development work into a versioned, navigable DAG of conversation nodes tied to a software project.

Conversations are ephemeral and flat, but real development is hierarchical and branching. memex gives each phase of work a structured node capturing what was built, what was decided, what was rejected, and what remains open. Edges represent context inheritance: each child node was started with knowledge of its parent.

🚨 memex's development is AI-assisted. Each PR is thoroughly reviewed and should maintain development best practices. Developers are encouraged to contribute in their preferred way, but please follow the development workflow detailed in [AGENTS.md](https://github.com/maxrios/memex/blob/main/AGENTS.md).

---

## Installation

```
cargo build --release
cp target/release/memex /usr/local/bin/
```

Or run directly from the project root:

```
cargo run -- <command>
```

---

## Quick Start

```bash
# Initialize a graph in your project
memex init

# Create a node for a new feature
memex node create --parent <root-id> --goal "Add user authentication" --tag auth

# During implementation, record decisions and artifacts incrementally
memex node edit --decision "Used JWT over session cookies for statelessness"
memex node edit --artifact "src/auth/mod.rs"
memex node edit --open-thread "Rate limiting not yet implemented"
memex node edit --rejected $'description = "Session-based auth"\nreason = "Requires sticky sessions, complicates horizontal scaling"'

# Mark the node as done
memex node resolve

# Later: generate a context payload to continue in a new conversation
memex context --format markdown

# View the full conversation history as a tree
memex graph view
```

---

## Commands

Node IDs can be shortened to any unambiguous prefix (e.g. `abc12345` → `abc1`).

### `memex init`

Initialize a `.memex/` directory in the current project.

### `memex node create`

Create a new conversation node.

| Flag | Description |
|---|---|
| `--parent <id>` | Parent node ID |
| `--goal "..."` | One-line goal for this node |
| `--git-ref <ref>` | Git branch or tag to associate |
| `--tag <tag>` | Tag the node (repeatable) |

### `memex node edit [id]`

Without flags, opens `$EDITOR` with a TOML template for bulk editing. With additive flags, updates individual fields without opening an editor:

| Flag | Effect |
|---|---|
| `--goal "..."` | Overwrite the goal |
| `--decision "..."` | Append a decision (repeatable) |
| `--artifact "..."` | Append a key artifact (repeatable) |
| `--open-thread "..."` | Append an open thread (repeatable) |
| `--rejected '...'` | Append a rejected approach as inline TOML (repeatable) |
| `--summary "..."` | Replace the entire summary from TOML (mutually exclusive with the flags above) |

The `--rejected` flag expects inline TOML with `description` and `reason` fields:

```bash
memex node edit --rejected $'description = "Alternative approach"\nreason = "Why it was rejected"'
```

### `memex node show [id]`

Display a node's full summary.

### `memex node list`

List all nodes with IDs, parent IDs, status, git ref, and goal.

### `memex node resolve [id]` / `abandon [id]` / `reopen [id]`

Transition a node's status between Active, Resolved, and Abandoned.

`resolve` and `abandon` prompt for confirmation when run in an interactive terminal:

```
Resolve node abc12345 "Add user authentication"? [y/N]
```

| Flag | Description |
|---|---|
| `--force` / `-y` | Skip the confirmation prompt (for scripts and CI) |

Non-interactive contexts (pipes, CI) skip the prompt automatically.

### `memex context [id]`

Generate a context payload. Walks the ancestor chain and formats it for pasting into a new conversation.

| Flag | Description |
|---|---|
| `--depth <N>` | Number of ancestors to include (default: 2) |
| `--format <fmt>` | Output format: `markdown` (default), `xml`, `plain` |

### `memex graph view`

ASCII tree of the full conversation graph. Shows status icons and marks the active node.

### `memex search <query>`

Full-text search across all node summaries, including goals, decisions, artifacts, open threads, rejected approaches, and tags.

---

## Workflow

The pattern for using `memex` alongside any project:

1. **Branch** — `git checkout -b <type>/<name>`
2. **Create a node** — `memex node create --parent <parent-id> --goal "what you're working on"`
3. **Implement** — as you work, record decisions and artifacts incrementally:
   ```
   memex node edit --decision "Chose X over Y because ..."
   memex node edit --artifact "src/new_module.rs"
   ```
4. **Resolve** — `memex node resolve` when the work is complete
5. **Commit** — stage source changes and `.memex/` files together
6. **Next session** — `memex context` generates a formatted ancestor summary to paste into a new conversation, so future sessions pick up where the last left off

---

## Node Summary Format

When editing a node with `$EDITOR` (`memex node edit` with no flags), a TOML template is opened:

```toml
goal = "What this conversation was working toward"

decisions = [
  "Specific decision made",
]

[[rejected_approaches]]
description = "Approach considered"
reason = "Why it was not used"

open_threads = [
  "Unresolved question or follow-up",
]

key_artifacts = [
  "src/relevant_file.rs",
]
```

You can also build this incrementally using the additive flags on `memex node edit` — see [Commands](#memex-node-edit-id) above.

---

## Configuration

`memex init` creates `.memex/config.toml` at the project root.

```toml
[git]
auto_prompt_on_branch = true
annotate_on_commit = false

[storage]
track_in_git = true
transcript_storage = "none"

[ui]
editor = ""      # defaults to $EDITOR
```

---

## Storage

| Path | Committed | Notes |
|---|---|---|
| `.memex/config.toml` | Yes | Shared project settings |
| `.memex/nodes/*.json` | Yes | Full conversation node history; each node's `parent_ids` is the source of truth for the DAG |
| `.memex/state.json` | No | Local session state (active node) |

```
.memex/
  config.toml       # committed - shared settings
  nodes/
    <uuid>.json     # committed - one file per node
  state.json        # gitignored - local session state
```
