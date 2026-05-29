#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"
clean_repo="$(mktemp -d)"
worker_bin="$repo_root/target/debug/anvics-worker"

cargo build -q -p anvics-cli --bin anvics-worker --manifest-path "$repo_root/Cargo.toml"

anvics() {
  ANVICS_COMMAND_EXECUTOR=worker \
  ANVICS_WORKER_BIN="$worker_bin" \
  cargo run -q -p anvics-cli --bin anvics --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

printf 'base\n' > "$target_repo/app.txt"

echo "Target repo: $target_repo"
cargo run -q -p anvics-cli --bin anvics --manifest-path "$repo_root/Cargo.toml" -- \
  --repo "$target_repo" command worker-check --worker-bin "$worker_bin"
anvics repo init
anvics snapshot create --message base

prepare_output="$(
  anvics agent prepare \
    --title "Worker process smoke" \
    --task "Verify command execution through anvics-worker."
)"
workspace="$(printf '%s\n' "$prepare_output" | value_after_prefix "workspace: ")"

accept_output="$(
  anvics agent accept \
    --workspace "$workspace" \
    --run-label "worker verify" \
    --run-summary "Worker process verified and changed app.txt." \
    --output "$target_repo/accepted.patch" \
    -- sh -c "printf 'worker smoke\n' > app.txt && cat app.txt"
)"
printf '%s\n' "$accept_output"
review="$(printf '%s\n' "$accept_output" | value_after_prefix "review: ")"
patch="$(printf '%s\n' "$accept_output" | value_after_prefix "patch: ")"

anvics review show "$review" --format markdown | grep 'executor: worker'

printf 'base\n' > "$clean_repo/app.txt"
git -C "$clean_repo" init -q
git -C "$clean_repo" apply "$patch"
test "$(cat "$clean_repo/app.txt")" = "worker smoke"

echo "Command worker process smoke test complete"
