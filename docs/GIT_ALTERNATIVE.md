# Product Thesis: Anvics

## Short Version

Anvics is an agent-native VCS: source control rebuilt around parallel AI agents, native work records, reviewable evidence, and conceptual publication.

Git was designed around humans intentionally preparing patches, arranging commits, naming branches, and resolving conflicts at human speed. AI agents expose the mismatch: they explore, backtrack, retry, generate many tiny edits, run tools, and work in parallel. The useful artifact is no longer just a file diff. It is the goal, plan, execution trace, tests, provenance, confidence, and final patch.

Git should be treated as a legacy interoperability, migration, and backup adapter. Anvics itself should be a new source-control system with its own repository identity, storage model, sync protocol, permissions, workspaces, traces, and review objects. Accepted outputs can still be rendered as commit/pushable Git artifacts, but Git should not define Anvics' architecture, data model, or mental model.

## Feasibility Read

This is feasible if "new system" means a real standalone source-control substrate, not a Git wrapper. Anvics should be thinkable without Git: native repositories, native sync, native review, native permissions, native source APIs. Git import/export exists because the world still runs on Git, not because Git is the foundation.

Several existing systems validate pieces of that path:

- Jujutsu (`jj`) proves that operation history and working-copy state can be separated from publication history.
- GitButler proves there is real demand for parallel branch/workspace UX on top of Git.
- Sapling proves that source-control UX, repository format, and large-scale storage can be separated.
- Pijul and Darcs prove there is value in modeling changes directly instead of only comparing snapshots.
- Radicle proves that forge objects such as issues, patches, and discussions can live beside code as signed, local-first collaboration data.
- OpenTelemetry, Bazel remote execution, and Nix show how to model provenance, reproducibility, and execution evidence.

The risky path is building a Git abstraction and calling it a replacement. The practical path is to build the right native system for day-to-day agent work, then provide Git exits for legacy hosts, CI, package publishing, mirrors, and backups.

After inspecting the adjacent codebases, the feasibility read gets sharper: the hard pieces already exist in partial form, but not in the right product shape. `jj` has the operation history, Pijul has durable change/conflict state, GitButler has hunk/workspace ownership and is already adding agent-trace metadata, and Sapling/Eden shows how far checkout/storage can be separated from UX at scale. The opportunity is to compose those lessons around a new primitive, not to reimplement every subsystem immediately.

### What Is Feasible Now

- Private isolated agent work threads.
- Automatic snapshots of file state.
- Operation logs for every agent command, test, retry, and edit.
- Semantic summaries and condensed diffs for human review.
- Legacy Git import/export for teams that still need GitHub/GitLab/CI compatibility.
- Multiple workspaces backed by cheap local or remote copy-on-write storage.

### What Is Hard But Plausible

- Fine-grained private/public visibility inside one repo.
- Work replay onto a newer base.
- Structured review objects stored in the source-control layer.
- Provenance-aware audit trails that remain readable.
- Partial checkouts and monorepo-scale performance.
- Commit synthesis: turning messy agent exploration into clean human-grade Git history.

### What Is Still Researchy

- Semantic merge that understands code intent.
- CRDT-like concurrent editing for code at high agent counts.
- Conflict resolution at write time instead of merge time.
- Automatically trustworthy model-generated intent graphs.
- Language-aware dependency/conflict detection that works across ecosystems.

## Core Thesis

Anvics should track intent, provenance, execution, and review as first-class primitives, with file snapshots as one materialized view.

Snapshots are not commits with a new name. A commit is a publication object: a meaningful change intended for project history. A snapshot is a state-capture object: the exact source tree a workspace had at a moment in an agent's work thread. Snapshots can be automatic, private, ugly, superseded, abandoned, compared, replayed, or used as evidence. They should not require human-grade messages, branch names, social review semantics, or permanent placement in project history.

Anvics should feel less like managing branches and more like managing parallel, reviewable work threads:

- Start work without deciding branch names, commits, or history structure.
- Let humans and agents edit freely in cheap isolated workspaces.
- Capture every meaningful state automatically or through harness-aware checkpoints.
- Represent work as tasks/proposals with intent, traces, tests, and semantic summaries.
- Delay public disclosure until work is ready.
- Publish to other systems only when needed; Git/GitHub export is a legacy compatibility path, not the internal unit of work.

## What Theo Seems To Want

### 1. Privacy is a source-control primitive

"Open source" should not mean every line, every in-flight PR, every package, and every security fix is public at all times.

Anvics should support:

- Private subtrees or packages inside otherwise public repos.
- Hidden in-flight work for open-source projects.
- Embargoed security fixes.
- Safer environment/secret handling.
- Access rules at the file, package, workspace, thread, or proposal level.

This is not just repository permissions. It is visibility over time.

### 2. Commits and branches are the wrong working primitives

Commits force developers to narrate history while they are still discovering the answer. Branches force up-front structure onto work that may change shape.

The better primitive is continuous captured work:

- Work is always tracked.
- Snapshots are cheap.
- History can be organized later.
- The user can edit code first and decide how to package it afterward.

`jj` points in the right direction because it reduces staging/commit ceremony and treats work-in-progress more naturally.

### 3. Worktrees should disappear into the substrate

Agents need parallelism. Current Git worktrees make parallelism feel bolted on.

Anvics should offer:

- Cheap copy-on-write workspaces.
- Multiple concurrent work threads over the same codebase and logical base.
- No full repo copy per agent.
- API-level source views and VFS projections before fallback filesystem materialization.
- Per-agent mutable overlays on shared immutable snapshots.
- Automatic freshness against main.
- Fast discard, fork, replay, and compare.
- Conflict and ownership detection at the `ChangeUnit`/hunk/path level.
- No manual worktree bookkeeping.

Workspaces should feel native, not like users are managing filesystem hacks. An agent should be able to start work in milliseconds against the same repo state as ten other agents, mutate its own overlay, run tools through a VFS projection, and throw the whole attempt away without leaving branches, worktrees, stashes, or half-clean directories behind.

The important lesson is not "build a mounted filesystem first." It is "source state must be projectable." Materialized directories can validate the product loop, but a VFS or equivalent projection is the path to cheap, lazy, path-compatible workspaces for large repos and many concurrent agents.

### 4. Source control needs an API, not just a filesystem

Agents should not need a full OS and filesystem just to inspect or modify source.

Anvics should expose source-control operations as APIs. A filesystem checkout should be one materialized view over source state, not the canonical interface.

This does not necessarily mean "store code in SQLite instead of files." The storage engine could be content-addressed storage, SQLite, object storage, a graph store, or something purpose-built. The product point is higher level: agents should not have to impersonate humans typing shell commands in a worktree just to read, edit, fork, snapshot, evaluate, or publish code. Files remain the compatibility layer for editors, language servers, build tools, and humans.

Anvics should also ship an agent skill or equivalent onboarding module. APIs alone are not enough: agents need concise procedural guidance that tells them to use work threads, snapshots, evidence bundles, native publication, and optional legacy Git export instead of falling back to `git worktree`, ad hoc shell edits, noisy commits, and PR-description archaeology.

- Read files.
- List trees.
- Search code.
- Write files.
- Apply patches.
- Create snapshots.
- Fork workspaces.
- Attach tool output.
- Run tests.
- Request review.
- Publish or export.

The API model should distinguish immutable `SourceSnapshot`s, mutable `WorkspaceView`s, owned `ChangeUnit`s, native `NativePublication`s, and optional `LegacyGitArtifact`s. This is the clearest place where Anvics stops being "Git with agent logs" and becomes a new source-control substrate.

## Broader Reply-Signal

The strongest recurring theme in the replies was that agents make Git history noisy.

People running coding agents report sessions ending with dozens of micro-commits. The resulting history is technically complete but semantically useless. Humans do not want to review every exploratory edit. They want to understand what the agent was trying to do, what it tried, what worked, what failed, and why the final change is safe.

That implies new primitives:

- Intent: the task, spec, or user request that caused the work.
- Plan: the agent's intended approach.
- Provenance: model, prompt, tools, dependencies, environment, and permissions.
- Execution trace: commands, tests, errors, retries, intermediate observations.
- Evaluation: tests, static analysis, benchmarks, screenshots, human checks.
- Semantic diff: what changed conceptually, not only line-by-line.
- Confidence/risk: what the agent believes is safe, uncertain, or unverified.
- Review object: structured comments and decisions living inside source control.

In this model, a patch is the output of a work record. It is not the whole record.

## AI Coding Agents as the Target User

The target user is not only "a developer who dislikes Git." The target user is an autonomous coding agent plus the human who supervises a fleet of them.

Current coding-agent products are converging on the same workflow shape:

- A task is assigned from chat, an issue, an IDE, Slack, a CLI, or an API.
- The agent gets an isolated local or cloud environment with a repo checkout.
- It reads repo instructions such as `AGENTS.md`, `CLAUDE.md`, rules, memories, skills, or playbooks.
- It plans, edits files, runs commands, tests, linters, typecheckers, browsers, and review tools.
- It exposes some live progress: plan, logs, diffs, command output, screenshots, or session state.
- It emits a branch, commit stack, patch, pull request, or locally applicable diff.
- A human reviews, requests revisions, accepts, rejects, reruns, or hands the work to another agent.

That pattern appears across Codex, Claude Code, GitHub Copilot cloud agent, Cursor/Windsurf, Devin, and Jules. The differences are mostly surface area: terminal vs IDE vs web vs GitHub issue vs Slack. The underlying source-control pressure is the same.

### What Agents Need From Source Control

Agents need source control to be a stateful execution substrate, not just a commit graph.

They need:

- Task identity: what was delegated, by whom, from where, and with what acceptance criteria.
- Environment identity: repo state, dependencies, secrets policy, sandbox, machine image, cached setup, and tool versions.
- Instruction identity: which `AGENTS.md`, `CLAUDE.md`, rules, skills, memories, hooks, playbooks, or org policies shaped the work.
- Workspace identity: which isolated filesystem/API view the agent wrote to.
- Execution identity: commands, tests, browser actions, tool calls, failures, retries, and observations.
- Change identity: which files, hunks, symbols, generated artifacts, and review comments belong to which task.
- Publication identity: which final Git commits/branches/PRs correspond to the richer internal record.
- Cost and quota identity: session time, premium requests, Actions minutes, ACUs, cloud VM time, or equivalent spend.

Git tracks almost none of this directly. Today's agents compensate with PR descriptions, logs, screenshots, hidden metadata, transcripts, and platform-specific session pages. That is exactly the gap.

### Product Implications

Anvics should treat agent work as a first-class runtime object.

Important primitives:

- `AgentSession`: a running or completed agent execution with inputs, permissions, environment, trace, and output.
- `WorkThread`: the task-level object that may contain multiple sessions, retries, subagents, and human edits.
- `Attempt`: one run against a base state, with a plan, trace, snapshots, and result.
- `SourceSnapshot`: immutable source tree state capture inside a work thread, not a publication object.
- `WorkspaceView`: mutable or read-only API/filesystem view over a snapshot, implemented as a cheap overlay rather than a full repo copy.
- `ChangeUnit`: file/hunk/symbol-level change owned by a work thread or attempt.
- `EvidenceBundle`: tests, command outputs, screenshots, browser traces, benchmarks, static analysis, and human checks.
- `InstructionBundle`: the resolved instructions and policies active for a session.
- `EnvironmentSnapshot`: setup state, dependency lockfiles, toolchain versions, container/VM image, env vars policy, and cache keys.
- `ReviewProjection`: the human-readable summary/diff/risk view generated from the richer record.
- `NativePublication`: accepted work published inside the system's own collaboration model.
- `LegacyGitArtifact`: commit, branch, patch stack, or PR emitted only for Git-era compatibility.

Anvics should not assume that the final PR is the truth. A PR is a legacy projection. The truth is the work thread and its accepted native publication.

### Where Existing Agents Point

Codex points toward parallel cloud workspaces, built-in worktrees, task evidence, and AGENTS.md-driven repo behavior. The key signal is that each task can run independently in an isolated environment, commit its changes there, and provide citations to terminal logs/test outputs before the user opens a PR or integrates locally.

Claude Code points toward tool-native local/remote agents: terminal, IDE, desktop, web, GitHub Actions, Slack, MCP, hooks, skills, memory, and multiple background agents. The source-control system should expect agents to be launched from many surfaces and to call external systems as part of normal work.

GitHub Copilot cloud agent points toward GitHub-native delegation: assign work from issues or chat, run in GitHub Actions, plan, change a branch, and optionally open a PR. It also shows that "cloud agent" and "IDE agent" are distinct modes with different source-control needs.

Cursor and Windsurf point toward IDE-native orchestration: rules, skills, MCP, local checks, worktrees, multiple agent instances, command centers, and quick review. The need here is fast local iteration plus parallelization without destroying the developer's current working directory.

Devin and Jules point toward long-running cloud sessions: repo indexing, organization permissions, audit logs, session APIs, scheduled tasks, suggested tasks, environment snapshots, PR metrics, and human takeover. The source-control layer needs durable session state and enterprise controls, not just a nicer branch UI.

The empirical GitHub studies are also useful. One recent dataset reports hundreds of thousands of agent-authored PRs across Codex, Devin, Copilot, Cursor, and Claude Code; other studies suggest merge success depends heavily on task type, with documentation, CI, build, and feature work behaving differently from bug fixes and performance changes. That means the system should classify work by task type, risk, and evidence, not treat all diffs as equivalent.

Sources:

- https://openai.com/codex/
- https://openai.com/index/introducing-codex/
- https://code.claude.com/docs/en/overview
- https://code.claude.com/docs/en/github-actions
- https://docs.github.com/en/copilot/concepts/agents/cloud-agent/about-cloud-agent
- https://cursor.com/docs
- https://docs.windsurf.com/windsurf/agent-command-center.md
- https://docs.devin.ai/get-started/devin-intro
- https://jules.google/docs/
- https://arxiv.org/abs/2602.09185
- https://arxiv.org/abs/2601.15195
- https://arxiv.org/abs/2602.08915

## Adjacent Technology Lessons

### Jujutsu (`jj`)

`jj` is the most important system to study.

Relevant ideas:

- The working copy is represented as a real commit.
- Most commands automatically snapshot working-copy changes.
- The operation log records repo-level operations and enables undo/restore.
- Multiple workspaces can share one repo.
- Legacy Git compatibility is a first-class migration path.

Product lesson: do not make users decide when work becomes "real." Treat work-in-progress as already captured, then let the user reshape and publish it later.

What not to copy blindly: `jj` is still a developer-facing VCS. It does not solve agent provenance, review evidence, private proposal workflows, or API-native source operations by itself.

Source: https://www.jj-vcs.dev/latest/

### GitButler

GitButler is the nearest product wedge.

Relevant ideas:

- Multiple branches can be applied to one working directory.
- Changes can be assigned to separate branch lanes.
- Each lane has its own staging area.
- Branches are extracted from a combined working state.

Product lesson: users want parallel work without branch checkout ceremony. For agents, the lane should become a work thread containing code, trace, tests, review, and publication state.

What not to copy blindly: GitButler is still fundamentally branch-oriented. Anvics should treat branches as export artifacts, not the core unit.

Source: https://docs.gitbutler.com/features/branch-management/virtual-branches

Code-level note: GitButler is closer to this thesis than the public positioning suggests. Its hunk-assignment crate gives hunks stable identities and assigns them to stacks/branches. Its hunk-dependency code models a workspace as multiple stacks merged into one worktree. Its oplog snapshots GitButler state transactionally. Most interestingly, its `but-agentlog` crate parses Codex/Claude transcript records, classifies messages/tool calls/tool results, records command outcomes, redacts records, and projects selected evidence into Git metadata linked to branches/reviews/changes.

That is a strong signal that "agent trail attached to source control" is not theoretical. The product gap is that GitButler currently makes it metadata around Git/branches. In Anvics, that trail should be the primary object, with Git output as a legacy projection.

### Sapling

Sapling is important for scale and UX.

Relevant ideas:

- Meta built it for monorepo scale and developer velocity.
- It separates source-control UX from repository format.
- It supports Git interop while using a different internal model.
- Smartlog and Interactive Smartlog make local change graphs legible.
- It is most proven in corporate, always-online, rebase-heavy monorepo environments.

Product lesson: the graph UI matters. Agent work will need an interactive smartlog-like view, but the nodes should be work threads, attempts, evaluations, and accepted outputs, not only commits.

What not to copy blindly: Sapling's strengths come from a strong organizational model and server infrastructure. A startup wedge should not require Meta-scale deployment assumptions.

Source: https://sapling-scm.com/docs/introduction/

Code-level note: Sapling/Eden reinforces that source-control UX and checkout mechanics can be separated. Smartlog makes the local graph legible by hiding irrelevant history and showing the work that matters. Sapling's stack workflow makes edit/split/fold/absorb operations feel normal. EdenFS is a virtual filesystem that lazily fetches tree/blob data from a backing store and can report status, switch commits, notify tools, and return hashes without eagerly materializing the whole repo.

For this product, that says two things. First, an agent smartlog should probably be a first-class UI: work threads, attempts, evaluations, risks, and accepted outputs. Second, virtual/remote-backed checkout is powerful but not the wedge. It becomes important once the product targets very large repos or cloud agents at scale.

### Pijul and Darcs

Pijul and Darcs are important for change theory, not necessarily product UX.

Relevant ideas:

- They model patches/changes rather than only snapshots.
- Pijul's change commutation aims to let independent changes apply in any order.
- Pijul treats conflicts as part of repository state rather than only as temporary working-directory markers.
- Pijul's own FAQ is explicit that it does not solve semantic correctness; language meaning still requires tests and review.

Product lesson: agent work should probably be stored as a durable change object with dependencies, not only as a final snapshot. Reordering, replaying, unapplying, and cherry-picking agent work will matter.

What not to copy blindly: patch algebra is not enough. Agents need semantic context, traces, tests, and intent. A mathematically sound text merge can still produce broken software.

Code-level note: Pijul is the most interesting system from the inside. Its core object is not a diff hunk in the Git sense. A change is made of graph atoms: new vertices and edge maps. Vertices represent introduced file content ranges; edges encode ordering, folder structure, deletion, parenthood, and pseudo/conflict connectivity. Changes carry dependencies, extra-known changes, metadata, content hashes, and typed hunks.

The most useful idea is that conflicts are real repository state. Pijul has explicit conflict variants such as name conflicts, order conflicts, zombie files, cyclic conflicts, and multiple names. Conflict resolutions are themselves typed recorded hunks, such as solving an order conflict or name conflict, and can be applied elsewhere or unrecorded independently. Its "zombie" machinery is especially relevant: deleted context that later receives independent changes is represented explicitly instead of being flattened into confusing merge text.

For agents, this is a big clue. Stale-context retries, overlapping edits, resurrected deleted code, and conflict resolutions should be durable objects with provenance. But Pijul's graph model is low-level and byte-oriented. It is a substrate inspiration, not the UX. A product should expose "this agent edited a stale function after another thread deleted it," not raw edge flags.

Sources:

- https://pijul.org/manual/why_pijul.html
- https://pijul.org/faq
- https://darcs.net/Using/Model

### Radicle

Radicle is relevant to the "GitHub is dying" part of the thesis.

Relevant ideas:

- It is a peer-to-peer code collaboration stack built on Git.
- Repositories are replicated across peers.
- Social artifacts are signed and stored in Git.
- Collaborative Objects can model issues, discussions, patches, and code review.

Product lesson: review, issue, proposal, and discussion objects should not have to live only in a centralized SaaS database. Agent work records can be durable collaboration objects with cryptographic authorship.

What not to copy blindly: decentralization is not the wedge for agent productivity. It is a useful durability and ownership property, not the first customer pain.

Source: https://radicle.dev/

### OpenTelemetry

OpenTelemetry is the right mental model for agent traces.

Relevant ideas:

- A trace is a hierarchy of spans.
- Spans have names, start/end timestamps, attributes, events, links, and status.
- Span events represent meaningful points in time.
- Span links model causal relationships across async work.

Product lesson: an agent work thread should emit trace-like records. Commands, tool calls, test runs, file edits, model calls, retries, and review actions are spans/events. This gives the review layer evidence instead of vibes.

What not to copy blindly: raw traces are too noisy for product UX. The system needs summarization, filtering, and risk extraction on top.

Source: https://opentelemetry.io/docs/concepts/signals/traces/

### Bazel Remote Execution and Content-Addressed Action Records

Bazel remote execution is relevant because agent work needs reproducible evidence.

Relevant ideas:

- Build and test actions can run remotely.
- Remote execution gives teams consistent execution environments.
- Build outputs can be reused across the team.
- The model depends on explicit inputs, actions, and outputs.

Product lesson: test/eval evidence should be attached to the exact code snapshot, environment, command, and inputs that produced it. A green check without reproducible context is weak evidence.

What not to copy blindly: Bazel requires build-system discipline many projects do not have. The agent layer should degrade gracefully for ordinary repos.

Source: https://bazel.build/remote/rbe

### Nix and Reproducible Environments

Nix is relevant to provenance.

Relevant ideas:

- Deterministic dependency references and sandboxed builds help produce reproducible artifacts.
- Reproducibility is not automatic; timestamps and other nondeterminism can still leak into outputs.

Product lesson: source control for agents should capture environment fingerprints. The question is not only "what code changed?" but "under what dependency and execution environment was this change produced and verified?"

What not to copy blindly: making every user adopt Nix would be too high-friction. Environment capture should support Nix-like rigor when available and lighter fingerprints when not.

Source: https://reproducible.nixos.org/

### CRDTs: Yjs and Automerge

CRDTs are relevant to concurrent workspaces.

Relevant ideas:

- Shared data types can be manipulated concurrently.
- Updates can sync automatically and merge without text-level merge conflicts.
- These systems are strongest for collaborative documents and structured shared state.

Product lesson: CRDTs may be useful for live workspace metadata, review comments, task state, cursors, and maybe active text buffers.

What not to copy blindly: code conflicts are often semantic. Automatically merging text does not mean the program still works.

Sources:

- https://docs.yjs.dev/
- https://automerge.org/

## Code-Level Lessons From Repository Inspection

### 1. Pijul: a serious merge layer becomes a graph engine

Pijul confirms that moving beyond Git's snapshot/diff model quickly becomes a graph problem.

Internally, Pijul's change records contain atoms like `NewVertex` and `EdgeMap`. The repository content graph tracks introduced byte ranges as vertices and relationships as edges. Edge flags encode whether something is a block, folder, parent edge, deleted edge, or pseudo edge used to preserve connectivity and conflict state.

Typed hunks include file moves/adds/deletes, edits, replacements, name-conflict solutions, order-conflict solutions, unresolved conflict states, and zombie resurrection. That is the important part: resolution is not just "remove conflict markers from a file." It is a portable, recordable change.

Product lesson: Anvics should model conflict, stale context, replay, and resolution explicitly. It should know that a thread edited deleted code, resolved an ordering conflict, or revived a removed file. Those should be reviewable states with authorship and evidence.

MVP caution: do not start by building a full Pijul-class patch algebra. It is deep infrastructure. Start by storing structured metadata about conflicts and hunk ownership on top of a simpler snapshot model, then graduate hot paths to graph-native operations if the product proves the need.

### 2. Jujutsu: operation history is separate from commit history

`jj`'s strongest contribution is not merely "better Git UX." It separates repository operations from commits.

The working copy is automatically snapshotted into commits. Repo-modifying commands create operation-log entries. The operation log tracks the repo view, including refs and each workspace's current working-copy commit, which enables undo/restore and handles concurrent operations more cleanly.

Product lesson: an agent work system needs at least two logs:

- A source-state operation log: workspace created, snapshot taken, base advanced, replay attempted, output accepted.
- An agent trace log: model calls, tool calls, commands, tests, observations, retries, and review decisions.

These logs should link to the same snapshots, but they should not be collapsed into Git commits. Git history is the publication artifact, not the complete record of how work happened.

### 3. GitButler: hunk ownership is the nearest practical wedge

GitButler's internals show that "many branches in one worktree" immediately requires stable hunk identity, assignment, dependency tracking, and transactional state recovery.

Its hunk-assignment model gives a hunk an ID, path, branch/stack assignment, line metadata, and diff payload. Its workspace model treats several stacks as merged into one worktree. Its hunk-dependency code has to reason about which stack or commit a worktree hunk belongs to as the combined workspace changes.

This maps almost directly to agents. Replace "branch lane" with "work thread." Every edit should have an owner, purpose, base, and confidence. Hunks can move between threads, be split, absorbed, superseded, or exported.

The surprising find is GitButler's agent log. It already parses Codex/Claude transcript JSONL, classifies tool calls, stores bounded/redacted session records, indexes sessions by branch/review/change, and projects an "Agent Trail" into Git metadata. That is extremely adjacent.

Product lesson: the initial wedge could be "GitButler for agent work, but trace-first." The durable product object is not a branch. It is a work thread with hunk ownership, execution evidence, review state, and publication output.

### 4. Sapling/Eden: scale is real, but probably second-order

Sapling's Smartlog proves the value of a graph view that hides irrelevant history and centers the user's active work. Its stack operations show that editing history can be normal when the UX makes the graph understandable.

EdenFS proves that checkout is a separable subsystem. It serves huge repos through a virtual filesystem, lazily fetching blobs/trees from an authoritative backing store, tracking modified files, switching source states, notifying downstream tools, and exposing object hashes efficiently.

Product lesson: source control for agents should not assume "one full local checkout per agent forever." Eventually, cloud agents and monorepos need API-addressable partial materialization. But this should not dominate the first product unless the initial customer is a very large monorepo.

### 5. The combined architecture

The product wants four internal layers:

1. Source-state operation log, inspired by `jj`.
2. Agent trace log, inspired by OpenTelemetry and GitButler's agent log.
3. Hunk/change ownership graph, inspired by GitButler and eventually Pijul.
4. Publication layer, exporting clean Git branches, PRs, patch stacks, or review bundles.

The core object should be a `WorkThread`:

- Task/intent/spec.
- Base source state.
- Visibility policy.
- Workspace materialization.
- Snapshots.
- Owned hunks/changes.
- Agent/human trace.
- Test/eval evidence.
- Conflict/stale-context state.
- Review decisions.
- Git/PR export mapping.

This is the product primitive that Git lacks.

## Revised Architecture After Code Inspection

The architecture should avoid two traps:

- Trap 1: "Build a universal Git replacement from scratch before proving the workflow."
- Trap 2: "Just annotate Git commits with AI metadata."

The better approach is Anvics: a new source-control system that owns the agent workflow internally and treats Git as a legacy adapter at the boundary.

Early implementation:

1. Store snapshots in the system's own model. Prefer a native content-addressed or graph store; use hidden Git commits only as a disposable implementation bridge, not as the product premise.
2. Store operation history separately, `jj`-style.
3. Store agent traces separately, OTel-style.
4. Store hunk identity and work-thread assignment, GitButler-style.
5. Store conflict/stale-context records as structured metadata before implementing a full graph merge engine.
6. Export accepted work through adapters: Git commits/branches/PRs for legacy systems, native review/publication objects for the product itself.

Later implementation:

1. Add Pijul-like graph-native change objects where they materially improve replay, unapply, conflict portability, or agent concurrency.
2. Add Eden-like partial materialization for cloud agents and huge repos.
3. Add semantic/language-aware conflict prediction as a higher layer over the change graph, not as the foundation.

This makes the first version feasible while still being honest that Anvics is a real new system. Legacy Git compatibility is an exit ramp, mirror, and migration aid, not the architecture's center of gravity.

## Rust Architecture

Anvics should be implemented as a Rust local service plus clients.

The local daemon, `anvicsd`, owns repository state. The CLI, API/MCP server, VFS mount, UI, and agent integrations are clients of that service. Direct mutation of repo storage by clients should be avoided because it bypasses policy, event ordering, overlay coordination, and crash recovery.

Rust crate boundaries should enforce the product boundary:

- `anvics-core`: Git-free domain types, IDs, events, actor attribution, policies, and primitive schemas.
- `anvics-store`: content-addressed object store, event log, overlay store, evidence references, and transactional persistence.
- `anvics-workspace`: `WorkspaceView`, per-thread overlays, snapshot creation, replay attempts, and materialization coordination.
- `anvics-vfs`: VFS mount/projection layer for developer-tool compatibility in monorepos.
- `anvics-index`: path, search, hunk, symbol, dependency, and change-unit indexes.
- `anvics-agent`: agent sessions, context packs, command events, evidence bundles, and learning notes.
- `anvics-conflict`: coordination signals, conflict sessions, resolution threads, and structured resolution contracts.
- `anvics-policy`: read/write/command/evidence/publication scopes, gates, redaction, and approvals.
- `anvics-sync`: local-first object sync, team relay protocol, visibility filtering, and replication state.
- `anvics-api`: local RPC/HTTP/MCP API over the daemon.
- `anvics-cli`: thin command-line client over the API.
- `anvics-legacy-git`: import/export adapter for Git repositories, commits, branches, patch stacks, and PR flows.

The core rule: `anvics-core`, `anvics-store`, `anvics-workspace`, `anvics-vfs`, `anvics-agent`, `anvics-conflict`, `anvics-policy`, and `anvics-sync` must not depend on Git. `anvics-legacy-git` depends outward on Anvics primitives. Git never flows inward as the data model.

A projection backend abstraction is a strategic v1 product direction, not a late optimization. Anvics must satisfy the core premise that the filesystem is a projection over source state, not the canonical source interface. The first real VFS backends should target Linux FUSE and macOS macFUSE, with `fuser` as the initial Rust candidate, if the proxy-mounted spike confirms that mounts pay for their complexity. For MVP 0, however, VFS maturity should not block product validation: a `materialized_dir` backend can prove the work-thread/review loop while Linux and macOS VFS prototypes run in parallel. Windows ProjFS should be researched/prototyped early, but Linux and macOS are the first real VFS targets.

The agentvfs deep dive sharpened this: VFS should sit behind a proxy-mounted workspace runtime, not become a workflow agents manage directly. The preferred path is for `command run` or a later proxy execution flow to resolve the workspace, mount or materialize it, classify the command, run it, summarize changed paths, attach evidence, and clean up. `workspace mount` remains useful as a low-level/debug escape hatch.

`anvicsd` should expose JSON-RPC over local sockets or Windows named pipes for v1. MCP should be a thin adapter over the same API, not the daemon's core protocol. API method names should be Anvics-native: `thread.*`, `workspace.*`, `review.*`, `publication.*`, and `coordination.*`, with Git only under explicit legacy export/import methods.

## Local-First Service, Remote Collaboration

Anvics should be local-first and distributed. Git is one familiar proof that this property matters, but it should not be the architectural template.

The right shape is:

1. **Local agent VCS service**
   - Runs on the developer or agent machine.
   - Owns source snapshots, work threads, workspace views, change units, agent sessions, traces, evidence bundles, review projections, native publication, and legacy Git export.
   - Exposes the source-control API to agents and tools.
   - Materializes filesystem checkouts only as compatibility views.
   - Shares immutable repo data across all local agents while giving each agent an isolated mutable overlay.

2. **Remote collaboration service**
   - Syncs the richer internal model for teams.
   - Owns shared work threads, review state, policies, RBAC, evidence retention, audit logs, accepted snapshots, and team-visible projections.
   - Acts as the collaboration service for agent work, not as a Git remote.

3. **Legacy Git adapters**
   - Import from GitHub/GitLab/local Git.
   - Track upstream Git refs when needed.
   - Export accepted snapshots as commits, branches, patch stacks, or PRs.
   - Preserve durable links back to the richer work-thread record.

The important distinction: Git syncs commits; Anvics syncs work. A native repository should be able to exist, sync, review, and publish without a Git remote at all.

Sync should work in multiple directions, but not everything should publish by default. Failed attempts, raw traces, secret-adjacent logs, speculative work, abandoned attempts, and private scratch state should remain local/private unless policy or user action exposes them. The publishable surface is the accepted snapshot, summarized evidence, selected trace, review projection, and native publication; a `LegacyGitArtifact` is derived only when needed.

This gives Anvics the operational benefit of distributed work without inheriting Git's mismatch for agents: treating commits as the only durable unit of meaning.

This also means Anvics must resist turning snapshots into a parallel commit history. If snapshots gain commit-like messages, branch rituals, merge ceremonies, and social review status, the product has recreated the problem under different names.

## Git As Legacy Adapter

Anvics should be internally sovereign and externally compatible.

Inside the system:

- Work is represented as work threads, snapshots, traces, reviews, conflict states, and hunk/change ownership.
- Visibility, provenance, replay, and review evidence are source-control primitives.
- Agents interact through APIs and isolated workspaces rather than branches and staging.
- History can be messy while discovery is happening and curated when work is accepted.

At the boundary:

- Accepted work can be rendered as one Git commit, a clean commit stack, a branch, a patch series, or a PR.
- The result can be pushed to GitHub/GitLab/remote Git without requiring downstream tools to understand the internal model.
- Commit messages can include durable references to the richer work-thread record.
- CI, package publishing, compliance, and existing review flows continue to work.

This distinction matters. If Git is the internal model, Anvics becomes "better Git UX." If Git is merely a legacy adapter, Anvics can invent the right primitives for agents while still providing escape hatches into today's ecosystem.

## Product Shape

### Internal System Model

Internally, the system is an append-only event log with deterministic snapshots.

The event log records what happened. Snapshots record what the source tree looked like. Review projections explain what matters. Native publications publish accepted work. Legacy Git artifacts adapt accepted work for Git-era systems. Those roles should stay separate.

Events could include:

- Workspace created.
- File changed.
- Agent command run.
- Test failed.
- Test passed.
- Dependency graph changed.
- Human commented.
- Agent retried.
- Proposal submitted.
- Review accepted.
- Patch published.

Snapshots are materialized from the log. Native publications can be created from accepted snapshots. Commit/pushable legacy Git artifacts can be rendered from native publications when needed.

### Command Execution Model

Anvics should provide command execution, but it should not become a full agent runner in v1.

When possible, agents should run commands through Anvics against a pinned `ExecutionView`: a specific `WorkspaceView` overlay version plus environment fingerprint and execution policy. Anvics records the command, actor attribution, environment policy, output, exit status, changed files, and evidence artifacts.

External agents may still run commands themselves in a VFS mount or fallback materialized directory and then attach a `CommandEvent`. That path is useful for adoption, but provenance is weaker because Anvics did not mediate the execution.

Command execution should run in a separate `anvics-worker` process coordinated by `anvicsd`. The worker runs commands, streams output and file effects back to the daemon, and can later evolve into remote/cloud workers without changing the core execution model. V1 command execution should enforce policy and capture provenance, but it should not claim to be a full malicious-code sandbox. Strong isolation belongs to containers, VMs, devcontainers, CI, or cloud sandboxes.

Controlled command execution over isolated workspace state is as important as the mount mechanics. A mount lets tools see files; the runtime turns that tool execution into attributable, reviewable source-control evidence.

Command side effects should first become neutral `FileEffectSet`s: before/after path and object metadata, not language-specific conclusions. Anvics can then apply config and policy, agents can add classifications and rationale, and only trackable source-relevant effects become `ChangeUnit`s. Ignored/cache output stays out of source changes, evidence candidates become attachment suggestions, and unresolved or sensitive effects are gated before native publication.

### Repository Configuration Model

Anvics should treat `Repository` as the top-level storage, sync, and security unit. Inside a repository, it should model `SourceRoot` and `PackageBoundary` objects so monorepos and multi-package systems are understandable from the start.

Portable source-tree configuration should live in `anvics.toml` when present. It should describe repo structure and tool compatibility:

- Source roots and package boundaries.
- Tracked generated source paths.
- Untracked generated output/cache paths.
- Evidence candidate paths such as test reports and coverage output.
- Tool hints such as test and lint commands.
- Agent instruction references such as `AGENTS.md`.

Private/local/org policy should not live in source-tree `anvics.toml`. Secrets policy, RBAC, evidence retention, visibility rules, relay credentials, and machine-local settings belong in Anvics metadata and policy stores.

`anvics.toml` should be optional but encouraged. Anvics can infer basic structure when it is absent, and `anvics repo init` or `anvics repo doctor` should generate or improve it. Agents may propose edits to `anvics.toml`, but those edits are source-control configuration changes and should trigger elevated review or policy gates before native publication.

The first implementation should apply root `anvics.toml` conservatively: use accepted repo config to classify generated, ignored/cache, and evidence-candidate paths, but do not let an agent's workspace edit to `anvics.toml` reinterpret that same review.

`anvics repo doctor --path <path>` should make this interpretation inspectable. Agents and operators can ask how paths classify before review time, without needing to infer behavior from hidden heuristics.

Generated paths should be split by intent. Tracked generated source can be part of native publications, but agents should normally modify the generator/schema/template and rerun generation rather than editing generated files directly. Direct edits to tracked generated source should require explicit rationale and policy approval. Untracked generated outputs stay out of source changes. Evidence candidate paths can produce attach suggestions, with retention and sharing controlled by evidence policy.

Reviews should show both layers: the full classified `FileEffect` list for transparency, and the smaller `ChangeUnit` list for source-control decisions. That keeps generated output, cache changes, and evidence candidates visible without forcing every tool artifact into the source-change model.

### Agent Adoption Layer

The system should include a small skill/instruction package for agents.

That skill should teach agents:

- Treat a thread as the task boundary.
- Read and write through source-control APIs when available.
- Use the workspace path Anvics provides; it may be materialized or mounted.
- Use low-level `workspace mount` only when instructed or when debugging a projection issue.
- Create snapshots at meaningful checkpoints.
- Attach compact command output, tests, screenshots, traces, and uncertainty as evidence.
- Keep failed attempts linked to the work thread.
- Publish natively only after review/acceptance; export legacy Git artifacts only when needed.

The skill should budget tokens aggressively. Raw logs, screenshots, traces, and reports should be stored by reference; review projections should contain short summaries, command names, exit status, key excerpts, and links. Agents should not dump full transcripts into review objects unless explicitly requested.

This is the bridge between the Anvics model and existing agents. Without it, even capable agents will keep imitating human Git workflows.

### Workspace Model

Replace branches/worktrees with work threads.

A work thread is:

- Based on a specific repo state.
- Isolated by default.
- Cheap to fork.
- Continuously captured.
- Allowed to be private, shared, or public.
- Reviewable as both semantic record and file diff.

Threads can be created by humans, agents, or automation. They can converge, be superseded, or be abandoned without polluting permanent history.

Under the hood, a work thread should not imply a complete repository copy. The expected implementation is shared immutable source storage plus per-thread overlays:

- `SourceSnapshot`: immutable base tree shared by many agents.
- `WorkspaceView`: mutable overlay for one thread/session.
- Workspace projection: materialized directory for MVP/fallback, VFS mount for cheap lazy path-compatible access when available.
- `ChangeUnit`: tracked changed path/hunk/symbol owned by the thread.

This is the replacement for Git worktrees. Agents get isolation and parallelism without multiplying whole checkouts or requiring users to clean up worktree directories.

### Review Model

PRs should be native source-control objects, not only social objects in GitHub.

A review should include:

- Intent/spec.
- Final diff.
- Condensed semantic summary.
- Test/eval evidence.
- Trace/provenance.
- Risk flags.
- Inline code comments.
- Human/agent decisions.

This lets humans review the reasoning and evidence, not just the patch.

### Conflict Model

Git resolves conflicts late, at merge time. Agent systems may need earlier coordination.

Possible directions:

- File or symbol ownership during active work.
- CRDT-like write-time collaboration for high-concurrency edits.
- Semantic merge based on AST/dependency graphs.
- Proposal consensus before code is materialized.
- Deterministic replay of event logs against a newer base.

The key is that "merge conflict" should not be the first moment the system understands two agents disagreed.

### Storage Model

The storage layer should likely be:

- Content-addressed.
- Copy-on-write.
- Efficient for large monorepos.
- Remote-first but locally cacheable.
- Addressable by API.
- Capable of partial checkout/materialization.

Git import/export matters for migration, backup, mirrors, and legacy CI, but Git object storage should not dictate the product model.

## Positioning

This should not be pitched as "Git with a better agent UI."

Better positioning:

> Anvics: an agent-native VCS for parallel AI software development, with native work records, native review, native sync, and optional legacy Git export.

That keeps the ambition intact. Teams can keep GitHub, CI, package publishing, mirrors, and backup workflows while moving the real source-control model into the new system.

## Design Principles

- Treat Git as a legacy adapter, not the internal model.
- Make native repositories useful without any Git remote.
- Optimize for many agents and one accountable human.
- Make privacy and disclosure timing first-class.
- Capture work continuously.
- Let history be curated after discovery.
- Prefer semantic records over noisy micro-commits.
- Make parallel workspaces cheap enough to be disposable.
- Expose everything through APIs.
- Keep human review legible.
- Preserve deterministic, auditable state.

## The Wedge

The best initial wedge is not replacing the whole Git ecosystem. It is making Git feel optional for agent work.

It is a native agent workspace and review system that can start from either a native repo or an imported Git repo:

1. Create a native repo or import a GitHub/GitLab repo.
2. Start N isolated agent work threads.
3. Capture every edit, command, test, and retry.
4. Produce native review projections with semantic summaries and condensed diffs.
5. Let a human accept, reject, combine, rerun, or publish work.
6. Publish accepted work natively.
7. Export to Git only when legacy systems need a commit, branch, PR, mirror, or backup.

This solves an immediate pain without making Git the product's source of truth.

## Build Discipline

Anvics should keep the ambitious native-VCS thesis, but it should not build the whole platform before proving one tight loop. The first proof should be small enough that a user can understand it quickly and an agent can use it without generating a mountain of process tokens.

MVP 0 should validate the loop: work thread, isolated workspace, snapshot, compact evidence, review projection, native publication, optional legacy Git export. The platform roadmap can then deepen the system with production VFS, command-worker mediation, richer `ChangeUnit`s, native sync, and structured conflict coordination.

For MVP 0:

- VFS is strategic for cheap path-compatible projections, but a materialized workspace backend is allowed while Linux/macOS proxy-mounted workspace prototypes run in parallel.
- Conflict handling means path/hunk overlap detection plus reviewable conflict notes, not semantic auto-resolution.
- Evidence is compact by default: raw artifacts by reference, review summaries by budget.
- Agent-facing instructions should expose the smallest useful vocabulary, not every internal object.

## MVP 0

MVP 0 should be an Anvics workbench with its own source-control model and optional legacy Git output:

1. Create a native repo or import a local/GitHub/GitLab repo.
2. Start two isolated work threads from prompts, issues, or CLI commands.
3. Give each thread a simple overlay or materialized workspace.
4. Capture snapshots at meaningful checkpoints.
5. Ingest compact command/test evidence from external or simulated agents.
6. Detect changed paths/hunks and flag overlap as a reviewable note.
7. Generate a review projection: intent, final diff, key evidence, risks, unresolved questions.
8. Let a human accept, reject, or manually combine work.
9. Publish accepted work as a native publication object.
10. Export accepted work as clean commit/pushable Git artifacts only for legacy integrations.

MVP 0 should not be blocked on replacing every Git host, permission model, CI integration, VFS backend, remote sync path, or conflict-resolution protocol. It should make Git unnecessary for the agent's working model, then emit Git when the outside world still needs Git.

## Research Appendix

Detailed research notes now live in [`research/`](research/README.md). The main synthesis is:

- First wedge: a neutral source-control substrate for agent work, focused on ingesting and reviewing external agent sessions before running agents directly.
- First integration surface: local CLI transcript capture plus native repo/work-thread APIs; GitHub App/webhook mapping is the legacy bridge for issues, branches, PRs, and commit artifacts.
- Core primitives: `Repository`, `SourceSnapshot`, `WorkThread`, `WorkspaceView`, `ChangeUnit`, `AgentSession`, `Attempt`, `EvidenceBundle`, `InstructionBundle`, `EnvironmentSnapshot`, `ReviewProjection`, `NativePublication`, and `LegacyGitArtifact`.
- Agent adoption: ship a first-party skill/instruction bundle so agents know how to use the new primitives.
- Legacy Git adapter: accepted native publications can be rendered as clean commit/pushable Git artifacts when needed; Git is not the internal model.

Research tracks:

- [01 - Agent Workflow Reality](research/01-agent-workflow-reality.md)
- [02 - Review And Trust](research/02-review-and-trust.md)
- [03 - Agent Failure Taxonomy](research/03-agent-failure-taxonomy.md)
- [04 - Provenance And Audit](research/04-provenance-and-audit.md)
- [05 - Privacy And Visibility](research/05-privacy-and-visibility.md)
- [06 - Legacy Git Adapter Design](research/06-git-boundary-design.md)
- [07 - Hunk And Change Ownership](research/07-hunk-change-ownership.md)
- [08 - Environment Reproducibility](research/08-environment-reproducibility.md)
- [09 - Agent Orchestration Boundary](research/09-agent-orchestration-boundary.md)
- [10 - Enterprise Controls](research/10-enterprise-controls.md)
- [11 - Source-Control API](research/11-source-control-api.md)
- [12 - Rust Architecture](research/12-rust-architecture.md)
- [13 - Repository Configuration](research/13-repo-config.md)

Related product artifact:

- [MVP 0](MVP_0.md)
- [Platform Roadmap](PLATFORM_ROADMAP.md)
- [Draft Anvics Skill](../skills/anvics-skill/SKILL.md)
- [Anvics Decision Log](ANVICS_DECISIONS.md)

## Competitive Position

This product should be judged against:

- GitHub PRs plus worktrees.
- Cursor/Claude/Codex cloud-agent flows.
- GitButler parallel branches.
- Graphite stacked diffs.
- `jj` for local power users.
- Agent observability/session-log tools.
- Internal monorepo systems at large companies.

The differentiator is not "better branching." It is "agent work becomes reviewable, reproducible, private, and publishable."

## Open Design Questions

- Is a work thread closer to a `jj` change, a Pijul patch set, a GitButler lane, or a trace-rooted task object?
- Should automatic snapshots happen on every command, every file edit, every test boundary, or only harness-defined checkpoints?
- What is the minimum useful provenance bundle?
- How much of the trace should be exposed to humans by default?
- Should private/public visibility apply to files, threads, packages, generated artifacts, or all of them?
- Can conflict prediction work at file/symbol/dependency level before true semantic merge exists?
- What is the cleanest legacy Git export model: one commit, a commit stack, a patch series, or a PR with hidden agent trace metadata?
- How does this integrate with existing CI without requiring a new build system?
- Should hunk identity be content-derived, position-derived, UUID-based like GitButler, or a hybrid?
- Is conflict state best modeled at file, hunk, symbol, graph, or work-thread level?
- Can Pijul-style zombie context become the right abstraction for stale agent context?
- Should agent transcripts live inside the source-control store, or should the store contain hashes, redacted projections, and external artifact links?
- What is the smallest useful boundary for accountability: task, attempt, snapshot, hunk, command, or review decision?
- Which agent integrations matter first: local CLI transcript capture, IDE extension, GitHub App, webhook/API, or cloud sandbox runner?
- Should the system run agents itself, orchestrate external agents, or start as a neutral source-control substrate that records what external agents do?
- How should cost/quota tracking influence source-control UX: per task, per session, per accepted change, or per reviewable artifact?

## One-Sentence Product Thesis

Build Anvics: private parallel work threads, continuous state capture, semantic provenance, API-native code operations, structured review, native publication, and legacy Git export only when needed.
