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
printf 'delete me\n' > "$target_repo/delete.txt"

anvics repo init >/dev/null
anvics snapshot create --message base >/dev/null

prepare="$(
  anvics agent prepare \
    --title "Restore smoke" \
    --task "Make an edit, restore part of it, then publish."
)"
workspace="$(printf '%s\n' "$prepare" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare" | value_after_prefix "workspace_path: ")"

printf 'bad\n' > "$workspace_path/app.txt"
printf 'scratch\n' > "$workspace_path/scratch.txt"
dry_run="$(anvics workspace restore "$workspace" --source base --path app.txt --path scratch.txt --reason "preview restore" --dry-run)"
printf '%s\n' "$dry_run" | grep -- 'Workspace restore dry run'
grep -- 'bad' "$workspace_path/app.txt"
test -e "$workspace_path/scratch.txt"

restore="$(anvics workspace restore "$workspace" --source base --path app.txt --path scratch.txt --reason "drop bad scratch edit")"
printf '%s\n' "$restore" | grep -- 'Restored workspace'
printf '%s\n' "$restore" | grep -- 'pre_restore_checkpoint:'
grep -- 'base' "$workspace_path/app.txt"
test ! -e "$workspace_path/scratch.txt"

printf 'checkpointed\n' > "$workspace_path/app.txt"
checkpoint="$(anvics agent checkpoint --workspace "$workspace" --summary "checkpointed app edit")"
checkpoint_id="$(printf '%s\n' "$checkpoint" | value_after_prefix "Created agent checkpoint ")"
test -n "$checkpoint_id"
printf 'lost\n' > "$workspace_path/app.txt"
anvics agent checkpoint list --workspace "$workspace" | grep -- "$checkpoint_id"
anvics agent checkpoint show "$checkpoint_id" | grep -- 'checkpointed app edit'
anvics agent checkpoint restore --workspace "$workspace" --checkpoint "$checkpoint_id" --reason "recover checkpointed edit" >/dev/null
grep -- 'checkpointed' "$workspace_path/app.txt"

printf 'published\n' > "$workspace_path/app.txt"
printf 'added\n' > "$workspace_path/added.txt"
rm "$workspace_path/delete.txt"
accept="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "verify publish" \
    --run-summary "Verified publication before revert." \
    --output "$target_repo/published.patch" \
    -- grep published app.txt
)"
publication="$(printf '%s\n' "$accept" | value_after_prefix "publication: ")"
test -n "$publication"

printf 'published\n' > "$target_repo/app.txt"
printf 'added\n' > "$target_repo/added.txt"
rm "$target_repo/delete.txt"
anvics snapshot create --message published-head >/dev/null

revert="$(
  anvics publish revert prepare \
    --publication "$publication" \
    --reason "smoke test revert"
)"
printf '%s\n' "$revert" | grep -- 'Prepared publication revert'
printf '%s\n' "$revert" | grep -- 'unresolved_cases: 0'
revert_workspace="$(printf '%s\n' "$revert" | value_after_prefix "workspace: ")"
revert_workspace_path="$(printf '%s\n' "$revert" | value_after_prefix "workspace_path: ")"
grep -- 'base' "$revert_workspace_path/app.txt"
grep -- 'delete me' "$revert_workspace_path/delete.txt"
test ! -e "$revert_workspace_path/added.txt"

revert_accept="$(
  anvics agent accept \
    --workspace "$revert_workspace" \
    --run-label "verify revert" \
    --run-summary "Verified inverse publication workspace." \
    --output "$target_repo/revert.patch" \
    -- grep base app.txt
)"
printf '%s\n' "$revert_accept" | grep -- 'Accepted agent workspace'
test -e "$target_repo/revert.patch"

clean="$(mktemp -d)"
printf 'published\n' > "$clean/app.txt"
printf 'added\n' > "$clean/added.txt"
git -C "$clean" init -q
git -C "$clean" apply "$target_repo/revert.patch"
grep -- 'base' "$clean/app.txt"
test ! -e "$clean/added.txt"

echo "Native revert/restore smoke test complete"
