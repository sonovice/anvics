# 13 - Repository Configuration

## Research Question

How should Anvics understand repository structure, package/source-root boundaries, generated paths, tool hints, and policy boundaries without hard-coding every programming ecosystem?

## Observed Facts

- Monorepos need first-class source-root/package boundaries. Agents should not treat a large repo as one undifferentiated folder.
- Agents and tools need normal paths, test commands, generated-path knowledge, ignored output paths, and instruction-file references.
- The core VCS should remain language/tool agnostic. Ecosystem-specific knowledge belongs in config, heuristics, plugins, policy, or agent suggestions.
- Some configuration is portable and belongs with source. Other configuration is private/org/local policy and must not live in the source tree.
- Changes to repo interpretation are sensitive: editing config can affect visibility, evidence, generated files, tests, and what agents are allowed to modify.

## Sources And Artifacts

- Main thesis: `../GIT_ALTERNATIVE.md`
- Source-control API note: `11-source-control-api.md`
- Rust architecture note: `12-rust-architecture.md`
- Privacy and visibility note: `05-privacy-and-visibility.md`
- Draft Anvics skill: `../../skills/anvics-skill/SKILL.md`

## Proposed Model

`Repository` is the top-level storage, sync, and security unit.

Inside a repository, Anvics should model:

- `SourceRoot`: source subtree with kind/metadata, such as app, package, service, library, docs, generated root, or test root.
- `PackageBoundary`: package/module boundary used for indexing, policy, review, active-work maps, and conflict risk.
- `external_ref`: tree entry that points to another source root/package/blob namespace without baking in Git submodule semantics.

Cross-repo work should not be a v1 transaction requirement, but the model should not prevent it.

## Configuration Sources

Priority order:

1. Explicit source-tree `anvics.toml`.
2. Heuristics/plugins for ecosystems such as Cargo, npm/pnpm, Go, Python, Java, etc.
3. Agent-suggested boundaries and config improvements.

Agent suggestions should be stored as claims or proposed config edits until accepted by policy/human review.

## `anvics.toml`

Source-tree `anvics.toml` should be portable repo configuration.

It should focus on:

- Repo name/identity hints.
- Source roots.
- Package boundaries.
- Tracked generated source paths.
- Untracked generated output/cache paths.
- Evidence candidate paths.
- Ignored/cache/output paths.
- Tool hints such as test/lint/typecheck commands.
- Agent instruction references.

Example:

```toml
[repo]
name = "my-repo"

[source_roots.web]
path = "apps/web"
kind = "application"

[source_roots.api]
path = "services/api"
kind = "service"

[generated]
tracked = ["src/generated/**"]
untracked = ["target/**", ".next/**", "dist/**"]

[ignore]
paths = ["node_modules/**"]

[evidence]
candidate_paths = ["test-results/**", "playwright-report/**", "coverage/**"]

[tools]
test = ["npm test"]
lint = ["npm run lint"]
typecheck = ["npm run typecheck"]

[agents]
instructions = ["AGENTS.md"]
```

`anvics.toml` should not contain:

- Secrets.
- User/org RBAC.
- Private visibility rules.
- Evidence retention policy.
- Relay credentials.
- Machine-local settings.

Those belong in Anvics metadata and policy stores.

## Agent Edits

Agents may propose edits to `anvics.toml` when they discover missing structure:

- Missing source roots.
- Missing generated paths.
- Missing ignore/cache paths.
- Missing evidence candidate paths.
- Tool command improvements.
- Instruction file references.
- Package boundaries.

Because `anvics.toml` changes how Anvics interprets the repo, edits should be treated as source-control configuration changes:

- Flag in review projection.
- Explain behavioral impact.
- Require elevated review or policy gate before native publication when configured.
- Record event attribution and evidence.

## Generated And Evidence Paths

Generated paths should be split by intent:

- `generated.tracked`: generated source that is intentionally part of the source tree and may appear in native publications.
- `generated.untracked`: generated output, build cache, or derived artifacts that should not become source `ChangeUnit`s.
- `evidence.candidate_paths`: tool outputs that may be useful as evidence, such as test reports, coverage, screenshots, browser traces, or benchmark output.

Tracked generated source should not be edited directly by default. Agents should prefer changing the generator, schema, source template, or build input and rerunning generation. If an agent directly edits a tracked generated file, Anvics should require:

- Explicit rationale.
- Review projection flag.
- Policy approval when configured.
- Evidence explaining why generator-level change was not used.

Evidence candidate paths should not mean "retain everything forever." They should generate attach suggestions after commands run. Retention, redaction, sharing, and size limits belong to evidence policy.

## Product Implications

- Repo structure should be inspectable and correctable, not magic.
- Agents should use `anvics.toml` and `ContextPack`s to understand source roots before editing.
- The code browser and active-work map should group work by source root/package where possible.
- Policies can bind to repo, source root, package, path, work thread, or publication scope.
- `[tools]` should remain simple command hints. It should not become a workflow engine, dependency graph, package-manager abstraction, or secret-injection mechanism in v1.

## MVP Impact

- Add `SourceRoot` and `PackageBoundary` to the conceptual model.
- Implement optional `anvics.toml` parsing. MVP 0.27 implements the first narrow read path for review classification: `generated.tracked`, `generated.untracked`, `ignore.paths`, and `evidence.candidate_paths`.
- Use only canonical root `anvics.toml` for classifying a review. A workspace edit to `anvics.toml` is itself a config change; it should not reinterpret or hide other effects in the same review.
- Add `anvics repo doctor` flows to inspect, then later generate/improve config. MVP 0.28 starts with inspection and path classification.
- Persist and render classified review file effects. MVP 0.29 keeps all classified effects visible and derives `ChangeUnit`s only from source-relevant effects.
- Support simple heuristics/plugins for common repo markers.
- Treat `anvics.toml` edits as policy-sensitive config changes.
- Distinguish tracked generated source, untracked generated output, and evidence candidate paths.
- Flag direct edits to tracked generated source unless policy allows them.
- Support simple named tool command hints, optionally scoped by source root later.

## Open Questions

- What is the minimum v1 TOML schema that is useful without becoming a package-manager abstraction?
- Should `anvics.toml` support inheritance/overrides in subdirectories?
- Should evidence candidate paths support command-specific matching, or only path globs in v1?
- Should source-root-scoped `[tools]` be in v1, or should root-level hints be enough initially?
- How should agent-suggested config changes be represented before acceptance?
- Should source-root/package detection live in `anvics-index`, `anvics-policy`, or a plugin crate?
