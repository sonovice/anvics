# Dogfood Trial 0006 - VFS Writeback Edge Cases

Date: 2026-05-29
Operator: Simon + Codex
Anvics source commit: `972b97a` plus MVP 0.20 working-tree changes
Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.mmpGGlReFd`

## Trial Goal

Validate the MVP 0.20 VFS writeback hardening with one real Codex CLI agent and operator acceptance through `agent accept --projection auto`.

## Agent Run

- Tool/version: `codex-cli 0.134.0`
- Thread: `bda30cab-3368-41ac-a79c-c6c80c3afbc1`
- Workspace: `6ae353f9-409e-4ac9-8b17-0292437222b0`
- Packet: `.anvics/agent-packets/bda30cab-3368-41ac-a79c-c6c80c3afbc1.md`
- Task: add one README sentence about opt-in VFS projection writeback coverage.
- Changed file: `README.md`
- Agent verification: `rg -n "Opt-in VFS projection now covers temp-file rename, directory rename, truncate, append, and nested delete writeback patterns\\." README.md`
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
  --run-label "vfs writeback dogfood verify" \
  --run-summary "Verified README VFS writeback dogfood change through auto projection." \
  --projection auto \
  --output "$TARGET_REPO/accepted-020.patch" \
  -- rg -n "Opt-in VFS projection now covers temp-file rename, directory rename, truncate, append, and nested delete writeback patterns\\." README.md
```

Result:

- Review: `1f7ffd17-6faf-4879-8483-bc12079f9a7f`
- Publication: `cf397c16-d7cf-44fd-b098-b9859a3f701f`
- Patch: `accepted-020.patch`
- Projection selected by `auto`: `fuse_mount`
- Fallback reason: none
- Command policy class: `read_only`
- Runtime metrics in review: setup `220ms`, command `25ms`, reconcile `33ms`, cleanup `0ms`, files `57`, bytes `716455`
- Risk findings: none
- Clean-copy `git apply`: passed

## Edge-Case Benchmark

`scripts/projection_runtime_benchmark.sh` passed after being expanded to cover temp-file rename, file rename, directory rename with children, truncate, append, nested create/delete, and delete.

- `materialized-dir`: patch apply passed, metrics emitted.
- `auto`: selected `fuse_mount`, patch apply passed, metrics emitted.
- `fuse-mount`: patch apply passed, metrics emitted.

## Observations

- The packet and skill still guide a real Codex CLI agent without extra coaching.
- `workspace diff` remains enough for the agent to understand its changes without Git.
- `auto` used FUSE successfully and did not record a fallback reason.
- Runtime-owned VFS remains the right boundary; no manual mount workflow was needed.
- The VFS hardening reduced the biggest editor/tool writeback risk without expanding the agent-facing surface.

## Product Takeaways

- Keep `materialized_dir` as the default and `auto` as the operator test path.
- Keep renames path-level for now: delete old path, add new path.
- Treat flush/release/fsync as compatibility acknowledgements until Anvics has a production crash-recovery story.
- Next work should continue from observed runtime reliability needs, not add a manual `workspace mount` command yet.
