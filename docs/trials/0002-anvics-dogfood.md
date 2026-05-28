# Dogfood Trial 0002 - Anvics On Anvics

Date: 2026-05-28
Operator: Simon + Codex
Anvics source commit: `d4a7a95`
Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ykTJLLi9sc`
Trial manifest: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ykTJLLi9sc/.anvics/dogfood-trial-manifest.env`

## Agent A

- Tool: `codex exec`
- Thread: `742f2718-be28-41e9-9004-46aa4c1fb1e1`
- Workspace: `58abd5eb-9f2b-47fe-9ac1-af94b039b1f0`
- Packet: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ykTJLLi9sc/.anvics/agent-packets/742f2718-be28-41e9-9004-46aa4c1fb1e1.md`
- Task: Add one small dogfood clarity note to `docs/AGENT_TEST_RUNBOOK.md`.
- Verification command: `rg -n 'dogfood|Dogfood|trial' docs/AGENT_TEST_RUNBOOK.md docs/trials`
- Exit code: 0
- Summary: Read the Anvics skill and packet, entered the workspace, and added a sentence telling operators to record each agent's coordination status note before acceptance.
- Artifact: none

## Agent B

- Tool: `codex exec`
- Thread: `987ac48e-1843-4c95-b75c-5a8706aff6ee`
- Workspace: `7aa198ac-9bd1-4040-a18c-362305a9c68a`
- Packet: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ykTJLLi9sc/.anvics/agent-packets/987ac48e-1843-4c95-b75c-5a8706aff6ee.md`
- Task: Add a small CLI/test improvement around `agent status --workspace`.
- Verification command: `cargo test -p anvics-cli --test cli workspace_show_and_agent_status_by_workspace_report_overlay_state`
- Exit code: 0
- Summary: Read the Anvics skill and packet, entered the workspace, and tightened the test so `agent status --workspace` must include the workspace id and workspace path.
- Artifact: command stdout/stderr captured by Anvics command evidence under `.anvics/artifacts/commands/09968d1a-d819-465a-a954-ddd71771dad3/`

## Friction

- Packet friction: The first attempted run used packet instructions only. The corrected run explicitly told each Codex CLI agent to read `skills/anvics-skill/SKILL.md` first. Future packets should include the skill path and an explicit "read this skill first" step.
- Workspace/path mistakes: Both agents stayed inside their materialized workspaces. Both still tried or wanted Git-style diff/status checks at least once; workspaces are intentionally not Git checkouts, so agents need an Anvics-native diff/status command for changed files.
- Coordination usefulness: `agent enter` showed the other active workspace immediately. Before either workspace had a snapshot, coordination correctly reported "unknown changes possible until snapshot/finish"; after Agent A finished, later review showed no path overlap for Agent B.
- Command/evidence friction: Anvics-run verification worked for the accepted workspace and stored command artifacts by reference. The command-worker path is now good enough for this style of trial.
- Risk findings: Publication was blocked by 15 secret-risk findings in existing intentional test fixtures in `crates/anvics-cli/tests/cli.rs`. The override path worked, but scanning the whole changed source file caused noisy findings unrelated to Agent B's two-line test assertion change.
- Review usefulness: The review captured changed paths, evidence, no-overlap status, risk findings, and next commands. It was enough to decide and publish.
- Patch export result: `legacy git export` wrote `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ykTJLLi9sc/accepted.patch`.

## Result

- Accepted thread: `987ac48e-1843-4c95-b75c-5a8706aff6ee`
- Accepted workspace: `7aa198ac-9bd1-4040-a18c-362305a9c68a`
- Review: `7918d2c8-9f39-4363-9930-6a6bfb24803c`
- Publication: `13915d25-3f2a-477d-a0a5-746a0fba9286`
- Legacy patch: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ykTJLLi9sc/accepted.patch`
- `git apply` result: passed on clean copy `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.EgYpBBTxMC`

## Product Takeaways

- Keep: packet plus skill plus `agent enter` was understandable to real Codex CLI agents; isolated workspaces worked; command-worker evidence and patch export completed the loop.
- Fix next: make the dogfood packet explicitly reference the Anvics skill; add an Anvics-native "what changed in my workspace?" command so agents do not reach for `git diff`; reduce secret-risk noise by scanning changed hunks or distinguishing pre-existing fixture risks from newly introduced risks.
- Defer: VFS, semantic merge, remote sync, and UI still should wait until the current CLI loop is smoother.
