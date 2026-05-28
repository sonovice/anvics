# 03 - Agent Failure Taxonomy

## Research Question

How do coding agents fail in real projects, and which failures should source control detect or make recoverable?

## Observed Facts

- Common failures include stale context, wrong scope, missing tests, broken CI, hallucinated APIs, overbroad refactors, bad PR packaging, and unresolved merge/rebase issues.
- Studies of failed agentic PRs analyze merge outcomes, task types, CI/build results, code changes, and review dynamics.
- Task type matters. Documentation and routine changes appear more likely to be accepted than broad feature, bug, performance, or build-system changes.
- Agent failures are often not pure model failures. They are workflow failures: poor task boundaries, weak environment evidence, noisy history, and insufficient review context.
- Pijul's explicit conflict and "zombie" context is a useful analogy for stale/deleted context failures.

## Sources And Artifacts

- Where Do AI Coding Agents Fail?: https://arxiv.org/abs/2601.15195
- Comparing AI Coding Agents: https://arxiv.org/abs/2602.08915
- AIDev dataset paper: https://arxiv.org/abs/2602.09185
- Do AI Coding Agents Log Like Humans?: https://arxiv.org/abs/2604.09409
- Local artifact: `/Users/simon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libpijul-1.0.0-beta.11/src/output/output.rs`
- Local artifact: `/Users/simon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libpijul-1.0.0-beta.11/src/missing_context.rs`

## Product Implications

- Classify work threads by task type and risk, not just changed files.
- Treat stale context as a first-class state: base moved, deleted context edited, dependency changed, or target symbol disappeared.
- Track unresolved uncertainty explicitly in review.
- Make failed attempts useful evidence, not disposable noise.

## MVP Impact

- Add `Attempt.status` values such as `completed`, `failed_tests`, `stale_base`, `blocked_policy`, `superseded`, and `abandoned`.
- Add risk flags for broad diff, missing tests, changed generated files, stale base, and repeated retries.
- Keep failed attempts linked to the work thread for reviewer context and future replay.

## Open Questions

- Which failure categories can be inferred cheaply from Git/test metadata in v1?
- Should the product block export when risk/evidence thresholds are not met?
- How aggressively should abandoned attempts be preserved versus summarized?

