#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"

anvics() {
  cargo run -q -p anvics-cli --bin anvics --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

printf 'base\n' > "$target_repo/app.txt"

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message base

prepare="$(
  anvics agent prepare \
    --title "Evidence supersede smoke" \
    --task "Edit app.txt."
)"
thread="$(printf '%s\n' "$prepare" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare" | value_after_prefix "workspace_path: ")"
printf 'accepted\n' > "$workspace_path/app.txt"

set +e
blocked="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "leaky verify" \
    --run-summary "Command emitted fixture secret." \
    --output "$target_repo/accepted.patch" \
    -- sh -c "printf 'OPENAI_API_KEY=sk-proj-1234567890abcdefghijklmnopqrstuvwxyz\n'" 2>&1
)"
blocked_status=$?
set -e
test "$blocked_status" -ne 0
printf '%s\n' "$blocked" | grep -- 'publication blocked'
printf '%s\n' "$blocked" | grep -- 'evidence supersede '
printf '%s\n' "$blocked" | grep -- '--reason "<audited reason>"'

status="$(anvics agent status --thread "$thread")"
review="$(printf '%s\n' "$status" | value_after_prefix "review: ")"
risk="$(anvics risk list --review "$review")"
printf '%s\n' "$risk" | grep -- 'CommandStdout'
evidence="$(printf '%s\n' "$risk" | value_after_prefix "evidence: ")"
test -n "$evidence"

anvics evidence supersede "$evidence" --reason "Obsolete leaky verification from smoke test" \
  | grep -- 'status: superseded'
anvics evidence list --thread "$thread" | grep -- 'No evidence'
anvics evidence list --thread "$thread" --include-superseded | grep -- "$evidence"

accept="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "safe verify" \
    --run-summary "Verified accepted app.txt without leaking command output." \
    --output "$target_repo/accepted.patch" \
    -- grep accepted app.txt
)"
printf '%s\n' "$accept" | grep -- 'Accepted agent workspace'
printf '%s\n' "$accept" | grep -- 'publication: '
test -e "$target_repo/accepted.patch"

clean="$(mktemp -d)"
printf 'base\n' > "$clean/app.txt"
git -C "$clean" init -q
git -C "$clean" apply "$target_repo/accepted.patch"
grep -- 'accepted' "$clean/app.txt"

echo "Evidence supersede smoke test complete"
