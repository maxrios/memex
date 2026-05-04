#!/usr/bin/env bash
# Measure context-payload sizes for a memex node across three sources:
#   - memex context --format markdown --depth 3 (the memex payload)
#   - git log + diff on the node's key_artifacts (a non-memex baseline)
#   - the GitHub PR title + body for the node's git_ref (the other obvious baseline)
#
# Prints byte and character counts and an approximate token count
# (chars / 4 — close enough for an order-of-magnitude comparison;
# do not quote these as exact GPT/Claude tokenizer outputs).
#
# Usage:
#   scripts/context-size.sh <node-id-prefix>
#
# Run from the repo root. Requires: cargo, python3, git, gh.
set -euo pipefail

NODE_ID="${1:-}"
if [[ -z "$NODE_ID" ]]; then
    echo "usage: $0 <node-id-prefix>" >&2
    exit 2
fi

NODE_FILE=$(ls .memex/nodes/${NODE_ID}*.json 2>/dev/null | head -1 || true)
if [[ -z "$NODE_FILE" ]]; then
    echo "error: no .memex/nodes/${NODE_ID}*.json found" >&2
    exit 1
fi

read_json() {
    python3 -c "import json,sys; d=json.load(open('$NODE_FILE')); print($1)"
}

GOAL=$(read_json "d['summary']['goal']")
ARTIFACTS=$(read_json "' '.join(d['summary'].get('key_artifacts', []))")
GIT_REF=$(read_json "d.get('git_ref') or ''")
BRANCH=$(printf '%s' "$GIT_REF" | sed 's/ (.*//')

count() {
    python3 -c "
import sys
s = sys.stdin.read()
chars = len(s)
bytes_ = len(s.encode('utf-8'))
print(f'{bytes_:>7} bytes / {chars:>7} chars  (~{chars // 4:>5} tok)')
"
}

printf 'node %s — %s\n\n' "${NODE_ID:0:8}" "$GOAL"

MEMEX=$(cargo run --quiet -- context "${NODE_ID}" --format markdown --depth 3 2>/dev/null)
printf '  memex context --depth 3 --format markdown : %s\n' "$(printf '%s' "$MEMEX" | count)"

if [[ -n "$ARTIFACTS" ]]; then
    # shellcheck disable=SC2086
    GITLOG=$(git log --pretty=fuller -p -- $ARTIFACTS 2>/dev/null)
    printf '  git log -p on key artifacts               : %s\n' "$(printf '%s' "$GITLOG" | count)"
    # shellcheck disable=SC2086
    GITLOG_NO_DIFF=$(git log --pretty=fuller -- $ARTIFACTS 2>/dev/null)
    printf '  git log on key artifacts (messages only)  : %s\n' "$(printf '%s' "$GITLOG_NO_DIFF" | count)"
else
    printf '  git log on key artifacts                  : (node has no key_artifacts)\n'
fi

if [[ -n "$BRANCH" ]]; then
    PR_NUM=$(gh pr list --state all --head "$BRANCH" --json number --jq '.[0].number // empty' 2>/dev/null || true)
    if [[ -n "$PR_NUM" ]]; then
        PR_BODY=$(gh pr view "$PR_NUM" --json title,body --jq '.title + "\n\n" + .body' 2>/dev/null || true)
        printf '  gh pr #%s title + body                    : %s\n' "$PR_NUM" "$(printf '%s' "$PR_BODY" | count)"
    else
        printf '  gh pr (no PR for branch %s)\n' "$BRANCH"
    fi
else
    printf '  gh pr (node has no git_ref)\n'
fi
