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

agent_a_output="$(
  anvics agent prepare \
    --title "Coordination Agent A" \
    --task "Change app.txt for Agent A."
)"
agent_b_output="$(
  anvics agent prepare \
    --title "Coordination Agent B" \
    --task "Inspect coordination status before editing app.txt."
)"

agent_a_workspace="$(printf '%s\n' "$agent_a_output" | value_after_prefix "workspace: ")"
agent_a_path="$(printf '%s\n' "$agent_a_output" | value_after_prefix "workspace_path: ")"
agent_b_workspace="$(printf '%s\n' "$agent_b_output" | value_after_prefix "workspace: ")"

anvics agent enter --workspace "$agent_a_workspace" --name "coordination-agent-a"
anvics agent enter --workspace "$agent_b_workspace" --name "coordination-agent-b"

printf 'agent a\n' > "$agent_a_path/app.txt"
anvics workspace snapshot "$agent_a_workspace" --message "Agent A checkpoint"

status_output="$(anvics coordination status --workspace "$agent_b_workspace")"
printf '%s\n' "$status_output"

grep -q "coordination-agent-a" <<<"$status_output"
grep -q "known_changed_paths: app.txt" <<<"$status_output"

echo "Coordination smoke test complete"
