#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"
clean_repo="$(mktemp -d)"

anvics() {
  cargo run -q -p anvics-cli --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

printf 'before\n' > "$target_repo/modified.txt"
printf 'delete me\n' > "$target_repo/deleted.txt"

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message base

prepare_output="$(
  anvics agent prepare \
    --title "Live agent packet smoke" \
    --task "Modify modified.txt, delete deleted.txt, and add added.txt."
)"

thread="$(printf '%s\n' "$prepare_output" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace_path: ")"
packet="$(printf '%s\n' "$prepare_output" | value_after_prefix "packet: ")"

echo "Prepared thread: $thread"
echo "Prepared workspace: $workspace"
echo "Agent packet: $packet"
anvics agent packet --thread "$thread"
anvics agent status --thread "$thread"

printf 'after\n' > "$workspace_path/modified.txt"
rm "$workspace_path/deleted.txt"
printf 'new\n' > "$workspace_path/added.txt"
printf 'compact scripted evidence\n' > "$target_repo/agent-summary.txt"

finish_output="$(
  anvics agent finish \
    --workspace "$workspace" \
    --command "scripted-live-agent" \
    --exit-code 0 \
    --summary "Scripted live agent modified, deleted, and added files" \
    --artifact "$target_repo/agent-summary.txt"
)"

review="$(printf '%s\n' "$finish_output" | value_after_prefix "review: ")"
review_markdown="$(printf '%s\n' "$finish_output" | value_after_prefix "review_markdown: ")"

echo "Review: $review"
echo "Review markdown: $review_markdown"
anvics review show "$review" --format markdown
anvics agent status --thread "$thread"

publication="$(
  anvics publish create --thread "$thread" --review "$review" |
    value_after_prefix "Created publication "
)"
patch="$target_repo/accepted.patch"
anvics legacy git export --publication "$publication" --output "$patch"

printf 'before\n' > "$clean_repo/modified.txt"
printf 'delete me\n' > "$clean_repo/deleted.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$patch"

test "$(cat "$clean_repo/modified.txt")" = "after"
test "$(cat "$clean_repo/added.txt")" = "new"
test ! -e "$clean_repo/deleted.txt"

echo "Live-agent packet smoke test complete"
