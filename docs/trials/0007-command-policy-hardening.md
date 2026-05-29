# Trial 0007 - Command Policy Hardening

Date: 2026-05-29

Source state: `be1ddd4-dirty`

## Goal

Validate MVP 0.22 command-file policy hardening with one real Codex CLI agent and an operator-run blocked/override command-file acceptance flow.

## Setup

- Target repo: disposable copy of `/Users/simon/Repos/anvics`
- Agent tool: `codex exec`
- Agent workspace: Anvics materialized workspace
- Task: add one README sentence mentioning command policy preview
- Note: `codex exec` required `--skip-git-repo-check` because the Anvics workspace is intentionally not a Git repository.

## Agent Result

- Read packet: yes
- Read Anvics skill: yes
- Ran `agent enter`: yes
- Used `workspace diff`: yes
- Ran `coordination status`: yes
- Created Git branch/worktree/commit: no
- Edited only workspace path: yes
- Verification: `rg` found the new README command-policy sentence
- Coordination result: no related work or potential clashes

The agent added:

```md
Command policy preview is available with `anvics command classify -- <program> [args...]` before running unfamiliar commands through Anvics.
```

## Operator Policy Flow

Blocked command-file accept:

```sh
anvics --repo "$TARGET_REPO" agent accept \
  --workspace "$WORKSPACE_ID" \
  --run-label "blocked network file" \
  --run-summary "Network command file should be blocked." \
  --output "$TARGET_REPO/accepted.patch" \
  --command-file "$TARGET_REPO/risky-verify.sh"
```

Result:

- Blocked with `command policy blocked Networked`
- `agent status` still showed `evidence_count: 0`
- Publication remained `unpublished`
- No patch was written

Policy preview:

```sh
anvics --repo "$TARGET_REPO" command classify --command-file "$TARGET_REPO/verify-accept.sh"
```

Result:

- `policy: networked`
- `blocked: true`
- override hint printed

Accepted with explicit override:

```sh
anvics --repo "$TARGET_REPO" agent accept \
  --workspace "$WORKSPACE_ID" \
  --run-label "command-file verify" \
  --run-summary "Verified README command policy preview sentence with an approved command file." \
  --allow-command-risk \
  --command-risk-reason "Dogfood approved git version plus README rg check." \
  --output "$TARGET_REPO/accepted.patch" \
  --command-file "$TARGET_REPO/verify-accept.sh"
```

Result:

- Review: `615e2d47-daa1-4e58-919e-62d774b839fb`
- Publication: `d5dbd26e-ad20-40d9-8ad4-6cfa653ded20`
- Review markdown included `policy: networked`
- Review markdown included the command policy override reason
- Exported patch applied cleanly to a clean copy

## Takeaways

- Command-file policy scanning closed the obvious bypass.
- `command classify` is useful as an operator preview before accepting risk.
- The packet/skill flow still works for real Codex CLI agents.
- Git-default friction remains visible: Codex CLI needed `--skip-git-repo-check` for a non-Git Anvics workspace.
- Next likely step is not more policy surface; it is either worker-process separation or explicitly documenting how external agent CLIs should be launched against non-Git Anvics workspaces.
