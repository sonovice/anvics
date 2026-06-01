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
name = "anvics-beta-trial"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF
cat > "$target_repo/src/lib.rs" <<'EOF'
pub fn greeting(name: &str) -> String {
    format!("hello, {name}")
}

pub fn status() -> &'static str {
    "base"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_mentions_name() {
        assert_eq!(greeting("agent"), "hello, agent");
    }
}
EOF
cat > "$target_repo/README.md" <<'EOF'
# Anvics Beta Trial Fixture

This repo is a disposable external beta trial target.
EOF
cp "$repo_root/skills/anvics-skill/SKILL.md" "$target_repo/skills/anvics-skill/SKILL.md"

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message "external beta base"

agent_a_task="Update README.md with one concise sentence describing the beta trial fixture. Do not edit Rust code. Verification suggestion: rg -n 'beta trial' README.md."
agent_b_task="Add a small Rust test for status() in src/lib.rs without changing public behavior. Verification suggestion: cargo test."

agent_a_output="$(
  anvics agent prepare \
    --title "External beta Agent A docs" \
    --task "$agent_a_task"
)"
agent_b_output="$(
  anvics agent prepare \
    --title "External beta Agent B test" \
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

manifest="$target_repo/.anvics/external-beta-trial-manifest.env"
cat > "$manifest" <<EOF
target_repo=$(shell_quote "$target_repo")
source_repo=$(shell_quote "$repo_root")
agent_a_thread=$(shell_quote "$agent_a_thread")
agent_a_workspace=$(shell_quote "$agent_a_workspace")
agent_a_workspace_path=$(shell_quote "$agent_a_workspace_path")
agent_a_packet=$(shell_quote "$agent_a_packet")
agent_a_verify_command=$(shell_quote "rg -n 'beta trial' README.md")
agent_b_thread=$(shell_quote "$agent_b_thread")
agent_b_workspace=$(shell_quote "$agent_b_workspace")
agent_b_workspace_path=$(shell_quote "$agent_b_workspace_path")
agent_b_packet=$(shell_quote "$agent_b_packet")
agent_b_verify_command=$(shell_quote "cargo test")
EOF

print_prompt() {
  local label="$1"
  local thread="$2"
  local workspace="$3"
  local workspace_path="$4"
  local packet="$5"
  local verify="$6"

  cat <<EOF

=== $label ===
thread: $thread
workspace: $workspace
workspace_path: $workspace_path
packet: $packet
suggested_verify: $verify

$(anvics agent launch-prompt --workspace "$workspace" --tool codex)
EOF
}

print_prompt "Agent A - external docs" "$agent_a_thread" "$agent_a_workspace" "$agent_a_workspace_path" "$agent_a_packet" "rg -n 'beta trial' README.md"
print_prompt "Agent B - external Rust test" "$agent_b_thread" "$agent_b_workspace" "$agent_b_workspace_path" "$agent_b_packet" "cargo test"

cat <<EOF

Manifest: $manifest

Success checklist:
- Both agents read the packet and Anvics skill.
- Both agents run agent enter before editing.
- Both agents use workspace diff and coordination status.
- At least one accepted patch applies cleanly to a clean copy.
- Record results in docs/trials/0011-external-repo-beta-trial.md.
EOF
