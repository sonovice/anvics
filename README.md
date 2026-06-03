# Anvics

Agent-native source control for humans supervising AI coding agents.

Anvics gives agents isolated workspaces, durable snapshots, compact evidence, review objects, recovery, conflict preparation, native publication, and legacy Git patch export without making Git branches or worktrees the core workflow.

## The Short Version

Use the Anvics skill and let your agent drive the workflow.

1. Give your agent this skill: [`skills/anvics-skill/SKILL.md`](skills/anvics-skill/SKILL.md).
2. Ask it:

```text
Read the Anvics skill and use Anvics as the source-control workflow.
Set up or verify the Anvics CLI if needed.
Prepare an isolated workspace for the task, preserve progress with checkpoints,
show me the review/evidence/risk state, and only publish/export after I approve.
```

That is the intended beta onboarding. The human should not have to memorize Anvics commands to start.

## What The Agent Should Do

An Anvics-aware agent should:

- install or locate the `anvics` CLI when needed;
- initialize a disposable or target Anvics repo;
- create a base snapshot;
- prepare an isolated workspace and task packet;
- enter the workspace and check coordination status;
- inspect changes with Anvics, not `git diff`;
- checkpoint recoverable progress;
- attach compact evidence;
- create a review;
- surface risks, conflicts, and recovery options to the human;
- publish/export only when the human asks.

The skill contains the operating rules. The README is just the front door.

## Current Status

Anvics is a working alpha moving toward private beta.

Working today:

- native local repos and immutable snapshots;
- isolated materialized workspaces;
- agent packets, launch prompts, context packs, and checkpoints;
- command evidence, policy classification, and secret-risk gates;
- review projections, native publications, and Git patch export;
- evidence superseding, workspace restore, checkpoint restore, and publication revert preparation;
- deterministic conflict analysis/preparation/verification;
- optional experimental VFS projection;
- local `anvicsd` HTTP bridge and an early operator web console.

Still rough:

- UI quality is not beta-ready;
- hosted sync is not implemented;
- Git export is patch-based;
- command policy is not a sandbox;
- VFS is optional and experimental;
- agents must still follow the skill/packet guidance.

## For Humans

The UI is meant for human supervision: review queues, evidence, risks, publications, recovery, conflicts, and audit history.

The agent-facing surface is different: skills, packets, CLI/API commands, and workspace paths.

## For Developers

Useful docs:

- [Agent Skill](skills/anvics-skill/SKILL.md)
- [Agent Test Runbook](docs/AGENT_TEST_RUNBOOK.md)
- [Local And Hosted UI](docs/UI.md)
- [MVP 0](docs/MVP_0.md)
- [Platform Roadmap](docs/PLATFORM_ROADMAP.md)
- [Product Thesis](docs/GIT_ALTERNATIVE.md)
- [Decision Log](docs/ANVICS_DECISIONS.md)

Manual checks before trusting a local build:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd apps/anvics-web
bun run check
bun run build
```

Use Bun for frontend work.
