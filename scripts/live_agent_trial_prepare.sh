#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="${1:-$(mktemp -d)}"

anvics() {
  cargo run -q -p anvics-cli --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

mkdir -p "$target_repo"
cat > "$target_repo/app.txt" <<'APP'
title: Anvics trial app
status: base
APP
cat > "$target_repo/notes.md" <<'NOTES'
# Trial Notes

- Keep this file concise.
NOTES
cat > "$target_repo/config.toml" <<'CONFIG'
name = "anvics-trial"
mode = "base"
CONFIG

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message base

agent_a_output="$(
  anvics agent prepare \
    --title "Agent A mixed task" \
    --task "Update notes.md with one concise bullet describing why isolated Anvics workspaces are useful. Also update app.txt so status says it was changed by Agent A during the overlap trial."
)"
agent_b_output="$(
  anvics agent prepare \
    --title "Agent B deliberate overlap" \
    --task "Update app.txt so status clearly says it was changed by Agent B during the overlap trial. Do not edit notes.md."
)"

print_packet() {
  local label="$1"
  local output="$2"
  local thread workspace workspace_path packet
  thread="$(printf '%s\n' "$output" | value_after_prefix "thread: ")"
  workspace="$(printf '%s\n' "$output" | value_after_prefix "workspace: ")"
  workspace_path="$(printf '%s\n' "$output" | value_after_prefix "workspace_path: ")"
  packet="$(printf '%s\n' "$output" | value_after_prefix "packet: ")"

  cat <<EOF

=== $label ===
thread: $thread
workspace: $workspace
workspace_path: $workspace_path
packet: $packet

Paste this prompt into the external agent:

You are working inside an Anvics task packet.

Read the packet at:
$packet

Follow it exactly. Work only inside the workspace path listed in the packet.
Do not create a Git branch, Git worktree, or Git commit.
When done, tell me a short command label, its exit code, a one-sentence summary, and optionally a command/evidence file path.
EOF
}

print_packet "Agent A - mixed non-overlap plus overlap task" "$agent_a_output"
print_packet "Agent B - overlap task" "$agent_b_output"

cat <<EOF

After agents finish, run:

scripts/live_agent_trial_verify.sh "$target_repo" \\
  <accepted-thread-id> <accepted-workspace-id> "<command>" <exit-code> "<summary>"

Agent A has one non-overlapping file change plus one deliberate same-file overlap with Agent B. Finish the competing workspace separately if you want overlap notes, then use the verify script to accept whichever result you want to publish.
EOF
