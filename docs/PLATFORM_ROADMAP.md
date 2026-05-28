# Anvics Platform Roadmap

This roadmap turns the product thesis and decision log into a long-form implementation sequence. It is **not** the first build plan. The first build plan is [`MVP_0.md`](MVP_0.md), which proves the tight agent-native loop before Anvics attempts the full platform.

The roadmap order remains dependency-driven: native model first, then storage, daemon/API, overlays, VFS, command execution, agent integration, review UI, conflict coordination, legacy Git export, and team relay. Roadmap phases can be split, deferred, or skipped until MVP 0 proves that the core work-thread/review loop is valuable.

## Guiding Constraints

- Anvics is native first; Git is a legacy adapter.
- Core crates must not depend on Git.
- `anvicsd` owns repository state.
- CLI/API/VFS/UI/agents are clients.
- VFS is a strategic v1 product direction, but it is not an MVP 0 blocker.
- MVP 0 may use a materialized workspace backend while Linux/macOS VFS prototypes run in parallel.
- Source-like content stays outside SQLite and is VFS/API-readable.
- Agents integrate through skill, CLI, API/MCP, VFS, and passive ingestion.

## Relationship To MVP 0

MVP 0 should prove:

- Local native repository state.
- Snapshots and isolated work threads.
- Simple overlay or materialized workspaces.
- Compact evidence capture.
- Review projection.
- Native publication.
- Optional legacy Git export.

MVP 0 should not be blocked by:

- Production VFS maturity.
- Full command-worker mediation.
- Semantic conflict resolution.
- MCP adapter completeness.
- Native team relay/sync.
- Enterprise controls.
- Rich symbol/invariant `ChangeUnit`s.

## Roadmap Phase 1 - Core Model Skeleton

Goal: define the native domain model without storage or Git leakage.

Crates:

- `anvics-core`

Scope:

- Opaque IDs:
  - `RepositoryId`
  - `SourceSnapshotId`
  - `WorkspaceViewId`
  - `WorkThreadId`
  - `AttemptId`
  - `AgentSessionId`
  - `ChangeUnitId`
  - `EventId`
- Core object structs:
  - `Repository`
  - `SourceSnapshot`
  - `WorkspaceView`
  - `WorkThread`
  - `Attempt`
  - `AgentSession`
  - `ChangeUnit`
  - `EvidenceBundle`
  - `ReviewProjection`
  - `NativePublication`
  - `LegacyGitArtifact`
- Event model:
  - event type
  - actor attribution
  - object refs
  - timestamp
  - policy/approval status
- Basic policy enum shells:
  - `ReadScope`
  - `WriteScope`
  - `CommandScope`
  - `EvidenceScope`
  - `PublicationScope`

Acceptance criteria:

- `anvics-core` compiles independently.
- No Git dependency.
- IDs serialize as opaque strings.
- Domain types are separate from API DTOs.

## Roadmap Phase 2 - Storage Spine

Goal: persist native source state and events.

Crates:

- `anvics-store`
- `anvics-core`

Scope:

- SQLite catalog/event database.
- BLAKE3-addressed CAS directory.
- Native object types:
  - blob
  - tree
  - snapshot manifest
  - evidence artifact placeholder
  - command log placeholder
- Deterministic Anvics tree format.
- Append-only event log.
- Transactional event append.
- Basic repository init.

Acceptance criteria:

- Can initialize `.anvics/`.
- Can store/read blobs by BLAKE3 object ID.
- Can store/read tree manifests.
- Can create a `SourceSnapshot`.
- Can append/list events.
- Source-like content is not stored as SQLite blobs.

## Roadmap Phase 3 - Daemon And Local API

Goal: make `anvicsd` the owner of repo state.

Crates:

- `anvics-api`
- `anvics-cli`
- `anvics-core`
- `anvics-store`

Scope:

- `anvicsd` process.
- JSON-RPC over Unix socket/macOS local socket.
- Windows named pipe design stub.
- API DTOs separate from core domain structs.
- Initial methods:
  - `repo.init`
  - `repo.get`
  - `thread.create`
  - `thread.get`
  - `thread.list`
  - `workspace.create_view`
  - `events.subscribe`
  - `events.list_since`
- Per-repo monotonic event sequence.
- At-least-once JSON-RPC notifications over long-lived connection.

Acceptance criteria:

- CLI talks to `anvicsd`; CLI does not mutate storage directly.
- API uses opaque IDs.
- Event notifications can be subscribed to and resumed.
- API has version field.

## Roadmap Phase 4 - Workspace Overlays

Goal: create cheap isolated mutable workspaces over shared snapshots.

Crates:

- `anvics-workspace`
- `anvics-store`
- `anvics-core`
- `anvics-api`

Scope:

- Overlay-backed `WorkspaceView`.
- Read path from base snapshot plus overlay.
- List tree from composed view.
- Write path into overlay.
- Apply patch into overlay.
- Diff overlay against base snapshot.
- Create new `SourceSnapshot` from overlay.
- Basic stale-base metadata.

Acceptance criteria:

- Two workspaces can share the same base snapshot and modify different files independently.
- No full repo copy is required.
- Snapshotting a workspace creates a deterministic new snapshot.
- Events record workspace writes and snapshot creation.

## Roadmap Phase 5 - VFS Prototype

Goal: make filesystem access a projection over `WorkspaceView`.

MVP 0 stance: a materialized workspace backend is enough for the first proof. Linux/macOS VFS work should proceed as a prototype, but product validation must not wait for production-quality FUSE/macFUSE behavior.

Crates:

- `anvics-vfs`
- `anvics-workspace`
- `anvics-store`

Scope:

- Backend trait with mount/unmount/capabilities.
- Linux FUSE backend using `fuser` candidate.
- macOS macFUSE backend using `fuser` candidate.
- `materialized_dir` fallback backend.
- Mount a `WorkspaceView`.
- Lazy reads from CAS/base snapshot.
- Writes captured into overlay.
- Directory listing.
- Basic file metadata.
- Basic file watcher/event integration investigation.

Acceptance criteria:

- A mounted workspace can be inspected with shell tools like `ls`, `cat`, `rg`.
- Writes through the mount update the overlay, not base snapshot.
- Unmount/remount preserves overlay state.
- Materialized fallback can produce a usable directory when VFS is unavailable.
- Linux and macOS backend prototypes both have smoke tests or manual verification notes.

## Roadmap Phase 6 - Command Worker

Goal: run commands with provenance against pinned source state.

MVP 0 stance: external agents may run commands themselves and attach compact command evidence. Full `anvics-worker` mediation is roadmap work.

Crates:

- `anvics-worker`
- `anvics-agent`
- `anvics-api`
- `anvics-workspace`

Scope:

- `ExecutionView` creation.
- `workspace.run_command`.
- Local `anvics-worker` process.
- Command spec:
  - cwd
  - args
  - env policy
  - timeout
  - output limits
  - network flag placeholder
- Stream stdout/stderr.
- Record exit status.
- Capture before/after file effects.
- Attach `CommandEvent`.
- Mark externally attached command events as weaker provenance.

Acceptance criteria:

- Command can run in mounted or fallback workspace path.
- Command event records exact execution view.
- Output and exit status are captured.
- Changed files produce `FileEffectSet`.
- Worker crash/command failure does not corrupt daemon state.

## Roadmap Phase 7 - FileEffect To ChangeUnit Pipeline

Goal: convert raw file effects into reviewable source-control units.

Crates:

- `anvics-agent`
- `anvics-index`
- `anvics-policy`
- `anvics-workspace`

Scope:

- `FileEffectSet` data model.
- Built-in labels:
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
- Classification provenance:
  - policy
  - heuristic
  - agent claim
  - tool
  - human
- `anvics.toml` generated/ignore/evidence parsing.
- Proposed `ChangeUnit` creation for source-relevant effects.
- Evidence artifact suggestions.
- Secret-risk hard gate placeholder.

Acceptance criteria:

- Cache/ignored output does not become `ChangeUnit`.
- Source changes become proposed `ChangeUnit`.
- Evidence candidate paths produce suggestions.
- Direct tracked generated-source edits are flagged.
- Secret-risk classification blocks publication in model/policy.

## Roadmap Phase 8 - Agent Integration Kit

Goal: let external agents use Anvics without Anvics owning model execution.

Artifacts/crates:

- `anvics-skill`
- `anvics-cli`
- `anvics-api`
- MCP adapter

Scope:

- Finalize Anvics `SKILL.md`.
- Generate/ship repo instruction templates:
  - `AGENTS.md`
  - `CLAUDE.md`
  - Cursor/Windsurf rule template later
- CLI commands:
  - `anvics repo init`
  - `anvics repo doctor`
  - `anvics thread create/list/status`
  - `anvics workspace mount/diff/snapshot`
  - `anvics evidence attach`
  - `anvics review create`
  - `anvics publication create`
  - `anvics legacy git export`
- MCP adapter over local API.
- Passive command/event attachment.
- Basic `ContextPack` creation.

Acceptance criteria:

- A shell-based agent can read the skill, create/join a work thread, mount a workspace, edit files, run commands, attach evidence, and request review.
- MCP client can call core API methods.
- External command events can be attached with weaker provenance.

## Roadmap Phase 9 - Review And Publication Minimal UI

Goal: make Anvics useful to a human supervising agent work.

Surface:

- Web or desktop UI over `anvicsd` API.

Scope:

- Publication-centric source browser.
- Current `NativePublication` file tree.
- Work thread list.
- Workspace status.
- Active work by file/module.
- Review projection view.
- Evidence summary.
- ChangeUnit list/diff.
- Native publication action.
- Legacy Git export preview/stub.

Acceptance criteria:

- User can inspect current source state.
- User can see active work threads and files they touch.
- User can review final diff plus evidence/risk summary.
- User can create a `NativePublication`.
- UI does not require Git concepts as primary navigation.

## Roadmap Phase 10 - Conflict Coordination

Goal: handle parallel agent interference before human merge pain.

MVP 0 stance: detect path/hunk overlap and create reviewable conflict notes. Structured multi-agent resolution contracts and auto-apply are roadmap work.

Crates:

- `anvics-conflict`
- `anvics-agent`
- `anvics-index`
- `anvics-policy`

Scope:

- `CoordinationSignal`.
- `ConflictSession`.
- Structured resolution contract.
- `ResolutionThread`.
- ChangeUnit overlap detection:
  - path
  - hunk
  - generated artifact
  - publication target
- Soft signals:
  - same file
  - same package/source root
  - dependency/config overlap
- Agent agreement model:
  - accept/reject/needs_human
  - task satisfaction statement
  - evidence
  - uncertainty
- Policy gate for auto-apply.

Acceptance criteria:

- Two work threads editing overlapping hunks trigger conflict.
- Agents can submit structured resolution contracts.
- If all accept and policy allows, Anvics creates resolution output.
- Non-trivial conflict creates `ResolutionThread`.
- Human sees conceptual resolution proposal, not merge markers.

## Roadmap Phase 11 - Legacy Git Adapter

Goal: provide old-world exits without contaminating the core.

Crates:

- `anvics-legacy-git`

Scope:

- Import existing Git repository into Anvics snapshot.
- Track source import metadata.
- Export `NativePublication` as a Git commit/branch.
- Optional PR body/reference generation.
- Durable link back to Anvics publication/work thread.

Acceptance criteria:

- Core crates do not depend on `anvics-legacy-git`.
- Can import a Git repo into Anvics.
- Can export accepted native publication to Git branch/commit.
- Exported artifact references Anvics record.

## Roadmap Phase 12 - Team Relay Skeleton

Goal: prove local-first object sync shape.

MVP 0 stance: keep all state local. Team relay and native sync begin after the local work-thread/review loop is useful.

Crates:

- `anvics-sync`

Scope:

- Local-to-relay object push/pull.
- Policy-filtered object selection.
- Repo metadata sync.
- Source snapshot sync.
- Work thread summary sync.
- Native publication sync.
- Evidence metadata sync without raw private artifacts by default.

Acceptance criteria:

- Two local Anvics services can sync through a simple relay.
- Private/secret-risk objects are not broadly synced.
- Remote service is not modeled as Git remote or commit graph.

## Suggested Roadmap Order

1. Phase 1: Core model skeleton.
2. Phase 2: Storage spine.
3. Phase 3: Daemon and local API.
4. Phase 4: Workspace overlays.
5. Phase 5: VFS prototype.
6. Phase 6: Command worker.
7. Phase 7: FileEffect to ChangeUnit pipeline.
8. Phase 8: Agent integration kit.
9. Phase 9: Review/publication UI.
10. Phase 10: Conflict coordination.
11. Phase 11: Legacy Git adapter.
12. Phase 12: Team relay skeleton.

## Roadmap Demo Target

The full platform demo should show:

1. Initialize or import a repo.
2. Start multiple work threads from prompts/issues.
3. Give each agent a separate overlay-backed `WorkspaceView`.
4. Mount workspaces through VFS.
5. Agents edit the same codebase without Git worktrees or full copies.
6. Commands run through `anvics-worker` and attach evidence.
7. File effects become `ChangeUnit`s.
8. UI shows active work by file/module.
9. Overlap triggers coordination/conflict.
10. Agents submit resolution contracts.
11. Human reviews conceptual projection.
12. Create `NativePublication`.
13. Optionally export legacy Git artifact.
