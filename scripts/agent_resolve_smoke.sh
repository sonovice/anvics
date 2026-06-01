#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_repo="$(mktemp -d)"
agent_command="cargo run -q -p anvics-cli --bin anvics --manifest-path $repo_root/Cargo.toml --"

anvics() {
  cargo run -q -p anvics-cli --bin anvics --manifest-path "$repo_root/Cargo.toml" -- --repo "$target_repo" "$@"
}

value_after_prefix() {
  local prefix="$1"
  sed -n "s/^${prefix}//p" | head -n 1
}

mkdir -p "$target_repo/src"
cat > "$target_repo/Cargo.toml" <<'EOF'
[package]
name = "anvics-resolve-smoke"
version = "0.1.0"
edition = "2021"
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
    fn describes_scores() {
        assert_eq!(describe_score(82), "strong");
        assert_eq!(describe_score(40), "needs work");
    }
}
EOF
cargo generate-lockfile -q --manifest-path "$target_repo/Cargo.toml"

echo "Target repo: $target_repo"
anvics repo init
anvics snapshot create --message base

prepare_candidate() {
  local title="$1"
  local task="$2"
  anvics agent prepare --title "$title" --task "$task" --agent-command "$agent_command"
}

finish_candidate() {
  local workspace="$1"
  local summary="$2"
  anvics agent finish \
    --workspace "$workspace" \
    --command "cargo test" \
    --exit-code 0 \
    --summary "$summary"
}

agent_a="$(prepare_candidate "Candidate A thresholds" "Make scores >= 90 excellent and 70..=89 passing.")"
agent_b="$(prepare_candidate "Candidate B labels" "Include the numeric score in every non-invalid label.")"
agent_c="$(prepare_candidate "Candidate C invalid negatives" "Return invalid score for negative inputs.")"

workspace_a="$(printf '%s\n' "$agent_a" | value_after_prefix "workspace: ")"
workspace_b="$(printf '%s\n' "$agent_b" | value_after_prefix "workspace: ")"
workspace_c="$(printf '%s\n' "$agent_c" | value_after_prefix "workspace: ")"
path_a="$(printf '%s\n' "$agent_a" | value_after_prefix "workspace_path: ")"
path_b="$(printf '%s\n' "$agent_b" | value_after_prefix "workspace_path: ")"
path_c="$(printf '%s\n' "$agent_c" | value_after_prefix "workspace_path: ")"

python3 - "$path_a/src/lib.rs" <<'PY'
from pathlib import Path
path = Path(__import__("sys").argv[1])
path.write_text('''pub fn describe_score(score: i32) -> String {
    if score >= 90 {
        "excellent".to_owned()
    } else if score >= 70 {
        "passing".to_owned()
    } else {
        "needs work".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_scores() {
        assert_eq!(describe_score(95), "excellent");
        assert_eq!(describe_score(82), "passing");
        assert_eq!(describe_score(40), "needs work");
    }
}
''')
PY
python3 - "$path_b/src/lib.rs" <<'PY'
from pathlib import Path
path = Path(__import__("sys").argv[1])
path.write_text('''pub fn describe_score(score: i32) -> String {
    if score >= 80 {
        format!("score {score}: strong")
    } else {
        format!("score {score}: needs work")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_scores() {
        assert_eq!(describe_score(82), "score 82: strong");
        assert_eq!(describe_score(40), "score 40: needs work");
    }
}
''')
PY
python3 - "$path_c/src/lib.rs" <<'PY'
from pathlib import Path
path = Path(__import__("sys").argv[1])
path.write_text('''pub fn describe_score(score: i32) -> String {
    if score < 0 {
        "invalid score".to_owned()
    } else if score >= 80 {
        "strong".to_owned()
    } else {
        "needs work".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_scores() {
        assert_eq!(describe_score(-1), "invalid score");
        assert_eq!(describe_score(82), "strong");
        assert_eq!(describe_score(40), "needs work");
    }
}
''')
PY

review_a="$(finish_candidate "$workspace_a" "Candidate A tests passed" | value_after_prefix "review: ")"
review_b="$(finish_candidate "$workspace_b" "Candidate B tests passed" | value_after_prefix "review: ")"
review_c="$(finish_candidate "$workspace_c" "Candidate C tests passed" | value_after_prefix "review: ")"

analysis="$(
  anvics conflict analyze \
    --review "$review_a" \
    --review "$review_b" \
    --review "$review_c"
)"
printf '%s\n' "$analysis" | grep -- 'conflict_analysis:'
printf '%s\n' "$analysis" | grep -- 'SamePathOverlappingHunks'

resolve="$(
  anvics conflict prepare \
    --review "$review_a" \
    --review "$review_b" \
    --review "$review_c" \
    --title "Resolve score candidates" \
    --task "Combine threshold, score label, and invalid-negative behavior." \
    --agent-command "$agent_command"
)"
resolver_workspace="$(printf '%s\n' "$resolve" | value_after_prefix "workspace: ")"
resolver_path="$(printf '%s\n' "$resolve" | value_after_prefix "workspace_path: ")"
resolver_packet="$(printf '%s\n' "$resolve" | value_after_prefix "packet: ")"

grep -- "$review_a" "$resolver_packet"
grep -- "$review_b" "$resolver_packet"
grep -- "$review_c" "$resolver_packet"
grep -- 'Candidate A tests passed' "$resolver_packet"
grep -- 'Deterministic Conflict Analysis' "$resolver_packet"
grep -- "$agent_command" "$resolver_packet"

cat > "$resolver_path/src/lib.rs" <<'EOF'
pub fn describe_score(score: i32) -> String {
    if score < 0 {
        "invalid score".to_owned()
    } else if score >= 90 {
        format!("score {score}: excellent")
    } else if score >= 70 {
        format!("score {score}: passing")
    } else {
        format!("score {score}: needs work")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_scores() {
        assert_eq!(describe_score(-1), "invalid score");
        assert_eq!(describe_score(95), "score 95: excellent");
        assert_eq!(describe_score(82), "score 82: passing");
        assert_eq!(describe_score(40), "score 40: needs work");
    }
}
EOF

anvics conflict status --workspace "$resolver_workspace" | grep -- 'passed: false'

accept="$(
  anvics agent accept \
    --workspace "$resolver_workspace" \
    --run-label "cargo test" \
    --run-summary "Resolved score behavior passed tests." \
    --output "$target_repo/resolved.patch" \
    -- cargo test
)"
review="$(printf '%s\n' "$accept" | value_after_prefix "review: ")"
publication="$(printf '%s\n' "$accept" | value_after_prefix "publication: ")"
test -n "$publication"
anvics conflict verify --workspace "$resolver_workspace" | grep -- 'passed: true'
anvics review show "$review" --format markdown | grep -- '## Source Reviews'
anvics review show "$review" --format markdown | grep -- "$review_a"

clean="$(mktemp -d)"
cp -R "$target_repo/Cargo.toml" "$target_repo/src" "$clean/"
git -C "$clean" init -q
git -C "$clean" apply "$target_repo/resolved.patch"
cargo test -q --manifest-path "$clean/Cargo.toml"

echo "Agent resolve smoke test complete"
