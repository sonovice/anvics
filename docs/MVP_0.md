# Anvics MVP 0

MVP 0 is the smallest proof that Anvics is useful for agent-native source control. It should prove the work-thread/review loop before building the full platform roadmap.

## Goal

Show that external coding agents can work in isolated Anvics work threads, produce compact evidence, and give a human a better review object than a raw Git diff or noisy commit history.

MVP 0 should make Git optional for the agent's working model while still exporting a commit/pushable artifact when the outside world needs one.

## Included

- Local native repository state with Anvics-owned snapshots.
- Isolated work threads with simple workspace overlays or materialized workspace directories.
- Snapshot capture at meaningful checkpoints.
- Compact evidence capture for commands, tests, and uncertainty.
- Review projection with intent, final diff, key evidence, risks, and unresolved questions.
- Native publication object for accepted work.
- Optional legacy Git export for accepted native publication.
- Minimal agent skill/instructions that teach the workflow without exposing the whole internal vocabulary.

## Explicitly Excluded

- Native team relay or multi-machine sync.
- Production-quality Linux/macOS VFS.
- Windows VFS.
- Full command-worker mediation.
- MCP adapter completeness.
- Semantic merge or automated conflict resolution.
- Structured multi-agent conflict contracts.
- Enterprise RBAC, retention, cost controls, or audit dashboards.
- Full web/desktop forge UI.
- Rich symbol/package/invariant `ChangeUnit` extraction beyond path/hunk plus generic tags.

## MVP 0 Flow

1. Initialize or import a repository into Anvics local state.
2. Create two work threads from two tasks or prompts.
3. Give each thread an isolated workspace view, using a materialized backend if VFS is not ready.
4. Let external or simulated agents edit files and run commands.
5. Capture snapshots and compact command evidence.
6. Detect changed paths/hunks and flag overlap as a reviewable conflict note.
7. Generate review projections for each thread.
8. Let the human accept one thread, reject one thread, or combine changes manually.
9. Create a native publication from the accepted snapshot.
10. Export a legacy Git artifact only after native acceptance.

## Evidence Budget

MVP 0 must avoid creating token-heavy review records.

- Store raw logs, screenshots, traces, and reports as artifacts or references.
- Put only command names, exit status, duration, and short summaries into review projections.
- Quote key excerpts only when they explain a failure, risk, or important verification result.
- Do not dump full agent transcripts into durable review objects by default.
- Preserve raw private traces behind retention and visibility rules, but review the summarized projection first.

## Conflict Handling

MVP 0 conflict handling is intentionally simple.

- Detect same-path and same-hunk overlap between active work threads.
- Attach overlap information to the review projection.
- Let agents or humans resolve by editing a workspace normally.
- Do not auto-apply semantic resolutions.
- Do not require `ConflictSession`, `ResolutionThread`, or resolution contracts for the first proof.

## VFS Stance

VFS remains central to the long-term Anvics thesis: the filesystem should be a projection, not the canonical source model.

For MVP 0, the implementation can use a materialized workspace backend so the product loop is not blocked on FUSE/macFUSE maturity. Linux and macOS VFS prototypes should run in parallel and graduate into the roadmap once the MVP proves the value of Anvics work threads.

## Acceptance Criteria

- A new engineer can understand the first build target from this file alone.
- Two isolated work threads can modify the same repo without Git worktrees.
- Review output is shorter and more useful than raw transcript plus diff.
- Raw evidence is stored or linked without flooding the review projection.
- Git export is derived from accepted native state, not used as the internal source of truth.
- The demo still proves Anvics is not a Git wrapper.

## Next Roadmap After MVP 0

If MVP 0 works, continue into [`PLATFORM_ROADMAP.md`](PLATFORM_ROADMAP.md):

- Harden workspace overlays.
- Promote VFS from prototype to primary workspace backend.
- Add command-worker mediation.
- Enrich `ChangeUnit` extraction.
- Build the review UI.
- Add structured conflict coordination.
- Add native team relay/sync.
