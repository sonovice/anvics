# Dogfood Trial 0003 - Skill-First Packet Validation

Date: 2026-05-28
Operator: Simon + Codex
Anvics source commit: `0371f68-dirty`
Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb`
Trial manifest: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb/.anvics/dogfood-trial-manifest.env`

## Trial Gate

- Agent A read the Anvics skill from the packet/prompt: yes
- Agent B read the Anvics skill from the packet/prompt: yes
- Agent A used `workspace diff` instead of Git status/diff: yes
- Agent B used `workspace diff` instead of Git status/diff: yes
- Agent A reported coordination status: yes
- Agent B reported coordination status: yes
- Any manual correction beyond the generated prompt: no; Agent B did run the operator acceptance template itself.

## Agent A

- Tool/version: `codex exec` v0.134.0
- Thread: `d8ade69c-c9ca-4ccc-b519-869af5583c44`
- Workspace: `35b68266-5559-420c-be47-91ef7347ccbf`
- Packet: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb/.anvics/agent-packets/d8ade69c-c9ca-4ccc-b519-869af5583c44.md`
- Task: Improve one small docs section in `docs/AGENT_TEST_RUNBOOK.md` for dogfood clarity.
- Changed files: `docs/AGENT_TEST_RUNBOOK.md`
- Verification command: `rg -n 'dogfood|Dogfood|trial' docs/AGENT_TEST_RUNBOOK.md docs/trials`
- Exit code: 0
- Summary: Added one dogfood runbook sentence asking agents to include final coordination status with verification.
- Artifact: `/private/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb/.anvics/reviews/57456ff8-03c2-4a6e-9da8-182d467f5112.md`
- Notes on skill/packet friction: no skill-read friction; final coordination reported Agent B as related work with unknown freshness until finish.

## Agent B

- Tool/version: `codex exec` v0.134.0
- Thread: `1d16b02b-bff5-49c3-b203-9a8648b4e3aa`
- Workspace: `f981f91c-e30d-4200-ae7f-a274845600d9`
- Packet: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb/.anvics/agent-packets/1d16b02b-bff5-49c3-b203-9a8648b4e3aa.md`
- Task: Make one small CLI/test improvement around agent status or workspace UX.
- Changed files: `crates/anvics-cli/tests/cli.rs`
- Verification command: `anvics --repo '/private/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb' agent accept --workspace f981f91c-e30d-4200-ae7f-a274845600d9 --run-label "cli targeted test" --run-summary "Targeted CLI workspace status test passed" -- cargo test -p anvics-cli --test cli workspace_show_and_agent_status_by_workspace_report_overlay_state`
- Exit code: 0
- Summary: Strengthened the CLI test so `agent status --workspace` must include the resolved workspace id and workspace path.
- Artifact: `/private/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb/.anvics/reviews/c27072c8-236f-4b67-994b-0aa86da42186.md`
- Notes on skill/packet friction: no skill-read or diff friction; Agent B interpreted the operator acceptance command template as runnable by the agent.

## Operator Flow

- Non-accepted workspace finish command: Agent A ran `agent finish` with external evidence.
- Accepted workspace accept command: Agent B ran `agent accept --run-label "cli targeted test" ...` and created the publication.
- Review: `c27072c8-236f-4b67-994b-0aa86da42186`
- Risk scan/list result: `No risk findings`
- Publication: `92c10d80-4a6e-4a0c-863a-087bbea31de0`
- Legacy patch: `/private/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.u3VqArjDAb/accepted.patch`
- `git apply` result: passed on clean copy `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.IqL2Co82kh`

## Friction

- Packet friction: skill-first instruction worked; both agents read the skill without extra correction.
- Workspace/path mistakes: none observed.
- Git fallback attempts: none observed.
- Coordination usefulness: useful; the second agent saw known changed paths from the first agent after finish and no overlap.
- Command/evidence friction: mostly smooth; Agent B used Anvics-run evidence and produced review/publication/patch in one command.
- Risk findings: no findings; MVP 0.13 hunk-level source scanning avoided the previous fixture-secret noise.
- Review usefulness: review included changed path, Anvics-run evidence, no-overlap status, and no secret-risk findings.
- Patch export result: exported patch applied cleanly.

## Result

- Gate result: passed with one follow-up on packet wording.
- Accepted thread: `1d16b02b-bff5-49c3-b203-9a8648b4e3aa`
- Accepted workspace: `f981f91c-e30d-4200-ae7f-a274845600d9`
- Product decision: current CLI loop is good enough to start the next architecture spike, but packet wording should clarify which commands are agent-run versus operator-run.

## Product Takeaways

- Keep: packet skill path, `agent enter`, `workspace diff`, coordination status, command-worker evidence, risk scan, review, publish, and patch export.
- Fix next: clarify packet command ownership so agents do not run operator acceptance unless explicitly asked.
- Defer: VFS, semantic merge, remote sync, and UI remain outside this validation slice.

Follow-up: packet wording was tightened after this run to separate agent-run commands from operator-run acceptance/publish/export commands.
