# Anvics

Agent-native VCS for the next generation of coders.

Anvics is a native source-control substrate for parallel AI-agent software development. Git remains a legacy import/export boundary, not the internal model.

## Start Here

- [MVP 0](docs/MVP_0.md): the first implementation target.
- [Platform Roadmap](docs/PLATFORM_ROADMAP.md): longer-term architecture after MVP 0 proves the loop.
- [Product Thesis](docs/GIT_ALTERNATIVE.md): the full Anvics thesis and research synthesis.
- [Decision Log](docs/ANVICS_DECISIONS.md): compact record of accepted product and architecture decisions.
- [Agent Test Runbook](docs/AGENT_TEST_RUNBOOK.md): first scripted and manual agent test flow.
- [Agent Skill](skills/anvics-skill/SKILL.md): draft instructions for agents working in Anvics repos.

## Development

The first Rust slice proves native local snapshots through the CLI:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/agent_smoke.sh
scripts/live_agent_packet_smoke.sh
```
