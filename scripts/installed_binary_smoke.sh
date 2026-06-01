#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
install_root="$(mktemp -d)"
target_repo="$(mktemp -d)"
clean_repo="$(mktemp -d)"

cargo install --path "$repo_root/crates/anvics-cli" --locked --root "$install_root" --quiet
anvics_bin="$install_root/bin/anvics"
test -x "$anvics_bin"

anvics() {
  "$anvics_bin" --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

printf 'base\n' > "$target_repo/app.txt"
mkdir -p "$target_repo/skills/anvics-skill"
cp "$repo_root/skills/anvics-skill/SKILL.md" "$target_repo/skills/anvics-skill/SKILL.md"

echo "Installed Anvics: $anvics_bin"
echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message base

prepare="$(
  anvics agent prepare \
    --title "Installed binary smoke" \
    --task "Edit app.txt so it says installed binary."
)"
thread="$(printf '%s\n' "$prepare" | value_after_prefix "thread: ")"
workspace="$(printf '%s\n' "$prepare" | value_after_prefix "workspace: ")"
workspace_path="$(printf '%s\n' "$prepare" | value_after_prefix "workspace_path: ")"

prompt="$(anvics agent launch-prompt --workspace "$workspace" --tool codex)"
printf '%s\n' "$prompt" | grep -- 'Anvics command prefix:'
if printf '%s\n' "$prompt" | grep -q -- 'cargo run -q'; then
  echo "installed binary launch prompt should not require repo-local cargo run" >&2
  exit 1
fi

anvics agent enter --workspace "$workspace" --name "installed-binary-smoke-agent"
printf 'installed binary\n' > "$workspace_path/app.txt"
anvics workspace diff "$workspace"
accept="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "verify installed binary" \
    --run-summary "Verified installed binary smoke edit." \
    --output "$target_repo/accepted.patch" \
    -- grep "installed binary" app.txt
)"
review="$(printf '%s\n' "$accept" | value_after_prefix "review: ")"
patch="$(printf '%s\n' "$accept" | value_after_prefix "patch: ")"
anvics review show "$review" --format markdown | grep -- 'verify installed binary'
anvics agent status --thread "$thread" | grep -- 'publication_status: published'

printf 'base\n' > "$clean_repo/app.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$patch"
grep -- 'installed binary' "$clean_repo/app.txt"

echo "Installed binary smoke test complete"
