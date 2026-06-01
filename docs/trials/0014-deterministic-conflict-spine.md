# Trial 0014: Deterministic Conflict Spine

Source state: MVP 0.42-0.45 working tree.

## Goal

Validate that Anvics can analyze competing candidate reviews, prepare a bounded resolver workspace, gate resolver publication with deterministic verification, and export a patch without assuming any specific agent runtime.

## Run

Executed:

```sh
scripts/agent_resolve_smoke.sh
```

The smoke creates a disposable Rust repo, prepares three candidate workspaces that edit the same function, finishes the three candidate reviews, runs `conflict analyze`, runs `conflict prepare`, edits the resolver workspace with a combined behavior, accepts it with Anvics-run `cargo test`, verifies the resolver contract, exports a legacy patch, and applies that patch to a clean copy.

## Observed Result

- `conflict analyze` created a durable analysis and classified the shared `src/lib.rs` edit as `SamePathOverlappingHunks` with `NeedsResolution`.
- `conflict prepare` generated a normal resolver thread/workspace and packet with deterministic conflict-analysis context.
- `conflict status` before evidence correctly reported `passed: false`.
- `agent accept --run-label ... -- cargo test` attached compact command evidence, created the resolver review/publication, and exported `resolved.patch`.
- `conflict verify` after acceptance reported `passed: true`.
- The final review markdown included `Source Reviews` and `Resolution Contract`.
- The exported patch applied cleanly to a fresh copy and `cargo test` passed.

## Takeaways

- The resolver packet is now narrower than the old free-form `agent resolve` packet: it separates deterministic analysis from semantic choice.
- Publication gates caught unrelated generated files during the first script run. The fixture now creates `Cargo.lock` before the base snapshot, which is the right test hygiene for this repo.
- The remaining semantic decision is explicit and bounded: preserve candidate intent for the unresolved overlapping function body.

## Recommendation

Keep the deterministic spine. Next useful work should polish CLI/operator ergonomics around conflict summaries and daemon parity tests, not start semantic merge or agent spawning.
