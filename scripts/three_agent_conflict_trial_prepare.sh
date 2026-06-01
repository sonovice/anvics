#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="${1:-$(mktemp -d)}"

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

export ANVICS_AGENT_COMMAND="cargo run -q -p anvics-cli --bin anvics --manifest-path $(shell_quote "$repo_root/Cargo.toml") --"

mkdir -p "$target_repo/src" "$target_repo/skills/anvics-skill"
cat > "$target_repo/Cargo.toml" <<'EOF'
[package]
name = "anvics-conflict-trial"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF
cat > "$target_repo/src/lib.rs" <<'EOF'
pub fn describe_score(score: i32) -> String {
    if score >= 80 {
        "strong".to_owned()
    } else {
        "needs work".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_strong_scores() {
        assert_eq!(describe_score(82), "strong");
    }

    #[test]
    fn describes_low_scores() {
        assert_eq!(describe_score(40), "needs work");
    }
}
EOF
cat > "$target_repo/README.md" <<'EOF'
# Anvics Three-Agent Conflict Trial

This disposable repo is used to test conflicting agent edits.
EOF
cp "$repo_root/skills/anvics-skill/SKILL.md" "$target_repo/skills/anvics-skill/SKILL.md"

(
  cd "$target_repo"
  cargo generate-lockfile >/dev/null 2>&1
)

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message "three-agent conflict base"

task_a="Change describe_score in src/lib.rs so scores >= 90 return excellent, scores 70 through 89 return passing, and lower scores still return needs work. Update tests. Verification suggestion: cargo test."
task_b="Change describe_score in src/lib.rs so every non-invalid return includes the numeric score, for example score 82: strong. Preserve the existing threshold behavior otherwise. Update tests. Verification suggestion: cargo test."
task_c="Change describe_score in src/lib.rs so negative scores return invalid score. Preserve the existing positive-score behavior otherwise. Update tests. Verification suggestion: cargo test."

prepare_agent() {
  local title="$1"
  local task="$2"
  anvics agent prepare --title "$title" --task "$task"
}

agent_a_output="$(prepare_agent "Conflict Agent A thresholds" "$task_a")"
agent_b_output="$(prepare_agent "Conflict Agent B score labels" "$task_b")"
agent_c_output="$(prepare_agent "Conflict Agent C invalid negatives" "$task_c")"

agent_a_thread="$(printf '%s\n' "$agent_a_output" | value_after_prefix "thread: ")"
agent_a_workspace="$(printf '%s\n' "$agent_a_output" | value_after_prefix "workspace: ")"
agent_a_workspace_path="$(printf '%s\n' "$agent_a_output" | value_after_prefix "workspace_path: ")"
agent_a_packet="$(printf '%s\n' "$agent_a_output" | value_after_prefix "packet: ")"

agent_b_thread="$(printf '%s\n' "$agent_b_output" | value_after_prefix "thread: ")"
agent_b_workspace="$(printf '%s\n' "$agent_b_output" | value_after_prefix "workspace: ")"
agent_b_workspace_path="$(printf '%s\n' "$agent_b_output" | value_after_prefix "workspace_path: ")"
agent_b_packet="$(printf '%s\n' "$agent_b_output" | value_after_prefix "packet: ")"

agent_c_thread="$(printf '%s\n' "$agent_c_output" | value_after_prefix "thread: ")"
agent_c_workspace="$(printf '%s\n' "$agent_c_output" | value_after_prefix "workspace: ")"
agent_c_workspace_path="$(printf '%s\n' "$agent_c_output" | value_after_prefix "workspace_path: ")"
agent_c_packet="$(printf '%s\n' "$agent_c_output" | value_after_prefix "packet: ")"

manifest="$target_repo/.anvics/three-agent-conflict-manifest.env"
cat > "$manifest" <<EOF
target_repo=$(shell_quote "$target_repo")
source_repo=$(shell_quote "$repo_root")
agent_a_thread=$(shell_quote "$agent_a_thread")
agent_a_workspace=$(shell_quote "$agent_a_workspace")
agent_a_workspace_path=$(shell_quote "$agent_a_workspace_path")
agent_a_packet=$(shell_quote "$agent_a_packet")
agent_b_thread=$(shell_quote "$agent_b_thread")
agent_b_workspace=$(shell_quote "$agent_b_workspace")
agent_b_workspace_path=$(shell_quote "$agent_b_workspace_path")
agent_b_packet=$(shell_quote "$agent_b_packet")
agent_c_thread=$(shell_quote "$agent_c_thread")
agent_c_workspace=$(shell_quote "$agent_c_workspace")
agent_c_workspace_path=$(shell_quote "$agent_c_workspace_path")
agent_c_packet=$(shell_quote "$agent_c_packet")
verify_command=$(shell_quote "cargo test")
EOF

print_prompt() {
  local label="$1"
  local thread="$2"
  local workspace="$3"
  local workspace_path="$4"
  local packet="$5"

  cat <<EOF

=== $label ===
thread: $thread
workspace: $workspace
workspace_path: $workspace_path
packet: $packet
suggested_verify: cargo test

$(anvics agent launch-prompt --workspace "$workspace" --tool codex)
EOF
}

print_prompt "Agent A - threshold change" "$agent_a_thread" "$agent_a_workspace" "$agent_a_workspace_path" "$agent_a_packet"
print_prompt "Agent B - numeric score labels" "$agent_b_thread" "$agent_b_workspace" "$agent_b_workspace_path" "$agent_b_packet"
print_prompt "Agent C - invalid negative scores" "$agent_c_thread" "$agent_c_workspace" "$agent_c_workspace_path" "$agent_c_packet"

cat <<EOF

Manifest: $manifest

Success checklist:
- All three agents read the packet and Anvics skill.
- All three agents edit only their workspace.
- All three agents use workspace diff, checkpoint, and coordination status.
- All three candidate reviews report overlap on src/lib.rs.
- A resolver workspace combines the three intents.
- The resolved patch applies cleanly and cargo test passes.
EOF
