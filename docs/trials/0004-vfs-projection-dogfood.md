# Dogfood Trial 0004 - Projection Runtime Acceptance

Date: 2026-05-28
Operator: Simon + Codex
Anvics source commit: `b496a4f` plus MVP 0.18 working-tree changes
Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/anvics-dogfood-0.18.M1EZlW`

## Trial Goal

Validate one real Codex CLI agent using an Anvics packet, then accept the result through `agent accept --projection auto`.

This was a focused runtime/VFS validation, not a two-agent coordination test.

## Agent Run

- Tool/version: `codex-cli 0.134.0`
- Thread: `2541fd26-784b-492a-a23e-1c4f1e5a8fe9`
- Workspace: `0b33af06-20e6-448a-bfca-d9d2d0dff034`
- Packet: `.anvics/agent-packets/2541fd26-784b-492a-a23e-1c4f1e5a8fe9.md`
- Task: clarify operator-only projection usage in `docs/AGENT_TEST_RUNBOOK.md`
- Changed file: `docs/AGENT_TEST_RUNBOOK.md`
- Agent verification: `rg -n "do not add them to agent packet commands|Projection Runtime Checks" docs/AGENT_TEST_RUNBOOK.md`
- Agent-reported result: exit 0

## Packet/Skill Behavior

- Agent read the packet before editing: yes.
- Agent read `skills/anvics-skill/SKILL.md` before editing: yes.
- Agent ran `agent enter`: yes.
- Agent edited only the workspace path: yes.
- Agent used `workspace diff`: yes.
- Agent avoided Git status/diff/branches/worktrees/commits: yes.
- Agent ran `coordination status`: yes.
- Coordination result: `related_work: none`, `potential_clashes: none`.

## Operator Acceptance

Accepted with Anvics-run verification:

```sh
anvics --repo "$TARGET_REPO" agent accept \
  --workspace "$WORKSPACE" \
  --run-label "vfs dogfood verify" \
  --run-summary "Verified dogfood docs change through auto projection." \
  --projection auto \
  --output "$TARGET_REPO/accepted.patch" \
  -- rg -n "do not add them to agent packet commands|Projection Runtime Checks" docs/AGENT_TEST_RUNBOOK.md
```

Result:

- Review: `bb2329a1-5fbe-4eaa-9e5c-3dbc600106ac`
- Publication: `969d0fa6-b10d-4490-a115-58cc1eca99ff`
- Patch: `accepted.patch`
- Projection selected by `auto`: `fuse_mount`
- Fallback reason: none
- Command policy class: `read_only`
- Risk findings: none
- Clean-copy `git apply`: passed

## Observations

- The generated packet successfully taught a real Codex CLI agent to read the skill, enter the workspace, use Anvics diff, and avoid Git defaults.
- `--projection auto` used FUSE successfully on this machine and review markdown recorded `projection: fuse_mount`.
- The review evidence stayed compact: command label, summary, stdout/stderr artifact paths, policy class, and projection.
- Token cost was still high for a tiny docs change because Codex read the packet, skill, and much of the runbook. This is acceptable for validation, but packet/skill brevity remains important.

## Product Takeaways

- Keep the packet-first workflow and explicit skill-read instruction.
- Keep projection flags operator-only; the dogfood edit added a sentence saying not to add projection flags to agent packet commands.
- Keep VFS behind `command run`/`agent accept`; no evidence from this run suggests agents need manual mount commands.
- Next VFS work should focus on runtime reliability and larger-repo economics, not new agent-facing workflow.
