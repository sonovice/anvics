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
anvics repo init >/dev/null
anvics snapshot create --message base >/dev/null

prepare="$(
  anvics agent prepare \
    --title "Recovery smoke" \
    --task "Edit app.txt and checkpoint salvage progress."
)"
workspace="$(printf '%s\n' "$prepare" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare" | value_after_prefix "workspace_path: ")"

anvics agent enter --workspace "$workspace" --name recovery-smoke-agent >/dev/null
printf 'salvaged\n' > "$workspace_path/app.txt"

recover_before="$(anvics agent recover --workspace "$workspace")"
printf '%s\n' "$recover_before" | grep 'current_changed_paths:'
printf '%s\n' "$recover_before" | grep 'Modified: app.txt'
printf '%s\n' "$recover_before" | grep 'latest_checkpoint: none'

checkpoint="$(anvics agent checkpoint --workspace "$workspace" --summary "salvaged app edit")"
printf '%s\n' "$checkpoint" | grep 'Created agent checkpoint'
printf '%s\n' "$checkpoint" | grep 'snapshot:'
printf '%s\n' "$checkpoint" | grep 'Modified: app.txt'

recover_after="$(anvics agent recover --workspace "$workspace")"
printf '%s\n' "$recover_after" | grep 'latest_checkpoint:'
printf '%s\n' "$recover_after" | grep 'checkpoint_changed_paths:'
printf '%s\n' "$recover_after" | grep 'next_commands:'

echo "Agent recovery smoke test complete"
