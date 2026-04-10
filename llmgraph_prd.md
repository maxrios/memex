# PRD: `llmgraph` — LLM Conversation Graph for Developer Workflows

## Overview

`llmgraph` is a CLI-first developer tool that organizes LLM conversations into a versioned, navigable DAG (directed acyclic graph) tied to a software project. It solves the core pain point of LLM-assisted development: conversations are ephemeral and flat, but real development work is hierarchical, branching, and context-dependent.

Each node in the graph represents a **conversation snapshot** — a curated summary of a chat session capturing what was being built, what was decided, what was rejected, and what remains open. Edges represent context inheritance: a child node was started with knowledge of its parent. The graph mirrors the structure of the codebase itself, optionally mapping directly onto git branches and commits.

---

## Goals

- Give developers a structured, visual history of LLM-assisted work on a project
- Enable fast context re-injection into new conversations without re-explaining from scratch
- Preserve decision archaeology: what was tried, decided, and ruled out — and why
- Integrate naturally into existing git-based workflows with minimal friction
- Be provider-agnostic: work across Claude, ChatGPT, Gemini, or any LLM

## Non-Goals

- This is not a chat client or LLM interface — it manages conversation metadata, not conversations themselves
- This is not a general note-taking or wiki tool
- Full conversation transcript storage is optional and out of scope for v1

---

## Core Concepts

### Node

A node is the fundamental unit of the graph. It represents a phase of LLM-assisted work — typically scoped to a feature, issue, or investigation.

```rust
struct ConversationNode {
    id: Uuid,
    parent_ids: Vec<Uuid>,           // supports merge nodes; empty = root
    git_ref: Option<String>,         // branch name, commit SHA, or tag
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    summary: NodeSummary,
    raw_transcript_ref: Option<Uri>, // optional link to full conversation
    tags: Vec<String>,
    status: NodeStatus,              // Active | Resolved | Abandoned
}

struct NodeSummary {
    goal: String,
    decisions: Vec<String>,
    rejected_approaches: Vec<RejectedApproach>,
    open_threads: Vec<String>,
    key_artifacts: Vec<String>,      // file paths, function names, etc.
}

struct RejectedApproach {
    description: String,
    reason: String,
}

enum NodeStatus {
    Active,
    Resolved,
    Abandoned,
}
```

### Edge

A directed edge from node A to node B means: "node B was started with context inherited from node A." Edges are implicitly created when a child node is created from a parent.

### Graph

The full project conversation history. Stored as a `.llmgraph/` directory at the project root, version-controlled alongside the code.

### Root Node

Every graph has a single root node representing the project itself. It holds top-level context: what the project does, its stack, its key architectural decisions. All other nodes descend from it.

### Context Payload

When a developer starts a new conversation and wants to inherit from a parent node, `llmgraph` generates a **context payload** — a formatted, compressed text block suitable for pasting or piping into an LLM. It includes the root node summary, any ancestor summaries along the path, and the immediate parent's full summary.

---

## Feature Specification

### F1: Graph Initialization

```
llmgraph init
```

- Creates `.llmgraph/` directory at the project root
- Creates `graph.json` (or equivalent) to store the node DAG
- Creates the root node with a template summary, prompting the developer to describe the project
- If a git repo is detected, records the current branch and HEAD SHA on the root node
- Adds `.llmgraph/` to `.gitignore` recommendation (or optionally to git tracking — user's choice)

### F2: Node Creation

```
llmgraph node create [--parent <node-id>] [--git-ref <ref>] [--tag <tag>]
```

- Creates a new node as a child of the specified parent (defaults to current active node)
- If `--git-ref` is not provided and a git repo is detected, auto-captures the current branch and HEAD
- Opens an editor (respects `$EDITOR`) with a pre-filled template summary draft
- Optionally: calls an LLM to draft the summary from a pasted conversation transcript (see F6)
- Sets the new node as the active node

**Auto-creation triggers (optional, configured via `.llmgraph/config.toml`):**

- On `git checkout -b <branch>`: prompts to create a child node for the new branch
- On `git commit`: optionally annotates the current active node with the commit SHA

### F3: Node Editing

```
llmgraph node edit [<node-id>]
```

- Opens the node summary in `$EDITOR`
- Tracks `updated_at` on save

### F4: Node Viewing

```
llmgraph node show [<node-id>]
```

- Prints the full node summary to stdout in a readable format
- Defaults to the currently active node

```
llmgraph node list
```

- Lists all nodes with ID, status, git ref, goal (truncated), and creation date

### F5: Graph Visualization

```
llmgraph graph view
```

- Renders the DAG as an ASCII tree in the terminal

```
llmgraph graph web
```

- Launches a local web server serving an interactive force-directed graph visualization
- Nodes are colored by status (Active / Resolved / Abandoned)
- Clicking a node shows its full summary in a sidebar
- Edges show direction of context inheritance

### F6: Context Payload Generation

```
llmgraph context [<node-id>]
```

- Generates a context payload for the given node (defaults to active node)
- Payload includes: root node summary → ancestor summaries (condensed) → parent node full summary
- Outputs to stdout; can be piped or copied
- Optional `--format [markdown|xml|plain]` flag to match the target LLM's preferred input style

Example output:

```
## Project Context
<root node summary>

## Ancestor Context
- [node abc123] Goal: Implement rate limiting. Decision: used token bucket algorithm.
- [node def456] Goal: Redis integration. Decision: deadpool-redis for connection pooling.

## Immediate Parent Context
Goal: Design the service discovery protocol
Decisions:
  - Services register via TTL-based heartbeat (30s default)
  - Registry backed by Redis sorted sets keyed by service name
Rejected:
  - Consul: too heavy for embedded use
  - Static config: doesn't support dynamic scaling
Open threads:
  - Deregistration on ungraceful shutdown not yet handled
Key artifacts: src/discovery.rs, src/registry.rs
```

### F7: LLM-Assisted Summary Drafting

```
llmgraph node summarize [<node-id>] --transcript <file>
```

- Accepts a conversation transcript (plain text or JSON export) as input
- Calls a configured LLM API to generate a draft `NodeSummary`
- Opens the draft in `$EDITOR` for review before saving
- Supports any OpenAI-compatible API endpoint (configured in `.llmgraph/config.toml`)

### F8: Node Linking (Manual Edge Creation)

```
llmgraph node link <child-id> --parent <parent-id>
```

- Adds an edge between two existing nodes
- Supports merge nodes (multiple parents)
- Useful for retroactively connecting related conversations

### F9: Node Status Management

```
llmgraph node resolve [<node-id>]
llmgraph node abandon [<node-id>]
llmgraph node reopen [<node-id>]
```

- Updates node status
- Resolved nodes are visually distinguished in the graph view

### F10: Git Hook Integration

```
llmgraph hooks install
llmgraph hooks uninstall
```

- Installs git hooks into `.git/hooks/`
- `post-checkout` hook: when a new branch is created, prompts to create a child conversation node
- `post-commit` hook: optionally annotates the active node with the commit SHA and message
- Hooks are non-blocking — if the developer declines, git operation proceeds normally

### F11: Search

```
llmgraph search <query>
```

- Full-text search across all node summaries
- Returns matching nodes with the matching field highlighted

### F12: Configuration

`.llmgraph/config.toml`:

```toml
[llm]
provider = "anthropic"          # anthropic | openai | openai-compatible
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"
base_url = ""                   # optional for openai-compatible endpoints

[git]
auto_prompt_on_branch = true    # prompt to create node on git checkout -b
annotate_on_commit = false      # annotate active node with commit SHAs

[storage]
track_in_git = true             # whether .llmgraph/ is committed to the repo
transcript_storage = "none"     # none | local | ref

[ui]
editor = ""                     # defaults to $EDITOR
web_port = 7777
```

---

## Data Storage

### Directory Structure

```
.llmgraph/
  config.toml
  graph.json          # DAG: nodes + edges
  nodes/
    <uuid>.json       # one file per node (for easier git diffs)
  transcripts/        # optional, if transcript_storage = "local"
    <uuid>.txt
```

### `graph.json` Schema

```json
{
  "version": "1",
  "root_id": "<uuid>",
  "active_id": "<uuid>",
  "edges": [
    { "from": "<uuid>", "to": "<uuid>" }
  ]
}
```

### Node File Schema (`nodes/<uuid>.json`)

```json
{
  "id": "<uuid>",
  "parent_ids": ["<uuid>"],
  "git_ref": "feat/rate-limiting",
  "created_at": "2025-04-10T12:00:00Z",
  "updated_at": "2025-04-10T14:30:00Z",
  "status": "Active",
  "tags": ["rate-limiting", "redis"],
  "summary": {
    "goal": "Design and implement two-level rate limiting",
    "decisions": [
      "Token bucket algorithm for per-connection limits",
      "Global bandwidth throttle via shared Redis counter"
    ],
    "rejected_approaches": [
      {
        "description": "Leaky bucket",
        "reason": "Doesn't handle burst traffic gracefully for our use case"
      }
    ],
    "open_threads": [
      "Behavior when Redis is unavailable — fail open or closed?"
    ],
    "key_artifacts": ["src/rate_limit.rs", "src/bandwidth.rs"]
  },
  "raw_transcript_ref": null
}
```

---

## CLI Summary

| Command | Description |
|---|---|
| `llmgraph init` | Initialize graph in current project |
| `llmgraph node create` | Create a new conversation node |
| `llmgraph node edit [id]` | Edit a node's summary |
| `llmgraph node show [id]` | Display a node's summary |
| `llmgraph node list` | List all nodes |
| `llmgraph node resolve [id]` | Mark node as resolved |
| `llmgraph node abandon [id]` | Mark node as abandoned |
| `llmgraph node link <child> --parent <id>` | Add a parent edge |
| `llmgraph node summarize --transcript <file>` | Draft summary from transcript via LLM |
| `llmgraph context [id]` | Generate context payload for LLM injection |
| `llmgraph graph view` | ASCII tree of the full graph |
| `llmgraph graph web` | Interactive web graph visualization |
| `llmgraph search <query>` | Search node summaries |
| `llmgraph hooks install` | Install git hooks |
| `llmgraph hooks uninstall` | Remove git hooks |

---

## Implementation Notes

### Language & Stack

- **Primary language:** Rust
- **CLI framework:** `clap` (v4) with derive macros
- **Serialization:** `serde` + `serde_json`
- **UUIDs:** `uuid` crate
- **Dates:** `chrono`
- **Terminal UI:** `crossterm` or `ratatui` for interactive prompts; plain stdout for non-interactive output
- **Web visualization:** Embedded minimal HTTP server (`axum` or `tiny_http`) serving a single HTML file with a D3.js force-directed graph; no external frontend build step
- **LLM API calls:** `reqwest` with async; support OpenAI-compatible `/v1/chat/completions` endpoint

### Architecture Notes

- The graph and node files are designed to be human-readable and git-diffable — one node per file, pretty-printed JSON
- All mutations go through a `GraphStore` abstraction so storage backend can be swapped (file system in v1, SQLite possible in v2)
- The web visualization server should serve the D3 graph data as a JSON endpoint and render entirely client-side — no templating engine needed
- Git integration uses direct process invocation (`git` subprocess) rather than libgit2 to keep the dependency footprint small

### Node Identity & Navigation

- Each session has an "active node" stored in `.llmgraph/state.json` (not committed)
- Active node is the implicit default for all commands that accept `[id]`
- `llmgraph node create` automatically sets the new node as active

---

## v1 Scope (MVP)

The following features constitute a shippable v1:

- F1: Graph initialization
- F2: Node creation (manual only, no auto-triggers)
- F3: Node editing
- F4: Node viewing (show + list)
- F5: ASCII graph view only (no web UI)
- F6: Context payload generation
- F11: Search
- F12: Configuration (basic)

The following are v2:

- F7: LLM-assisted summary drafting
- F5 (web): Interactive web visualization
- F10: Git hook integration
- F8: Manual edge linking UI improvements
- Multi-user / collaborative graph sharing

---

## Success Criteria

- A developer can initialize a graph, create nodes for a multi-week project, and reconstruct the full decision history from the graph alone
- Context payload generation produces an output that, when pasted into a new LLM conversation, allows the LLM to answer questions about prior decisions without any additional explanation from the developer
- The tool adds less than 60 seconds of overhead per conversation session
- The `.llmgraph/` directory can be committed to a repo and a new team member can navigate the conversation history without any additional tooling
