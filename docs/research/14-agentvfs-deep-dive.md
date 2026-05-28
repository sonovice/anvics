# 14 - Agentvfs Deep Dive

## Research Question

What should Anvics learn from agentvfs before starting VFS work, and how should that change the next roadmap milestone?

## Observed Facts

- I inspected a local clone of `agentvfs` at `/tmp/agentvfs-study`, commit `2ec1227`, package version `0.1.8`.
- The README frames agentvfs as a runtime and isolation layer for agents working on forked workspaces, not only as a FUSE library.
- `docs/architecture.md` describes a high-level path of agent, proxy boundary, mounted forked workspace, and CLI tools.
- `documentation/docs/advanced/fuse-mount.md` treats the FUSE mount as a workspace projection that lets normal tools operate on forked state.
- `documentation/docs/advanced/agent-integration.md` emphasizes agents running ordinary commands through a prepared workspace boundary.
- `src/runtime/proxy.rs` is the most important artifact for Anvics: the proxy coordinates workspace lookup, command classification, checkpoints, mount sessions, command execution, changed-file summary, and cleanup.
- `src/runtime/policy.rs` uses coarse top-level command classification before execution. The shape is closer to operational policy than syscall sandboxing.
- `src/mount/filesystem.rs` uses `fuser` and handles filesystem-level details such as inode/path lookup and open-file state.
- The mount implementation models open files through write states such as open, dirty, persisting, flushed, and closed. That is a concrete warning that write handling and cleanup are a first-class part of VFS work.
- `src/storage/blobstore.rs` supports a shared blob store where forks can be cheap metadata changes over shared content.
- Change summaries in the runtime are event/audit-derived when available, with full tree comparison as a fallback pattern.

## Sources And Artifacts

- [agentvfs crate](https://crates.io/crates/agentvfs)
- [agentvfs repository](https://github.com/neul-labs/agentvfs)
- Local clone: `/tmp/agentvfs-study` at commit `2ec1227`
- `/tmp/agentvfs-study/README.md`
- `/tmp/agentvfs-study/docs/architecture.md`
- `/tmp/agentvfs-study/documentation/docs/advanced/fuse-mount.md`
- `/tmp/agentvfs-study/documentation/docs/advanced/agent-integration.md`
- `/tmp/agentvfs-study/src/runtime/proxy.rs`
- `/tmp/agentvfs-study/src/runtime/policy.rs`
- `/tmp/agentvfs-study/src/mount/filesystem.rs`
- `/tmp/agentvfs-study/src/storage/blobstore.rs`

## Product Implications

Anvics should borrow the runtime shape, not the product semantics. The key lesson is that VFS is valuable as a substrate, but the product boundary should be a proxy-mounted workspace runtime:

- Agents should normally receive a prepared workspace path and Anvics commands, not instructions to manually create and tear down mounts.
- `command run` or a later proxy execution command should own mount lifecycle when possible: resolve workspace, classify command, create mount/materialized path, run command, capture evidence, summarize file effects, and clean up.
- Low-level `workspace mount` is still useful, but it should be a debug/manual tool rather than the default agent workflow.
- Command policy should start coarse and explicit: read-only, mutating, destructive, networked, host-escape-risk, and interactive.
- Anvics should not claim syscall-level sandboxing from this model. Strong isolation remains the job of containers, VMs, devcontainers, CI, or cloud sandboxes.
- File-effect summaries should prefer runtime/mount event data where available, with deterministic tree diff as fallback.
- Shared content-addressed blobs plus metadata/overlay forks match Anvics' desired workspace model.

## What To Borrow

- Proxy boundary over raw filesystem access.
- Mount as compatibility substrate for tools that need paths.
- Command policy classification before execution.
- Mount session lifecycle owned by runtime/proxy where possible.
- Open-file write states and explicit flush/release/fsync behavior as VFS prototype risks.
- Event/audit-based change summaries to reduce expensive whole-tree scanning.
- Shared blob store plus metadata forks for cheap parallel workspaces.

## What Not To Borrow

- The vault mental model. Anvics is source control, not a generic secure vault.
- Source-control semantics from agentvfs. Anvics needs native work threads, snapshots, review projections, publications, policy gates, and legacy Git export.
- A direct dependency on agentvfs. Anvics should own its workspace model and likely prototype lower-level mount behavior with `fuser`.
- Treating VFS as the agent-facing abstraction. VFS should make existing tools work; Anvics packets, CLI/API, and runtime workflows should remain the agent interface.

## MVP Impact

- Rename the next VFS milestone to **MVP 0.15: Proxy-Mounted Workspace Spike**.
- The first spike should prove one command running through one mounted workspace on Linux/macOS, returning changed paths, cleaning up the mount, and falling back to materialized execution if FUSE/macFUSE is unavailable.
- Keep materialized workspaces as the compatibility fallback and as the current validation path.
- The product requirement is a projection backend abstraction. VFS is the likely low-copy backend for large repos and many concurrent agents, but not every Anvics workflow needs a mount.
- Do not add agentvfs as a dependency.
- Do not begin with an agent-facing mount workflow. Begin with runtime-owned mount lifecycle behind command execution.

## Open Questions

- Can `fuser` cover both Linux FUSE and macOS macFUSE cleanly enough for the first Anvics prototype?
- Should proxy-mounted command execution become an `anvics-runtime` crate immediately, or stay inside existing CLI/store boundaries for the spike?
- How should Anvics persist VFS write journals into the overlay model?
- What is the minimum event/audit stream needed to replace tree-diff-only file-effect summaries?
- How much symlink, hard link, xattr, permission, and watcher behavior is required before real monorepo tools feel comfortable?
- How should Anvics expose mount failures to agents without teaching them low-level FUSE troubleshooting?
