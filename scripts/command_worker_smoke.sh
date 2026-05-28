#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"
clean_repo="$(mktemp -d)"

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

prepare_output="$(
  anvics agent prepare \
    --title "Command worker smoke" \
    --task "Edit app.txt and let Anvics verify the result."
)"
thread="$(printf '%s\n' "$prepare_output" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace_path: ")"

anvics agent enter --workspace "$workspace" --name "command-worker-smoke-agent"
anvics coordination status --workspace "$workspace"

accept_output="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "verify app" \
    --run-summary "Anvics verified app.txt before accepting." \
    --output "$target_repo/accepted.patch" \
    -- sh -c "printf 'verified by anvics\n' > app.txt && grep 'verified by anvics' app.txt"
)"
printf '%s\n' "$accept_output"
review="$(printf '%s\n' "$accept_output" | value_after_prefix "review: ")"
patch="$(printf '%s\n' "$accept_output" | value_after_prefix "patch: ")"

anvics review show "$review" --format markdown | grep -E 'anvics-run:|stdout:|verify app|policy: mutating|file effects: modified `app.txt`'
anvics agent status --thread "$thread" | grep 'publication_status: published'
anvics events list --since 0 | grep -E 'CommandStarted|CommandFinished|LegacyPatchExported'

printf 'base\n' > "$clean_repo/app.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$patch"
test "$(cat "$clean_repo/app.txt")" = "verified by anvics"

echo "Command worker smoke test complete"
