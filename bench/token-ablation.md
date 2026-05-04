# Token-size ablation: memex context vs. git baselines

Cheap, deterministic comparison of context-payload sizes for three representative
nodes. **No agent runs, no LLM judge, no statistics** — just bytes and characters
on the wire. Approximate tokens at chars / 4 (close enough for an order-of-magnitude
comparison; not exact tokenizer output).

This was the alternative to issue #38's full N=5 multi-arm benchmark. See
`scripts/context-size.sh` for the script and the PR description for the methodology
discussion.

## Method

For each node:

- **memex**: `cargo run -- context <id> --format markdown --depth 3`
- **git log (messages only)**: `git log --pretty=fuller -- <key_artifacts>` — what a
  thoughtful non-memex user would actually scroll through.
- **git log -p (with diffs)**: same plus full diffs — the "everything available"
  upper bound an agent could be given.
- **PR body**: `gh pr view <num>` title + body for the PR matching the node's `git_ref`.

Reproduce with `scripts/context-size.sh <node-id>` from the repo root. Numbers
captured 2026-05-03 against `main` at `44219c2`.

## Results

### Decision-heavy: `5a17fac4` — Audit and correct memex graph parent misassignments

Has 4 decisions and 3 rejected approaches in its memex summary.

| Source                              | Bytes  | Chars  | ~Tokens |
| ----------------------------------- | -----: | -----: | ------: |
| memex context --depth 3             |  3,759 |  3,735 |     933 |
| git log -p on key artifacts         | 34,640 | 34,603 |   8,650 |
| git log (messages only)             | 15,809 | 15,778 |   3,944 |
| gh pr #20 title + body              |  1,181 |  1,175 |     293 |
| **realistic baseline** (log + PR)   | 16,990 | 16,953 |   4,237 |

memex is **~22%** of the realistic baseline, and is the only source that contains
the three rejected approaches (Reparent LICENSE to 269bfac9, leave the linear chain
intact, reparent d85b81 away from 970717 — each with its reason).

### Refactor with motivation: `b95077e3` — Remove graph.edges dual representation

Has 1 decision and 1 rejected approach.

| Source                              | Bytes  | Chars  | ~Tokens |
| ----------------------------------- | -----: | -----: | ------: |
| memex context --depth 3             |  2,630 |  2,626 |     656 |
| git log -p on key artifacts         | 81,605 | 81,461 |  20,365 |
| git log (messages only)             |  7,879 |  7,867 |   1,966 |
| gh pr #44 title + body              |    988 |    980 |     245 |
| **realistic baseline** (log + PR)   |  8,867 |  8,847 |   2,211 |

memex is **~30%** of the realistic baseline. The rejected approach ("add a
check_integrity() call instead — Two representations are still maintained;
integrity checks only catch divergence after the fact") is in the memex payload
but absent from the commit message and the PR body.

### Shallow / negative control: `27ed3dfa` — Declare MSRV

Has 2 decisions and 0 rejected approaches. Memex shouldn't help much here.

| Source                              | Bytes  | Chars  | ~Tokens |
| ----------------------------------- | -----: | -----: | ------: |
| memex context --depth 3             |  2,862 |  2,854 |     713 |
| git log -p on key artifacts         |  3,984 |  3,980 |     995 |
| git log (messages only)             |  2,322 |  2,318 |     579 |
| gh pr #37 title + body              |    493 |    491 |     122 |
| **realistic baseline** (log + PR)   |  2,815 |  2,809 |     701 |

memex is **~102%** of the baseline — slightly *more* tokens, with high content
overlap ("rust-version = 1.74 because clap 4 requires it" appears in both). For
shallow tasks the project-context preamble that memex always emits (root goal,
key decisions, key artifacts) is fixed overhead that dominates the payload.

## Honest reading

- On the decision-heavy and refactor cases, memex is 3–5× smaller than the
  realistic non-memex context **and** carries information (the rejected
  approaches) that the non-memex sources don't have at all. The token win is
  real and the qualitative win is larger.
- On the shallow case, memex is roughly token-neutral and offers no
  decision-context the commit message doesn't already convey. The fixed
  ~700-token project preamble is a real cost; for a one-line `Cargo.toml` edit,
  pasting the commit message would be cheaper.
- The right framing for users: "memex gives you a token-cheap, decision-rich
  context payload for substantive tasks; for trivial changes the overhead may
  not be worth it." That's a more credible claim than "memex is always smaller."

## Caveats

- Three nodes is not a sample. The point of this file is to make the comparison
  **reproducible and inspectable**, not to make a statistical claim. Run the
  script on your own nodes and judge for yourself.
- Token counts are approximate (chars / 4). The realistic-baseline column adds
  log + PR; users may paste more or less than that. The ratios are what matter.
- All three nodes come from this repo, which is authored by memex users. A
  non-memex repo would have different commit-message and PR-body length
  distributions, which affects the baseline.
- See PR description (`refs #38`) for why we did *not* run the full N=5 multi-arm
  benchmark.
