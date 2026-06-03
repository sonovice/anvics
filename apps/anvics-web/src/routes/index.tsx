import {
  JSX,
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createMemo,
  createResource,
  createSignal,
} from "solid-js";
import {
  changeLabel,
  diffWorkspace,
  fetchEventsSince,
  fetchHealth,
  fetchReviewInbox,
  fetchSnapshots,
  fetchThreads,
  formatDate,
  isPublished,
  repoRoot,
  rpc,
  shortId,
  showWorkspace,
  summarizeInbox,
} from "../lib/anvics-api";
import type {
  AgentLaunchPrompt,
  ChangedPath,
  ConflictAnalysis,
  EvidenceSummary,
  NativePublication,
  RepositoryEvent,
  ReviewInboxItem,
  ReviewProjection,
  RiskFinding,
  SourceSnapshot,
  WorkThread,
  WorkspaceView,
} from "../lib/anvics-api";

type Section =
  | "inbox"
  | "threads"
  | "source"
  | "conflicts"
  | "recovery"
  | "publications"
  | "events"
  | "settings";

type QueueFilter =
  | "all"
  | "needs_review"
  | "risk"
  | "conflict"
  | "unpublished"
  | "published"
  | "no_evidence";

type QueueSort = "newest" | "risk" | "conflict" | "unreviewed";

type ActionState = {
  status: "idle" | "running" | "success" | "error";
  message: string;
};

const sections: Array<{ id: Section; label: string }> = [
  { id: "inbox", label: "Inbox" },
  { id: "threads", label: "Threads" },
  { id: "source", label: "Source" },
  { id: "conflicts", label: "Conflicts" },
  { id: "recovery", label: "Recovery" },
  { id: "publications", label: "Publications" },
  { id: "events", label: "Events" },
  { id: "settings", label: "Settings" },
];

export default function OperatorConsoleRoute() {
  const [activeSection, setActiveSection] = createSignal<Section>("inbox");
  const [selectedThreadId, setSelectedThreadId] = createSignal<string | null>(null);
  const [filter, setFilter] = createSignal<QueueFilter>("all");
  const [sort, setSort] = createSignal<QueueSort>("newest");
  const [reloadToken, setReloadToken] = createSignal(0);
  const [action, setAction] = createSignal<ActionState>({
    status: "idle",
    message: "No operator action has run in this browser session.",
  });

  const refresh = () => setReloadToken((value) => value + 1);
  const [health] = createResource(reloadToken, fetchHealth);
  const [items] = createResource(reloadToken, () => fetchReviewInbox());
  const [threads] = createResource(reloadToken, fetchThreads);
  const [snapshots] = createResource(reloadToken, fetchSnapshots);
  const [events] = createResource(reloadToken, () => fetchEventsSince(0));

  const inbox = createMemo(() => items() ?? []);
  const threadList = createMemo(() => threads()?.threads ?? inbox().map((item) => item.thread));
  const snapshotList = createMemo(() => snapshots()?.snapshots ?? []);
  const eventList = createMemo(() => events()?.events ?? []);
  const summary = createMemo(() => summarizeInbox(inbox()));

  const filteredInbox = createMemo(() =>
    inbox()
      .filter((item) => matchesFilter(item, filter()))
      .sort((left, right) => compareItems(left, right, sort())),
  );
  const selected = createMemo(() => {
    const rows = filteredInbox();
    return (
      rows.find((item) => item.thread.id === selectedThreadId()) ??
      rows.find((item) => !isPublished(item)) ??
      rows[0] ??
      inbox()[0]
    );
  });

  createEffect(() => {
    const current = selected();
    if (current && selectedThreadId() !== current.thread.id) {
      setSelectedThreadId(current.thread.id);
    }
  });

  const runAction = async (label: string, task: () => Promise<string>) => {
    setAction({ status: "running", message: `${label} running...` });
    try {
      const message = await task();
      setAction({ status: "success", message });
      refresh();
    } catch (error) {
      setAction({
        status: "error",
        message: error instanceof Error ? error.message : String(error),
      });
    }
  };

  return (
    <div class="anvics-app">
      <a class="skip-link" href="#workspace">
        Skip to workspace
      </a>

      <aside class="app-sidebar">
        <div class="brand-lockup">
          <div class="brand-mark" aria-hidden="true">
            A
          </div>
          <div>
            <p class="eyebrow">Anvics</p>
            <h1>Operator Console</h1>
          </div>
        </div>
        <nav class="primary-nav" aria-label="Anvics sections">
          <For each={sections}>
            {(entry) => (
              <button
                class="nav-item"
                classList={{ active: activeSection() === entry.id }}
                onClick={() => setActiveSection(entry.id)}
              >
                {entry.label}
              </button>
            )}
          </For>
        </nav>
        <div class="sidebar-note">
          <strong>Local mode</strong>
          <span>Agents use packets and CLI. Humans use this console.</span>
        </div>
      </aside>

      <div class="app-main">
        <header class="repo-header">
          <div>
            <p class="eyebrow">Repository</p>
            <h2>{repoRoot || "No repository configured"}</h2>
          </div>
          <div class="repo-header-actions">
            <DaemonBadge loading={health.loading} healthy={Boolean(health()?.ok)} />
            <span class="event-seq">event seq {latestSequence(eventList())}</span>
            <button class="button button-secondary" onClick={refresh}>
              Refresh
            </button>
          </div>
        </header>

        <Show when={!repoRoot}>
          <Notice
            tone="risk"
            title="Missing repository"
            text="Set VITE_ANVICS_REPO to an initialized Anvics repository path before using the local operator console."
          />
        </Show>

        <ActionBanner state={action()} />

        <main id="workspace" class="workspace-surface">
          <OverviewBand summary={summary()} />
          <Switch>
            <Match when={activeSection() === "inbox"}>
              <InboxSection
                items={filteredInbox()}
                selected={selected()}
                filter={filter()}
                sort={sort()}
                loading={items.loading}
                error={items.error}
                onFilter={setFilter}
                onSort={setSort}
                onSelect={(item) => setSelectedThreadId(item.thread.id)}
                runAction={runAction}
              />
            </Match>
            <Match when={activeSection() === "threads"}>
              <ThreadsSection
                threads={threadList()}
                inbox={inbox()}
                selected={selected()}
                runAction={runAction}
              />
            </Match>
            <Match when={activeSection() === "source"}>
              <SourceSection snapshots={snapshotList()} selected={selected()} runAction={runAction} />
            </Match>
            <Match when={activeSection() === "conflicts"}>
              <ConflictsSection inbox={inbox()} runAction={runAction} />
            </Match>
            <Match when={activeSection() === "recovery"}>
              <RecoverySection inbox={inbox()} selected={selected()} runAction={runAction} />
            </Match>
            <Match when={activeSection() === "publications"}>
              <PublicationsSection inbox={inbox()} runAction={runAction} />
            </Match>
            <Match when={activeSection() === "events"}>
              <EventsSection events={eventList()} />
            </Match>
            <Match when={activeSection() === "settings"}>
              <SettingsSection health={health()} events={eventList()} />
            </Match>
          </Switch>
        </main>
      </div>
    </div>
  );
}

function OverviewBand(props: { summary: ReturnType<typeof summarizeInbox> }) {
  return (
    <section class="overview-band" aria-label="Repository summary">
      <SummaryMetric label="Needs review" value={props.summary.needsReview} tone="pending" />
      <SummaryMetric label="Risk queues" value={props.summary.risk} tone="risk" />
      <SummaryMetric label="Conflicts" value={props.summary.conflicts} tone="conflict" />
      <SummaryMetric label="Published" value={props.summary.published} tone="published" />
      <SummaryMetric label="Workspaces" value={props.summary.workspaces} tone="neutral" />
    </section>
  );
}

function InboxSection(props: {
  items: ReviewInboxItem[];
  selected?: ReviewInboxItem;
  filter: QueueFilter;
  sort: QueueSort;
  loading: boolean;
  error: unknown;
  onFilter: (filter: QueueFilter) => void;
  onSort: (sort: QueueSort) => void;
  onSelect: (item: ReviewInboxItem) => void;
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  return (
    <section class="console-grid">
      <div class="panel queue-panel">
        <PanelHeading
          eyebrow="Review Inbox"
          title="Work queue"
          detail={props.loading ? "Loading" : `${props.items.length} shown`}
        />
        <QueueControls
          filter={props.filter}
          sort={props.sort}
          onFilter={props.onFilter}
          onSort={props.onSort}
        />
        <Switch>
          <Match when={props.error}>
            <EmptyState title="Could not load inbox" text={String(props.error)} />
          </Match>
          <Match when={props.items.length === 0}>
            <EmptyState title="No matching work" text="Change the filter or prepare an agent thread." />
          </Match>
          <Match when={true}>
            <ol class="queue-list">
              <For each={props.items}>
                {(item) => (
                  <li>
                    <QueueRow
                      item={item}
                      active={props.selected?.thread.id === item.thread.id}
                      onSelect={() => props.onSelect(item)}
                    />
                  </li>
                )}
              </For>
            </ol>
          </Match>
        </Switch>
      </div>

      <div class="panel detail-panel">
        <Show
          when={props.selected}
          fallback={<EmptyState title="Select work" text="Pick a thread to inspect evidence, risk, publication, and recovery actions." />}
        >
          {(item) => <ReviewDetail item={item()} runAction={props.runAction} />}
        </Show>
      </div>

      <aside class="panel context-panel">
        <Show
          when={props.selected}
          fallback={<EmptyState title="No context" text="Operator context appears after selecting a thread." />}
        >
          {(item) => <OperatorContext item={item()} runAction={props.runAction} />}
        </Show>
      </aside>
    </section>
  );
}

function QueueControls(props: {
  filter: QueueFilter;
  sort: QueueSort;
  onFilter: (filter: QueueFilter) => void;
  onSort: (sort: QueueSort) => void;
}) {
  const filters: Array<{ id: QueueFilter; label: string }> = [
    { id: "all", label: "All" },
    { id: "needs_review", label: "Needs review" },
    { id: "risk", label: "Risk" },
    { id: "conflict", label: "Conflict" },
    { id: "unpublished", label: "Unpublished" },
    { id: "published", label: "Published" },
    { id: "no_evidence", label: "No evidence" },
  ];
  return (
    <div class="queue-controls">
      <div class="segmented-control" aria-label="Queue filter">
        <For each={filters}>
          {(entry) => (
            <button
              class="segment"
              classList={{ active: props.filter === entry.id }}
              onClick={() => props.onFilter(entry.id)}
            >
              {entry.label}
            </button>
          )}
        </For>
      </div>
      <label class="select-label">
        Sort
        <select value={props.sort} onInput={(event) => props.onSort(event.currentTarget.value as QueueSort)}>
          <option value="newest">Newest</option>
          <option value="risk">Risk first</option>
          <option value="conflict">Conflict first</option>
          <option value="unreviewed">Unreviewed first</option>
        </select>
      </label>
    </div>
  );
}

function QueueRow(props: { item: ReviewInboxItem; active: boolean; onSelect: () => void }) {
  const risks = () => props.item.latest_risk_findings ?? [];
  const review = () => props.item.latest_review;
  return (
    <button class="queue-row" classList={{ active: props.active }} onClick={props.onSelect}>
      <div class="queue-row-top">
        <StatusBadge item={props.item} />
        <span>{formatDate(props.item.thread.created_at)}</span>
      </div>
      <h3>{props.item.thread.title}</h3>
      <p>{props.item.thread.task}</p>
      <div class="queue-row-meta">
        <span>{props.item.workspaces.length} workspace</span>
        <span>{props.item.evidence_count} evidence</span>
        <span>{review()?.changed_paths.length ?? 0} paths</span>
        <Show when={risks().length > 0}>
          <span class="risk-text">{risks().length} risk</span>
        </Show>
      </div>
    </button>
  );
}

function ReviewDetail(props: {
  item: ReviewInboxItem;
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  const review = () => props.item.latest_review;
  const risks = () => props.item.latest_risk_findings ?? [];
  const primaryWorkspace = () => props.item.workspaces[0];
  return (
    <article class="review-detail">
      <header class="review-header">
        <div>
          <StatusBadge item={props.item} />
          <h2>{props.item.thread.title}</h2>
          <p>{props.item.thread.task}</p>
        </div>
        <dl class="id-stack">
          <KeyValue label="Thread" value={shortId(props.item.thread.id)} />
          <KeyValue label="Review" value={shortId(review()?.id)} />
          <KeyValue label="Base" value={shortId(review()?.base_snapshot ?? props.item.thread.base_snapshot)} />
        </dl>
      </header>

      <div class="action-strip">
        <button
          class="button button-primary"
          disabled={!review() || isPublished(props.item)}
          onClick={() => publishReview(props.item, props.runAction)}
        >
          Publish review
        </button>
        <button
          class="button button-secondary"
          disabled={props.item.publication_ids.length === 0}
          onClick={() => exportPublication(props.item, props.runAction)}
        >
          Export patch
        </button>
        <button
          class="button button-secondary"
          disabled={!review()}
          onClick={() => riskScan(props.item, props.runAction)}
        >
          Run risk scan
        </button>
      </div>

      <Show when={review()} fallback={<EmptyState title="No review yet" text="Finish or accept a workspace to create a review projection." />}>
        {(currentReview) => (
          <div class="ledger-stack">
            <LedgerSection title="Changed paths" detail={`${currentReview().changed_paths.length} path(s)`}>
              <ChangedPathTable paths={currentReview().changed_paths} />
            </LedgerSection>
            <LedgerSection title="Evidence" detail={`${currentReview().evidence.length} active record(s)`}>
              <EvidenceTable evidence={currentReview().evidence} runAction={props.runAction} />
            </LedgerSection>
            <LedgerSection title="Risk notes" detail={risks().length ? `${risks().length} finding(s)` : "No findings"}>
              <RiskList findings={risks()} />
            </LedgerSection>
            <LedgerSection title="Overlap notes" detail={currentReview().overlap_notes.length ? `${currentReview().overlap_notes.length} note(s)` : "No overlaps"}>
              <NoteList notes={currentReview().overlap_notes} empty="No overlap notes on this review." />
            </LedgerSection>
          </div>
        )}
      </Show>

      <Show when={primaryWorkspace()}>
        {(workspace) => (
          <WorkspaceActionPanel
            workspace={workspace()}
            item={props.item}
            runAction={props.runAction}
          />
        )}
      </Show>
    </article>
  );
}

function WorkspaceActionPanel(props: {
  workspace: WorkspaceView;
  item: ReviewInboxItem;
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  return (
    <div class="operator-card">
      <div>
        <p class="eyebrow">Workspace</p>
        <h3>{shortId(props.workspace.id)}</h3>
        <p class="path-text">{props.workspace.materialized_path}</p>
      </div>
      <div class="action-grid">
        <button class="button button-secondary" onClick={() => showWorkspaceDiff(props.workspace, props.runAction)}>
          Inspect diff
        </button>
        <button class="button button-secondary" onClick={() => launchPrompt(props.workspace, props.runAction)}>
          Launch prompt
        </button>
        <button class="button button-secondary" onClick={() => checkpointWorkspace(props.workspace, props.runAction)}>
          Checkpoint
        </button>
        <button class="button button-secondary" onClick={() => recoverWorkspace(props.workspace, props.runAction)}>
          Recover
        </button>
        <button class="button button-secondary" onClick={() => restoreWorkspace(props.workspace, props.runAction)}>
          Restore
        </button>
        <button class="button button-primary" disabled={isPublished(props.item)} onClick={() => acceptWorkspaceRun(props.workspace, props.runAction)}>
          Accept with command
        </button>
      </div>
    </div>
  );
}

function OperatorContext(props: {
  item: ReviewInboxItem;
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  const review = () => props.item.latest_review;
  return (
    <div class="operator-context">
      <PanelHeading eyebrow="Operator Context" title="Decision state" detail={decisionText(props.item)} />
      <div class="context-list">
        <KeyValue label="Thread status" value={props.item.thread.status} />
        <KeyValue label="Publication count" value={String(props.item.publication_ids.length)} />
        <KeyValue label="Evidence count" value={String(props.item.evidence_count)} />
        <KeyValue label="Risk findings" value={String((props.item.latest_risk_findings ?? []).length)} />
        <KeyValue label="Conflict analysis" value={shortId(props.item.thread.conflict_analysis_id)} />
      </div>
      <Show when={review()?.source_review_ids?.length}>
        <div class="compact-section">
          <h4>Source reviews</h4>
          <p>{review()?.source_review_ids?.map(shortId).join(", ")}</p>
        </div>
      </Show>
      <div class="compact-section">
        <h4>Recommended path</h4>
        <p>{recommendedPath(props.item)}</p>
      </div>
    </div>
  );
}

function ThreadsSection(props: {
  threads: WorkThread[];
  inbox: ReviewInboxItem[];
  selected?: ReviewInboxItem;
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  return (
    <section class="two-column">
      <div class="panel">
        <PanelHeading eyebrow="Threads" title="Agent work threads" detail={`${props.threads.length} thread(s)`} />
        <DataTable
          headers={["Status", "Title", "Base", "Created"]}
          rows={props.threads.map((thread) => [
            thread.status,
            thread.title,
            shortId(thread.base_snapshot),
            formatDate(thread.created_at),
          ])}
        />
      </div>
      <div class="panel">
        <PanelHeading eyebrow="Thread detail" title={props.selected?.thread.title ?? "Select from Inbox"} detail="Workspaces, launch, evidence" />
        <Show when={props.selected} fallback={<EmptyState title="No selected thread" text="Use the Inbox to select a thread." />}>
          {(item) => (
            <div class="ledger-stack">
              <LedgerSection title="Task" detail={shortId(item().thread.id)}>
                <p>{item().thread.task}</p>
              </LedgerSection>
              <LedgerSection title="Workspaces" detail={`${item().workspaces.length} workspace(s)`}>
                <For each={item().workspaces}>
                  {(workspace) => (
                    <div class="workspace-row">
                      <div>
                        <strong>{shortId(workspace.id)}</strong>
                        <span>{workspace.materialized_path}</span>
                      </div>
                      <button class="button button-secondary" onClick={() => launchPrompt(workspace, props.runAction)}>
                        Prompt
                      </button>
                    </div>
                  )}
                </For>
              </LedgerSection>
            </div>
          )}
        </Show>
      </div>
    </section>
  );
}

function SourceSection(props: {
  snapshots: SourceSnapshot[];
  selected?: ReviewInboxItem;
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  const selectedWorkspace = () => props.selected?.workspaces[0];
  return (
    <section class="two-column">
      <div class="panel">
        <PanelHeading eyebrow="Source" title="Native snapshots" detail={`${props.snapshots.length} snapshot(s)`} />
        <DataTable
          headers={["Snapshot", "Message", "Created"]}
          rows={props.snapshots.map((snapshot) => [
            shortId(snapshot.id),
            snapshot.message ?? "none",
            formatDate(snapshot.created_at),
          ])}
        />
      </div>
      <div class="panel">
        <PanelHeading eyebrow="Diff" title="Selected workspace source view" detail="Native, not Git" />
        <Show when={selectedWorkspace()} fallback={<EmptyState title="No workspace selected" text="Select a thread in the Inbox to inspect its workspace diff." />}>
          {(workspace) => (
            <div class="operator-card">
              <p class="path-text">{workspace().materialized_path}</p>
              <button class="button button-secondary" onClick={() => showWorkspaceDiff(workspace(), props.runAction)}>
                Load changed paths and patch
              </button>
            </div>
          )}
        </Show>
      </div>
    </section>
  );
}

function ConflictsSection(props: {
  inbox: ReviewInboxItem[];
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  const candidates = () => props.inbox.filter((item) => Boolean(item.latest_review));
  return (
    <section class="two-column">
      <div class="panel">
        <PanelHeading eyebrow="Conflicts" title="Candidate reviews" detail={`${candidates().length} review candidate(s)`} />
        <DataTable
          headers={["Review", "Thread", "Paths", "Conflict"]}
          rows={candidates().map((item) => [
            shortId(item.latest_review?.id),
            item.thread.title,
            String(item.latest_review?.changed_paths.length ?? 0),
            item.thread.conflict_analysis_id ? shortId(item.thread.conflict_analysis_id) : "none",
          ])}
        />
      </div>
      <div class="panel">
        <PanelHeading eyebrow="Resolve" title="Deterministic conflict spine" detail="Analyze, prepare, verify" />
        <div class="action-grid">
          <button class="button button-secondary" onClick={() => analyzeConflict(props.runAction)}>
            Analyze reviews
          </button>
          <button class="button button-secondary" onClick={() => prepareConflict(props.runAction)}>
            Prepare resolver
          </button>
          <button class="button button-secondary" onClick={() => verifyConflict(props.runAction)}>
            Verify workspace
          </button>
        </div>
        <Notice
          tone="neutral"
          title="Deterministic first"
          text="Anvics analyzes competing reviews and prepares bounded resolver work. The browser does not spawn agents."
        />
      </div>
    </section>
  );
}

function RecoverySection(props: {
  inbox: ReviewInboxItem[];
  selected?: ReviewInboxItem;
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  const workspaces = () => props.inbox.flatMap((item) => item.workspaces.map((workspace) => ({ workspace, item })));
  return (
    <section class="two-column">
      <div class="panel">
        <PanelHeading eyebrow="Recovery" title="Workspace recovery" detail={`${workspaces().length} workspace(s)`} />
        <For each={workspaces()}>
          {({ workspace, item }) => (
            <div class="workspace-row">
              <div>
                <strong>{item.thread.title}</strong>
                <span>{shortId(workspace.id)} · {workspace.latest_snapshot ? `latest ${shortId(workspace.latest_snapshot)}` : "no snapshot"}</span>
              </div>
              <div class="row-actions">
                <button class="button button-secondary" onClick={() => recoverWorkspace(workspace, props.runAction)}>
                  Recover
                </button>
                <button class="button button-secondary" onClick={() => restoreWorkspace(workspace, props.runAction)}>
                  Restore
                </button>
              </div>
            </div>
          )}
        </For>
      </div>
      <div class="panel">
        <PanelHeading eyebrow="Audit-safe recovery" title="Evidence and revert tools" detail="Append-only" />
        <div class="action-grid">
          <button class="button button-secondary" onClick={() => supersedeEvidence(props.runAction)}>
            Supersede evidence
          </button>
          <button class="button button-secondary" onClick={() => preparePublicationRevert(props.runAction)}>
            Prepare publication revert
          </button>
        </div>
        <Notice
          tone="neutral"
          title="Nothing rewinds canonical history"
          text="Restores create checkpoints and records. Publication reverts prepare inverse work that still needs review and publication."
        />
      </div>
    </section>
  );
}

function PublicationsSection(props: {
  inbox: ReviewInboxItem[];
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  const publications = () =>
    props.inbox.flatMap((item) =>
      item.publication_ids.map((id) => ({
        id,
        title: item.thread.title,
        thread: item.thread.id,
        review: item.latest_review?.id,
        snapshot: item.latest_review?.final_snapshot,
      })),
    );
  return (
    <section class="two-column">
      <div class="panel">
        <PanelHeading eyebrow="Publications" title="Native publication timeline" detail={`${publications().length} publication(s)`} />
        <DataTable
          headers={["Publication", "Thread", "Review", "Snapshot"]}
          rows={publications().map((publication) => [
            shortId(publication.id),
            publication.title,
            shortId(publication.review),
            shortId(publication.snapshot),
          ])}
        />
      </div>
      <div class="panel">
        <PanelHeading eyebrow="Actions" title="Export and revert" detail="Legacy adapter only" />
        <div class="action-grid">
          <button class="button button-secondary" onClick={() => exportPublication(undefined, props.runAction)}>
            Export patch by id
          </button>
          <button class="button button-secondary" onClick={() => preparePublicationRevert(props.runAction)}>
            Prepare revert
          </button>
        </div>
      </div>
    </section>
  );
}

function EventsSection(props: { events: RepositoryEvent[] }) {
  const [kindFilter, setKindFilter] = createSignal("");
  const filtered = createMemo(() =>
    props.events.filter((event) =>
      kindFilter() ? event.kind.toLowerCase().includes(kindFilter().toLowerCase()) : true,
    ),
  );
  return (
    <section class="panel">
      <PanelHeading eyebrow="Audit" title="Append-only events" detail={`${filtered().length} shown`} />
      <input
        class="text-input"
        placeholder="Filter by event kind"
        value={kindFilter()}
        onInput={(event) => setKindFilter(event.currentTarget.value)}
      />
      <div class="event-stream">
        <For each={filtered()}>
          {(event) => (
            <article class="event-row">
              <span class="event-sequence">#{event.sequence}</span>
              <div>
                <strong>{event.kind}</strong>
                <p>{event.subject_id ? `subject ${shortId(event.subject_id)}` : "repository event"} · {formatDate(event.created_at)}</p>
              </div>
            </article>
          )}
        </For>
      </div>
    </section>
  );
}

function SettingsSection(props: { health?: { ok: boolean; version: number }; events: RepositoryEvent[] }) {
  return (
    <section class="two-column">
      <div class="panel">
        <PanelHeading eyebrow="Settings" title="Local app configuration" detail="Private beta" />
        <div class="context-list">
          <KeyValue label="Repo root" value={repoRoot || "not configured"} />
          <KeyValue label="Daemon HTTP" value={import.meta.env.VITE_ANVICS_HTTP_URL ?? "http://127.0.0.1:3897"} />
          <KeyValue label="API version" value={props.health?.version ? String(props.health.version) : "unknown"} />
          <KeyValue label="Latest event" value={String(latestSequence(props.events))} />
          <KeyValue label="Patch output default" value="<repo>/accepted.patch" />
        </div>
      </div>
      <div class="panel">
        <PanelHeading eyebrow="Hosted" title="Native hosted roadmap" detail="Scaffolded" />
        <Notice
          tone="neutral"
          title="Hosted mode comes after local operator confidence"
          text="The app already reserves /{username}/{project}. Hosted will store native Anvics snapshots, reviews, events, and blobs through native push sync, not GitHub integration."
        />
      </div>
    </section>
  );
}

function EvidenceTable(props: {
  evidence: EvidenceSummary[];
  runAction: (label: string, task: () => Promise<string>) => void;
}) {
  return (
    <Show when={props.evidence.length > 0} fallback={<EmptyState title="No evidence" text="Accept or finish a workspace with compact evidence." compact />}>
      <table class="data-table">
        <thead>
          <tr>
            <th>Command</th>
            <th>Summary</th>
            <th>Exit</th>
            <th>Action</th>
          </tr>
        </thead>
        <tbody>
          <For each={props.evidence}>
            {(entry) => (
              <tr>
                <td>{entry.command_label ?? entry.command}</td>
                <td>{entry.summary}</td>
                <td>{entry.exit_code}</td>
                <td>
                  <button class="link-button" onClick={() => supersedeEvidence(props.runAction, entry.id)}>
                    Supersede
                  </button>
                </td>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </Show>
  );
}

function ChangedPathTable(props: { paths: ChangedPath[] }) {
  return (
    <Show when={props.paths.length > 0} fallback={<EmptyState title="No changed paths" text="This review has no changed paths." compact />}>
      <table class="data-table">
        <thead>
          <tr>
            <th>Status</th>
            <th>Path</th>
          </tr>
        </thead>
        <tbody>
          <For each={props.paths}>
            {(path) => (
              <tr>
                <td>
                  <span class={`status-chip ${path.status}`}>{path.status}</span>
                </td>
                <td>{path.path}</td>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </Show>
  );
}

function RiskList(props: { findings: RiskFinding[] }) {
  return (
    <Show when={props.findings.length > 0} fallback={<EmptyState title="No risk findings" text="No findings on the latest review." compact />}>
      <div class="note-list">
        <For each={props.findings}>
          {(finding) => (
            <div class="risk-note">
              <strong>{finding.severity} · {finding.detector}</strong>
              <span>{finding.target_kind}: {finding.target_path}</span>
              <Show when={finding.evidence_id}>
                <span>evidence {shortId(finding.evidence_id)}</span>
              </Show>
              <p>{finding.redacted_excerpt}</p>
            </div>
          )}
        </For>
      </div>
    </Show>
  );
}

function NoteList(props: { notes: string[]; empty: string }) {
  return (
    <Show when={props.notes.length > 0} fallback={<EmptyState title="No notes" text={props.empty} compact />}>
      <ul class="note-list">
        <For each={props.notes}>{(note) => <li>{note}</li>}</For>
      </ul>
    </Show>
  );
}

function DataTable(props: { headers: string[]; rows: string[][] }) {
  return (
    <Show when={props.rows.length > 0} fallback={<EmptyState title="No records" text="No data is available for this section yet." compact />}>
      <table class="data-table">
        <thead>
          <tr>
            <For each={props.headers}>{(header) => <th>{header}</th>}</For>
          </tr>
        </thead>
        <tbody>
          <For each={props.rows}>
            {(row) => (
              <tr>
                <For each={row}>{(cell) => <td>{cell}</td>}</For>
              </tr>
            )}
          </For>
        </tbody>
      </table>
    </Show>
  );
}

function LedgerSection(props: { title: string; detail: string; children: JSX.Element }) {
  return (
    <section class="ledger-section">
      <header>
        <h3>{props.title}</h3>
        <span>{props.detail}</span>
      </header>
      {props.children}
    </section>
  );
}

function PanelHeading(props: { eyebrow: string; title: string; detail?: string }) {
  return (
    <header class="panel-heading">
      <div>
        <p class="eyebrow">{props.eyebrow}</p>
        <h2>{props.title}</h2>
      </div>
      <Show when={props.detail}>
        <span>{props.detail}</span>
      </Show>
    </header>
  );
}

function SummaryMetric(props: { label: string; value: number; tone: string }) {
  return (
    <div class={`summary-metric ${props.tone}`}>
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </div>
  );
}

function StatusBadge(props: { item: ReviewInboxItem }) {
  const risks = () => (props.item.latest_risk_findings ?? []).length;
  const label = () => {
    if (risks() > 0) return "Blocked by risk";
    if (props.item.thread.conflict_analysis_id) return "Conflict";
    if (isPublished(props.item)) return "Published";
    if (props.item.latest_review) return "Needs review";
    return "Active";
  };
  return <span class={`status-badge ${label().toLowerCase().replaceAll(" ", "-")}`}>{label()}</span>;
}

function KeyValue(props: { label: string; value: string }) {
  return (
    <div class="key-value">
      <dt>{props.label}</dt>
      <dd>{props.value}</dd>
    </div>
  );
}

function DaemonBadge(props: { loading: boolean; healthy: boolean }) {
  return (
    <span class="daemon-badge" classList={{ healthy: props.healthy, loading: props.loading }}>
      {props.loading ? "checking daemon" : props.healthy ? "daemon healthy" : "daemon unavailable"}
    </span>
  );
}

function ActionBanner(props: { state: ActionState }) {
  return (
    <div class="action-banner" classList={{ running: props.state.status === "running", success: props.state.status === "success", error: props.state.status === "error" }}>
      <strong>{props.state.status}</strong>
      <span>{props.state.message}</span>
    </div>
  );
}

function EmptyState(props: { title: string; text: string; compact?: boolean }) {
  return (
    <div class="empty-state" classList={{ compact: props.compact }}>
      <strong>{props.title}</strong>
      <p>{props.text}</p>
    </div>
  );
}

function Notice(props: { tone: "neutral" | "risk"; title: string; text: string }) {
  return (
    <div class={`notice ${props.tone}`}>
      <strong>{props.title}</strong>
      <p>{props.text}</p>
    </div>
  );
}

function matchesFilter(item: ReviewInboxItem, filter: QueueFilter): boolean {
  switch (filter) {
    case "needs_review":
      return Boolean(item.latest_review) && !isPublished(item);
    case "risk":
      return (item.latest_risk_findings ?? []).length > 0;
    case "conflict":
      return Boolean(item.thread.conflict_analysis_id || item.latest_review?.source_review_ids?.length);
    case "unpublished":
      return !isPublished(item);
    case "published":
      return isPublished(item);
    case "no_evidence":
      return item.evidence_count === 0;
    case "all":
      return true;
  }
}

function compareItems(left: ReviewInboxItem, right: ReviewInboxItem, sort: QueueSort): number {
  if (sort === "risk") return riskScore(right) - riskScore(left) || newest(left, right);
  if (sort === "conflict") return conflictScore(right) - conflictScore(left) || newest(left, right);
  if (sort === "unreviewed") return reviewScore(right) - reviewScore(left) || newest(left, right);
  return newest(left, right);
}

function newest(left: ReviewInboxItem, right: ReviewInboxItem): number {
  return Date.parse(right.thread.created_at) - Date.parse(left.thread.created_at);
}

function riskScore(item: ReviewInboxItem): number {
  return (item.latest_risk_findings ?? []).length;
}

function conflictScore(item: ReviewInboxItem): number {
  return item.thread.conflict_analysis_id ? 1 : 0;
}

function reviewScore(item: ReviewInboxItem): number {
  return item.latest_review && !isPublished(item) ? 1 : 0;
}

function latestSequence(events: RepositoryEvent[]): number {
  return events.reduce((max, event) => Math.max(max, event.sequence), 0);
}

function decisionText(item: ReviewInboxItem): string {
  if ((item.latest_risk_findings ?? []).length > 0) return "Blocked by risk";
  if (isPublished(item)) return "Published";
  if (item.latest_review) return "Ready for operator review";
  return "Agent work active";
}

function recommendedPath(item: ReviewInboxItem): string {
  if ((item.latest_risk_findings ?? []).length > 0) {
    return "Inspect risk findings. Supersede obsolete evidence or publish only with an explicit override reason.";
  }
  if (isPublished(item)) {
    return "Inspect publication or export a legacy patch for downstream systems.";
  }
  if (item.latest_review) {
    return "Review changed paths and evidence, then publish or restore/checkpoint before accepting.";
  }
  return "Use launch prompt or context pack, then wait for agent evidence.";
}

function promptRequired(label: string, fallback = ""): string | null {
  const value = window.prompt(label, fallback)?.trim();
  return value ? value : null;
}

function splitCommand(value: string): string[] {
  return value.match(/(?:[^\s"]+|"[^"]*")+/g)?.map((part) => part.replace(/^"|"$/g, "")) ?? [];
}

function publishReview(item: ReviewInboxItem, runAction: (label: string, task: () => Promise<string>) => void) {
  const review = item.latest_review;
  if (!review) return;
  const allowSecretRisk = (item.latest_risk_findings ?? []).length > 0 && window.confirm("This review has risk findings. Publish with a secret-risk override?");
  const overrideReason = allowSecretRisk ? promptRequired("Override reason") : null;
  if (allowSecretRisk && !overrideReason) return;
  runAction("Publish review", async () => {
    const result = await rpc<{ type: "publish_create"; publication: NativePublication }>({
      method: "publish_create",
      thread: item.thread.id,
      review: review.id,
      allow_secret_risk: Boolean(allowSecretRisk),
      override_reason: overrideReason,
      allow_resolution_risk: false,
      resolution_risk_reason: null,
    });
    return `Created publication ${shortId(result.publication.id)}.`;
  });
}

function exportPublication(item: ReviewInboxItem | undefined, runAction: (label: string, task: () => Promise<string>) => void) {
  const publication = item?.publication_ids[0] ?? promptRequired("Publication id to export");
  if (!publication) return;
  const output = promptRequired("Patch output path", `${repoRoot}/accepted.patch`);
  if (!output) return;
  runAction("Export legacy patch", async () => {
    const result = await rpc<{ type: "legacy_git_export"; output: string }>({
      method: "legacy_git_export",
      publication,
      output,
    });
    return `Exported legacy patch to ${result.output}.`;
  });
}

function riskScan(item: ReviewInboxItem, runAction: (label: string, task: () => Promise<string>) => void) {
  const review = item.latest_review?.id;
  if (!review) return;
  runAction("Risk scan", async () => {
    const result = await rpc<{ type: "risk_scan"; scan: { id: string; findings: RiskFinding[] } }>({
      method: "risk_scan",
      review,
    });
    return `Created risk scan ${shortId(result.scan.id)} with ${result.scan.findings.length} finding(s).`;
  });
}

function supersedeEvidence(runAction: (label: string, task: () => Promise<string>) => void, evidenceId?: string) {
  const id = evidenceId ?? promptRequired("Evidence id to supersede");
  if (!id) return;
  const reason = promptRequired("Supersede reason");
  if (!reason) return;
  runAction("Supersede evidence", async () => {
    const result = await rpc<{ type: "evidence_superseded"; evidence: EvidenceSummary }>({
      method: "evidence_supersede",
      id,
      reason,
    });
    return `Superseded evidence ${shortId(result.evidence.id)}.`;
  });
}

function showWorkspaceDiff(workspace: WorkspaceView, runAction: (label: string, task: () => Promise<string>) => void) {
  runAction("Workspace diff", async () => {
    const summary = await diffWorkspace(workspace.id, "summary");
    const patch = await diffWorkspace(workspace.id, "patch");
    return `${summary.changed_paths.length} changed path(s).\n${patch.patch?.slice(0, 1400) ?? "No patch output."}`;
  });
}

function launchPrompt(workspace: WorkspaceView, runAction: (label: string, task: () => Promise<string>) => void) {
  runAction("Launch prompt", async () => {
    const result = await rpc<{ type: "agent_launch_prompt"; prompt: AgentLaunchPrompt }>({
      method: "agent_launch_prompt",
      workspace: workspace.id,
      tool: "generic",
    });
    return result.prompt.command ?? result.prompt.prompt;
  });
}

function checkpointWorkspace(workspace: WorkspaceView, runAction: (label: string, task: () => Promise<string>) => void) {
  const summary = promptRequired("Checkpoint summary", "Operator checkpoint from UI.");
  if (!summary) return;
  runAction("Create checkpoint", async () => {
    const result = await rpc<{ type: "agent_checkpoint"; checkpoint: { id: string } }>({
      method: "agent_checkpoint",
      workspace: workspace.id,
      summary,
    });
    return `Created checkpoint ${shortId(result.checkpoint.id)}.`;
  });
}

function recoverWorkspace(workspace: WorkspaceView, runAction: (label: string, task: () => Promise<string>) => void) {
  runAction("Recover workspace", async () => {
    const result = await rpc<{ type: "agent_recover"; recovery: { current_changed_paths: ChangedPath[]; notes: string[] } }>({
      method: "agent_recover",
      workspace: workspace.id,
    });
    return `Recovery found ${result.recovery.current_changed_paths.length} changed path(s). ${result.recovery.notes.join(" ")}`;
  });
}

function restoreWorkspace(workspace: WorkspaceView, runAction: (label: string, task: () => Promise<string>) => void) {
  const source = promptRequired("Restore source: base, latest, snapshot:<id>, checkpoint:<id>, publication:<id>", "base");
  if (!source) return;
  const reason = promptRequired("Restore reason");
  if (!reason) return;
  const dryRun = !window.confirm("Apply restore now? Cancel runs a dry-run.");
  runAction(dryRun ? "Restore dry-run" : "Restore workspace", async () => {
    const result = await rpc<{ type: "workspace_restore"; restore: { id: string; changed_paths: ChangedPath[]; dry_run: boolean } }>({
      method: "workspace_restore",
      id: workspace.id,
      source,
      paths: [],
      reason,
      dry_run: dryRun,
    });
    return `${result.restore.dry_run ? "Planned" : "Created"} restore ${shortId(result.restore.id)} with ${result.restore.changed_paths.length} changed path(s).`;
  });
}

function acceptWorkspaceRun(workspace: WorkspaceView, runAction: (label: string, task: () => Promise<string>) => void) {
  const label = promptRequired("Verification label", "verify");
  if (!label) return;
  const summary = promptRequired("Verification summary", "Verified workspace changes.");
  if (!summary) return;
  const command = promptRequired("Verification command", "cargo test --workspace");
  if (!command) return;
  const output = promptRequired("Patch output path", `${repoRoot}/accepted.patch`);
  if (!output) return;
  runAction("Accept workspace", async () => {
    const result = await rpc<{ type: "agent_accept"; acceptance: { publication: NativePublication; patch_path: string; review: ReviewProjection } }>({
      method: "agent_accept_run",
      workspace: workspace.id,
      argv: splitCommand(command),
      command_file: null,
      command_label: label,
      cwd: null,
      timeout_seconds: 120,
      summary,
      artifact_path: null,
      projection: "materialized_dir",
      mount_root: null,
      output_path: output,
      allow_secret_risk: false,
      override_reason: null,
      allow_resolution_risk: false,
      resolution_risk_reason: null,
      allow_command_risk: false,
      command_risk_reason: null,
    });
    return `Accepted review ${shortId(result.acceptance.review.id)}, publication ${shortId(result.acceptance.publication.id)}, patch ${result.acceptance.patch_path}.`;
  });
}

function analyzeConflict(runAction: (label: string, task: () => Promise<string>) => void) {
  const ids = promptRequired("Review ids, comma-separated");
  const reviews = ids?.split(",").map((id) => id.trim()).filter(Boolean) ?? [];
  if (reviews.length < 2) return;
  runAction("Analyze conflict", async () => {
    const result = await rpc<{ type: "conflict_analyze"; analysis: ConflictAnalysis; markdown_path: string }>({
      method: "conflict_analyze",
      reviews,
    });
    return `Created conflict analysis ${shortId(result.analysis.id)} at ${result.markdown_path}.`;
  });
}

function prepareConflict(runAction: (label: string, task: () => Promise<string>) => void) {
  const ids = promptRequired("Review ids, comma-separated");
  const reviews = ids?.split(",").map((id) => id.trim()).filter(Boolean) ?? [];
  if (reviews.length < 2) return;
  const title = promptRequired("Resolver title", "Resolve competing edits");
  if (!title) return;
  const task = promptRequired("Resolver task", "Preserve the best behavior from all candidates.");
  if (!task) return;
  runAction("Prepare conflict resolver", async () => {
    const result = await rpc<{ type: "conflict_prepare"; preparation: { preparation: { workspace: WorkspaceView; packet_path: string } } }>({
      method: "conflict_prepare",
      reviews,
      title,
      task,
      agent_command: null,
    });
    return `Prepared resolver workspace ${shortId(result.preparation.preparation.workspace.id)}. Packet: ${result.preparation.preparation.packet_path}.`;
  });
}

function verifyConflict(runAction: (label: string, task: () => Promise<string>) => void) {
  const workspace = promptRequired("Resolver workspace id");
  if (!workspace) return;
  runAction("Verify conflict resolution", async () => {
    const result = await rpc<{ type: "conflict_verify"; verification: { passed: boolean; findings: string[] } }>({
      method: "conflict_verify",
      workspace,
    });
    return `Conflict verification ${result.verification.passed ? "passed" : "failed"}: ${result.verification.findings.join("; ") || "no findings"}.`;
  });
}

function preparePublicationRevert(runAction: (label: string, task: () => Promise<string>) => void) {
  const publication = promptRequired("Publication id to revert");
  if (!publication) return;
  const reason = promptRequired("Revert reason");
  if (!reason) return;
  runAction("Prepare publication revert", async () => {
    const result = await rpc<{ type: "publish_revert_prepare"; revert: { plan: { id: string; workspace_id: string } } }>({
      method: "publish_revert_prepare",
      publication,
      base_snapshot: null,
      reason,
    });
    return `Prepared revert plan ${shortId(result.revert.plan.id)} in workspace ${shortId(result.revert.plan.workspace_id)}.`;
  });
}
