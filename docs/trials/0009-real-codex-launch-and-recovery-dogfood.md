# Dogfood Trial 0009 - Real Codex Launch And Recovery

Date: 2026-06-01
Source state: `6fa70b5`
Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.PRXCywxlVA`
Agent tool: `codex exec v0.135.0`

## Goal

Validate that real Codex CLI agents can start from generated Anvics packets, read the skill, work in non-Git Anvics workspaces, preserve progress with checkpoints, coordinate with another active agent, and produce accept/export-ready work without manual Git coaching.

## Launch

Both agents were launched with:

```sh
codex exec --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check --cd <workspace-path> -
```

The generated dogfood prompts used `ANVICS_AGENT_COMMAND` so packet commands pointed at the repo-local Cargo invocation instead of assuming `anvics` was installed globally.

## Agent A

- Thread: `4f06e571-86a7-4d41-b4e7-8966cb3c330a`
- Workspace: `ad43ad7c-080b-4332-a526-8095fddd4741`
- Task: add one small dogfood clarity note to `docs/AGENT_TEST_RUNBOOK.md`.
- Skill read: yes.
- Context pack used: no.
- Workspace diff used: yes.
- Checkpoint: `25c2dd00-83da-4116-a685-c266facdadab`.
- Coordination result: Agent B had unknown changes during Agent A's final check, but no known overlap yet.
- Final change: added a short note that generated packets are authoritative, including command prefixes and final-report requirements.
- Verification: `rg -n 'dogfood|Dogfood|trial' docs/AGENT_TEST_RUNBOOK.md docs/trials`, exit code 0.
- Publication: accepted with explicit secret-risk override after a false positive in preserved command stdout.

## Agent B

- Thread: `495de006-c7bc-4d51-9628-f5ebe5c3a9ad`
- Workspace: `272371c4-288f-4adf-a684-47e2257de463`
- Task: make a small CLI/test improvement near workspace status behavior.
- Skill read: yes.
- Context pack used: yes.
- Workspace diff used: yes.
- Checkpoint: `cec8cfb7-b4a3-4b0f-9d9e-5aae93b455a8`.
- Coordination result: `potential_clashes: none`; Agent A had changes only in `docs/AGENT_TEST_RUNBOOK.md`.
- Final change: tightened `agent status --workspace` CLI test coverage for `latest_snapshot` before and after workspace snapshotting.
- Verification: `cargo test -p anvics-cli --test cli workspace_show_and_agent_status_by_workspace_report_overlay_state`, exit code 0.
- Publication: accepted normally.

## Product Findings

- The launch path now works with real Codex CLI when the prompt includes `--skip-git-repo-check`, a write-enabled disposable workspace, and the generated Anvics command prefix.
- Crash recovery checkpoints were useful in a real agent run. Both agents created checkpoints before final reporting, making interrupted work recoverable.
- Coordination awareness worked: each agent saw the other active workspace and either unknown-change risk or known non-overlap.
- Context packs exposed a bug: generated packets honored `ANVICS_AGENT_COMMAND`, but `agent context-pack` still rendered bare `anvics` commands. This was fixed immediately after the trial.
- Secret-risk gating correctly preserved failed accept state, but the experience showed a rough edge: preserved risky evidence on a thread can contaminate later publish attempts even after a safer verification command is used. This needs a future supersede/ignore evidence flow rather than relying only on override.

## Patch Export

- Agent A patch: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.PRXCywxlVA/agent-a.patch`
- Agent B patch: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.PRXCywxlVA/agent-b.patch`
- Both patches passed `git apply --check` against the live Anvics repo before being applied.

## Takeaways

- The current packet/skill/recovery loop is good enough for repeated real Codex dogfood.
- The next meaningful reliability slice should address evidence superseding after blocked accept attempts.
- Keep Anvics command-prefix propagation as a required invariant for all generated agent-facing artifacts.
