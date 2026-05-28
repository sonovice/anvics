# 09 - Agent Orchestration Boundary

## Research Question

Should the product run agents, orchestrate external agents, or begin as a neutral source-control substrate that records agent work?

## Observed Facts

- The agent ecosystem is fragmented across Codex, Claude Code, Copilot, Cursor, Windsurf, Devin, Jules, and likely more.
- Each agent already has its own runtime, permission model, UI, trace format, and Git handoff path.
- GitButler's `but-agentlog` shows that ingesting transcripts from existing agents can create value without owning the agent runtime.
- Windsurf and Cursor demonstrate IDE-native orchestration and parallel local/background agents.
- Devin and Jules demonstrate cloud-session APIs and scheduled/suggested tasks.

## Sources And Artifacts

- Claude Code overview: https://code.claude.com/docs/en/overview
- Claude Code GitHub Actions: https://code.claude.com/docs/en/github-actions
- Windsurf Agent Command Center: https://docs.windsurf.com/windsurf/agent-command-center.md
- Windsurf Worktrees: https://docs.windsurf.com/windsurf/cascade/worktrees.md
- Devin create session API: https://docs.devin.ai/api-reference/v1/sessions/create-a-new-devin-session.md
- Jules API overview: https://jules.google/docs/api/reference/overview/
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-agentlog/src/capture.rs`

## Product Implications

- Starting as a neutral substrate avoids competing with every agent runtime immediately.
- The first integration should make existing agent work more reviewable and governable.
- The product can later add its own runner once it understands workflow and evidence requirements.
- API-first design matters because agents will be launched from many surfaces.

## MVP Impact

- Recommended v1 posture: record and normalize external agent work; do not own execution yet.
- First integrations: local CLI transcript capture plus GitHub App/webhook for branch/PR mapping.
- Expose an ingestion API for `AgentSession`, trace events, evidence, native publications, and legacy Git artifacts.

## Open Questions

- Which transcript formats should be first-class in v1?
- Should the product provide a lightweight runner for demos, even if production starts as ingestion?
- How much orchestration is needed for "rerun" if the original agent/runtime is external?
