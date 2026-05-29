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
scripts/secret_risk_smoke.sh
scripts/dogfood_trial_prepare.sh
scripts/live_agent_trial_prepare.sh
```

`agent_smoke.sh` creates two threads and workspaces, applies scripted edits, attaches compact evidence, creates a review, and publishes one result.

`live_agent_packet_smoke.sh` uses the same convenience flow intended for live agents: `agent prepare`, scripted workspace edits, `agent accept`, review markdown, native publication, and legacy Git patch export. It also verifies that the exported patch applies to a clean Git checkout.

`live_agent_trial_prepare.sh` creates a temporary toy repo and prints two paste-ready external-agent prompts. Use it for the first real Codex/Claude/Cursor trial.

`daemon_agent_smoke.sh` runs the same accept/export loop through `anvicsd` with `--use-daemon`, proving the local socket API can drive the MVP flow.

`coordination_smoke.sh` prepares two agent workspaces, enters both sessions, records one known change, and shows the other agent the related active work before it edits.

`command_worker_smoke.sh` accepts a workspace through Anvics-run verification, stores stdout/stderr artifacts by reference, publishes the result, and verifies the exported patch applies.

`secret_risk_smoke.sh` proves the safety gate: command output with a secret-like value blocks acceptance, risk output stays redacted, an explicit override records the reason, and the exported patch still applies.

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

2. Prepare a live-agent packet:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent prepare \
     --title "Live agent test" \
     --task "Edit app.txt so it clearly identifies the live agent run."
   ```

3. Inspect the packet and current status:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent packet --thread "<thread-id>"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" agent status --thread "<thread-id>"
   ```

4. Open the printed packet path, then paste this prompt into Codex, Claude, Cursor, or another agent CLI:

   ```text
   You are working inside an Anvics task packet.

   Read the packet at:
   <packet-path>

   Read the Anvics skill path named in the packet, then follow the skill and packet exactly.
   Work only inside the workspace path listed in the packet.
   Run the packet's agent enter command before editing.
   Use the packet's workspace diff command instead of Git status or Git diff.
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

   If publication is blocked by a secret-risk finding, inspect the review and risk list:

   ```sh
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" risk list --review "<review-id>"
   cargo run -q -p anvics-cli --bin anvics -- --repo "$target_repo" review show "<review-id>" --format markdown
   ```

   Fix the workspace and rerun acceptance when the finding is real. Use an override only for a known false positive or intentionally local fixture:

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

After the actual run, copy `docs/trials/0002-anvics-dogfood-template.md` to `docs/trials/0002-anvics-dogfood.md` and fill it with the observed friction, review quality, risk findings, and patch export result.

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
- The agent used Anvics `workspace diff` rather than Git status or Git diff inside the workspace.
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
- Conflict handling is path-level overlap notes, not semantic resolution.
- The live-agent flow is manual and tool-agnostic; Anvics does not run the agent yet.
