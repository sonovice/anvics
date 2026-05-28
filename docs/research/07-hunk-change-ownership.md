# 07 - Hunk And Change Ownership

## Research Question

How should the system assign ownership of changes across multiple agents, attempts, and review transformations?

## Observed Facts

- GitButler's hunk-assignment model gives hunks stable IDs and assigns them to stacks/branches.
- GitButler's hunk-dependency code shows that multiple branch lanes in one worktree require nontrivial dependency and ownership reasoning.
- Sapling's stack workflow includes split, fold, amend, and absorb, which are exactly the transformations humans want after messy discovery.
- `jj` models working-copy changes as commits and separates operation history from commit history.
- Pijul models changes as graph atoms and makes conflict/resolution state portable.

## Sources And Artifacts

- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-hunk-assignment/src/lib.rs`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-hunk-dependency/src/lib.rs`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/sapling/website/docs/overview/stacks.md`
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/jj/docs/working-copy.md`
- Local artifact: `/Users/simon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/libpijul-1.0.0-beta.11/src/change.rs`

## Product Implications

- Hunk ownership is central to parallel agent work.
- Ownership must survive split, squash, absorb, rerun, and conflict resolution.
- The product should let humans move changes between work threads without losing provenance.
- Full Pijul-style graph algebra is likely too heavy for v1, but its durable conflict model is directionally right.
- `ChangeUnit`s should be derived from source-relevant `FileEffect`s, not from every command side effect.
- Classification should be layered: Anvics proposes mechanically, agents can refine, and policy gates unresolved or sensitive effects.

## FileEffect To ChangeUnit Pipeline

`FileEffectSet` is the raw observation: a command or edit changed paths from one object state to another.

`ChangeUnit` is the source-control object used for ownership, review, conflict detection, and publication.

Pipeline:

1. Capture neutral `FileEffectSet`.
2. Apply repo config and policy:
   - ignored paths
   - tracked generated source
   - untracked generated output/cache
   - evidence candidate paths
   - read/write scopes
3. Accept agent/tool/human classifications as claims with confidence and rationale.
4. Create proposed `ChangeUnit`s only for source-relevant effects.
5. Gate unknown, outside-scope, generated-source, config, or sensitive effects before publication.

This keeps cache/log/evidence churn out of source review while preserving enough metadata for audit.

## MVP Impact

- Start with stable `ChangeUnit` records for files/hunks tied to a work thread and attempt.
- Use GitButler-style stable IDs plus content/path metadata.
- Track transformations as events: split, combine, supersede, accept, export.
- Convert source-relevant `FileEffect`s into proposed `ChangeUnit`s.
- Store classification provenance on `ChangeUnit`s.

## Open Questions

- Should hunk IDs be UUID-based, content-derived, location-derived, symbol-derived, or hybrid?
- Should v1 own hunks only, or also symbols and generated artifacts?
- How much provenance should follow a hunk after squash/split?
- Which `FileEffect` classifications should require human/policy approval before becoming publishable?
