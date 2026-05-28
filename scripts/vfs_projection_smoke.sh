#!/usr/bin/env bash
set -euo pipefail

repo="$(mktemp -d)"
clean="$(mktemp -d)"
patch="$repo/accepted.patch"

anvics() {
  cargo run -q -p anvics-cli --bin anvics --features vfs-fuse -- --repo "$repo" "$@"
}

echo "Target repo: $repo"
printf 'base\n' > "$repo/app.txt"
printf 'remove\n' > "$repo/delete.txt"

anvics repo init >/dev/null
anvics snapshot create --message base >/dev/null

prepare="$(anvics agent prepare \
  --title "VFS projection smoke" \
  --task "Modify app.txt, add added.txt, and delete delete.txt through a mounted projection.")"
workspace="$(printf '%s\n' "$prepare" | sed -n 's/^workspace: //p')"

set +e
run_output="$(anvics command run \
  --workspace "$workspace" \
  --label "vfs edit" \
  --summary "Edited files through FUSE projection" \
  --projection fuse-mount \
  -- \
  sh -c "cat app.txt >/dev/null && printf 'vfs changed\n' > app.txt && printf 'new\n' > added.txt && rm delete.txt" 2>&1)"
status=$?
set -e

if [ "$status" -ne 0 ]; then
  if printf '%s\n' "$run_output" | grep -Eiq 'projection unavailable|FUSE|fuse|macFUSE|mount failed|device not configured|operation not permitted'; then
    echo "Skipping VFS projection smoke: FUSE/macFUSE is unavailable in this environment."
    printf '%s\n' "$run_output"
    exit 0
  fi
  printf '%s\n' "$run_output"
  exit "$status"
fi

printf '%s\n' "$run_output"
printf '%s\n' "$run_output" | grep 'exit_code: 0'
printf '%s\n' "$run_output" | grep 'projection: fuse_mount'
printf '%s\n' "$run_output" | grep 'file_effects:'
printf '%s\n' "$run_output" | grep -- '- Modified: app.txt'
printf '%s\n' "$run_output" | grep -- '- Added: added.txt'
printf '%s\n' "$run_output" | grep -- '- Deleted: delete.txt'

accept="$(anvics agent accept \
  --workspace "$workspace" \
  --run-label "verify vfs result" \
  --run-summary "Verified mounted projection changes after persistence" \
  --projection materialized-dir \
  --output "$patch" \
  -- \
  sh -c "grep 'vfs changed' app.txt && test -f added.txt && test ! -e delete.txt")"

printf '%s\n' "$accept"
test -f "$patch"

cp "$repo/app.txt" "$clean/app.txt"
cp "$repo/delete.txt" "$clean/delete.txt"
(
  cd "$clean"
  git init -q
  git apply "$patch"
  grep 'vfs changed' app.txt
  test -f added.txt
  test ! -e delete.txt
)

echo "VFS projection smoke test complete"
