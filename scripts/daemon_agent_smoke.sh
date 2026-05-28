#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"
socket="$target_repo/anvics.sock"

echo "Target repo: $target_repo"

cargo run -q -p anvics-cli --bin anvicsd --manifest-path "$repo_root/Cargo.toml" -- --socket "$socket" &
daemon_pid="$!"
trap 'kill "$daemon_pid" >/dev/null 2>&1 || true' EXIT

for _ in {1..200}; do
  if [[ -S "$socket" ]]; then
    break
  fi
  sleep 0.02
done

anvics() {
  ANVICS_DAEMON_SOCKET="$socket" cargo run -q -p anvics-cli --bin anvics --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" --use-daemon "$@"
}

anvics daemon ping
anvics repo init
printf 'before\n' > "$target_repo/modified.txt"
printf 'delete me\n' > "$target_repo/deleted.txt"
anvics snapshot create --message base

prepare_output="$(
  anvics agent prepare \
    --title "Daemon smoke" \
    --task "Modify modified.txt, delete deleted.txt, and add added.txt."
)"
printf '%s\n' "$prepare_output"
thread="$(printf '%s\n' "$prepare_output" | sed -n 's/^thread: //p')"
workspace="$(printf '%s\n' "$prepare_output" | sed -n 's/^workspace: //p')"
workspace_path="$(printf '%s\n' "$prepare_output" | sed -n 's/^workspace_path: //p')"
anvics agent enter --workspace "$workspace" --name "daemon-smoke-agent"

printf 'after\n' > "$workspace_path/modified.txt"
rm "$workspace_path/deleted.txt"
printf 'new\n' > "$workspace_path/added.txt"
anvics coordination status --workspace "$workspace"

accept_output="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "daemon-smoke-verify" \
    --run-summary "Daemon smoke modified, deleted, and added files." \
    --output "$target_repo/accepted.patch" \
    -- sh -c "cat modified.txt && cat added.txt"
)"
printf '%s\n' "$accept_output"
review="$(printf '%s\n' "$accept_output" | sed -n 's/^review: //p')"

anvics review show "$review" --format markdown
anvics agent status --thread "$thread"
anvics events list --since 0 | grep -E 'CommandStarted|CommandFinished|LegacyPatchExported'

clean_repo="$(mktemp -d)"
printf 'before\n' > "$clean_repo/modified.txt"
printf 'delete me\n' > "$clean_repo/deleted.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$target_repo/accepted.patch"

echo "Daemon smoke test complete"
