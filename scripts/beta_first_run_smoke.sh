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

printf '# Trial app\n\nstatus: base\n' > "$target_repo/README.md"
printf 'base\n' > "$target_repo/app.txt"
mkdir -p "$target_repo/skills/anvics-skill"
cp "$repo_root/skills/anvics-skill/SKILL.md" "$target_repo/skills/anvics-skill/SKILL.md"

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message base
anvics repo doctor --path README.md --path app.txt

prepare="$(
  anvics agent prepare \
    --title "Private beta first run" \
    --task "Update README.md so status mentions private beta first run. Keep the change tiny."
)"
thread="$(printf '%s\n' "$prepare" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare" | value_after_prefix "workspace_path: ")"
packet="$(printf '%s\n' "$prepare" | value_after_prefix "packet: ")"

echo "Thread: $thread"
echo "Workspace: $workspace"
echo "Packet: $packet"
anvics agent launch-prompt --workspace "$workspace" --tool codex
anvics agent enter --workspace "$workspace" --name "scripted-beta-agent"
anvics agent context-pack --workspace "$workspace"

printf '# Trial app\n\nstatus: private beta first run\n' > "$workspace_path/README.md"
anvics workspace diff "$workspace"
anvics agent checkpoint --workspace "$workspace" --summary "First-run README update before acceptance."
anvics coordination status --workspace "$workspace"

accept="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "verify first run" \
    --run-summary "Verified private beta first-run README update." \
    --output "$target_repo/accepted.patch" \
    -- rg -n "private beta first run" README.md
)"
review="$(printf '%s\n' "$accept" | value_after_prefix "review: ")"
patch="$(printf '%s\n' "$accept" | value_after_prefix "patch: ")"

anvics review show "$review" --format markdown
anvics risk list --review "$review"
anvics agent status --thread "$thread"

printf '# Trial app\n\nstatus: base\n' > "$clean_repo/README.md"
printf 'base\n' > "$clean_repo/app.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$patch"
grep -- "private beta first run" "$clean_repo/README.md"

echo "Private beta first-run smoke test complete"
