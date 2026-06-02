# Anvics Agent Test Runbook

This runbook tests Anvics with deterministic scripted flows and one manual live-agent flow. The goal is to validate threads, materialized workspaces, compact evidence, reviews, overlap notes, native publication, and legacy Git patch export without using Git branches or worktrees as the agent working model.

## Scripted Smoke Test

From the Anvics repo:

```sh
scripts/agent_smoke.sh
scripts/live_agent_packet_smoke.sh
scripts/daemon_agent_smoke.sh
scripts/coordination_smoke.sh
scripts/command_worker_smoke.sh
scripts/command_worker_process_smoke.sh
scripts/secret_risk_smoke.sh
scripts/evidence_supersede_smoke.sh
scripts/agent_instructions_smoke.sh
scripts/agent_context_pack_smoke.sh
scripts/agent_recovery_smoke.sh
scripts/agent_launch_prompt_smoke.sh
scripts/beta_first_run_smoke.sh
scripts/installed_binary_smoke.sh
scripts/external_beta_trial_prepare.sh
scripts/three_agent_conflict_trial_prepare.sh
scripts/dogfood_trial_prepare.sh
scripts/live_agent_trial_prepare.sh
```

`agent_smoke.sh` creates two threads and workspaces, applies scripted edits, attaches compact evidence, creates a review, and publishes one result.

`live_agent_packet_smoke.sh` uses the same convenience flow intended for live agents: `agent prepare`, scripted workspace edits, `agent accept`, review markdown, native publication, and legacy Git patch export. It also verifies that the exported patch applies to a clean Git checkout.

`live_agent_trial_prepare.sh` creates a temporary toy repo and prints two paste-ready external-agent prompts. Use it for the first real Codex/Claude/Cursor trial.

`daemon_agent_smoke.sh` runs the same accept/export loop through `anvicsd` with `--use-daemon`, proving the local socket API can drive the MVP flow.

`coordination_smoke.sh` prepares two agent workspaces, enters both sessions, records one known change, and shows the other agent the related active work before it edits.

`command_worker_smoke.sh` accepts a workspace through Anvics-run verification, stores stdout/stderr artifacts by reference, publishes the result, and verifies the exported patch applies.

`command_worker_process_smoke.sh` checks `anvics-worker` protocol health, then runs the same acceptance loop through the opt-in worker process executor and verifies that review evidence records `executor: worker`.

`agent_instructions_smoke.sh` renders and installs `AGENTS.md`/`CLAUDE.md` templates, verifies overwrite protection, and checks that the generated guidance tells external agents to use Anvics instead of Git defaults.

`agent_context_pack_smoke.sh` renders a scoped, refreshable context pack for a workspace, including task, packet/skill paths, classified workspace changes, and coordination status.

`agent_recovery_smoke.sh` proves interrupted agent work can be inspected with `agent recover` and preserved with `agent checkpoint` before acceptance.

`secret_risk_smoke.sh` proves the safety gate: command output with a secret-like value blocks acceptance, risk output stays redacted, an explicit override records the reason, and the exported patch still applies.

`evidence_supersede_smoke.sh` proves the recovery path for obsolete risky evidence: a blocked accept preserves evidence, the operator supersedes that evidence with a reason, then a clean accept publishes without a secret-risk override.

`agent_launch_prompt_smoke.sh` verifies the operator prompt for external agent CLIs, including Codex's non-Git workspace flag.

`beta_first_run_smoke.sh` exercises the private-beta happy path: init, snapshot, prepare, launch prompt, workspace diff, checkpoint, coordination, accept, risk/review inspection, and patch application.

`installed_binary_smoke.sh` installs `anvics` with `cargo install --path crates/anvics-cli --locked`, then runs the beta accept/export loop through the installed binary instead of repo-local `cargo run`.

`external_beta_trial_prepare.sh` creates a disposable non-Anvics Rust repo, prepares two agent packets, and writes `.anvics/external-beta-trial-manifest.env` for a real two-agent private beta trial.

`three_agent_conflict_trial_prepare.sh` creates a disposable Rust repo where three agents intentionally edit the same function in `src/lib.rs`, then writes `.anvics/three-agent-conflict-manifest.env` for conflict-resolution dogfood.

`dogfood_trial_prepare.sh` creates a disposable copy of the Anvics repo, prepares two realistic agent packets, and writes `.anvics/dogfood-trial-manifest.env` for a real dogfood run.

## Daemon Check

For manual daemon testing, run:

```sh
socket_dir="$(mktemp -d)"
socket="$socket_dir/anvics.sock"
cargo run -q -p anvics-cli --bin anvicsd -- --socket "$socket"
```

In another shell:

```sh
cargo run -q -p anvics-cli --bin anvics -- daemon ping --socket "$socket"
ANVICS_DAEMON_SOCKET="$socket" cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" --use-daemon repo status
```

## Manual Live-Agent Test

1. Create a temporary target repo:

   ```sh
   target_repo="$(mktemp -d)"
   printf 'base\n' > "$target_repo/app.txt"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" repo init
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" snapshot create --message base
   ```

   Optional config sanity check:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" repo doctor --path src/lib.rs --path target/debug/app
   ```

   Use `repo doctor --path <path>` when a task depends on whether a path is source, generated output, cache, or evidence. It reports the accepted root `anvics.toml` interpretation; workspace edits to `anvics.toml` do not affect the same review.

   To make a target repo agent-ready before preparing packets, render or install repo-level instructions:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent instructions
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent instructions --install
   ```

   The install writes `AGENTS.md` and `CLAUDE.md` only when they do not already exist. Use `--force` only when intentionally replacing existing repo guidance.

2. Prepare a live-agent packet:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent prepare \
     --title "Live agent test" \
     --task "Edit app.txt so it clearly identifies the live agent run."
   ```

3. Inspect the packet and current status:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent packet --thread "<thread-id>"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent context-pack --workspace "<workspace-id>"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent launch-prompt --workspace "<workspace-id>" --tool codex
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent status --thread "<thread-id>"
   ```

4. For Codex CLI, use the generated `agent launch-prompt --tool codex` command. It includes `--skip-git-repo-check` because Anvics workspaces are not Git worktrees and may not contain a `.git` directory.

   The launched agent must also be able to run the Anvics command prefix printed in the packet. If `anvics` is not installed on `PATH`, prepare the packet with `ANVICS_AGENT_COMMAND="<exact command prefix>"` so generated commands use that prefix. If Codex defaults to a read-only sandbox, choose an operator-approved write-enabled sandbox mode for the disposable workspace before launching.

   For Claude, Cursor, or another agent CLI, open the printed packet path, then paste this prompt:

   ```text
   You are working inside an Anvics task packet.

   Read the packet at:
   <packet-path>

   Read the Anvics skill path named in the packet, then follow the skill and packet exactly.
   Work only inside the workspace path listed in the packet.
   This workspace may not be a Git repository; use the tool's non-Git workspace mode if it has one.
   Use the Anvics command prefix printed in the packet. Do not guess with Git if the command is unavailable.
   Run the packet's agent enter command before editing.
   Use the packet's workspace diff command instead of Git status or Git diff.
   Run agent checkpoint before a long pause, handoff, risky edit, or expected context loss.
   Run coordination status before finishing and report any potential clashes.
   Do not create a Git branch, Git worktree, or Git commit.
   When done, tell me a short command label, its exit code, a one-sentence summary, and optionally a command/evidence file path.
   ```

5. Accept the agent task from the Anvics repo, using the printed workspace id:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent accept \
     --workspace "<workspace-id>" \
     --run-label "verify live agent result" \
     --run-summary "Anvics verified the live-agent workspace before accepting." \
     --output "$target_repo/accepted.patch" \
     -- sh -c "cat app.txt"
   ```

   Anvics blocks networked, host-escape-risk, and interactive verification commands by default. Preview unfamiliar commands with `anvics command classify -- <program> [args...]` or `anvics command classify --command-file <path>`. If the operator intentionally approves that risk, add `--allow-command-risk --command-risk-reason "<reason>"` to the `agent accept` or `command run` invocation. This records an audited override on the command event; it is not a sandbox.

   If Anvics cannot run the verification command, fall back to externally-reported evidence:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent accept \
     --workspace "<workspace-id>" \
     --command "manual live-agent run" \
     --exit-code 0 \
     --summary "Live agent edited app.txt in the materialized workspace." \
     --output "$target_repo/accepted.patch"
   ```

   For multi-command verification, put the commands in a small file and use `--run-label "<short label>" --run-summary "<short summary>" --command-file <path>` for Anvics-run evidence, or `--command-file <path> --label "<short label>"` when attaching externally-run evidence. Anvics-run command files are policy-scanned before execution. Add `--artifact <path>` if the agent produced a compact result file worth linking.

6. Inspect the accepted result:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" review show "<review-id>" --format markdown
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent status --thread "<thread-id>"
   ```

   `agent finish`, `review show`, `publish create`, and `legacy git export` remain available when you want to inspect or publish each step manually.

   If the agent crashes, exits early, or loses context before finishing, inspect salvageable progress first:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent recover --workspace "<workspace-id>"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent checkpoint --workspace "<workspace-id>" --summary "Salvaged interrupted agent progress."
   ```

   To restore a workspace safely, use an audited Anvics restore command instead of deleting files by hand:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent checkpoint list --workspace "<workspace-id>"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent checkpoint restore --workspace "<workspace-id>" --checkpoint "<checkpoint-id>" --reason "Recover checkpointed progress."
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" workspace restore "<workspace-id>" --source base --path "app.txt" --reason "Back out the bad app.txt edit."
   ```

   To revert accepted work, prepare an inverse publication thread. The original publication stays immutable:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" publish revert prepare --publication "<publication-id>" --reason "Revert accepted change after validation."
   ```

   If publication is blocked by a secret-risk finding, inspect the review and risk list:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" risk list --review "<review-id>"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" review show "<review-id>" --format markdown
   ```

   Fix the workspace and rerun acceptance when the finding is real. If the risk belongs to obsolete evidence from a failed accept attempt, supersede that evidence with an audited reason, then rerun `agent accept` to create a clean review:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" evidence supersede "<evidence-id>" --reason "Obsolete failed verification output."
   ```

   Use an override only for a known false positive or intentionally local fixture:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" publish create \
     --thread "<thread-id>" \
     --review "<review-id>" \
     --allow-secret-risk \
     --override-reason "Short audited reason."
   ```

## Two-Agent Trial

Use the harness for the first real validation run:

```sh
scripts/live_agent_trial_prepare.sh
```

Paste the two generated prompts into two external agents. When an agent finishes, verify and accept the result you want to publish:

```sh
scripts/live_agent_trial_verify.sh "$target_repo" \
  "<thread-id>" \
  "<workspace-id>" \
  "manual live-agent run" \
  0 \
  "Live agent completed the accepted task."
```

Append an artifact path as the final argument when the accepted agent produced a compact artifact worth linking.

Capture observations in `docs/trials/LIVE_AGENT_TRIAL_TEMPLATE.md`. Do not create a trial log until an actual live run has happened.

## Dogfood Trial

Use the dogfood harness before expanding VFS/runtime work:

```sh
scripts/dogfood_trial_prepare.sh
```

Paste the two generated prompts into two external agents. Accept one completed workspace with `scripts/live_agent_trial_verify.sh` using the target repo, thread id, workspace id, verification command, exit code, and summary printed or reported by the agents.

- Treat each generated packet as authoritative, including its Anvics command prefix and final-report requirements.

After the actual run, copy `docs/trials/0002-anvics-dogfood-template.md` to `docs/trials/0002-anvics-dogfood.md` and fill it with the observed friction, review quality, risk findings, and patch export result.

## Private Beta Trial

For the private beta readiness gate, run the first path through an installed binary and a disposable external repo:

```sh
scripts/beta_first_run_smoke.sh
scripts/installed_binary_smoke.sh
scripts/external_beta_trial_prepare.sh
```

Then launch the generated prompts with real external agent CLIs. For Codex, use the generated `agent launch-prompt --tool codex` output or this shape:

```sh
codex exec --skip-git-repo-check --cd "<workspace-path>" - < "<prompt-file>"
```

Accept the finished workspaces with Anvics-run verification, export one patch per publication, and verify the patches apply to a clean copy. Record the result under `docs/trials/` with a clear `ship/private-beta/no-go` recommendation.

## Three-Agent Conflict Trial

To test same-file conflict pressure, prepare the conflict fixture:

```sh
scripts/three_agent_conflict_trial_prepare.sh
```

Launch the three generated prompts in external agents. Finish each workspace as a candidate review, then run deterministic conflict analysis before preparing the resolver workspace:

```sh
anvics --repo "$target_repo" conflict analyze \
  --review "<agent-a-review>" \
  --review "<agent-b-review>" \
  --review "<agent-c-review>"
```

Then prepare the bounded resolver workspace:

```sh
anvics --repo "$target_repo" conflict prepare \
  --review "<agent-a-review>" \
  --review "<agent-b-review>" \
  --review "<agent-c-review>" \
  --title "Resolve competing edits" \
  --task "Combine the compatible intent from all candidates." \
  --agent-command "$ANVICS_AGENT_COMMAND"
```

`conflict prepare` applies deterministic safe changes first and leaves only true conflict cases for the current assigned agent, a human, or an operator-selected external agent. Anvics does not spawn one. Use `conflict status --workspace <resolver-workspace>` before acceptance, accept with Anvics-run verification, then run `conflict verify --workspace <resolver-workspace>` and verify the exported patch applies to a clean copy.

Current limitation: conflict verification is deterministic and publication-gated for resolver work, but the semantic choice for overlapping intent still comes from the assigned agent/operator.

## Projection Runtime Checks

Operators can ask Anvics-run verification commands to use a projection backend:

```sh
anvics --repo "$target_repo" agent accept \
  --workspace "$workspace_id" \
  --run-label "verify" \
  --run-summary "Verified accepted workspace" \
  --projection auto \
  --output "$target_repo/accepted.patch" \
  -- <program> [args...]
```

Use `--projection auto` when testing VFS behavior without making FUSE availability a hard requirement. Use `--projection fuse-mount` only when the binary was built with `vfs-fuse` and the machine has working FUSE/macFUSE support. These flags are for operator-run `command run` and `agent accept --run-*` verification; do not add them to agent packet commands. Agents should still edit the normal workspace path from the packet and should not manually manage mounts.

## What To Check

- The agent edited only the workspace path.
- The agent knew how to use `agent context-pack` when it needed refreshed task, diff, or coordination context.
- The agent used Anvics `workspace diff` rather than Git status or Git diff inside the workspace.
- `workspace diff` showed classification labels such as `source`, `generated_tracked`, `generated_untracked`, `cache`, or `evidence_candidate` when relevant.
- No Git branch, worktree, or commit was needed.
- Evidence is a short summary, not a transcript dump.
- Secret-like values are not copied into evidence summaries or review notes.
- The review shows changed paths and evidence.
- Risky operator-run commands are blocked unless an explicit command-risk override reason is recorded.
- Secret-risk findings block publication unless an operator records an explicit override reason.
- Publication points to the accepted native snapshot.
- The exported patch applies to a clean copy of the base files with `git apply`.

## Known Limitations

- Workspaces are still materialized directories under `.anvics/workspaces/`.
- Patch export is the only legacy Git artifact in this slice; commit creation and push come later.
- Conflict handling has a deterministic spine: analysis, safe auto-apply, resolver packet, verification, and publication gates. It is not semantic auto-resolution.
- The live-agent flow is manual and tool-agnostic; Anvics does not run the agent yet.

## Three-Agent Conflict Resolution

After candidate agents finish and produce reviews, analyze the competing reviews:

```sh
anvics --repo "$target_repo" conflict analyze \
  --review "<agent-a-review>" \
  --review "<agent-b-review>" \
  --review "<agent-c-review>"
```

Then prepare a resolver workspace directly from the competing reviews:

```sh
anvics --repo "$target_repo" conflict prepare \
  --review "<agent-a-review>" \
  --review "<agent-b-review>" \
  --review "<agent-c-review>" \
  --title "Resolve competing edits" \
  --task "Combine the compatible intent from all candidates." \
  --agent-command "$ANVICS_AGENT_COMMAND"
```

`conflict prepare` requires all source reviews to share one base snapshot. It creates or reuses a deterministic conflict analysis, creates a normal resolver thread/workspace, applies safe independent and non-overlapping hunk changes, writes a packet containing candidate review paths, changed paths, evidence summaries, overlap notes, and unresolved conflict cases, and records the source review ids plus conflict analysis id on the final resolver review.

`agent resolve` remains as a compatibility wrapper for resolver preparation, but new runbooks should use `conflict analyze` and `conflict prepare`.

Use the generated resolver packet with the current assigned agent, an operator, or any external agent CLI the operator chooses. Anvics does not spawn or require a resolver agent. Then accept the resolver workspace through the normal operator flow:

```sh
anvics --repo "$target_repo" agent accept \
  --workspace "<resolver-workspace>" \
  --run-label "cargo test" \
  --run-summary "Resolved candidate behavior passed tests." \
  --output resolved.patch \
  -- cargo test
```

After acceptance:

```sh
anvics --repo "$target_repo" conflict verify --workspace "<resolver-workspace>"
```

Resolver publication is blocked by default if deterministic verification detects unrelated paths, dropped safe changes, missing source review links, unresolved cases without final output, or missing compact evidence. Override only with an audited `--allow-resolution-risk --resolution-risk-reason "<reason>"`.
