---
name: memex
description: Workflow for repositories tracked with memex — a CLI that records development as a versioned DAG of nodes under .memex/. Use whenever you are about to make a code change in a repo containing a .memex/ directory, when the user references memex, nodes, or tracked work, or when asked to create, resolve, abandon, or edit a node. Covers orienting in the graph via `memex context`, finding the right parent, branching, creating a node before writing code, recording decisions/artifacts/open-threads/rejected alternatives as you go, and resolving cleanly.
---

# memex workflow

memex tracks development as a versioned DAG of nodes stored under `.memex/`. Every feature, fix, or refactor gets a node. This skill is the contract for working in a memex-enabled repository.

## Detect

A repository is memex-enabled if `.memex/` exists at the repo root. If absent, this skill does not apply — ask the user whether to run `memex init` before proceeding.

Confirm the CLI is on PATH:

```
memex --version
```

If the command is missing, stop and tell the user — do not try to write to `.memex/` by hand.

## Orient before doing anything else

Whenever you are joining in-progress work, resuming after a break, or evaluating a candidate parent for new work, start with:

```
memex context --format xml
```

This returns the project root's goal/decisions, the trimmed ancestor chain, and the active node's full summary in a single structured payload. Prefer this over chaining `memex node show` + `memex graph` — one call, output shaped for parsing.

- `memex context --id <id> --format xml` to orient on a specific node instead of the active one.
- `--format xml` for agent consumption (most reliable to parse); `--format markdown` if the user will read it.
- `--depth N` to widen the ancestor window on deep branches; `--depth 0` for a minimal root+current payload.

## Before any code change

1. **Find the parent.** Choose based on what your work *depends on*, not what is most recent.
   - `memex graph view` — visualize the DAG.
   - `memex node list` — IDs, parents, statuses, goals.
   - `memex search <keyword>` — full-text search over node summaries; use domain terms (e.g. `config`, `search`, `rename`).
   - Attach to the node whose scope you're extending, even if it isn't the tip.
   - Once you have a candidate, run `memex context --id <candidate> --format xml` to confirm its scope matches what you're about to attach to.

2. **Branch** from main: `git checkout -b <type>/<name>` (`feat/...`, `fix/...`, `chore/...`, `test/...`, `docs/...`).

3. **Create the node before writing code:**

   ```
   memex node create --parent <id> --goal "<goal>"
   ```

   A short placeholder goal is fine if scope is uncertain — refine it later with `memex node edit --goal "..."`.

## While implementing

Record context as you go, not after. Each flag appends without touching other fields (except `--goal`, which overwrites):

```
memex node edit --decision "chose X over Y because Z"
memex node edit --artifact "path/to/key/file"
memex node edit --open-thread "question to revisit"
memex node edit --rejected $'description = "Alternative approach"\nreason = "Why rejected"'
memex node edit --goal "Updated goal if scope changed"
```

Use `--summary` only for a full bulk replacement (e.g. bootstrapping from a plan).

**Two prompts to keep in mind:**

- For every `--decision` you record, ask: *what alternative did I deliberately not take, and why?* If there's an answer, that's a `--rejected`.
- Before resolving, ask: *what question did I defer, what caveat did I notice, what did I leave for later?* Each one is an `--open-thread`.

`--rejected` and `--open-thread` are the under-used fields. Using them is the difference between a node that captures *why* the work landed this way and one that only captures *what* shipped.

## Finishing

1. Run the project's verification commands (lint, format, tests). All must pass before resolving.
2. Resolve or abandon:
   - `memex node resolve` when the work is complete.
   - `memex node abandon` if the approach turned out wrong or was superseded — leave a note in the summary explaining why.
3. Commit source changes **and** `.memex/nodes/` together — they describe the same unit of work.
4. **Never** commit `.memex/state.json`. It is per-developer active-node state and creates guaranteed merge conflicts. The default `.memex/.gitignore` already excludes it; do not override.

## Don't

- Don't edit resolved nodes. They are immutable history — record changes in the *current* node, even if the rename or move touches files an older node listed as artifacts.
- Don't write directly to `.memex/` or bundle multiple nodes per file. Always use the CLI.
- Don't skip node creation for "small" changes. The graph's value is completeness; one missing node breaks the chain for everything downstream.
