# Anvics Decision Log

This is the compact checkpoint of the grilled Anvics design. It captures the decisions as if the planning session were done, so future work can continue without relying on chat context.

## Product Thesis

- Product name: **Anvics**.
- Meaning: agent-native VCS, with pronounceability added.
- Anvics is a native source-control substrate for AI-agent software development, not a Git abstraction layer.
- Git is legacy interop, migration, backup, mirror, CI, package ecosystem, and export plumbing.
- Anvics must be thinkable without Git: native repositories, native sync, native review, native permissions, native source APIs, native publication.
- The first product surface is not an agent runner. It is a native repository/workspace/history/review UI plus ingestion/integration for external agents.

## Foundation

- Build a real native VCS substrate first; the agent workbench is the first surface area.
- Canonical model: append-only event/operation log plus immutable content-addressed source snapshots.
- `SourceSnapshot` is a state-capture object, not a commit substitute or publication object.
- `NativePublication` is the accepted official repository history object.
- Native history is a DAG of `NativePublication`s with typed edges, not a Git-style commit DAG.
- `NativePublication`s point to exact `SourceSnapshot`s and index `ChangeUnit`s plus event provenance.
- `LegacyGitArtifact` is optional output for Git-era systems.

## MVP 0 Vs Roadmap

- MVP 0 proves one tight loop: thread, workspace, snapshot, compact evidence, review projection, native publication, optional legacy Git export.
- The platform roadmap preserves the larger architecture: production VFS, command worker, conflict coordination, native sync, richer policy, and richer `ChangeUnit`s.
- The decision log contains internal vocabulary for the eventual system. Agents and MVP 0 do not need every object as a first-class workflow concept.
- MVP 0 can use a materialized workspace backend while Linux/macOS VFS prototypes run in parallel.
- MVP 0 conflict handling is overlap detection plus reviewable conflict notes, not semantic auto-resolution.
- MVP 0 evidence is compact by default: raw artifacts by reference, short review summaries by budget.
- Team relay/native sync is post-MVP.

## Core Objects

These are internal product/domain objects. They are not all mandatory agent-facing terms for MVP 0.

- `Repository`: top-level storage, sync, and security unit.
- `SourceSnapshot`: immutable source tree state capture.
- `WorkspaceView`: mutable/read-only view over a snapshot, backed by cheap overlay.
- `WorkThread`: stable task/intent container.
- `Attempt`: one approach/run inside a work thread.
- `AgentSession`: one concrete agent execution.
- `ExecutionView`: pinned source/environment view for command execution.
- `CommandEvent`: command, output, exit status, environment, file effects.
- `FileEffectSet`: neutral before/after path/object effects.
- `ChangeUnit`: source-control relevant unit derived from file effects or direct edits.
- `EvidenceBundle`: tests, logs, screenshots, benchmarks, traces, reports.
- `ReviewProjection`: intent/evidence/risk/decision review object.
- `NativePublication`: accepted native publication.
- `LegacyGitArtifact`: optional Git commit/branch/PR/patch-stack export.
- `CoordinationSignal`: soft warning that work may interfere.
- `ConflictSession`: structured agent collaboration over conflicts.
- `ResolutionThread`: separate thread for non-trivial conflict resolution.
- `ResolutionProposal`: conceptual resolution with tradeoffs/evidence.
- `LearningNote`: scoped, attributable, reviewable lesson from prior work.
- `ContextPack`: scoped, refreshable context delivered to an agent.
- `SourceRoot` and `PackageBoundary`: repo-internal structure for monorepos.

## Work Threads And Attempts

- `WorkThread` is the stable task/intent plus intended publication unit.
- A work thread can contain multiple attempts, agent sessions, human edits, replays, and reviews.
- New `Attempt` when same task goal and acceptance criteria are retried differently.
- New `WorkThread` when task goal, acceptance criteria, publication unit, policy scope, or visibility changes.
- Work thread lifecycle: draft, active, paused, blocked, reviewing, accepted, published, superseded, abandoned.
- Failed/abandoned work is preserved in the work graph but collapsed/summarized by default.
- Failed work can produce `LearningNote`s, not raw failed traces by default.

## Parallel Work And Worktree Replacement

- Worktrees should disappear into the substrate.
- Many agents can work over the same codebase/base snapshot without full repo copies.
- Implementation model: shared immutable source storage plus per-agent/per-thread mutable overlays.
- `WorkspaceView` is the overlay-backed workspace primitive.
- VFS mount is the primary filesystem projection for tools that need paths.
- VFS mount is not the primary agent-facing abstraction; normal agents should use Anvics packets, CLI/API, and runtime-provided workspace paths.
- Materialized directory is the MVP/fallback projection; VFS is the strategic low-copy projection for large repos and many concurrent agents.
- Agents should not create Git worktrees or full repo copies for parallel work.

## VFS

- `anvics-vfs` is a strategic v1 product direction if Anvics wants cheap path-compatible workspace projections at scale; it is not required for every workflow.
- The filesystem is a projection over Anvics source state, not canonical source state.
- VFS is a compatibility projection for editors, shells, language servers, tests, and other tools that require paths.
- VFS is not canonical state and not the product's primary control plane.
- The preferred path is proxy/command runtime ownership of mount lifecycle; manual `workspace mount` is a debug/manual escape hatch.
- First real VFS targets: Linux FUSE and macOS macFUSE.
- Initial Rust candidate: `fuser`.
- Windows ProjFS should be researched/prototyped early; Dokan/WinFSP are fallback candidates.
- MVP 0 may use `materialized_dir` to prove the product loop while VFS prototypes run in parallel.
- `materialized_dir` backend is required for MVP, locked-down machines, CI, containers, and fallback. It can remain a supported backend, but it does not solve the full no-copy worktree replacement at scale.
- `anvics-vfs` owns Anvics' backend abstraction; do not let one crate/platform API define the workspace model.
- MVP 0.17 proved an opt-in `fuse_mount` backend behind `command run` and `agent accept --run-*`; it is not the default workflow yet.
- Runtime-owned mounts are preferable to manual agent-managed mounts: Anvics can classify the command, capture file effects, reconcile writes, attach evidence, and clean up the mount.
- MVP 0.20 hardened the spike for common writeback patterns including temp-file rename, file/directory rename, truncate, append, nested create/delete, and delete.
- Flush/release/fsync are compatibility acknowledgements in the spike, not durability guarantees; persistence remains Anvics' command reconciliation step.
- The current FUSE backend is a spike with an in-memory command-session mirror. Production VFS still needs lazy CAS/base hydration, crash recovery, Windows research, and broader dogfood.
- Start with a small mount/capabilities backend trait; keep detailed file-operation plumbing backend-specific until prototypes teach more.
- Open-file write state, flush/release/fsync behavior, and clean unmount semantics are core prototype risks.

## Conflict And Coordination

- VCS owns mechanical concurrency guarantees; agents own semantic coordination.
- Mechanical guarantees: isolated `WorkspaceView`s, immutable snapshots, stable `ChangeUnit` identity, overlap detection, policy gates, audit, publication consistency.
- Agents negotiate meaning: intent, conceptual resolution, code changes, tradeoffs, task satisfaction, evidence.
- MVP 0 conflict handling is same-path/same-hunk overlap detection plus a reviewable conflict note.
- Roadmap hard overlap starts `ConflictSession`.
- Soft risk starts `CoordinationSignal`.
- Agents collaboratively resolve conflicts as soon as they appear.
- `ConflictSession` requires structured resolution contracts from each participating agent, not transcript inference.
- Auto-apply resolution if all agents accept, tasks remain satisfied, evidence passes, policy allows, and no high-risk gate blocks.
- Human involvement is conceptual, not line-level merge labor.
- Roadmap non-trivial conflicts create `ResolutionThread` by default.
- Direct shared `ResolutionChangeUnit` only for mechanically trivial cases.
- Full `ConflictSession`, `ResolutionThread`, resolution contracts, and auto-apply are roadmap features, not MVP 0 requirements.

## Freshness And Replay

- Agents may continue stale work.
- Publication requires freshness, replay, coordination, or explicit policy approval.
- Active work threads receive graded `FreshnessSignal`s:
  - informational
  - watch
  - coordination
  - blocking
- Stale-base replay is modeled as agent-assisted `ReplayAttempt`, not purely automatic rebase.

## Agents And Integration

- Anvics does not own agent execution in v1.
- External agents integrate through:
  - Anvics skill/instructions
  - CLI
  - local JSON-RPC API
  - MCP adapter
  - VFS mount
  - passive transcript/log/diff/PR ingestion
- MVP 0 needs only the skill, CLI/local API basics, materialized workspace support, and passive evidence attachment.
- MCP adapter and VFS mount integration are roadmap integration surfaces.
- Agents receive scoped `ContextPack`s, not raw full repo memory/history.
- VCS builds baseline context pack; agents can request scoped expansions.
- Every `AgentSession` records exact context pack and expansions as provenance.
- Do not store hidden chain-of-thought; store tool-visible trace, explicit rationale, summaries, uncertainty, and resolution contracts.

## Review And Publication

- Native review centers on `ReviewProjection`, not raw diffs or raw traces.
- Review projection includes intent, acceptance criteria, final diff, conceptual summary, important decisions, tests/evidence, conflicts/resolutions, policy gates, risks, affected files/modules/symbols, native publication target, and optional legacy export preview.
- Humans can drill into diffs/logs but should see conceptual context first.
- Evidence in review projections is compact by default: command names, exit status, short summaries, key excerpts, and artifact links. Raw transcripts/logs are stored by reference unless explicitly requested.
- `NativePublication` means accepted work is part of the official repository state.
- Publication records why/how state became official, not only latest files.

## Learning Notes

- `LearningNote` is first-class optional source-control object.
- Produced from failed/superseded work as structured memory.
- Scoped to repo/package/file/symbol/task type.
- Attributable, reviewable, expirable, conflict-aware, compact.
- Visibility is policy-governed and cannot exceed source evidence unless redacted/approved.
- Future agents get learning notes through context packs.

## Authorship And Attribution

- Do not use fixed authorship roles as core schema.
- Authorship/accountability is event-level attribution.
- Every event records actor chain, action, produced/modified objects, policy/approval status.
- Authorship views are projections: who requested, who edited, who ran, who approved, who published.
- Operation actor chains include human requester, agent executor, tool/service executor, active policy, and approval status.

## Privacy, Visibility, And Secrets

- Visibility applies to work threads, attempts, evidence, traces, files, generated artifacts, native publications, and legacy Git exports.
- Raw traces are private by default with retention controls; durable summarized trace/provenance stays with work record.
- Secret-risk content is a hard default gate for publication, sync, and evidence sharing.
- Secret detection layers:
  - built-in scanners
  - repo/org policy
  - agent claims
  - privileged human/security override
- If content is `secret_risk`: do not publish, do not sync broadly, do not attach as shared evidence.
- Quarantine/redact before broad storage/sync; do not rely on UI-only redaction.
- Agents can suggest false positives but cannot clear `secret_risk` alone.

## Policy

- The VCS enforces user/org policy, not its own taste.
- Agents can request broader permission but cannot silently grant it.
- V1 policies: `ReadScope`, `WriteScope`, `CommandScope`, `EvidenceScope`, `PublicationScope`.
- Keep policy simple and explicit in v1, not full enterprise RBAC cathedral.
- Source/runtime/evidence boundaries are governed by policy objects.

## Repository Configuration

- `Repository` is top-level storage/sync/security unit.
- `SourceRoot` and `PackageBoundary` exist from v1.
- `external_ref` tree entry is generic; do not bake in Git submodule semantics.
- Cross-repo transactions deferred, but model should not prevent them.
- Source-root/package detection priority:
  1. source-tree `anvics.toml`
  2. heuristics/plugins
  3. agent suggestions
- Source-tree `anvics.toml` is optional but encouraged.
- `anvics repo init` and `anvics repo doctor` generate/improve config.
- No nested/inherited `anvics.toml` in v1.
- Private/org/local policy lives in Anvics metadata, not `anvics.toml`.
- Agents may propose `anvics.toml` edits; config changes are policy-sensitive.

## `anvics.toml`

- Focus on portable repo structure and tool compatibility.
- Include source roots, package boundaries, generated paths, ignored paths, evidence candidate paths, tool hints, agent instruction references.
- Do not include secrets, RBAC, retention, relay credentials, or machine-local settings.
- Tool hints are simple named command lists, not workflow/task-runner semantics.
- Source-root-scoped tools can come later if needed.
- Generated paths:
  - `generated.tracked`: generated source that may be published but should not be directly edited by default.
  - `generated.untracked`: output/cache that should not become source changes.
  - `evidence.candidate_paths`: artifacts that can produce attach suggestions.
- Direct edits to tracked generated source require rationale and policy approval.

## Command Execution

- Anvics provides command execution, but is not a full agent runner.
- Canonical path: agent asks Anvics to run command against pinned `ExecutionView`.
- Fallback: external agent runs command itself and attaches `CommandEvent` with weaker provenance.
- `anvics-worker` executes commands; `anvicsd` coordinates and records.
- MVP 0 may rely on externally attached command events with compact evidence.
- Roadmap local worker first; design protocol for future remote/cloud workers.
- Roadmap command execution enforces policy and captures provenance, but does not claim to be a malicious-code sandbox.
- Strong isolation belongs to containers, VMs, devcontainers, CI, cloud sandboxes.
- Command policy can cover cwd, env rules, secrets, network, timeout, output size, allowed command patterns, source write capture, evidence artifact rules.
- Command/proxy execution should own workspace resolution, optional mount lifecycle, file-effect summaries, evidence attachment, and cleanup where possible.
- Start with coarse top-level command policy classes before claiming syscall-level control: read-only, mutating, destructive, networked, host-escape-risk, and interactive.
- MVP 0.21 makes coarse command policy operational: networked, host-escape-risk, and interactive commands are blocked by default and require an explicit audited operator override reason.
- MVP 0.22 closes the command-file gap: Anvics-run command files are classified from their contents before execution, and operators can preview classification with `command classify` without creating evidence.
- MVP 0.24 introduces `anvics-worker` as an opt-in local command executor while keeping in-process execution as the default. This proves the worker protocol and provenance field without changing the user-facing command workflow.
- MVP 0.25 adds `command worker-check` so operators can verify the local worker binary/protocol before opting into worker-backed execution.
- Destructive and unknown commands still run by default in MVP 0.21 because the execution surface is an isolated workspace and repo-specific policy is not mature enough yet.
- Policy classification is an operational gate and evidence signal, not a guarantee that Anvics is a sandbox.

## File Effects And Change Units

- Command side effects first become neutral `FileEffectSet`s.
- `FileEffectSet` records before/after path/object metadata, not language-specific conclusions.
- Future `FileEffectSet`s should prefer event/audit-derived file effects when a runtime or mount can provide them, with deterministic tree diff as fallback.
- MVP 0.26 derives heuristic `FileEffectSet`s from review changed paths and renders proposed `ChangeUnit`s directly in review markdown.
- The first implementation is deliberately mechanical: one proposed `ChangeUnit` per source-relevant path effect, no semantic grouping, no rename identity, and no repo-specific policy parser.
- Classification layers: config, policy, tools, humans, agent claims.
- Built-in classification labels:
  - `source`
  - `generated_tracked`
  - `generated_untracked`
  - `evidence_candidate`
  - `cache`
  - `lockfile`
  - `config`
  - `secret_risk`
  - `binary`
  - `unknown`
- Everything else is repo/plugin-defined.
- Only source-relevant effects become `ChangeUnit`s.
- Cache/ignored output stays out of source changes.
- Evidence candidates become `EvidenceArtifact` suggestions.
- Unknown/sensitive/out-of-scope effects are gated before publication.
- Anvics mechanically proposes `ChangeUnit`s, agents refine, policy/humans gate unresolved or sensitive cases.

## Change Units And Invariants

- `ChangeUnit` schema should stay future-proof/multi-resolution.
- MVP 0 implements path/hunk plus generic tags only.
- Symbol/package/config/schema/generated/publication-target/dependency/test/invariant scopes are roadmap schema/extensions, not MVP 0 extraction requirements.
- Agents may attach conceptual invariant/change claims as structured metadata.
- Invariant claims have status: claimed, partially verified, verified, contradicted, stale, needs_human.
- Invariant verification blocks publication only when repo/org policy marks that scope/type as required.

## Storage

- Logical stores behind API, not one prematurely-fixed physical engine:
  - object store
  - event log
  - overlay store
  - index store
  - evidence store
  - policy store
  - sync store
- V1 physical storage: SQLite for catalog/event/metadata/relationships/index metadata plus content-addressed object store for source-like/frequently read content.
- SQLite is not where agents regularly read source from.
- Source/docs/config/generated source/test fixtures/evidence bytes live outside SQLite and are VFS/API-readable.
- CAS uses BLAKE3-addressed object files in simple directory layout.
- Packfiles deferred until performance demands them.
- Anvics defines native deterministic tree/snapshot format; do not reuse Git tree objects internally.
- Tree entries support file, directory, symlink, external_ref.
- Symlinks are first-class with policy around materialization and root escape.

## Rust Architecture

- Implementation language for local service/storage/VFS/indexing/sync/API/CLI/legacy adapter: Rust.
- CLI: `anvics`.
- Daemon: `anvicsd`.
- Crate prefix: `anvics-*`.
- Core crate boundaries:
  - `anvics-core`
  - `anvics-store`
  - `anvics-workspace`
  - `anvics-vfs`
  - `anvics-index`
  - `anvics-agent`
  - `anvics-conflict`
  - `anvics-policy`
  - `anvics-sync`
  - `anvics-api`
  - `anvics-cli`
  - `anvics-worker`
  - `anvics-legacy-git`
- Core/storage/workspace/VFS/agent/conflict/policy/sync must not depend on Git.
- Git logic only in `anvics-legacy-git`, which depends outward on Anvics primitives.
- `anvicsd` owns repo state; CLI/API/VFS/UI/agents are clients.
- No client mutates repo storage directly.

## Local API

- `anvicsd` exposes JSON-RPC over Unix domain sockets/macOS local sockets and Windows named pipes.
- MCP is thin adapter over `anvics-api`, not core daemon protocol.
- API DTOs separate from `anvics-core` domain types.
- v1 uses `serde` JSON and opaque external IDs.
- API supports request/response plus event subscriptions.
- Native method groups:
  - `repo.*`
  - `thread.*`
  - `workspace.*`
  - `agent.*`
  - `evidence.*`
  - `review.*`
  - `publication.*`
  - `coordination.*`
  - `events.*`
- Git only under explicit legacy methods.
- Event subscriptions use JSON-RPC notifications over long-lived local connection.
- Per-repo monotonic `event_seq`; at-least-once delivery; resumable `events.list_since`.
- Events are compact routing records with object refs/summaries; full objects fetched separately.
- Event streams are policy-filtered; cursors may skip invisible events.

## Sync

- Native sync replicates policy-filtered source-control objects/work records, not folders or commit graphs.
- MVP 0 is local-only.
- Roadmap architecture: local-first client plus team relay.
- Peer-to-peer/federated sync deferred.
- Relay is policy-aware object sync/collaboration service, not Git remote.
- Synced objects include repo metadata, blobs/trees/snapshots, work threads, overlays where shared, change units, session summaries, command events, evidence subject to policy, coordination/conflict/resolution/review/publication objects, legacy export records.

## UI And Product Surface

- First UI is repo/workspace/history/review control plane, not full forge replacement and not agent wrapper.
- Default repo UI: publication-centric source browser.
- It shows current `NativePublication`, file tree/search, recent publication graph/history, active work by file/module, freshness/conflict indicators, review/evidence status.
- History UI: linked work graph and publication graph, not commit log.
- Code browser surfaces active work by file/module inline/side rail.
- MVP 0 demo: two external or simulated agents work in isolated threads over one repo, produce compact evidence, create review projections, a human accepts or manually combines work, native publication is created, legacy Git export optional.
- Full platform demo: several agents work on one repo/monorepo using overlay workspaces and VFS, conflict/coordination happens automatically, human sees conceptual review, native publication is created, legacy Git export optional.

## API/CLI Naming

- Avoid Git-like porcelain in v1:
  - no `commit`
  - no `branch`
  - no `checkout`
  - no `merge`
  - no `rebase`
- Use Anvics verbs:
  - thread
  - workspace
  - snapshot
  - review
  - publish
  - replay
  - coordinate
  - resolve
- Legacy Git commands are visibly namespaced under `legacy git`.

## Open Design Questions Remaining

- Exact `anvics-vfs` backend implementation details for Linux/macOS and install flow for macFUSE.
- Windows VFS choice: ProjFS vs Dokan vs WinFSP.
- Exact physical storage engine details around SQLite/CAS indexing and performance.
- API event stream implementation shape beyond JSON-RPC notification basics.
- Source-root-scoped `[tools]` in v1 or later.
- Minimum environment fingerprint.
- Built-in secret detector quality/noise balance.
- Which `FileEffect` classifications require human approval by default.

## Next Artifact

- [MVP 0](MVP_0.md)
- [Platform Roadmap](PLATFORM_ROADMAP.md)
