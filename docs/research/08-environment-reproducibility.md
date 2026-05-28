# 08 - Environment Reproducibility

## Research Question

What environment evidence is needed to trust and replay agent work?

## Observed Facts

- Agents run in diverse environments: local IDEs, terminals, GitHub Actions, cloud sandboxes, VM snapshots, devcontainers, and custom enterprise infrastructure.
- Codex uses isolated cloud environments configured by setup scripts and repo instructions.
- Copilot coding agent uses GitHub-hosted infrastructure/GitHub Actions concepts.
- Jules exposes environment setup and environment snapshots.
- Devin exposes hypervisors, sessions, secrets, and enterprise infrastructure APIs.
- Bazel/RBE and Nix show stronger models for reproducible actions and environments, but both can be too high-friction for ordinary repos.

## Sources And Artifacts

- OpenAI Codex intro: https://openai.com/index/introducing-codex/
- GitHub Copilot coding agent docs: https://docs.github.com/copilot/concepts/coding-agent/about-copilot-coding-agent
- Jules environment setup: https://jules.google/docs/environment/
- Jules environment snapshots changelog: https://jules.google/docs/changelog/2025-08-05/
- Devin infrastructure APIs: https://docs.devin.ai/api-reference/v3/hypervisors/hypervisors.md
- Bazel Remote Execution: https://bazel.build/remote/rbe
- Nix reproducible builds: https://reproducible.nixos.org/

## Product Implications

- The system needs an `EnvironmentSnapshot` primitive, even if v1 starts with lightweight fingerprints.
- Verification evidence must point to the exact snapshot/environment that produced it.
- Secret access and environment variables need policy metadata, not raw value capture.
- Replay will be unreliable unless base source state and environment state are both tracked.
- Commands should run against a pinned `ExecutionView` when mediated by Anvics: exact workspace overlay version, source snapshot, environment fingerprint, and execution policy.
- Externally-run commands can be attached as `CommandEvent`s, but should be marked as weaker provenance.

## Command Execution Model

Anvics should support command execution without becoming a full agent runner.

Canonical path:

1. Agent requests command execution through Anvics.
2. `anvicsd` pins an `ExecutionView`.
3. `anvics-worker` runs the command against that view.
4. Output, exit status, file effects, and evidence artifacts stream back to `anvicsd`.
5. Anvics records a `CommandEvent` and attaches relevant evidence.

Fallback path:

1. External agent runs a command itself in a VFS mount or fallback materialized directory.
2. Agent attaches command metadata/output afterward.
3. Anvics records the event as externally-run with weaker provenance.

V1 command execution should enforce execution policy and capture provenance, not promise complete security sandboxing. Policies can cover cwd, env allow/deny rules, secret access, network allowance, timeout, max output size, allowed command patterns, source write capture, and evidence artifact rules. Strong isolation should be delegated to containers, VMs, devcontainers, CI runners, or cloud sandboxes.

`anvics-worker` should be a separate process coordinated by `anvicsd`. V1 can ship it as a subcommand or same-binary process, but it should be conceptually separate so remote/cloud workers can be added later.

## File Effects

Command writes should be captured as neutral `FileEffectSet`s before they become source-control changes.

Minimum shape:

- `FileEffectSet`
  - `base_execution_view_id`
  - `command_event_id`
  - `effects`
- `FileEffect`
  - path
  - operation: create, modify, delete, rename, chmod, or symlink change
  - before object ID, if any
  - after object ID, if any
  - before/after metadata
  - size delta
  - ignored-by-config marker
  - outside-write-scope marker
  - classifications

Classifications are layered on top of neutral effects:

- source: agent claim, policy, heuristic, human, or tool
- label: source, generated tracked, generated untracked, evidence candidate, cache, lockfile, config, or unknown
- confidence
- rationale
- policy status

The core should not hard-code language or tool semantics. Agents can use tool knowledge to classify effects, but those classifications remain claims unless policy/tooling/humans verify them.

Conversion rule:

- tracked source effects become `ChangeUnit`s
- tracked generated source effects become `ChangeUnit`s with generated-source risk/metadata
- ignored/cache output does not become `ChangeUnit`s
- evidence candidates become `EvidenceArtifact` suggestions
- outside-scope writes become policy violations or approval requests
- unknown effects require classification before native publication

Anvics should mechanically propose `ChangeUnit`s from file effects, agents can refine them, and policy/human gates should block unresolved or sensitive cases.

## MVP Impact

- Capture base commit, setup command/script reference, package manager lockfiles, tool versions where available, OS/container label, env policy, and cache key.
- Attach every test/eval command to an environment snapshot ID.
- Do not require Nix/Bazel; support them as stronger evidence when present.
- Add `ExecutionView`, `CommandEvent`, and `FileEffectSet` to the execution/evidence model.
- Implement local `anvics-worker` first; design worker messages so remote/cloud workers can later report command output, file effects, evidence artifacts, heartbeat, cancellation, and exit status.
- Convert trackable source-relevant `FileEffect`s into proposed `ChangeUnit`s; keep cache/evidence/ignored effects out of source changes.

## Open Questions

- What is the minimum environment fingerprint that gives useful trust without annoying users?
- What command policy defaults are strict enough to be useful without blocking normal agent workflows?
- How should secret access be represented without leaking secrets?
- Which classifications should be built-in labels versus repo/plugin-defined labels?
