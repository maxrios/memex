# llmgraph

A CLI tool for organizing LLM-assisted development work into a versioned, navigable DAG of conversation nodes tied to a software project.

LLM conversations are ephemeral and flat, but real development is hierarchical and branching. `llmgraph` gives each phase of work a structured node capturing what was built, what was decided, what was rejected, and what remains open. Edges represent context inheritance: each child node was started with knowledge of its parent.

---

## Installation

```
cargo build --release
cp target/release/llmgraph /usr/local/bin/
```

Or run directly from the project root:

```
cargo run -- <command>
```

---

## Quick Start

```bash
# Initialize a graph in your project
llmgraph init

# Create a node for a new feature
llmgraph node create --parent <root-id>

# After your work session, fill in the summary
llmgraph node edit

# Generate a context payload to paste into a new LLM conversation
llmgraph context

# View the full conversation history as a tree
llmgraph graph view
```

---

## Commands

| Command | Description |
|---|---|
| `llmgraph init` | Initialize a graph in the current project |
| `llmgraph node create` | Create a new conversation node |
| `llmgraph node edit [id]` | Edit a node's summary in `$EDITOR` |
| `llmgraph node show [id]` | Display a node's full summary |
| `llmgraph node list` | List all nodes |
| `llmgraph node resolve [id]` | Mark a node as resolved |
| `llmgraph node abandon [id]` | Mark a node as abandoned |
| `llmgraph node reopen [id]` | Reopen a resolved or abandoned node |
| `llmgraph context [id]` | Generate a context payload for LLM injection |
| `llmgraph graph view` | ASCII tree of the full conversation graph |
| `llmgraph search <query>` | Search across all node summaries |

Node IDs can be shortened to any unambiguous prefix (e.g. `abc12345` → `abc1`).

---

## Development Workflow

`llmgraph` is used to track its own development. The pattern for any new feature:

1. **Create a branch** - `git checkout -b feat/<name>`
2. **Create a node** - `llmgraph node create --parent <parent-id>` with a placeholder goal
3. **Implement** the feature
4. **Write the node summary** - `llmgraph node edit` to capture decisions, rejected approaches, and key artifacts
5. **Resolve the node** - `llmgraph node resolve`
6. **Commit and push** - stage source changes and open a PR

The `llmgraph context` command generates a formatted summary of the ancestor chain suitable for pasting into a new LLM conversation, so future sessions can pick up where the last one left off without re-explaining history.

---

## Configuration

`llmgraph init` creates `.llmgraph/config.toml` at the project root.

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
| `.llmgraph/config.toml` | Yes | Shared project settings |
| `.llmgraph/graph.json` | Yes | DAG edges and root node pointer |
| `.llmgraph/nodes/*.json` | Yes | Full conversation node history |
| `.llmgraph/state.json` | No | Local session state (active node) |
| `.llmgraph/transcripts/` | No | Raw transcripts, if stored locally |

### Storage layout

```
.llmgraph/
  config.toml       # committed - shared settings
  graph.json        # committed - DAG structure
  nodes/
    <uuid>.json     # committed - one file per node
  state.json        # gitignored - local session state
```

---

## Node Summary Format

When editing a node (`llmgraph node edit`), a TOML template is opened in `$EDITOR`:

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
