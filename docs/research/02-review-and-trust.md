# 02 - Review And Trust

## Research Question

What evidence makes an agent-generated change reviewable enough for a human to accept?

## Observed Facts

- A diff alone is too thin. Agent work needs intent, plan, test evidence, tool output, environment, and risk summary.
- Existing agents expose partial review surfaces: PRs, branch diffs, task summaries, terminal logs, screenshots, CI status, or session pages.
- Copilot coding agent, Jules, Codex, Cursor, and Windsurf all converge on "agent does work elsewhere, human reviews a final artifact."
- Empirical PR studies suggest acceptance varies by task type and review dynamics; a generic review UX is likely too blunt.
- Review needs to separate code result from agent behavior. The human wants to know both "what changed?" and "how did this run behave?"

## Sources And Artifacts

- GitHub Copilot coding agent docs: https://docs.github.com/copilot/concepts/coding-agent/about-copilot-coding-agent
- Jules review/code docs: https://jules.google/docs/code/
- Cursor docs: https://cursor.com/docs
- Windsurf Quick Review: https://docs.windsurf.com/windsurf/quick-review.md
- Comparing AI Coding Agents: https://arxiv.org/abs/2602.08915
- Why Are Agentic Pull Requests Merged or Rejected?: https://arxiv.org/abs/2605.22534
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-agentlog/src/projection.rs`

## Product Implications

- The review object should be generated from the work thread, not from the Git diff alone.
- Review should classify task type, risk, touched area, verification strength, and unresolved uncertainty.
- Humans need drill-down: summary first, then trace/evidence if something smells wrong.
- Review should support rerun, revise, split, squash, handoff, and accept.

## MVP Impact

- Define `ReviewProjection` with intent, final diff, summary, tests, risks, provenance, native publication mapping, and optional legacy Git export mapping.
- Define `EvidenceBundle` with commands, CI, screenshots, browser traces, logs, and static analysis.
- Add a "confidence is evidence-backed" rule: no green label without pointing to evidence.

## Open Questions

- What is the minimum trusted review bundle for the first wedge?
- Should reviewers see raw traces by default, or only risk-filtered trace excerpts?
- Should review projections be stored as durable source-control objects or regenerated from immutable evidence?
