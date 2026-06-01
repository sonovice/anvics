# Dogfood Trial 0010 - Evidence Supersede Recovery

Date: 2026-06-01
Source state: `d0e948a` plus private-beta-readiness working tree
Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ApSP728VGN`
Agent tool: `codex exec v0.135.0`

## Goal

Validate the MVP 0.35 recovery path with a real Codex agent: create useful work, intentionally block acceptance with obsolete risky evidence, supersede that evidence, rerun clean acceptance, and verify the exported patch applies.

## Agent Run

- Thread: `9803f3b9-3e64-4121-b063-ef7464ebcab1`
- Workspace: `10821de6-8b92-4228-b756-9f436a6d15d4`
- Task: update `README.md` so status mentions evidence supersede dogfood.
- Skill read: yes.
- Context pack used: no; the packet had enough context.
- Workspace diff used: yes.
- Checkpoint: `58d49285-29df-4d67-88f4-db303e0a0e12`.
- Coordination result: `related_work: none`, `potential_clashes: none`.
- Verification reported by agent: `rg -n "^status: evidence supersede dogfood$" README.md`, exit code 0.
- Change: one-line update from `status: base` to `status: evidence supersede dogfood`.

## Recovery Flow

1. Operator intentionally ran `agent accept` with a verification command that emitted an OpenAI-style fixture secret.
2. Publication was blocked by secret-risk findings, and the recovery hint printed an exact `evidence supersede 0254c57b-0689-43d9-96e4-d9200c008567 --reason "<audited reason>"` command.
3. Operator ran `risk list --review <review-id>` and confirmed the finding belonged to command stdout evidence.
4. Operator superseded evidence `0254c57b-0689-43d9-96e4-d9200c008567` with reason `Obsolete leaky verification from MVP 0.36 dogfood`.
5. Operator reran `agent accept` with safe verification: `rg -n '^status: evidence supersede dogfood$' README.md`.
6. Publication succeeded without `--allow-secret-risk`.

## Result

- Final review: `c1f383e7-c69a-428c-be86-ad0206ea8aea`
- Publication: `38541d5e-8cdb-4e0c-a433-c84ba217f731`
- Patch: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.ApSP728VGN/accepted.patch`
- Patch verification: applied cleanly to a clean copy with `git apply`.

## Findings

- The evidence-supersede recovery path is understandable when the recovery hint prints an exact evidence id.
- The new exact supersede hint is a meaningful UX improvement over the placeholder command from MVP 0.35.
- The packet still contained bare `anvics` because this manual setup did not set `ANVICS_AGENT_COMMAND` before `agent prepare`; the outer launch prompt compensated by explicitly giving Codex the repo-local Cargo prefix. Installed-binary beta users should not hit this.

## Recommendation

Continue toward private beta readiness. Evidence superseding is good enough for technical beta users, but first-run docs should keep recommending an installed `anvics` binary so packets and context packs do not need repo-local command overrides.
