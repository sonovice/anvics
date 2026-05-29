#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="${1:-$(mktemp -d)}"
source_commit="$(git -C "$repo_root" rev-parse --short HEAD)"
source_state="$source_commit"
if ! git -C "$repo_root" diff --quiet || ! git -C "$repo_root" diff --cached --quiet; then
  source_state="$source_commit-dirty"
fi

anvics() {
  cargo run -q -p anvics-cli --bin anvics --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

shell_quote() {
  printf "%q" "$1"
}

mkdir -p "$target_repo"
if command -v rsync >/dev/null 2>&1; then
  rsync -a \
    --exclude '.git' \
    --exclude '.anvics' \
    --exclude 'target' \
    "$repo_root"/ "$target_repo"/
else
  (
    cd "$repo_root"
    tar --exclude './.git' --exclude './.anvics' --exclude './target' -cf - .
  ) | (
    cd "$target_repo"
    tar -xf -
  )
fi

echo "Target repo: $target_repo"
echo "Source state: $source_state"
anvics repo init
anvics snapshot create --message "dogfood base $source_state"

agent_a_task="Improve one small docs section in docs/AGENT_TEST_RUNBOOK.md for dogfood clarity. Keep the edit tiny: one short sentence or one concise bullet. Do not touch Rust code. Verification suggestion: rg -n 'dogfood|Dogfood|trial' docs/AGENT_TEST_RUNBOOK.md docs/trials."
agent_b_task="Make one small CLI/test improvement in crates/anvics-cli/tests/cli.rs around agent status or workspace UX if straightforward. Keep the edit tiny and avoid broad refactors. Verification suggestion: cargo test -p anvics-cli --test cli workspace_show_and_agent_status_by_workspace_report_overlay_state."

agent_a_output="$(
  anvics agent prepare \
    --title "Dogfood Agent A docs polish" \
    --task "$agent_a_task"
)"
agent_b_output="$(
  anvics agent prepare \
    --title "Dogfood Agent B CLI test polish" \
    --task "$agent_b_task"
)"

agent_a_thread="$(printf '%s\n' "$agent_a_output" | value_after_prefix "thread: ")"
agent_a_workspace="$(printf '%s\n' "$agent_a_output" | value_after_prefix "workspace: ")"
agent_a_workspace_path="$(printf '%s\n' "$agent_a_output" | value_after_prefix "workspace_path: ")"
agent_a_packet="$(printf '%s\n' "$agent_a_output" | value_after_prefix "packet: ")"

agent_b_thread="$(printf '%s\n' "$agent_b_output" | value_after_prefix "thread: ")"
agent_b_workspace="$(printf '%s\n' "$agent_b_output" | value_after_prefix "workspace: ")"
agent_b_workspace_path="$(printf '%s\n' "$agent_b_output" | value_after_prefix "workspace_path: ")"
agent_b_packet="$(printf '%s\n' "$agent_b_output" | value_after_prefix "packet: ")"

manifest="$target_repo/.anvics/dogfood-trial-manifest.env"
cat > "$manifest" <<EOF
target_repo=$(shell_quote "$target_repo")
source_repo=$(shell_quote "$repo_root")
source_state=$(shell_quote "$source_state")
agent_a_thread=$(shell_quote "$agent_a_thread")
agent_a_workspace=$(shell_quote "$agent_a_workspace")
agent_a_workspace_path=$(shell_quote "$agent_a_workspace_path")
agent_a_packet=$(shell_quote "$agent_a_packet")
agent_a_verify_command=$(shell_quote "rg -n 'dogfood|Dogfood|trial' docs/AGENT_TEST_RUNBOOK.md docs/trials")
agent_b_thread=$(shell_quote "$agent_b_thread")
agent_b_workspace=$(shell_quote "$agent_b_workspace")
agent_b_workspace_path=$(shell_quote "$agent_b_workspace_path")
agent_b_packet=$(shell_quote "$agent_b_packet")
agent_b_verify_command=$(shell_quote "cargo test -p anvics-cli --test cli workspace_show_and_agent_status_by_workspace_report_overlay_state")
EOF

print_prompt() {
  local label="$1"
  local thread="$2"
  local workspace="$3"
  local workspace_path="$4"
  local packet="$5"
  local verify="$6"
  local launch_prompt
  launch_prompt="$(anvics agent launch-prompt --workspace "$workspace" --tool codex)"

  cat <<EOF

=== $label ===
thread: $thread
workspace: $workspace
workspace_path: $workspace_path
packet: $packet
suggested_verify: $verify

Generated Codex CLI launch guidance:

$launch_prompt

Generic prompt for another external agent:

You are working inside an Anvics dogfood task packet.

Read the packet at:
$packet

Before editing, read the Anvics skill at:
$workspace_path/skills/anvics-skill/SKILL.md

Follow the skill and packet exactly. Work only inside the workspace path listed in the packet.
This workspace may not be a Git repository; if your agent CLI requires it, use its non-Git workspace flag.
Run the packet's agent enter command before editing.
Use the packet's workspace diff command instead of Git status or Git diff.
Run coordination status before finishing and report any potential clashes.
Do not create a Git branch, Git worktree, or Git commit.
Keep the change intentionally small.
When done, tell me the verification command you ran, its exit code, a one-sentence summary, and any compact artifact path.
EOF
}

print_prompt \
  "Agent A - docs polish" \
  "$agent_a_thread" \
  "$agent_a_workspace" \
  "$agent_a_workspace_path" \
  "$agent_a_packet" \
  "rg -n 'dogfood|Dogfood|trial' docs/AGENT_TEST_RUNBOOK.md docs/trials"

print_prompt \
  "Agent B - CLI/test polish" \
  "$agent_b_thread" \
  "$agent_b_workspace" \
  "$agent_b_workspace_path" \
  "$agent_b_packet" \
  "cargo test -p anvics-cli --test cli workspace_show_and_agent_status_by_workspace_report_overlay_state"

cat <<EOF

Manifest: $manifest

Success checklist for this Anvics dogfood validation:

- Each agent confirms it read the Anvics skill from its workspace.
- Each agent edits only the workspace path from the packet.
- Each agent runs agent enter before editing.
- Each agent uses workspace diff instead of Git status or Git diff.
- Each agent runs coordination status before finishing.
- The accepted workspace can produce a review, publication, and legacy patch.
- The exported patch applies cleanly to a clean copy.

After agents finish, accept the chosen workspace with:

scripts/live_agent_trial_verify.sh "$target_repo" \\
  <accepted-thread-id> <accepted-workspace-id> "<verification command>" <exit-code> "<summary>"

Record the actual run under docs/trials/ after the live run completes.
EOF
