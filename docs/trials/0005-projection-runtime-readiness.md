# Dogfood Trial 0005 - Projection Runtime Readiness

Date: 2026-05-29
Operator: Simon + Codex
Anvics source commit: `1f09d62` plus MVP 0.19 working-tree changes
Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/anvics-dogfood-0.19.f6Ui4G`

## Trial Goal

Validate one real Codex CLI agent using an Anvics packet, then accept the result through `agent accept --projection auto` with runtime metrics visible in review evidence.

## Agent Run

- Tool/version: `codex-cli 0.134.0`
- Thread: `1a23ad27-b891-4b87-9129-075ddc9cf44a`
- Workspace: `da9186e2-b42e-44d0-bbc6-e08a9ce15138`
- Packet: `.anvics/agent-packets/1a23ad27-b891-4b87-9129-075ddc9cf44a.md`
- Task: add one README sentence about projection runtime diagnostics in Anvics-run command evidence
- Changed file: `README.md`
- Agent verification: `rg -n "Projection runtime diagnostics are now visible in Anvics-run command evidence\\." README.md`
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
  --run-label "runtime metrics dogfood verify" \
  --run-summary "Verified README dogfood change through auto projection." \
  --projection auto \
  --output "$TARGET_REPO/accepted.patch" \
  -- rg -n "Projection runtime diagnostics are now visible in Anvics-run command evidence\\." README.md
```

Result:

- Review: `a009eabd-ad8c-4d88-acb1-61b5bea5947a`
- Publication: `f1742628-8ec3-41ef-93e0-e1e2267b57ff`
- Patch: `accepted.patch`
- Projection selected by `auto`: `fuse_mount`
- Fallback reason: none
- Command policy class: `read_only`
- Runtime metrics in review: setup `214ms`, command `25ms`, reconcile `32ms`, cleanup `0ms`, files `56`, bytes `698400`
- Risk findings: none
- Clean-copy `git apply`: passed

## Benchmark Result

`scripts/projection_runtime_benchmark.sh` passed on this machine.

- `materialized-dir`: patch apply passed, metrics emitted.
- `auto`: selected `fuse_mount`, patch apply passed, metrics emitted.
- `fuse-mount`: patch apply passed, metrics emitted.

## Observations

- Runtime metrics are compact enough for review evidence and useful for comparing projection backends.
- `auto` used FUSE successfully and did not record a fallback reason.
- The packet/skill workflow still makes the agent do the right thing without extra coaching.
- The dogfood task remains token-heavy relative to the change size, but no new packet friction appeared.

## Product Takeaways

- Keep runtime metrics diagnostic-only for now.
- Keep `materialized_dir` as default and `auto` as the recommended operator test path.
- Keep VFS command/runtime-owned; the dogfood did not suggest adding manual mount workflow.
- Next readiness work should target repeated/larger dogfood and VFS writeback edge cases, not new agent-facing commands.
