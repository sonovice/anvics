# 05 - Privacy And Visibility

## Research Question

How should Anvics handle private work, public artifacts, and selective disclosure?

## Observed Facts

- Theo's original signal emphasizes that open source does not mean every in-flight fix, package, security patch, or attempt should be public immediately.
- Current Git/GitHub workflows tend to expose branch/PR artifacts earlier than teams may want, especially for public repos.
- Agent traces may contain sensitive details: prompts, filenames, logs, environment shape, secrets policy, internal URLs, vulnerability details, or failed attempts.
- Enterprise agent products expose roles, organization permissions, git permissions, secrets, audit logs, and policy surfaces.
- Radicle's collaborative objects show that review and discussion can be source-controlled objects, not only SaaS database rows.

## Sources And Artifacts

- Radicle overview: https://radicle.dev/
- Devin git permissions: https://docs.devin.ai/api-reference/v3/git-permissions/organizations-git-providers-permissions.md
- Devin secrets: https://docs.devin.ai/api-reference/v3/secrets/organizations-secrets.md
- GitHub repository visibility docs: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/managing-repository-settings/setting-repository-visibility
- Claude Code hooks security warning: https://docs.anthropic.com/en/docs/claude-code/hooks
- Local artifact: `/Users/simon/Documents/Codex/2026-05-27/please-read-the-post-https-x/repos/gitbutler/crates/but-agentlog/src/redaction.rs`

## Product Implications

- Visibility must apply to work threads, attempts, evidence, traces, files, generated artifacts, native publications, and legacy Git exports.
- Public PRs should be able to reference private work-thread records without exposing them.
- Security fixes need embargoed work threads and controlled publication.
- Redaction and retention are product primitives, not admin afterthoughts.
- `secret_risk` must be a hard default gate for publication, sync, and evidence sharing until redacted or privileged approval clears it.
- Secret-risk content should be quarantined/redacted before broad storage or sync; UI-only redaction is insufficient.

## Secret Risk And Redaction

Secret detection should be layered:

1. Built-in scanner for common API keys, private keys, tokens, env files, and high-entropy strings with suspicious context.
2. Repo/org policy with custom patterns, sensitive paths, and allowlisted false positives.
3. Agent claims that mark content as secret-like or suspected false positive.
4. Privileged human/security override.

If a `FileEffect`, evidence artifact, command output, or trace chunk is classified `secret_risk`:

- Do not publish.
- Do not sync broadly.
- Do not attach as shared evidence.
- Quarantine raw content if policy allows retention.
- Expose only redacted content by default.
- Record detection metadata, redaction status, and approval attribution without exposing the secret value.

Agents may suggest false positives, but should not be able to clear `secret_risk` alone.

## MVP Impact

- Implement simple visibility levels in the model: `private`, `team`, `public-artifact-only`.
- Generate public review projections that omit raw traces unless explicitly allowed.
- Store export metadata so a public native publication or legacy Git artifact can point to a private internal record.
- Add built-in `secret_risk` classification and default hard gates.
- Add quarantine/redaction state for traces, evidence artifacts, command outputs, and file effects.

## Open Questions

- Does v1 need file-level privacy or only thread/evidence-level privacy?
- How should public open-source PRs link to private evidence without making maintainers suspicious?
- Should private failed attempts ever be visible to external reviewers?
- What built-in secret detectors are good enough for v1 without producing too much noise?
