# 10 - Enterprise Controls

## Research Question

What controls are necessary for teams to allow agent-generated work into real repositories?

## Observed Facts

- Enterprise agent products expose RBAC, SSO/SCIM, audit logs, secrets, git permissions, usage analytics, guardrails, and organization-level policy.
- Claude Code hooks and MCP controls show the need for lifecycle policy and tool governance.
- Windsurf and Devin expose team/admin surfaces around analytics, roles, policies, and usage.
- Copilot coding agent consumes GitHub Actions/premium request allowances, so cost and quota are part of operational control.
- Security research around prompt injection and GitHub events suggests source-control triggers can become attack surfaces.

## Sources And Artifacts

- Devin audit logs: https://docs.devin.ai/api-reference/v3/audit-logs/enterprise-audit-logs.md
- Devin git permissions: https://docs.devin.ai/api-reference/v3/git-permissions/organizations-git-providers-permissions.md
- Devin guardrail violations: https://docs.devin.ai/api-reference/v3/guardrail-violations/enterprise-guardrail-violations.md
- Windsurf RBAC: https://docs.windsurf.com/windsurf/accounts/rbac-role-management.md
- Windsurf enterprise policies: https://docs.windsurf.com/windsurf/enterprise-policies.md
- GitHub Copilot coding agent docs: https://docs.github.com/copilot/concepts/coding-agent/about-copilot-coding-agent
- Claude Code hooks: https://docs.anthropic.com/en/docs/claude-code/hooks

## Product Implications

- Enterprise controls should bind to source-control objects: work thread, session, attempt, evidence, native publication, and legacy Git artifact.
- Command/tool policy and secret policy should be recorded as part of provenance.
- Cost/quota metadata should be visible per session and per accepted artifact.
- Audit should answer: who delegated, what agent ran, what it accessed, what it changed, who accepted it, and what was pushed.
- Secret-risk events should create hard default policy gates for publication, sync, and evidence sharing.
- Redaction, quarantine, false-positive approval, and privileged override should be auditable source-control events.

## MVP Impact

- Add organization/project policy metadata to work threads.
- Track actor, agent, permissions, visibility, and export decision.
- Provide audit export in a simple structured format before building a full admin console.
- Add `secret_risk` policy gate with privileged approval/redaction path.

## Open Questions

- Which controls are must-have for first team adoption versus enterprise later?
- Should v1 enforce command policies or only record policy context?
- How should prompt-injection risk from issues/PR comments affect automatic triggers?
- Who can clear a `secret_risk` event, and how should that approval be represented?
