---
name: anvics
description: Use this when working in an Anvics repository. Guides agents to use Anvics threads, workspaces, snapshots, evidence, review, and publish flows instead of ad hoc Git branches, scratch commits, or filesystem-only workflows.
---

# Anvics

Use Anvics as the source-control interface. Treat the filesystem as a compatibility view for tools that require paths.

An Anvics workspace may not be a Git repository. If your CLI complains about a missing Git repository, use the tool's non-Git workspace option and continue with the packet's Anvics commands.

## First-Run Operator Setup

If the operator asks you to "use Anvics", "set up Anvics", "run the Anvics workflow", or similar, take care of the setup loop instead of making the operator learn commands.

1. Check whether the `anvics` command is available.
2. If it is not available, look for this repository's CLI source and use the repo-local Cargo command prefix or install the CLI from source when the operator has asked for setup.
3. Install or render repo-level agent instructions with `agent instructions --install` when appropriate and when doing so will not overwrite existing guidance.
4. Initialize the target repo only when the operator has clearly chosen that repo or asks for a disposable trial.
5. Create the base snapshot, prepare the workspace, and show the operator the packet/launch guidance.
6. Keep the human-facing summary conceptual: active work, evidence, risk, recovery, review, publish/export. Do not dump every CLI command unless the operator asks.

The intended onboarding is: the human installs or points you at this skill, then you drive the Anvics workflow and explain the decisions.

## Core Workflow

1. Create or join a `thread` for the task.
2. Inspect the current `snapshot`.
3. Create or select an isolated `workspace`.
4. Run `agent enter` for the workspace before editing.
5. Use `agent context-pack` when you need a compact refreshed briefing for task, diff, and coordination state.
6. Edit through Anvics APIs when available, or through the workspace path when tools need files.
7. Run `coordination status` before finishing.
8. Use `workspace diff` to inspect changed files before finishing; do not rely on Git status or Git diff inside an Anvics workspace.
9. Run `agent checkpoint` at meaningful pause, handoff, or risk boundaries so progress can be recovered if the agent exits unexpectedly.
10. Prefer Anvics-run verification when possible; attach compact `evidence`.
11. Request `review` when the work is ready.
12. `publish` after acceptance; export to Git only when a legacy system needs it.

## Delegation And Resolution

Do not spawn or delegate to additional agents for normal Anvics work unless the operator explicitly asks you to. Anvics prepares workspaces and packets; it does not require or assume Codex, Claude, Cursor, or any other specific agent runtime.

When resolving competing reviews, the current assigned agent should usually read the source reviews and resolve the work in its own workspace instead of asking for a new agent. If the operator explicitly requests delegation, or if your harness automatically uses subagents, pass along the Anvics context before any delegated agent reads or edits code:

- The path to this skill.
- The task packet path when one exists.
- The repository path, thread id, workspace id, and workspace path.
- The packet's Anvics command prefix, especially when `anvics` is not installed on `PATH`.
- The packet's `agent enter`, `agent context-pack`, `workspace diff`, and `coordination status` commands.
- The rule that the workspace path is the only editable area.

Any delegated agent must read this skill, use the packet's Anvics command prefix, run `agent enter` with its own agent name before editing, use `agent context-pack` when it needs a refreshed brief, use `workspace diff` instead of Git diff/status, and report coordination status before returning work. Do not ask delegated agents to create Git branches, Git worktrees, Git commits, or publications unless the operator explicitly asks for legacy Git output.

## Source Access

Prefer Anvics API reads before materializing files:

- List the tree.
- Read specific paths.
- Search code.
- Check snapshot freshness.

Use a filesystem workspace only when a compiler, test runner, formatter, language server, browser, debugger, or similar tool genuinely needs paths. Do not create manual Git worktrees or full repo copies for parallel work.

## Editing

Every write belongs to the current thread and workspace.

- Start by running the packet's `agent enter` command.
- Use the packet's Anvics command prefix exactly. If it is unavailable, report that blocker instead of switching to Git.
- Use structured patch/write APIs when possible.
- Refresh task, diff, and coordination context with the packet's `agent context-pack` command when useful.
- Inspect local changes with the packet's `workspace diff` command.
- Preserve recoverable progress with `agent checkpoint` before long pauses, risky edits, handoffs, or context loss.
- If work needs to be backed out, ask the operator to use `workspace restore` or `agent checkpoint restore` with an audited reason. Do not manually delete files to simulate a restore.
- Keep generated-file edits explicit.
- If the base is stale, refresh or fork the workspace instead of forcing the edit.
- If another thread touches the same path or hunk, record the overlap before continuing.
- Before finishing, run `coordination status` and summarize any potential clashes in evidence or review notes.
- When the operator explicitly asks you to accept with verification, prefer `agent accept --run-label ... --run-summary ... -- <program> [args...]` so Anvics records command provenance and artifacts itself.
- Risky operator-run verification commands may be blocked by command policy. Do not add override flags yourself unless the operator explicitly approves the command risk and provides the reason.
- Do not run `agent accept`, `publish create`, or `legacy git export` unless the operator explicitly asks you to accept, publish, or export.

Do not use commits as scratchpad history. Keep failed or abandoned attempts attached to the thread as summarized evidence.
If an agent crashes or exits before finishing, the operator should use `agent recover --workspace <id>` to inspect current changed paths and the latest checkpoint. If checkpointed progress should become the workspace state again, the operator should use `agent checkpoint restore --workspace <id> --checkpoint <checkpoint-id> --reason "<audited reason>"`.

## Evidence Budget

Attach enough evidence for review, but keep it compact.

- Prefer Anvics-run command evidence over self-reported command evidence.
- Record command name, exit status, duration, and a short result summary.
- Store raw logs, screenshots, traces, and reports as artifacts or references.
- Quote only key excerpts that explain a failure, risk, or verification result.
- Avoid dumping full transcripts unless explicitly requested.
- Do not paste secrets, tokens, private keys, `.env` values, or credentials into summaries, review notes, or inline evidence.
- If you suspect a secret appeared in source changes or command output, say so plainly and let Anvics risk scanning block publication for operator review.
- Record uncertainty and unverified areas directly.

Evidence should support review. It should not become a second codebase made of logs.

## Command Policy

Anvics classifies Anvics-run commands before execution, including commands loaded from `--command-file`. Read-only, mutating, destructive, and unknown commands are allowed by default inside the isolated workspace. Networked, host-escape-risk, and interactive commands are blocked unless the operator explicitly approves them with `--allow-command-risk --command-risk-reason "<reason>"`.

This is a provenance and workflow gate, not a sandbox. If a blocked command seems necessary, report the exact command and why it is needed, then wait for the operator. Operators can preview unfamiliar commands with `anvics command classify -- <program> [args...]` or `anvics command classify --command-file <path>`.

## Review And Publish

Before publish:

- Create a final snapshot.
- Attach verification evidence.
- Generate a review with final diff, intent, risks, evidence, and unresolved questions.
- Run or expect a risk scan before publication; agents should not bypass secret-risk findings.
- Wait for required acceptance or policy approval.

Publish through the native Anvics flow. Export to Git only after native acceptance, and only when a legacy Git system needs a commit, branch, patch series, or pull request.

## Avoid Legacy Defaults

- Do not create ad hoc Git branches as the task boundary.
- Do not make noisy intermediate commits to preserve memory.
- Do not use Git status or Git diff as the source of truth inside an Anvics workspace.
- Do not discard failed attempts without summarizing why.
- Do not treat the filesystem workspace as canonical when Anvics APIs are available.
- Do not export to Git before review evidence is attached.
