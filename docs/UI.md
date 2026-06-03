# Anvics Local And Hosted UI

The first Anvics UI is an operator app for supervising agent work. It is no longer just a read-only Review Inbox: local mode is an operator console with Inbox, Threads, Source, Conflicts, Recovery, Publications, Events, and Settings sections.

## Local Mode

Local mode talks to `anvicsd` through an opt-in localhost HTTP bridge. The existing Unix-socket JSON-RPC API remains unchanged.

Start the bridge:

```sh
anvics daemon start --socket /tmp/anvics.sock --http 127.0.0.1:3897
```

Useful endpoints:

- `GET /api/health`
- `GET /api/review-inbox?repo=<absolute-repo-path>`
- `POST /api/rpc` with the same `ApiRequest` JSON used by the socket API

CORS is only emitted for localhost origins in local mode.

## Web App

The SolidStart app lives in `apps/anvics-web`.

```sh
cd apps/anvics-web
VITE_ANVICS_HTTP_URL=http://127.0.0.1:3897 \
VITE_ANVICS_REPO=/absolute/path/to/anvics/repo \
bun run dev
```

Current local screens:

- `/`: local operator console over the daemon HTTP bridge
- Inbox: queue filters, selected review detail, evidence, risk notes, changed paths, publication/export actions, workspace accept/recovery actions
- Threads: thread/workspace overview and launch-prompt access
- Source: native snapshots and selected workspace diff entrypoint
- Conflicts: conflict analysis, resolver preparation, and verification entrypoints
- Recovery: workspace recover/restore, checkpoint, evidence supersede, publication revert preparation
- Publications: native publication timeline and legacy patch export entrypoint
- Events: append-only audit stream
- Settings: local repo/daemon configuration and hosted roadmap notes
- `/login`, `/register`, `/dashboard`: hosted identity/dashboard shells reserved for the native sync milestone
- `/{username}/{project}`: hosted project shell using the future hosted URL shape

Use Bun for frontend dependency management and scripts.

The app uses `POST /api/rpc` for most browser actions. Mutating actions must keep the same audit rules as the CLI: restore/revert/evidence supersede and risk overrides require explicit reasons, and the UI must show resulting ids after the action.

## Hosted Direction

Hosted Anvics should reuse the same UI client contract, but backed by a Rust hosted API instead of a local daemon. The hosted product target remains:

- email login
- unique usernames
- project URLs at `/{username}/{project}`
- native Anvics repo data, not GitHub integration
- local-to-hosted native push sync
- hosted review/source/history surfaces

Hosted does not execute local agents, commands, VFS projections, or workspace restores in the first milestone.
