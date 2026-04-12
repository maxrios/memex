# memex

A CLI tool for organizing LLM-assisted development work into a versioned, navigable DAG of conversation nodes tied to a software project.

LLM conversations are ephemeral and flat, but real development is hierarchical and branching. `memex` gives each phase of work a structured node capturing what was built, what was decided, what was rejected, and what remains open. Edges represent context inheritance: each child node was started with knowledge of its parent.

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
memex node create --parent <root-id>

# After your work session, fill in the summary
memex node edit

# Generate a context payload to paste into a new LLM conversation
memex context

# View the full conversation history as a tree
memex graph view
```

---

## Commands

| Command | Description |
|---|---|
| `memex init` | Initialize a graph in the current project |
| `memex node create` | Create a new conversation node |
| `memex node edit [id]` | Edit a node's summary in `$EDITOR` |
| `memex node show [id]` | Display a node's full summary |
| `memex node list` | List all nodes with parent IDs, status, git ref, and goal |
| `memex node resolve [id]` | Mark a node as resolved |
| `memex node abandon [id]` | Mark a node as abandoned |
| `memex node reopen [id]` | Reopen a resolved or abandoned node |
| `memex context [id] [--depth N]` | Generate a context payload for LLM injection (default: 2 ancestors) |
| `memex graph view` | ASCII tree of the full conversation graph |
| `memex search <query>` | Search across all node summaries |

Node IDs can be shortened to any unambiguous prefix (e.g. `abc12345` → `abc1`).

---

## Development Workflow

`memex` is used to track its own development. The pattern for any new feature:

1. **Create a branch** - `git checkout -b feat/<name>`
2. **Create a node** - `memex node create --parent <parent-id>` with a placeholder goal
3. **Implement** the feature
4. **Write the node summary** - `memex node edit` to capture decisions, rejected approaches, and key artifacts
5. **Resolve the node** - `memex node resolve`
6. **Commit and push** - stage source changes and open a PR

The `memex context` command generates a formatted summary of the ancestor chain suitable for pasting into a new LLM conversation, so future sessions can pick up where the last one left off without re-explaining history.

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
web_port = 7777
```

### What gets committed

| Path | Committed | Notes |
|---|---|---|
| `.memex/config.toml` | Yes | Shared project settings |
| `.memex/graph.json` | Yes | DAG edges and root node pointer |
| `.memex/nodes/*.json` | Yes | Full conversation node history |
| `.memex/state.json` | No | Local session state (active node) |
| `.memex/transcripts/` | No | Raw transcripts, if stored locally |

### Storage layout

```
.memex/
  config.toml       # committed - shared settings
  graph.json        # committed - DAG structure
  nodes/
    <uuid>.json     # committed - one file per node
  state.json        # gitignored - local session state
```

---

## Node Summary Format

When editing a node (`memex node edit`), a TOML template is opened in `$EDITOR`:

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
