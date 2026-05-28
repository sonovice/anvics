# Live Agent Trial

Date: 2026-05-28
Operator: Codex
Target repo: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.FlO4KuUrVX`
Anvics commit: `aec3760`

## Agent A

- Tool: Codex worker agent
- Task: Update `notes.md` with one concise bullet about isolated Anvics workspaces, and update `app.txt` for the overlap trial.
- Packet path: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.FlO4KuUrVX/.anvics/agent-packets/13c8f274-d68c-43ea-9646-ea550d3865a1.md`
- Workspace path: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.FlO4KuUrVX/.anvics/workspaces/90998118-02c1-4313-808b-92e1e898a802/files`
- Finish command: `printf 'notes.md'; sed -n '1,80p' notes.md; printf 'app.txt'; sed -n '1,80p' app.txt`
- Exit code: `0`
- Summary: Agent A updated `notes.md` and `app.txt` for the mixed overlap trial.

## Agent B

- Tool: Codex worker agent
- Task: Update `app.txt` so status clearly says it was changed by Agent B during the overlap trial.
- Packet path: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.FlO4KuUrVX/.anvics/agent-packets/96c6a4bf-5bae-4bfd-9f2a-047040982a41.md`
- Workspace path: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.FlO4KuUrVX/.anvics/workspaces/22f59218-ef29-4e0a-9d68-2254249e6ee3/files`
- Finish command: `sed -n '1,120p' app.txt`
- Exit code: `0`
- Summary: Agent B updated `app.txt` for the deliberate overlap trial.

## Observations

- Packet friction: Both agents followed the packet and stayed inside their assigned workspace paths.
- Workspace mistakes: None observed.
- Evidence quality: Compact summaries worked, but command strings can become awkward when they include multiple shell snippets.
- Review usefulness: The accepted Agent A review showed task, changed paths, evidence, and next commands in one readable markdown projection.
- Conflict or overlap notes: Agent A review correctly reported that Agent B also changed `app.txt`.
- Export result: `scripts/live_agent_trial_verify.sh` created a native publication and exported `accepted.patch`; `git apply` verified the patch against a clean copy.
- Product takeaways: The core loop works. The next friction is reducing manual copying between packet, finish, review, publish, and export.

## Decision

- Accepted thread: `13c8f274-d68c-43ea-9646-ea550d3865a1`
- Review: `eecb9f47-ef8f-469a-9b30-097ddd61021c`
- Publication: `f4f7c1e0-e51b-4d4c-a986-5e54db977c37`
- Legacy patch: `/var/folders/zv/p_qkzppj07n4t10f5c19gp480000gn/T/tmp.FlO4KuUrVX/accepted.patch`
- Follow-up fixes: Add a single command to finish, review, publish, and export an accepted workspace; improve command evidence ergonomics for multi-command verification.

## Follow-Up

MVP 0.4 addresses the main trial friction with `anvics agent accept`, a single operator command that attaches evidence, snapshots the workspace, creates review/publication records, exports the legacy patch, and prints the audit ids.
