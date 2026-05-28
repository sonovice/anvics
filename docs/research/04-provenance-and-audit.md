# 04 - Provenance And Audit

## Research Question

What provenance must be recorded for agent work to be accountable, reviewable, and enterprise-safe?

## Observed Facts

- Agent provenance spans more than code: prompt/task, model, tools, commands, files read/written, environment, permissions, policy, tests, reviewer decisions, native publication, and optional legacy Git artifact.
- GitButler's `but-agentlog` already parses Codex/Claude transcripts into records, classifies tool calls, redacts data, and projects evidence into Git metadata.
- OpenTelemetry's trace/span/event model maps naturally onto agent sessions and tool calls.
- Claude Code hooks and MCP tooling show that agents are extensible and policy-sensitive; audit needs to record tool use and lifecycle boundaries.
- Devin exposes enterprise session, audit, PR review, permission, and usage APIs, suggesting audit/provenance is already a buyer concern.

## Sources And Artifacts

- OpenTelemetry traces: https://opentelemetry.io/docs/concepts/signals/traces/
- Claude Code hooks: https://docs.anthropic.com/en/docs/claude-code/hooks
- Devin audit logs: https://docs.devin.ai/api-reference/v3/audit-logs/enterprise-audit-logs.md
- Devin session insights: https://docs.devin.ai/api-reference/v3/sessions/get-enterprise-session-insights.md
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-agentlog/src/transcript.rs`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-agentlog/src/gitmeta/write.rs`

## Product Implications

- Provenance should be a first-class source-control object, not a PR-description appendix.
- Store raw records separately from redacted reviewer projections.
- Use hashes/references for sensitive or large artifacts.
- Make every native publication and legacy Git artifact traceable back to a work thread and agent session.

## MVP Impact

- Define `AgentSession` trace records using an OTel-like shape: event name, timestamp, actor, tool, input reference, output reference, status.
- Define retention tiers: raw transcript, redacted transcript, evidence digest, public PR summary.
- Store reviewer decisions as provenance events.

## Open Questions

- What raw data is too sensitive to store by default?
- Should provenance live in the source-control store, external object storage, or both?
- Should provenance hashes be embedded in Git commit messages, Git notes, refs, or PR metadata?
