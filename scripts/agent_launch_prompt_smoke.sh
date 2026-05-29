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
    --title "Agent launch prompt smoke" \
    --task "Edit app.txt."
)"
thread="$(printf '%s\n' "$prepare_output" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace_path: ")"
packet="$(printf '%s\n' "$prepare_output" | value_after_prefix "packet: ")"

before_status="$(anvics agent status --thread "$thread")"
before_events="$(anvics events list --since 0)"

codex_prompt="$(anvics agent launch-prompt --workspace "$workspace" --tool codex)"
printf '%s\n' "$codex_prompt"
printf '%s\n' "$codex_prompt" | grep -- '--skip-git-repo-check'
printf '%s\n' "$codex_prompt" | grep -- '--cd'
printf '%s\n' "$codex_prompt" | grep -- "$workspace_path"
printf '%s\n' "$codex_prompt" | grep -- "$packet"
printf '%s\n' "$codex_prompt" | grep -- 'skills/anvics-skill/SKILL.md'
printf '%s\n' "$codex_prompt" | grep -- 'not Git worktrees'
printf '%s\n' "$codex_prompt" | grep -- 'agent context-pack'
printf '%s\n' "$codex_prompt" | grep -- 'workspace diff'

generic_prompt="$(anvics agent launch-prompt --workspace "$workspace")"
printf '%s\n' "$generic_prompt" | grep -- '## Prompt'
if printf '%s\n' "$generic_prompt" | grep -q -- '--skip-git-repo-check'; then
  echo "generic launch prompt should not include Codex-only flags" >&2
  exit 1
fi

after_status="$(anvics agent status --thread "$thread")"
after_events="$(anvics events list --since 0)"
test "$before_status" = "$after_status"
test "$before_events" = "$after_events"

echo "Agent launch prompt smoke test complete"
