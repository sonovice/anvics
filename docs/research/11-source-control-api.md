# 11 - Source-Control API

## Research Question

What API should Anvics expose so agents can inspect, modify, snapshot, evaluate, review, and publish code without treating a full OS/filesystem checkout as the only interface?

## Observed Facts

- Existing forge APIs expose useful pieces, but they are Git/repository APIs, not agent work APIs. GitHub separates contents APIs from raw Git database APIs; GitLab exposes repository files and commits APIs. These are good artifact APIs, but they do not model task intent, agent sessions, evidence, retries, or review projections.
- Existing VCS internals show better substrate shapes. `jj` has backend/tree/file operations plus an operation log. Sapling/Eden separates virtual filesystem materialization from tree/blob backing-store access. Pijul stores changes as graph atoms with dependencies and metadata.
- Agent systems already need more than file IO. They need to attach trace events, evidence, environment snapshots, review summaries, native publications, and optional legacy Git artifacts to a task-level object.
- Language Server Protocol shows how tools can expose semantic code operations through a protocol separate from filesystem access, but language semantics should be v1.5 rather than the first source-control API dependency.
- OpenTelemetry shows a useful trace model for commands, tool calls, evaluation runs, and links between asynchronous attempts.

## Sources And Artifacts

- GitHub REST Git database API: https://docs.github.com/en/rest/git
- GitHub repository contents API: https://docs.github.com/en/rest/repos/contents
- GitLab repository files API: https://docs.gitlab.com/api/repository_files/
- Language Server Protocol specification: https://microsoft.github.io/language-server-protocol/
- OpenTelemetry tracing API: https://opentelemetry.io/docs/specs/otel/trace/api/
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/jj/lib/src/git_backend.rs`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/jj/lib/src/working_copy.rs`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/sapling/eden/fs/store/BackingStore.h`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/sapling/eden/fs/store/IObjectStore.h`
- Local artifact: `/Users/simon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libpijul-1.0.0-beta.11/src/change.rs`

## Proposed API Model

The system should expose three distinct access surfaces:

1. **Filesystem projection**
   - A VFS mount for existing tools, IDEs, test runners, language servers, and humans.
   - Useful for compatibility, but not the canonical model.
   - This keeps adoption practical: editors, build tools, `ripgrep`, language servers, and tests still expect paths.
   - Materialized directories are fallback projections when VFS mounting is unavailable.

2. **Virtual snapshot access**
   - Read-only or mutable views over source snapshots without requiring a complete checkout.
   - Supports tree listing, path reads, search, patch application, and snapshot creation.
   - Lets many agents share the same immutable base while each gets an isolated mutable overlay.

3. **Source-control workflow API**
   - Operations on work threads, attempts, sessions, evidence, review projections, native publications, and legacy Git artifacts.
   - This is the Anvics layer Git lacks.

This is not primarily a claim about putting code in SQLite, although SQLite or another database could be one implementation detail. The deeper claim is that the filesystem should be a projection of source-control state, not the only way to operate on it.

The API should be paired with an agent skill or equivalent instruction bundle. Agents need an operating manual for the new source-control model: which APIs to call, when to snapshot, how to attach evidence, how to preserve failed attempts, when to materialize a filesystem checkout, when to publish natively, and when to export legacy Git artifacts.

Core API-addressable resources:

- `Repository`: imported source repository and policy container.
- `SourceSnapshot`: immutable source tree state capture; not a publication object or commit substitute.
- `WorkThread`: task-level unit containing attempts, sessions, evidence, reviews, and exports.
- `WorkspaceView`: mutable or read-only view over a snapshot, usually backed by a cheap overlay instead of a full repo copy.
- `ChangeUnit`: file/hunk/symbol-level change owned by a work thread or attempt.
- `AgentSession`: one agent execution with trace and environment metadata.
- `EvidenceBundle`: tests, commands, logs, screenshots, browser traces, CI, and benchmarks.
- `ReviewProjection`: human-readable review bundle generated from source state and evidence.
- `NativePublication`: accepted work published inside the system's own collaboration model.
- `LegacyGitArtifact`: commit, branch, patch stack, or PR emitted only for Git-era compatibility.

## Agent Skill Layer

The product should ship a concise skill for Codex/Claude/Cursor/Windsurf-style agents. This should be treated as part of the product surface, not documentation garnish.

The skill should be short and procedural:

- Start every task by creating or joining a `WorkThread`.
- Inspect source through `get_snapshot`, `list_tree`, `read_path`, and `search` before materializing a filesystem checkout.
- Make edits through `apply_patch` or structured change APIs when available.
- Materialize a filesystem checkout only for tools that require files, such as language servers, build systems, test runners, or browser tooling.
- Create snapshots after meaningful milestones, not after every noisy intermediate thought.
- Attach trace events, command output, test results, screenshots, and uncertainty to the current `AgentSession`.
- Preserve failed or superseded attempts as context instead of erasing them.
- Request review through `create_review_projection`.
- Publish through the native publication flow after the work thread is accepted.
- Export a legacy Git artifact only when an external Git-era system needs one.

The skill should also warn against defaulting to legacy habits:

- Do not create ad hoc Git branches as the task boundary.
- Do not use commits as scratchpad history.
- Do not hide test failures in PR descriptions.
- Do not discard failed attempts without attaching the reason to the work thread.

This mirrors how today's `AGENTS.md`, Claude skills, Cursor rules, and Windsurf workflows teach agents local project behavior. The difference is that this skill teaches the source-control substrate itself.

## MVP API Surface

The v1 API should be small and task-centered:

`repo.*`

- `repo.init`
- `repo.import_legacy_git`
- `repo.get`
- `repo.list_publications`

`thread.*`

- `thread.create`: create a task object with base repo/snapshot, source surface, owner, visibility, and acceptance criteria.
- `thread.get`
- `thread.list`
- `thread.update_status`
- `thread.abandon`

`workspace.*`

- `workspace.create_view`: create an isolated mutable or read-only overlay over a snapshot.
- `workspace.mount`
- `workspace.unmount`
- `workspace.read_path`: read file/blob contents from a snapshot or workspace view.
- `workspace.list_tree`: list directories/files from a snapshot or workspace view.
- `workspace.search`: search paths/content over an indexed snapshot and overlay.
- `workspace.apply_patch`: apply a patch to a workspace view under a work thread.
- `workspace.write_path`
- `workspace.diff`
- `workspace.run_command`: run a command against a pinned `ExecutionView` via `anvics-worker`.
- `workspace.snapshot`: freeze current workspace view into an immutable source snapshot.

`agent.*`

- `agent.start_session`
- `agent.attach_trace_event`: append an agent/tool/test event to an agent session.
- `agent.attach_command_event`: attach an externally-run or worker-run command event.
- `agent.finish_session`

`evidence.*`

- `evidence.attach`: attach command output, CI result, screenshot, benchmark, or other verification artifact.
- `evidence.get`

`review.*`

- `review.create_projection`: generate a human review bundle from final snapshot, diff, trace, evidence, and risk flags.
- `review.get_projection`

`publication.*`

- `publication.create`: publish accepted work inside the native collaboration model.
- `publication.get`
- `publication.export_legacy_git`: render accepted snapshot/change units as legacy commit/pushable Git output when needed.

`coordination.*`

- `coordination.list_signals`
- `coordination.create_conflict_session`
- `coordination.submit_resolution_contract`
- `coordination.create_resolution_thread`

`events.*`

- `events.subscribe`: subscribe to freshness, coordination, conflict, policy, review, publication, and workspace events.

The local daemon API should use JSON-RPC over local sockets or named pipes in v1. API DTOs should live in `anvics-api`, separate from `anvics-core` domain types, use `serde` JSON, version the API, and expose opaque IDs rather than storage details. MCP should be an adapter over this API.

Command execution through `workspace.run_command` has stronger provenance because Anvics pins the `ExecutionView`, applies execution policy, and captures file effects. Externally-run commands attached through `agent.attach_command_event` are allowed for external agent adoption, but should be marked with weaker provenance.

`FileEffectSet`s should expose neutral before/after path/object metadata. Classifications such as source, generated, cache, lockfile, config, evidence candidate, or unknown should come from config, policy, tools, humans, or agent claims. API clients should be able to fetch file effects, proposed `ChangeUnit`s, and classification provenance separately.

Semantic APIs such as `find_symbol`, `rename_symbol`, `call_graph`, or AST-aware conflict detection should be designed for v1.5. They are valuable, but v1 should not depend on language-specific correctness to prove the source-control layer.

## Failure And Security Model

- Every write must belong to a `WorkThread` and `AgentSession` or human actor.
- Every snapshot is immutable once created.
- Snapshots must remain state captures, not social history objects. They should not require human-grade messages, branch names, or review status.
- Every native publication or legacy export must point back to accepted evidence and review decisions.
- `apply_patch` must detect stale base, invalid patch, path permission denial, generated-file policy violations, and concurrent ownership conflicts.
- Trace and evidence attachment must support redacted/public projections and private raw artifacts.
- Filesystem materialization should be treated as a cache/view. The canonical source state is the snapshot plus operation log.

## Product Implications

- This is where Anvics becomes a new source-control system rather than agent observability around Git.
- Agents should not need to clone a full repo or manage worktrees just to inspect source, propose edits, or attach verification.
- Parallel agents should share immutable repo data and receive isolated `WorkspaceView` overlays, so running ten agents does not mean ten full repository copies.
- The API should let different agents operate against the same conceptual substrate even if their runtimes differ.
- GitHub/GitLab APIs remain important for legacy import/export, backup, and migration, but the internal API should model work, evidence, and review directly without assuming Git exists.

## MVP Impact

- Add `SourceSnapshot`, `WorkspaceView`, and `ChangeUnit` to the core primitive glossary.
- Treat the first implementation as a neutral source-control API substrate with optional filesystem materialization.
- Make cheap overlay-backed `WorkspaceView`s a v1 requirement; this is the direct replacement for Git worktrees.
- Ship a first-party agent skill/instruction bundle alongside the API so agents know how to use the new primitives.
- Build ingestion/publication around `WorkThread` and `NativePublication`, with `LegacyGitArtifact` as an adapter object.
- Defer language-semantic APIs until after snapshot/change/evidence flows work.

## Open Questions

- Should v1 expose this as REST, GraphQL, local RPC, CLI, or an embedded SDK first?
- Should `apply_patch` accept only unified diffs in v1, or also structured change units?
- How much search/indexing is needed before agents can work without a full checkout?
- Can external language servers operate against `WorkspaceView` without materializing a full filesystem?
- Should run/eval APIs execute commands directly, or only attach evidence produced by external runners in v1?
