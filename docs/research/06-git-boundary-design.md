# 06 - Legacy Git Adapter Design

## Research Question

How should the system import from and export to Git-era infrastructure without becoming a Git abstraction internally?

## Observed Facts

- Current agents generally produce a branch, commit, patch, or PR because the ecosystem accepts those artifacts.
- Codex, Copilot coding agent, Cursor background agents, Jules, and Devin all use GitHub/PR flows as output or handoff.
- GitButler shows that advanced workspace state can be maintained separately while still producing Git refs.
- `jj` proves that Git-compatible storage and Git export can coexist with a different working model.
- Sapling supports Git interop while offering a different UX and stack workflow.
- These are migration and compatibility signals, not reasons to make Git the internal substrate.

## Sources And Artifacts

- OpenAI Codex intro: https://openai.com/index/introducing-codex/
- GitHub Copilot coding agent docs: https://docs.github.com/copilot/concepts/coding-agent/about-copilot-coding-agent
- Cursor Background Agents: https://docs.cursor.com/background-agents
- Jules running tasks: https://jules.google/docs/running-tasks/
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/jj/docs/git-compatibility.md`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-workspace/src/lib.rs`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/sapling/website/docs/git/sapling-stack.md`

## Product Implications

- The internal product object is `WorkThread`; the native accepted output is `NativePublication`; the legacy export artifact is `LegacyGitArtifact`.
- Git export should support at least one clean commit and later a commit stack/patch series, but native review/publication should not depend on Git.
- Commit messages or PR descriptions should carry durable references to work-thread/evidence records.
- Git export should be reproducible from accepted snapshots.
- A snapshot must be defined as a private/internal state capture, not as a publication object or commit substitute.
- The product should be local-first and distributed without copying Git's commit/branch/remote mental model.
- Internal sync should operate on work threads, source snapshots, evidence, review state, and exports; Git remotes should remain legacy compatibility, mirror, backup, or migration endpoints.

## Snapshots Are Not Commits

Commits and snapshots should have different jobs:

- A commit is a publication object meant for project history.
- A snapshot is a state-capture object inside a work thread.
- A commit should be human-meaningful and reviewable as history.
- A snapshot can be automatic, private, ugly, superseded, abandoned, compared, replayed, or used as evidence.
- A commit crosses the legacy Git adapter boundary.
- A snapshot stays inside Anvics unless accepted work is exported.

The product should not ask humans to review a chain of snapshots. Review should be built from intent, trace, evidence, risk, final diff, and selected intermediate states only when they explain the work. If snapshots acquire branch names, commit messages, merge rituals, and social review semantics, they have become commits under a different name.

## Local And Remote Architecture

The architecture should have three layers:

1. **Local Anvics service**
   - Owns snapshots, workspace views, work threads, change units, agent sessions, evidence, review projections, native publication, and legacy Git export.
   - Gives agents a structured API before they need a filesystem checkout.
   - Can operate offline or privately until the user publishes selected state.

2. **Remote collaboration service**
   - Syncs team-visible work records, review decisions, accepted snapshots, policies, RBAC, audit logs, and evidence retention.
   - Should be understood as the collaboration layer for agent work, not merely a Git object remote.

3. **Legacy Git adapters**
   - Import source state and upstream refs from existing Git hosts.
   - Export accepted work as clean Git commits, branches, patch stacks, or PRs.
   - Preserve links from legacy Git artifacts back to the richer internal work-thread record.

The shorthand is: Git syncs commits; this product syncs work. A native repository should be able to exist, sync, review, and publish without Git. Multi-direction sync matters, but publication should be selective. Raw traces, failed attempts, secret-adjacent logs, speculative work, and abandoned attempts should stay local/private by default.

## MVP Impact

- Define `LegacyGitArtifact` with type, branch/ref, commit IDs, PR URL if present, exported native publication ID, exported snapshot ID, and work-thread reference.
- Support legacy Git export as a single branch/PR first, while keeping native review as the primary product surface.
- Keep stack export as a design target, not v1 requirement.
- Start with a local native service plus GitHub/GitLab import/export; add remote work-thread sync once the local model is useful.

## Open Questions

- Should v1 embed provenance references in commit messages, Git notes, hidden refs, or PR body only?
- Should export preserve agent attempts as commits, or synthesize clean human-grade commits?
- How should the system handle force-push, rebase, and PR edits made outside the product?
- Which state is eligible for automatic sync, and which requires explicit publish/share action?
