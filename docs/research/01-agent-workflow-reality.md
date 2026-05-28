# 01 - Agent Workflow Reality

## Research Question

How are coding agents actually used today, and what source-control workflow is implied by that use?

## Observed Facts

- Agent work starts from many surfaces: IDE chat, terminal, GitHub issue, PR review, Slack, API, scheduled task, or web UI.
- The dominant execution shape is task -> isolated environment -> plan -> edit -> run tools/tests -> produce diff/branch/PR -> human review.
- Current products already assume parallelism. Codex runs tasks in separate cloud environments; Cursor and Windsurf expose background agents/worktrees/spaces; Devin and Jules expose long-running sessions; Copilot coding agent works in GitHub-hosted environments.
- Git worktrees solve the isolation problem awkwardly: users still manage directories, stale bases, cleanup, and duplicated materialization. Anvics should make parallel work a substrate feature instead.
- The human workflow is fleet supervision, not single-agent pair programming. Users need to scan many attempts, compare outcomes, interrupt bad runs, rerun from better bases, and accept only clean outputs.
- Git branches and PRs are the artifact format, but the real work object is the task/session with trace, environment, instructions, and outcome.

## Sources And Artifacts

- OpenAI Codex: https://openai.com/index/introducing-codex/
- GitHub Copilot coding agent: https://docs.github.com/copilot/concepts/coding-agent/about-copilot-coding-agent
- Cursor Background Agents: https://docs.cursor.com/background-agents
- Windsurf Agent Command Center: https://docs.windsurf.com/windsurf/agent-command-center.md
- Jules running tasks: https://jules.google/docs/running-tasks/
- Devin sessions API: https://docs.devin.ai/api-reference/v3/sessions/get-enterprise-session.md
- AIDev dataset paper: https://arxiv.org/abs/2602.09185

## Product Implications

- Model the task/session as the primary source-control object.
- Support multiple entrypoints: UI, CLI, GitHub App, webhook, and API.
- Make parallel work visible through a smartlog-like work graph, not a branch list.
- Give every agent an isolated `WorkspaceView` overlay on shared source data rather than a full repo copy or manual worktree.
- Preserve handoff context: what was asked, what base was used, what the agent tried, what it changed, and what remains uncertain.

## MVP Impact

- Create `WorkThread` as the task-level object.
- Create `AgentSession` for every external or internal run.
- Capture base Git commit, source surface, task prompt, agent identity, status, output artifact, and trace references.
- Start with neutral ingestion of external agents before building a full agent runner.

## Open Questions

- Which ingestion should be first: local transcript capture, GitHub App, IDE extension, webhook API, or cloud sandbox runner?
- Does the first UI optimize for one developer supervising many runs, or a team supervising shared agent work?
- How much live progress is necessary for v1 versus post-run review?
