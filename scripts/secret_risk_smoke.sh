#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"
clean_repo="$(mktemp -d)"
secret="sk-proj-1234567890abcdefghijklmnopqrstuvwxyz"

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
    --title "Secret risk smoke" \
    --task "Edit app.txt and prove secret-risk output blocks publication."
)"
thread="$(printf '%s\n' "$prepare_output" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace_path: ")"

anvics agent enter --workspace "$workspace" --name "secret-risk-smoke-agent"
printf 'safe accepted change\n' > "$workspace_path/app.txt"

set +e
accept_output="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "leaky verify" \
    --run-summary "Command intentionally emitted a fixture secret." \
    --output "$target_repo/accepted.patch" \
    -- sh -c "printf 'OPENAI_API_KEY=${secret}\\n'" 2>&1
)"
accept_status=$?
set -e

printf '%s\n' "$accept_output"
test "$accept_status" -ne 0
printf '%s\n' "$accept_output" | grep 'publication blocked'
test ! -f "$target_repo/accepted.patch"

status_output="$(anvics agent status --thread "$thread")"
printf '%s\n' "$status_output" | grep 'publication_status: unpublished'
review="$(printf '%s\n' "$status_output" | value_after_prefix "review: ")"

risk_output="$(anvics risk list --review "$review")"
printf '%s\n' "$risk_output" | grep 'openai_token'
if printf '%s\n' "$risk_output" | grep -q "$secret"; then
  echo "risk output leaked raw secret" >&2
  exit 1
fi

publish_output="$(
  anvics publish create \
    --thread "$thread" \
    --review "$review" \
    --allow-secret-risk \
    --override-reason "Intentional fixture secret emitted by smoke test."
)"
printf '%s\n' "$publish_output"
publication="$(printf '%s\n' "$publish_output" | value_after_prefix "Created publication ")"

anvics legacy git export --publication "$publication" --output "$target_repo/accepted.patch"
anvics review show "$review" --format markdown | grep -E 'Risk Notes|openai_token|Intentional fixture secret'

printf 'base\n' > "$clean_repo/app.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$target_repo/accepted.patch"
test "$(cat "$clean_repo/app.txt")" = "safe accepted change"

echo "Secret risk smoke test complete"
