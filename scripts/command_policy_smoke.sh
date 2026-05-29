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
    --title "Command policy smoke" \
    --task "Verify command policy gates and overrides."
)"
thread="$(printf '%s\n' "$prepare_output" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace_path: ")"

for program in curl docker vim; do
  set +e
  blocked_output="$(
    anvics command run \
      --workspace "$workspace" \
      --label "blocked $program" \
      --summary "Command policy should block $program." \
      -- "$program" --version 2>&1
  )"
  blocked_status=$?
  set -e
  printf '%s\n' "$blocked_output"
  test "$blocked_status" -ne 0
  printf '%s\n' "$blocked_output" | grep 'command policy blocked'
done

anvics agent status --thread "$thread" | grep 'evidence_count: 0'
events_output="$(anvics events list --since 0)"
if printf '%s\n' "$events_output" | grep -Eq 'CommandStarted|EvidenceAttached'; then
  echo "blocked command created command/evidence events" >&2
  exit 1
fi

override_output="$(
  anvics command run \
    --workspace "$workspace" \
    --label "git version" \
    --summary "Operator approved checking the Git binary version." \
    --allow-command-risk \
    --command-risk-reason "Operator approved local binary version check." \
    -- git --version
)"
printf '%s\n' "$override_output"
printf '%s\n' "$override_output" | grep 'policy: networked'
printf '%s\n' "$override_output" | grep 'command_policy_override: Operator approved local binary version check.'

printf 'accepted with command policy override\n' > "$workspace_path/app.txt"

set +e
blocked_accept="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "network accept" \
    --run-summary "Network verification should be blocked." \
    --output "$target_repo/accepted.patch" \
    -- curl https://example.com 2>&1
)"
blocked_accept_status=$?
set -e
printf '%s\n' "$blocked_accept"
test "$blocked_accept_status" -ne 0
printf '%s\n' "$blocked_accept" | grep 'command policy blocked'
test ! -f "$target_repo/accepted.patch"

accept_output="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "git version accept" \
    --run-summary "Operator approved risky verification before accepting." \
    --allow-command-risk \
    --command-risk-reason "Operator approved accept-time Git version check." \
    --output "$target_repo/accepted.patch" \
    -- git --version
)"
printf '%s\n' "$accept_output"
review="$(printf '%s\n' "$accept_output" | value_after_prefix "review: ")"
patch="$(printf '%s\n' "$accept_output" | value_after_prefix "patch: ")"

anvics review show "$review" --format markdown | grep -E 'policy: networked|command policy override: Operator approved accept-time Git version check'
anvics agent status --thread "$thread" | grep 'publication_status: published'

printf 'base\n' > "$clean_repo/app.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$patch"
test "$(cat "$clean_repo/app.txt")" = "accepted with command policy override"

echo "Command policy smoke test complete"
