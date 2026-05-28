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

thread_a="$(anvics thread create --title "Agent A" --task "Change app.txt to identify Agent A" | value_after_prefix "Created thread ")"
thread_b="$(anvics thread create --title "Agent B" --task "Change app.txt to identify Agent B" | value_after_prefix "Created thread ")"

workspace_a_output="$(anvics workspace create --thread "$thread_a")"
workspace_b_output="$(anvics workspace create --thread "$thread_b")"
workspace_a="$(printf '%s\n' "$workspace_a_output" | value_after_prefix "Created workspace ")"
workspace_b="$(printf '%s\n' "$workspace_b_output" | value_after_prefix "Created workspace ")"
workspace_a_path="$(printf '%s\n' "$workspace_a_output" | value_after_prefix "path: ")"
workspace_b_path="$(printf '%s\n' "$workspace_b_output" | value_after_prefix "path: ")"

printf 'agent a\n' > "$workspace_a_path/app.txt"
printf 'agent b\n' > "$workspace_b_path/app.txt"

anvics evidence attach \
  --thread "$thread_a" \
  --command "scripted-agent-a" \
  --exit-code 0 \
  --summary "Scripted Agent A changed app.txt"

anvics evidence attach \
  --thread "$thread_b" \
  --command "scripted-agent-b" \
  --exit-code 0 \
  --summary "Scripted Agent B changed app.txt"

anvics workspace snapshot "$workspace_a" --message "Agent A result"
anvics workspace snapshot "$workspace_b" --message "Agent B result"

review_a="$(anvics review create --thread "$thread_a" | tee /dev/stderr | value_after_prefix "Created review ")"
anvics review show "$review_a"
anvics publish create --thread "$thread_a" --review "$review_a"

echo "Smoke test complete"
