# Anvics Agent Test Runbook

This runbook tests Anvics with one deterministic scripted flow and one manual live-agent flow. The goal is to validate threads, materialized workspaces, compact evidence, reviews, overlap notes, and native publication without using Git branches or worktrees.

## Scripted Smoke Test

From the Anvics repo:

```sh
scripts/agent_smoke.sh
```

The script creates a temporary target repo, initializes Anvics, creates two threads and workspaces, applies scripted edits, attaches compact evidence, creates a review, and publishes one result.

## Manual Live-Agent Test

1. Create a temporary target repo:

   ```sh
   target_repo="$(mktemp -d)"
   printf 'base\n' > "$target_repo/app.txt"
   cargo run -q -p anvics-cli -- --repo "$target_repo" repo init
   cargo run -q -p anvics-cli -- --repo "$target_repo" snapshot create --message base
   ```

2. Create a thread:

   ```sh
   cargo run -q -p anvics-cli -- --repo "$target_repo" thread create \
     --title "Live agent test" \
     --task "Edit app.txt so it clearly identifies the live agent run."
   ```

3. Create a workspace for that thread and copy the printed `path:` value:

   ```sh
   cargo run -q -p anvics-cli -- --repo "$target_repo" workspace create --thread "<thread-id>"
   ```

4. Give the live agent this instruction:

   ```text
   Work only inside this Anvics workspace path:
   <workspace-path>

   Task: edit app.txt so it clearly identifies this live agent run.

   Do not create a Git branch, Git worktree, or Git commit. Keep output compact.
   ```

5. Attach compact evidence:

   ```sh
   cargo run -q -p anvics-cli -- --repo "$target_repo" evidence attach \
     --thread "<thread-id>" \
     --command "manual live-agent run" \
     --exit-code 0 \
     --summary "Live agent edited app.txt in the materialized workspace."
   ```

6. Snapshot the workspace, create a review, and publish:

   ```sh
   cargo run -q -p anvics-cli -- --repo "$target_repo" workspace snapshot "<workspace-id>" \
     --message "live agent result"

   cargo run -q -p anvics-cli -- --repo "$target_repo" review create --thread "<thread-id>"
   cargo run -q -p anvics-cli -- --repo "$target_repo" review show "<review-id>"
   cargo run -q -p anvics-cli -- --repo "$target_repo" publish create \
     --thread "<thread-id>" \
     --review "<review-id>"
   ```

## What To Check

- The agent edited only the workspace path.
- No Git branch, worktree, or commit was needed.
- Evidence is a short summary, not a transcript dump.
- The review shows changed paths and evidence.
- Publication points to the accepted native snapshot.
