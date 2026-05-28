# 12 - Rust Architecture

## Research Question

How should Anvics be structured as a Rust system so the core remains a native VCS substrate rather than a Git wrapper?

## Observed Facts

- The local service must coordinate VFS mounts, workspace overlays, policy enforcement, indexing, command provenance, and multi-agent concurrency. A direct-file CLI would bypass too many invariants.
- Rust is a strong fit for the local daemon, VFS integration, content-addressed storage, crash recovery, indexing, and concurrent agent workloads.
- Anvics needs a strict dependency boundary: native domain/storage/workspace crates must not depend on Git. Git import/export belongs in an adapter crate.
- VFS and overlay-backed `WorkspaceView`s are v1 product surfaces, not late optimizations, because agents need `grep`, shell tools, LSPs, test runners, and monorepo-compatible partial materialization.
- A real projection backend abstraction is required to satisfy the Anvics product thesis. VFS is the likely low-copy backend for large repos and many concurrent agents, but the agent-facing product boundary should be a proxy-mounted workspace runtime, not manual mount management.
- Materialized directories are a fallback and MVP proving surface, not the primary long-term workspace model.
- The first product should integrate external agents through skill/instruction files, CLI, local API/MCP, VFS, and passive ingestion, not by owning agent execution.
- The agentvfs deep dive suggests that command/proxy execution should own workspace resolution, command policy classification, mount lifecycle, file-effect summaries, evidence attachment, and cleanup where possible.

## Sources And Artifacts

- Main thesis: `../GIT_ALTERNATIVE.md`
- Source-control API note: `11-source-control-api.md`
- Legacy Git adapter note: `06-git-boundary-design.md`
- Agent workflow note: `01-agent-workflow-reality.md`
- Draft Anvics skill: `../../skills/anvics-skill/SKILL.md`
- Agentvfs deep dive: `14-agentvfs-deep-dive.md`

## Proposed Rust Crate Boundaries

- `anvics-core`: Git-free domain types, object IDs, events, actor attribution, policies, primitive schemas, and shared error types.
- `anvics-store`: content-addressed object store, immutable blobs/trees/snapshots, append-only event log, overlay persistence, evidence references, and transactional durability.
- `anvics-workspace`: `WorkspaceView`, per-thread overlays, snapshot creation, replay attempts, materialization coordination, and write capture.
- `anvics-runtime` or equivalent provisional boundary: workspace resolution, command execution, mount lifecycle, coarse policy classification, file-effect summary, and evidence attachment. This can initially live inside `anvics-store`/`anvics-cli` until the boundary earns its own crate.
- `anvics-vfs`: VFS mount/projection layer, lazy hydration, file watching, sparse roots, developer-tool compatibility, and backend abstraction.
- `anvics-index`: path/content search, hunk extraction, symbol/dependency indexing, `ChangeUnit` indexes, and freshness metadata.
- `anvics-agent`: `AgentSession`, `Attempt`, `ContextPack`, `CommandEvent`, `ExecutionView`, `EvidenceBundle`, `FileEffectSet`, and `LearningNote`.
- `anvics-conflict`: `CoordinationSignal`, `ConflictSession`, `ResolutionThread`, `ResolutionProposal`, and structured resolution contracts.
- `anvics-policy`: read/write/command/evidence/publication scopes, policy gates, approvals, redaction, and retention.
- `anvics-sync`: local-first object sync, team relay protocol, visibility filtering, replication state, and remote collaboration.
- `anvics-api`: local RPC/HTTP/MCP API over `anvicsd`.
- `anvics-cli`: thin CLI client over the API.
- `anvics-worker`: command execution worker coordinated by `anvicsd`.
- `anvics-legacy-git`: Git import/export adapter for repositories, commits, branches, patch stacks, and PR-oriented flows.

## Dependency Rules

- `anvics-core` must not depend on Git, UI, CLI, VFS, or agent-specific runtimes.
- `anvics-store`, `anvics-workspace`, `anvics-vfs`, `anvics-agent`, `anvics-conflict`, `anvics-policy`, and `anvics-sync` must not depend on `anvics-legacy-git`.
- `anvics-runtime`, if split out, must depend on Anvics source/workspace abstractions and adapters, not on Git as a storage model.
- `anvics-legacy-git` depends outward on Anvics primitives and translates to/from Git-era artifacts.
- `anvics-cli`, `anvics-api`, and UI clients should not mutate repo storage directly; they talk to `anvicsd`.
- Language/tool-specific intelligence belongs in plugins, indexes, agents, or policy modules, not in the core storage model.

## Local API Strategy

`anvicsd` should expose JSON-RPC over Unix domain sockets on Unix/macOS and named pipes on Windows for v1.

Reasons:

- Simple for agents, CLIs, and scripts to call.
- Easy to inspect and debug.
- Avoids generated-code and schema-tooling drag while the API shape is still moving.
- Works well for local-only privileged operations.
- Keeps the daemon protocol separate from any one agent ecosystem.

MCP should be implemented as a thin adapter over `anvics-api`, not as the daemon's core protocol. Optional localhost HTTP can be added later for UI/devtools if needed.

API design rules:

- Define domain types in `anvics-core`.
- Define request/response DTOs separately in `anvics-api`.
- Use `serde` JSON for v1.
- Version the API from day one.
- Keep external IDs opaque.
- Do not expose SQLite schema, CAS layout, or internal storage details.
- Support request/response methods plus an event subscription stream for freshness, coordination, conflict, review, policy, and publication events.
- Use Anvics-native method names; keep Git under explicit legacy names.

Initial method groups:

- `repo.*`
- `thread.*`
- `workspace.*`
- `agent.*`
- `evidence.*`
- `review.*`
- `publication.*`
- `coordination.*`
- `events.*`

## Local Service Model

`anvicsd` should own repository state and expose all mutation through a transactional API.

Responsibilities:

- Maintain append-only event log.
- Own content-addressed source storage.
- Manage overlay-backed `WorkspaceView`s.
- Serve VFS mounts and optional materialized directories.
- Enforce policy gates.
- Record actor attribution.
- Capture command/evidence metadata.
- Create snapshots and native publications.
- Coordinate conflict sessions and replay attempts.
- Maintain indexes asynchronously.
- Sync policy-filtered objects with the team relay.
- Route legacy Git import/export through `anvics-legacy-git`.

## Worker Model

`anvicsd` should coordinate command execution, but commands should run in a separate `anvics-worker` process.

Responsibilities of `anvicsd`:

- Check execution policy.
- Pin `ExecutionView`.
- Resolve materialized or mounted execution path.
- Own mount lifecycle when the runtime can safely do so.
- Start/cancel worker jobs.
- Receive output, status, file effects, and evidence references.
- Record `CommandEvent`s transactionally.
- Apply accepted file effects to the relevant `WorkspaceView`.

Responsibilities of `anvics-worker`:

- Run commands against a pinned VFS/materialized execution path.
- Stream stdout/stderr chunks.
- Report exit status and resource metadata.
- Summarize file effects, preferably from runtime/mount event data with tree diff as fallback.
- Attach candidate evidence artifacts.
- Send heartbeat/cancellation status.

MVP 0 may rely on externally attached command evidence. The roadmap should use a local worker, with a protocol designed so future remote/cloud workers can execute against a supplied `ExecutionView` and report back without mutating canonical state directly.

## VFS Backend Strategy

`anvics-vfs` should define Anvics' own backend abstraction. The product must not let one crate or platform API define the workspace model.

The agentvfs lesson is that VFS should usually sit behind a proxy/runtime boundary. Agents should receive a usable workspace path and Anvics commands, while the runtime handles mount creation, policy classification, command execution, changed-path summaries, evidence attachment, and unmount. A low-level `workspace mount` remains valuable for debugging and unusual tool flows, but it is not the happy path.

First real VFS backends:

- Linux FUSE.
- macOS macFUSE.

Initial Rust candidate:

- `fuser`, because it supports libfuse/macFUSE-oriented development and is mature enough to prototype against.

Required fallback backend:

- `materialized_dir`, for MVP 0, locked-down machines, CI, containers, or environments where VFS mounting is unavailable.

Early research/prototype target:

- Windows ProjFS, with Dokan/WinFSP as fallback candidates if ProjFS is not viable.

Important distinction: materialized directories are acceptable for MVP 0 and compatibility fallback, but they are not Anvics' primary long-term answer to source access. The core source state remains `WorkspaceView` over shared immutable storage plus mutable overlay; VFS is the normal filesystem projection once the product loop is proven.

The next VFS milestone should therefore be a proxy-mounted workspace spike: mount one workspace, run a command through that mounted projection, collect file effects, unmount cleanly, and fall back to materialized execution when FUSE/macFUSE is unavailable.

Clients:

- `anvics` CLI.
- Local API/MCP clients.
- VFS mount processes.
- `anvics-worker` local command processes.
- Desktop/web UI.
- External agents using the Anvics skill.
- Passive ingestion tools.

## Crash And Concurrency Expectations

- Event log is the durable source of truth for operations.
- Mutations are transactional.
- Immutable objects make reads cheap and concurrent.
- Per-workspace overlay writes proceed independently.
- Publication and shared coordination require stronger serialization.
- VFS writes are journaled before becoming accepted overlay state.
- Command runs use pinned `ExecutionView`s so evidence maps to exact source and environment state.
- On restart, `anvicsd` reconciles mounted views, running/orphaned commands, overlay checkpoints, partial evidence artifacts, and stale indexes.

## Product Implications

- Rust architecture should enforce the product thesis: Anvics is native first, Git is an adapter.
- VFS and overlay workspaces must be designed early because they are the direct replacement for Git worktrees.
- The runtime/proxy boundary is as important as mount mechanics: controlled command execution over isolated workspaces is the place where Anvics turns source projection into trustworthy evidence.
- The API and CLI should use Anvics verbs: thread, workspace, snapshot, review, publish, replay, coordinate, resolve. Avoid Git-like porcelain outside `legacy git`.
- Keep external-agent integration broad: skill/instructions, CLI, API/MCP, VFS, and passive ingestion.

## MVP Impact

- Start a Rust workspace with the crate boundaries above, even if some crates are initially thin.
- Implement `anvicsd` as the owner of repo state.
- Make `anvics-cli` a client over the local service API.
- Expose JSON-RPC over local sockets/named pipes as the v1 daemon API.
- Implement MCP as a thin adapter over `anvics-api`.
- Implement local `anvics-worker` for command execution against pinned `ExecutionView`s.
- Keep `anvics-core` and storage/workspace crates Git-free from the first commit.
- Treat projection backend abstraction, proxy-mounted runtime behavior, and overlay-backed `WorkspaceView`s as v1 requirements. `anvics-vfs` is the likely production projection backend if the spike proves mounts pay for their complexity.
- Ship Linux FUSE and macOS macFUSE backends in v1 if at all possible; keep `materialized_dir` as fallback.
- Put all Git logic in `anvics-legacy-git`.

## Open Questions

- Can `fuser` meet both Linux FUSE and macOS macFUSE needs, or do we need separate backend crates?
- Should proxy-mounted command execution live in a separate `anvics-runtime` crate immediately, or stay inside existing CLI/store boundaries until the spike clarifies the split?
- What minimum event/audit stream is needed to replace tree-diff-only file effect summaries?
- What is the minimum macFUSE installation flow that does not wreck developer adoption?
- What is the right Windows VFS path: ProjFS, Dokan, WinFSP, or another projection API?
- What IPC protocol should `anvicsd` use with local and future remote workers?
- What event stream shape should JSON-RPC subscriptions use: long-lived connection notifications, server-sent local stream, or a separate socket?
- What physical storage engine should back `anvics-store` first: SQLite plus object files, RocksDB, sled/redb, or a custom CAS layout?
- How much of `anvics-index` must be synchronous for v1 versus eventually consistent?
