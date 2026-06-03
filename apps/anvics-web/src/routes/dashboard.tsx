import { A } from "@solidjs/router";

export default function DashboardRoute() {
  return (
    <div class="anvics-app hosted-shell">
      <aside class="app-sidebar">
        <div class="brand-lockup">
          <div class="brand-mark" aria-hidden="true">
            A
          </div>
          <div>
            <p class="eyebrow">Hosted Anvics</p>
            <h1>Dashboard</h1>
          </div>
        </div>
        <nav class="primary-nav">
          <button class="nav-item active">Projects</button>
          <button class="nav-item">Recent reviews</button>
          <button class="nav-item">Sync status</button>
        </nav>
      </aside>
      <main class="app-main">
        <header class="repo-header">
          <div>
            <p class="eyebrow">Hosted shell</p>
            <h2>Native projects</h2>
          </div>
          <A class="button button-secondary" href="/">
            Local console
          </A>
        </header>
        <section class="workspace-surface">
          <div class="panel">
            <header class="panel-heading">
              <div>
                <p class="eyebrow">Projects</p>
                <h2>Awaiting native push sync</h2>
              </div>
              <span>0 projects</span>
            </header>
            <div class="review-detail">
              <p>
                This dashboard will list pushed native Anvics projects, recent review
                activity, and sync health. Hosted does not execute local agents or commands.
              </p>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}
