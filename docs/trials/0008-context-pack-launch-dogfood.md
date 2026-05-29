# Dogfood Trial 0008 - Context Pack Launch Guidance

Date: 2026-05-29

Source commit: `8d3fe6f`

Disposable target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.vUX7cP11Qn`

## Goal

Validate that generated packets and Codex launch prompts are enough for real `codex exec` agents to read the Anvics skill, refresh context packs, enter sessions, use `workspace diff`, and complete small work without extra coaching.

## Result

The trial did not reach editing. Both Codex agents read the packet and skill, then blocked before making changes.

Observed blockers:

- `anvics` was not installed on `PATH` in the launched agent shells, so `agent enter`, `agent context-pack`, `workspace diff`, and `coordination status` failed with exit code 127.
- The default `codex exec` launch used a read-only sandbox, so patch application and Cargo verification could not write to the workspace.
- The agents correctly avoided Git fallback and reported the Anvics/tooling blockers instead of creating Git branches, worktrees, commits, or ad hoc diffs.

## Agent A

- Tool: `codex exec`
- Task: improve one small docs section in `docs/AGENT_TEST_RUNBOOK.md`.
- Read Anvics skill: yes.
- Refreshed context pack: attempted, blocked by missing `anvics`.
- Used `workspace diff`: attempted, blocked by missing `anvics`.
- Coordination status: attempted, blocked by missing `anvics`.
- Verification: `rg -n 'dogfood|Dogfood|trial' docs/AGENT_TEST_RUNBOOK.md docs/trials`, exit code 0.
- Final status: no changes made.

## Agent B

- Tool: `codex exec`
- Task: make a small CLI/test improvement in `crates/anvics-cli/tests/cli.rs`.
- Read Anvics skill: yes.
- Refreshed context pack: attempted, blocked by missing `anvics`.
- Used `workspace diff`: attempted, blocked by missing `anvics`.
- Coordination status: attempted, blocked by missing `anvics`.
- Edit: attempted, blocked by read-only sandbox.
- Verification: `cargo test -p anvics-cli --test cli workspace_show_and_agent_status_by_workspace_report_overlay_state`, exit code 101 because Cargo could not write in the read-only workspace.
- Final status: no changes made.

## Product Takeaways

- Packet/skill wiring worked: both agents read the right instructions and avoided Git reflexes.
- Launch guidance must be operational, not just conceptual. Packets need an explicit Anvics command prefix so dogfood can use a repo-local `cargo run ... --manifest-path ... --` prefix when `anvics` is not globally installed.
- Codex launch guidance must state that the agent needs write access to the disposable workspace. The generated command should stay conservative, but the operator must choose a write-enabled Codex sandbox mode for real editing.
- Crash/interruption recovery is now clearly relevant: if an agent exits, crashes, or is killed after editing but before `agent finish`/`agent accept`, Anvics needs a way to recover changed workspace progress without relying on the agent's final report.

## Follow-Up

- Add `ANVICS_AGENT_COMMAND` support to generated packets and launch prompts.
- Update dogfood preparation to set `ANVICS_AGENT_COMMAND` to the repo-local Cargo invocation.
- Update runbooks/skill wording around Anvics command prefixes and write-enabled Codex launches.
- Plan MVP 0.34 around explicit crash-progress salvage, such as checkpoint/recover/status commands over existing workspaces.
