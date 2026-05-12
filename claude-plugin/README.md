# memex Claude plugin

A Claude Code plugin that ships a single skill — `memex` — teaching autonomous agents how to use the [memex](../README.md) CLI in any repository tracked with it.

## What it does

memex tracks development as a versioned DAG of nodes under `.memex/`. The workflow (find a parent, branch, create a node, record decisions, resolve) lives in [AGENTS.md](../AGENTS.md) for contributors to *this* repository. The plugin externalizes that workflow as a Claude skill so any agent working in *any* memex-enabled repo picks up the same guidance without copy-pasting instructions into a project's `CLAUDE.md`.

The skill is triggered when:

- The agent is about to make a code change in a repo containing a `.memex/` directory.
- The user mentions memex, nodes, or tracked work.
- The agent is asked to create, resolve, abandon, or edit a node.

## Layout

```
claude-plugin/
├── .claude-plugin/
│   └── plugin.json
├── skills/
│   └── memex/
│       └── SKILL.md
└── README.md
```

## Installation

The memex repository is itself a Claude Code marketplace (see `.claude-plugin/marketplace.json` at the repo root). Add the marketplace, then install the plugin:

```
claude plugin marketplace add /path/to/memex      # or a GitHub URL once published
claude plugin install memex@memex
```

The `memex` CLI must also be installed and on `PATH` — the skill instructs the agent to verify this via `memex --version` before acting.

## Testing locally

The cheapest checks are static:

```
claude plugin validate /path/to/memex             # marketplace manifest
claude plugin validate /path/to/memex/claude-plugin   # plugin manifest
```

For a real end-to-end test, install the plugin (see above), then start a fresh Claude Code session in a *separate* memex-enabled directory — not the memex repo itself, since its AGENTS.md would prime the model artificially. A minimal fixture:

```
mkdir /tmp/memex-test && cd /tmp/memex-test
git init && memex init
memex node edit --goal "Some toy project" --decision "Some decision"
memex node create --parent <root-id> --goal "First feature"
```

Then ask the agent in that session to add a feature. The skill should trigger and the agent should orient via `memex context` before doing anything else. Iterate on `SKILL.md`, then `claude plugin marketplace update memex` to reload.

## Versioning

The plugin lives inside the memex repository so its skill stays in lockstep with the CLI it describes. Any PR that changes a memex subcommand or flag should update `skills/memex/SKILL.md` in the same diff.
