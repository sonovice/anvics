#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 6 ]]; then
  cat >&2 <<'USAGE'
Usage:
  scripts/live_agent_trial_verify.sh <target-repo> <thread-id> <workspace-id> <command> <exit-code> <summary> [artifact-path]
USAGE
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$1"
thread="$2"
workspace="$3"
command_name="$4"
exit_code="$5"
summary="$6"
artifact="${7:-}"
patch="$target_repo/accepted.patch"
clean_repo="$(mktemp -d)"

anvics() {
  cargo run -q -p anvics-cli --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

finish_args=(
  agent finish
  --workspace "$workspace"
  --command "$command_name"
  --exit-code "$exit_code"
  --summary "$summary"
)
if [[ -n "$artifact" ]]; then
  finish_args+=(--artifact "$artifact")
fi

finish_output="$(anvics "${finish_args[@]}")"
review="$(printf '%s\n' "$finish_output" | value_after_prefix "review: ")"

echo "$finish_output"
anvics review show "$review" --format markdown
anvics agent status --thread "$thread"

publication="$(
  anvics publish create --thread "$thread" --review "$review" |
    tee /dev/stderr |
    value_after_prefix "Created publication "
)"
anvics legacy git export --publication "$publication" --output "$patch"

cp "$target_repo/app.txt" "$clean_repo/app.txt"
cp "$target_repo/notes.md" "$clean_repo/notes.md"
cp "$target_repo/config.toml" "$clean_repo/config.toml"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$patch"

echo "Verified exported patch with git apply"
echo "target_repo: $target_repo"
echo "patch: $patch"
