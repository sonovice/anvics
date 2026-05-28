# Research Appendix

This folder holds the detailed research behind Anvics and `../GIT_ALTERNATIVE.md`.

The main thesis should stay concise and product-oriented. These notes can be messier and more evidence-heavy.

## Tracks

| # | Topic | File | Status |
|---|---|---|---|
| 01 | Agent workflow reality | [01-agent-workflow-reality.md](01-agent-workflow-reality.md) | Draft synthesis |
| 02 | Review and trust | [02-review-and-trust.md](02-review-and-trust.md) | Draft synthesis |
| 03 | Agent failure taxonomy | [03-agent-failure-taxonomy.md](03-agent-failure-taxonomy.md) | Draft synthesis |
| 04 | Provenance and audit | [04-provenance-and-audit.md](04-provenance-and-audit.md) | Draft synthesis |
| 05 | Privacy and visibility | [05-privacy-and-visibility.md](05-privacy-and-visibility.md) | Draft synthesis |
| 06 | Legacy Git adapter design | [06-git-boundary-design.md](06-git-boundary-design.md) | Draft synthesis |
| 07 | Hunk and change ownership | [07-hunk-change-ownership.md](07-hunk-change-ownership.md) | Draft synthesis |
| 08 | Environment reproducibility | [08-environment-reproducibility.md](08-environment-reproducibility.md) | Draft synthesis |
| 09 | Agent orchestration boundary | [09-agent-orchestration-boundary.md](09-agent-orchestration-boundary.md) | Draft synthesis |
| 10 | Enterprise controls | [10-enterprise-controls.md](10-enterprise-controls.md) | Draft synthesis |
| 11 | Source-control API | [11-source-control-api.md](11-source-control-api.md) | Draft synthesis |
| 12 | Rust architecture | [12-rust-architecture.md](12-rust-architecture.md) | Draft synthesis |
| 13 | Repository configuration | [13-repo-config.md](13-repo-config.md) | Draft synthesis |
| 14 | Agentvfs deep dive | [14-agentvfs-deep-dive.md](14-agentvfs-deep-dive.md) | Draft synthesis |

## Current Synthesis

Anvics should be a neutral source-control substrate for agent work, not a new agent runtime on day one and not a Git abstraction layer. The substrate should expose API-native source operations, with filesystem checkout as an optional materialized view, and ship an agent skill/instruction bundle so agents learn the new workflow instead of recreating Git habits. The immediate worktree replacement is overlay-backed `WorkspaceView`s: many agents can work over the same codebase without full repo copies or manual worktree cleanup.

Architecturally, Anvics should be local-first and distributed without copying Git's commit/branch/remote mental model. The local Anvics service owns source snapshots, work threads, evidence, review projections, and native publication. The remote Anvics service syncs team-visible work records, policies, review state, audit data, and accepted snapshots. GitHub/GitLab are legacy import/export, mirror, backup, and migration endpoints.

Implementation should use Rust for the local daemon, storage, workspace overlays, projection backends, indexing, sync, API, CLI, and legacy Git adapter. Core Anvics crates must remain Git-free; Git belongs only in `anvics-legacy-git`. The agentvfs deep dive reframes the next VFS work as a proxy-mounted workspace runtime: mounts are compatibility projections for tools, while command/proxy execution should own mount lifecycle and changed-file summaries when possible. Materialized directories remain valid for MVP and fallback; VFS is the low-copy backend for large repos and many concurrent agents. MVP 0 should prove the local work-thread/review loop before implementing the full platform roadmap.

Repository understanding should come from optional source-tree `anvics.toml`, plus heuristics/plugins and agent suggestions. Private/org policy lives in Anvics metadata, not in source-tree config.

Recommended MVP 0 wedge:

1. Create or import a local repository into Anvics state.
2. Run two isolated work threads with simple overlay/materialized workspaces.
3. Capture snapshots and compact evidence from external or simulated agents.
4. Produce review projections that beat raw transcript-plus-diff review.
5. Create a native publication and export a legacy Git artifact only after acceptance.

Eventual model:

`Repository`, `SourceSnapshot`, `WorkThread`, `WorkspaceView`, `ChangeUnit`, `AgentSession`, `Attempt`, `EvidenceBundle`, `InstructionBundle`, `EnvironmentSnapshot`, `ReviewProjection`, `NativePublication`, and `LegacyGitArtifact` remain the broader vocabulary, but MVP 0 should not force agents or implementers to handle all of them directly.

Related product artifact:

- [Draft Anvics Skill](../../skills/anvics-skill/SKILL.md)
- [MVP 0](../MVP_0.md)
- [Anvics Decision Log](../ANVICS_DECISIONS.md)
- [Platform Roadmap](../PLATFORM_ROADMAP.md)

## Research Standard

Each note should keep these sections:

- Observed facts
- Sources and artifacts
- Product implications
- MVP impact
- Open questions
