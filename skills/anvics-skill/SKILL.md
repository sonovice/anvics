---
name: anvics
description: Use this when working in an Anvics repository. Guides agents to use Anvics threads, workspaces, snapshots, evidence, review, and publish flows instead of ad hoc Git branches, scratch commits, or filesystem-only workflows.
---

# Anvics

Use Anvics as the source-control interface. Treat the filesystem as a compatibility view for tools that require paths.

## Core Workflow

1. Create or join a `thread` for the task.
2. Inspect the current `snapshot`.
3. Create or select an isolated `workspace`.
4. Run `agent enter` for the workspace before editing.
5. Edit through Anvics APIs when available, or through the workspace path when tools need files.
6. Run `coordination status` before finishing.
7. Use `workspace diff` to inspect changed files before finishing; do not rely on Git status or Git diff inside an Anvics workspace.
8. Create a new `snapshot` at meaningful checkpoints.
9. Prefer Anvics-run verification when possible; attach compact `evidence`.
10. Request `review` when the work is ready.
11. `publish` after acceptance; export to Git only when a legacy system needs it.

## Delegation And Subagents

If you spawn or delegate to another agent, pass along the Anvics context before it reads or edits code:

- The path to this skill.
- The task packet path when one exists.
- The repository path, thread id, workspace id, and workspace path.
- The packet's `agent enter`, `workspace diff`, and `coordination status` commands.
- The rule that the workspace path is the only editable area.

Each subagent must read this skill, run `agent enter` with its own agent name before editing, use `workspace diff` instead of Git diff/status, and report coordination status before returning work. Do not ask subagents to create Git branches, Git worktrees, Git commits, or publications unless the operator explicitly asks for legacy Git output.

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
- Use structured patch/write APIs when possible.
- Inspect local changes with the packet's `workspace diff` command.
- Keep generated-file edits explicit.
- If the base is stale, refresh or fork the workspace instead of forcing the edit.
- If another thread touches the same path or hunk, record the overlap before continuing.
- Before finishing, run `coordination status` and summarize any potential clashes in evidence or review notes.
- When the operator explicitly asks you to accept with verification, prefer `agent accept --run-label ... --run-summary ... -- <program> [args...]` so Anvics records command provenance and artifacts itself.
- Do not run `agent accept`, `publish create`, or `legacy git export` unless the operator explicitly asks you to accept, publish, or export.

Do not use commits as scratchpad history. Keep failed or abandoned attempts attached to the thread as summarized evidence.

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
