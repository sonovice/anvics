# Anvics

Agent-native source control for parallel AI coding agents.

Anvics is a local-first VCS/runtime substrate where agents work in isolated Anvics workspaces instead of Git branches or worktrees. Git remains a legacy import/export boundary: Anvics can export an accepted result as a Git patch, but Git is not the internal model.

## Current Status

Anvics is a working alpha moving toward private beta. The core loop works today:

- initialize a native Anvics repo
- create immutable source snapshots
- prepare isolated agent workspaces and task packets
- launch external agent CLIs with Anvics guidance
- inspect workspace changes without Git
- checkpoint and recover unfinished agent progress
- capture compact command evidence
- review changed paths, evidence, risk notes, and overlap notes
- publish accepted native work
- export a legacy Git patch
- inspect review inbox data through an opt-in localhost HTTP bridge and early web UI scaffold

Materialized workspaces are the default execution surface. The FUSE/VFS projection remains optional and experimental.

## Install From Source

```sh
git clone https://github.com/sonovice/anvics.git
cd anvics
cargo install --path crates/anvics-cli --locked
```

This installs the `anvics` CLI. During development you can also run it without installing:

```sh
cargo run -q -p anvics-cli --bin anvics -- --help
```

## First 10-Minute Trial

Create a disposable target repo:

```sh
target_repo="$(mktemp -d)"
printf '# Trial\n\nstatus: base\n' > "$target_repo/README.md"
anvics --repo "$target_repo" repo init
anvics --repo "$target_repo" snapshot create --message base
```

Prepare an agent packet:

```sh
anvics --repo "$target_repo" agent prepare \
  --title "First Anvics trial" \
  --task "Update README.md so the status mentions an Anvics trial."
```

Use the printed workspace id to generate launch guidance:

```sh
anvics --repo "$target_repo" agent launch-prompt --workspace "<workspace-id>" --tool codex
```

Paste the generated prompt into your agent CLI. The agent should read the packet and skill, run `agent enter`, edit only the workspace path, use `workspace diff`, checkpoint if needed, and report verification evidence.

Accept the finished workspace:

```sh
anvics --repo "$target_repo" agent accept \
  --workspace "<workspace-id>" \
  --run-label "verify first trial" \
  --run-summary "Verified README update." \
  --output "$target_repo/accepted.patch" \
  -- rg -n "Anvics trial" README.md
```

Inspect the result:

```sh
anvics --repo "$target_repo" review show "<review-id>" --format markdown
git apply --check "$target_repo/accepted.patch"
```

## Recovery Basics

If an agent crashes or loses context, inspect and preserve work before discarding it:

```sh
anvics --repo "$target_repo" agent recover --workspace "<workspace-id>"
anvics --repo "$target_repo" agent checkpoint --workspace "<workspace-id>" --summary "Recovered interrupted progress."
```

If publication is blocked by obsolete evidence from a failed accept attempt:

```sh
anvics --repo "$target_repo" risk list --review "<review-id>"
anvics --repo "$target_repo" evidence supersede "<evidence-id>" --reason "Obsolete failed verification output."
```

Then rerun `agent accept` to create a clean review. Use `--allow-secret-risk --override-reason "<reason>"` only for intentional fixtures or known false positives.

## Private Beta Smokes

Run these before trusting a local build:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/beta_first_run_smoke.sh
scripts/installed_binary_smoke.sh
scripts/evidence_supersede_smoke.sh
scripts/agent_context_pack_smoke.sh
scripts/agent_recovery_smoke.sh
scripts/secret_risk_smoke.sh
```

For a real external-repo trial:

```sh
scripts/external_beta_trial_prepare.sh
```

## Local Review UI

The first web surface is an operator Review Inbox, not a marketing site. Start the local daemon bridge:

```sh
anvics daemon start --socket /tmp/anvics.sock --http 127.0.0.1:3897
```

Then run the SolidStart app from `apps/anvics-web` with `VITE_ANVICS_REPO` pointed at an initialized Anvics repo:

```sh
cd apps/anvics-web
VITE_ANVICS_HTTP_URL=http://127.0.0.1:3897 \
VITE_ANVICS_REPO="$target_repo" \
bun run dev
```

See [Local And Hosted UI](docs/UI.md) for the current bridge endpoints and hosted roadmap.

## Known Limitations

- Local-only: no native sync, hosted remote, or team relay yet.
- CLI-first: the review UI is an early scaffold and not yet a complete operator console.
- Materialized workspaces are the default; VFS/FUSE support is opt-in and experimental.
- Command policy is a coarse provenance gate, not a sandbox.
- Secret scanning is intentionally conservative and can false-positive.
- Legacy Git export currently produces patches; Anvics does not create or push commits for you.
- Agents must follow the packet/skill instructions. Anvics does not yet supervise arbitrary external processes.

## More Docs

- [MVP 0](docs/MVP_0.md)
- [Platform Roadmap](docs/PLATFORM_ROADMAP.md)
- [Product Thesis](docs/GIT_ALTERNATIVE.md)
- [Decision Log](docs/ANVICS_DECISIONS.md)
- [Agent Test Runbook](docs/AGENT_TEST_RUNBOOK.md)
- [Agent Skill](skills/anvics-skill/SKILL.md)
