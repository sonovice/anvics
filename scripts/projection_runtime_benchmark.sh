#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

anvics_for_repo() {
  local repo="$1"
  shift
  cargo run -q -p anvics-cli --bin anvics --features vfs-fuse --manifest-path "$repo_root/Cargo.toml" -- --repo "$repo" "$@"
}

populate_repo() {
  local repo="$1"
  mkdir -p "$repo/src/module_01" "$repo/src/module_02/dir_to_rename" "$repo/docs" "$repo/fixtures/medium" "$repo/target/cache"
  printf 'Projection runtime benchmark\n' > "$repo/README.md"
  printf 'base app\n' > "$repo/src/module_01/file_001.txt"
  printf 'rename me\n' > "$repo/src/module_01/rename_me.txt"
  printf 'delete me\n' > "$repo/src/module_02/file_002.txt"
  printf 'child\n' > "$repo/src/module_02/dir_to_rename/child.txt"
  for n in $(seq 1 8); do
    module="$(printf '%02d' "$n")"
    mkdir -p "$repo/src/module_$module"
    for m in $(seq 1 12); do
      file="$(printf '%03d' "$m")"
      printf 'module=%s file=%s base\n' "$module" "$file" > "$repo/src/module_$module/file_$file.txt"
    done
  done
  for n in $(seq 1 5); do
    file="$(printf '%02d' "$n")"
    head -c 65536 /dev/zero | tr '\0' "$file" > "$repo/fixtures/medium/blob_$file.txt"
  done
  printf 'ignored\n' > "$repo/.DS_Store"
  printf 'ignored cache\n' > "$repo/target/cache/output.txt"
}

run_case() {
  local projection="$1"
  local repo clean patch run_output status prepare workspace accept
  repo="$(mktemp -d)"
  clean="$(mktemp -d)"
  patch="$repo/accepted-$projection.patch"
  populate_repo "$repo"

  echo
  echo "=== projection: $projection ==="
  echo "target_repo: $repo"
  anvics_for_repo "$repo" repo init >/dev/null
  anvics_for_repo "$repo" snapshot create --message "benchmark base" >/dev/null
  prepare="$(anvics_for_repo "$repo" agent prepare \
    --title "Projection runtime benchmark $projection" \
    --task "Edit, add, and delete files through $projection.")"
  workspace="$(printf '%s\n' "$prepare" | sed -n 's/^workspace: //p')"

  set +e
  run_output="$(anvics_for_repo "$repo" command run \
    --workspace "$workspace" \
    --label "benchmark $projection edit" \
    --summary "Edited benchmark files through $projection" \
    --projection "$projection" \
    -- \
    sh -c ": > src/module_01/file_001.txt && printf 'changed through $projection\n' >> src/module_01/file_001.txt && printf 'new through $projection\n' > src/module_03/added_from_projection.tmp && mv src/module_03/added_from_projection.tmp src/module_03/added_from_projection.txt && mv src/module_01/rename_me.txt src/module_01/renamed_by_projection.txt && mv src/module_02/dir_to_rename src/module_02/renamed_dir && mkdir -p src/module_04/transient/nested && printf 'gone\n' > src/module_04/transient/nested/file.txt && rm -r src/module_04/transient && rm src/module_02/file_002.txt" 2>&1)"
  status=$?
  set -e

  if [ "$status" -ne 0 ]; then
    if [ "$projection" = "fuse-mount" ] && printf '%s\n' "$run_output" | grep -Eiq 'projection unavailable|FUSE|fuse|macFUSE|mount failed|device not configured|operation not permitted'; then
      echo "skipped: FUSE/macFUSE unavailable"
      printf '%s\n' "$run_output"
      return 0
    fi
    printf '%s\n' "$run_output"
    return "$status"
  fi

  printf '%s\n' "$run_output" | grep -E '^(projection|projection_fallback_reason|projection_setup_ms|command_ms|reconcile_ms|cleanup_ms|projection_files|projection_bytes):'
  printf '%s\n' "$run_output" | grep -- '- Modified: src/module_01/file_001.txt'
  printf '%s\n' "$run_output" | grep -- '- Added: src/module_03/added_from_projection.txt'
  printf '%s\n' "$run_output" | grep -- '- Added: src/module_01/renamed_by_projection.txt'
  printf '%s\n' "$run_output" | grep -- '- Deleted: src/module_01/rename_me.txt'
  printf '%s\n' "$run_output" | grep -- '- Added: src/module_02/renamed_dir/child.txt'
  printf '%s\n' "$run_output" | grep -- '- Deleted: src/module_02/dir_to_rename/child.txt'
  printf '%s\n' "$run_output" | grep -- '- Deleted: src/module_02/file_002.txt'

  accept="$(anvics_for_repo "$repo" agent accept \
    --workspace "$workspace" \
    --run-label "verify benchmark $projection" \
    --run-summary "Verified benchmark projection result." \
    --projection materialized-dir \
    --output "$patch" \
    -- \
    sh -c "grep 'changed through $projection' src/module_01/file_001.txt && test -f src/module_03/added_from_projection.txt && test -f src/module_01/renamed_by_projection.txt && test ! -e src/module_01/rename_me.txt && test -f src/module_02/renamed_dir/child.txt && test ! -e src/module_02/dir_to_rename && test ! -e src/module_04/transient && test ! -e src/module_02/file_002.txt")"
  printf '%s\n' "$accept" | grep -E '^(review|publication|patch):'

  populate_repo "$clean"
  git -C "$clean" init -q
  git -C "$clean" apply "$patch"
  grep "changed through $projection" "$clean/src/module_01/file_001.txt" >/dev/null
  test -f "$clean/src/module_03/added_from_projection.txt"
  test -f "$clean/src/module_01/renamed_by_projection.txt"
  test ! -e "$clean/src/module_01/rename_me.txt"
  test -f "$clean/src/module_02/renamed_dir/child.txt"
  test ! -e "$clean/src/module_02/dir_to_rename"
  test ! -e "$clean/src/module_04/transient"
  test ! -e "$clean/src/module_02/file_002.txt"
  echo "patch_apply: ok"
}

run_case materialized-dir
run_case auto
run_case fuse-mount

echo
echo "Projection runtime benchmark complete"
