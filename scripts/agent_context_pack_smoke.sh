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
mkdir -p "$target_repo/skills/anvics-skill"
printf '# Anvics\n' > "$target_repo/skills/anvics-skill/SKILL.md"

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message base

prepare_output="$(
  anvics agent prepare \
    --title "Agent context pack smoke" \
    --task "Edit app.txt."
)"
workspace="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace_path: ")"
printf 'changed\n' > "$workspace_path/app.txt"

pack="$(anvics agent context-pack --workspace "$workspace")"
printf '%s\n' "$pack" | grep -- '# Anvics Context Pack'
printf '%s\n' "$pack" | grep -- 'Edit app.txt'
printf '%s\n' "$pack" | grep -- 'workspace diff'
printf '%s\n' "$pack" | grep -- 'Modified: `app.txt` (source)'
printf '%s\n' "$pack" | grep -- 'Potential clashes: none'

written="$(anvics agent context-pack --workspace "$workspace" --write)"
context_path="$(printf '%s\n' "$written" | value_after_prefix "context_pack: ")"
test -e "$context_path"
grep -- '# Anvics Context Pack' "$context_path"

echo "Agent context pack smoke test complete"
