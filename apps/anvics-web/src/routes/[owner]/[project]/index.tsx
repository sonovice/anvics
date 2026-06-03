import { useParams } from "@solidjs/router";

export default function HostedProjectRoute() {
  const params = useParams();
  return (
    <div class="anvics-app hosted-shell">
      <aside class="app-sidebar">
        <div class="brand-lockup">
          <div class="brand-mark" aria-hidden="true">
            A
          </div>
          <div>
            <p class="eyebrow">Hosted Anvics</p>
            <h1>{params.owner}/{params.project}</h1>
          </div>
        </div>
        <nav class="primary-nav" aria-label="Hosted project sections">
          <button class="nav-item active">Overview</button>
          <button class="nav-item">Inbox</button>
          <button class="nav-item">Source</button>
          <button class="nav-item">Publications</button>
          <button class="nav-item">Events</button>
        </nav>
      </aside>

      <main class="app-main">
        <header class="repo-header">
          <div>
            <p class="eyebrow">Project</p>
            <h2>/{params.owner}/{params.project}</h2>
          </div>
          <span class="daemon-badge healthy">hosted shell</span>
        </header>
        <section class="workspace-surface">
          <div class="panel">
            <PanelHeading eyebrow="Hosted roadmap" title="Native repo review surface" detail="Not GitHub-backed" />
            <div class="review-detail">
              <p>
                This route is reserved for the hosted Anvics app. It will render the same
                operator views as local mode, backed by pushed native Anvics snapshots,
                reviews, publications, risks, conflicts, restores, and events.
              </p>
              <div class="overview-band">
                <Summary label="Identity" value="email + username" />
                <Summary label="Storage" value="Postgres + object store" />
                <Summary label="Sync" value="local push" />
                <Summary label="Execution" value="local only" />
                <Summary label="GitHub" value="not required" />
              </div>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}

function PanelHeading(props: { eyebrow: string; title: string; detail: string }) {
  return (
    <header class="panel-heading">
      <div>
        <p class="eyebrow">{props.eyebrow}</p>
        <h2>{props.title}</h2>
      </div>
      <span>{props.detail}</span>
    </header>
  );
}

function Summary(props: { label: string; value: string }) {
  return (
    <div class="summary-metric neutral">
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </div>
  );
}
