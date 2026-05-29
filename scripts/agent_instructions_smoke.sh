#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"

anvics() {
  cargo run -q -p anvics-cli --bin anvics --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

echo "Target repo: $target_repo"
anvics repo init

rendered="$(anvics agent instructions --target agents)"
printf '%s\n' "$rendered" | grep -- 'AGENTS.md'
printf '%s\n' "$rendered" | grep -- 'Anvics Agent Instructions'
printf '%s\n' "$rendered" | grep -- 'workspace diff'
printf '%s\n' "$rendered" | grep -- 'Do not create Git branches'
test ! -e "$target_repo/AGENTS.md"

installed="$(anvics agent instructions --install)"
printf '%s\n' "$installed" | grep -- 'wrote:'
printf '%s\n' "$installed" | grep -- 'AGENTS.md'
printf '%s\n' "$installed" | grep -- 'CLAUDE.md'
test -e "$target_repo/AGENTS.md"
test -e "$target_repo/CLAUDE.md"
grep -- 'Anvics Agent Instructions' "$target_repo/AGENTS.md"
grep -- 'Anvics Agent Instructions' "$target_repo/CLAUDE.md"

if anvics agent instructions --install >/tmp/anvics-agent-instructions-overwrite.out 2>&1; then
  echo "agent instructions --install should refuse to overwrite existing files" >&2
  exit 1
fi
grep -- 'already exists' /tmp/anvics-agent-instructions-overwrite.out

anvics agent instructions --target agents --install --force

echo "Agent instructions smoke test complete"
