export type ChangeStatus = "added" | "modified" | "deleted";

export type ChangedPath = {
  path: string;
  status: ChangeStatus;
};

export type SourceSnapshot = {
  id: string;
  root_tree: string;
  created_at: string;
  message?: string | null;
};

export type WorkThread = {
  id: string;
  title: string;
  task: string;
  base_snapshot: string;
  source_review_ids?: string[];
  conflict_analysis_id?: string | null;
  publication_revert_plan_id?: string | null;
  status: string;
  created_at: string;
};

export type WorkspaceView = {
  id: string;
  thread_id: string;
  base_snapshot: string;
  latest_snapshot?: string | null;
  materialized_path: string;
  created_at?: string;
};

export type EvidenceSummary = {
  id: string;
  command: string;
  command_label?: string | null;
  command_file?: string | null;
  cwd?: string | null;
  summary: string;
  exit_code: number;
  artifact_path?: string | null;
  stdout_path?: string | null;
  stderr_path?: string | null;
  findings?: RiskFinding[];
  created_at?: string;
};

export type EvidenceRecord = EvidenceSummary & {
  thread_id?: string;
  command_event_id?: string | null;
  superseded_at?: string | null;
  superseded_reason?: string | null;
};

export type RiskFinding = {
  id: string;
  scan_id?: string;
  review_id: string;
  evidence_id?: string | null;
  severity: string;
  detector: string;
  target_kind: string;
  target_path: string;
  line?: number | null;
  redacted_excerpt?: string | null;
};

export type ReviewProjection = {
  id: string;
  thread_id: string;
  base_snapshot: string;
  final_snapshot: string;
  source_review_ids?: string[];
  conflict_analysis_id?: string | null;
  publication_revert_plan_id?: string | null;
  changed_paths: ChangedPath[];
  evidence: EvidenceSummary[];
  overlap_notes: string[];
  created_at: string;
};

export type NativePublication = {
  id: string;
  thread_id: string;
  accepted_snapshot: string;
  review_id: string;
  created_at: string;
};

export type RepositoryEvent = {
  id: string;
  sequence: number;
  kind: string;
  subject_id?: string | null;
  created_at: string;
};

export type AgentCheckpoint = {
  id: string;
  thread_id: string;
  workspace_id: string;
  snapshot_id: string;
  summary: string;
  changed_paths: ChangedPath[];
  created_at: string;
};

export type AgentStatus = {
  thread: WorkThread;
  workspaces: WorkspaceView[];
  evidence_count: number;
  review_ids: string[];
  publication_ids: string[];
};

export type AgentRecovery = {
  thread: WorkThread;
  workspace: WorkspaceView;
  current_changed_paths: ChangedPath[];
  latest_checkpoint?: AgentCheckpoint | null;
  active_sessions: unknown[];
  notes: string[];
};

export type AgentLaunchPrompt = {
  tool: "generic" | "codex";
  thread_id: string;
  workspace_id: string;
  repo_path: string;
  workspace_path: string;
  packet_path: string;
  skill_path?: string | null;
  prompt: string;
  command?: string | null;
};

export type ResolutionVerification = {
  workspace_id: string;
  thread_id: string;
  conflict_analysis_id?: string | null;
  passed: boolean;
  findings: string[];
  current_changed_paths: ChangedPath[];
  created_at: string;
};

export type ConflictAnalysis = {
  id: string;
  base_snapshot: string;
  input_reviews: Array<{
    review_id: string;
    thread_id: string;
    title: string;
    final_snapshot: string;
    changed_paths: ChangedPath[];
    evidence_summaries: string[];
  }>;
  path_cases: Array<{
    path: string;
    kind: string;
    safety: string;
    review_ids: string[];
    summary: string;
  }>;
  created_at: string;
};

export type ReviewInboxItem = {
  thread: WorkThread;
  workspaces: WorkspaceView[];
  evidence_count: number;
  review_ids: string[];
  publication_ids: string[];
  latest_review?: ReviewProjection | null;
  latest_risk_findings?: RiskFinding[];
};

export type ReviewInboxSummary = {
  total: number;
  needsReview: number;
  risk: number;
  published: number;
  conflicts: number;
  workspaces: number;
};

export type WorkspaceDiffResult = {
  changed_paths: ChangedPath[];
  file_effects?: unknown[];
  patch?: string | null;
};

export type WorkspaceShowResult = {
  workspace: WorkspaceView;
  changed_paths?: ChangedPath[] | null;
};

export type ApiResult = { type: string; [key: string]: unknown };

type ApiResponse = {
  id: number;
  version: number;
  result: ApiResult;
};

const httpBase =
  import.meta.env.VITE_ANVICS_HTTP_URL?.replace(/\/$/, "") ??
  "http://127.0.0.1:3897";

export const repoRoot = import.meta.env.VITE_ANVICS_REPO ?? "";

let nextRequestId = 1;

export async function rpc<T extends ApiResult>(
  method: Record<string, unknown>,
  repo = repoRoot,
): Promise<T> {
  if (!repo) {
    throw new Error("Set VITE_ANVICS_REPO to an initialized Anvics repository path.");
  }
  const response = await fetch(`${httpBase}/api/rpc`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      id: nextRequestId++,
      repo,
      method,
    }),
  });
  if (!response.ok) {
    throw new Error(`Anvics HTTP bridge returned ${response.status}`);
  }
  const payload = (await response.json()) as ApiResponse;
  if (payload.result.type === "error") {
    throw new Error(String(payload.result.message ?? "Anvics API error"));
  }
  return payload.result as T;
}

type ReviewInboxResponse = {
  id: number;
  version: number;
  result:
    | ({ type: "error"; message: string })
    | { type: "review_inbox"; items: ReviewInboxItem[] };
};

export async function fetchReviewInbox(repo = repoRoot): Promise<ReviewInboxItem[]> {
  if (!repo) {
    throw new Error("Set VITE_ANVICS_REPO to an initialized Anvics repository path.");
  }
  const response = await fetch(
    `${httpBase}/api/review-inbox?repo=${encodeURIComponent(repo)}`,
  );
  if (!response.ok) {
    throw new Error(`Anvics HTTP bridge returned ${response.status}`);
  }
  const payload = (await response.json()) as ReviewInboxResponse;
  if (payload.result.type === "error") {
    throw new Error(payload.result.message);
  }
  return payload.result.items;
}

export async function fetchHealth() {
  const response = await fetch(`${httpBase}/api/health`);
  if (!response.ok) {
    throw new Error(`Anvics daemon health returned ${response.status}`);
  }
  return (await response.json()) as { ok: boolean; version: number };
}

export async function fetchThreads() {
  return rpc<{ type: "thread_list"; threads: WorkThread[] }>({ method: "thread_list" });
}

export async function fetchSnapshots() {
  return rpc<{ type: "snapshot_list"; snapshots: SourceSnapshot[] }>({
    method: "snapshot_list",
  });
}

export async function fetchEventsSince(sequence = 0) {
  return rpc<{ type: "events_since"; events: RepositoryEvent[] }>({
    method: "events_since",
    sequence,
  });
}

export async function showWorkspace(id: string) {
  return rpc<{ type: "workspace_show" } & WorkspaceShowResult>({
    method: "workspace_show",
    id,
  });
}

export async function diffWorkspace(id: string, format: "summary" | "patch" = "summary") {
  return rpc<{ type: "workspace_diff" } & WorkspaceDiffResult>({
    method: "workspace_diff",
    id,
    format,
    classify: true,
  });
}

export async function showReview(id: string, format: "json" | "markdown" = "json") {
  return rpc<ApiResult>({
    method: "review_show",
    id,
    format,
  });
}

export function changeLabel(change: ChangedPath): string {
  const label =
    change.status === "added"
      ? "Added"
      : change.status === "deleted"
        ? "Deleted"
        : "Modified";
  return `${label}: ${change.path}`;
}

export function summarizeInbox(items: ReviewInboxItem[]): ReviewInboxSummary {
  return {
    total: items.length,
    needsReview: items.filter((item) => !isPublished(item) && Boolean(item.latest_review)).length,
    risk: items.filter((item) => (item.latest_risk_findings ?? []).length > 0).length,
    published: items.filter(isPublished).length,
    conflicts: items.filter((item) => Boolean(item.thread.conflict_analysis_id)).length,
    workspaces: items.reduce((sum, item) => sum + item.workspaces.length, 0),
  };
}

export function isPublished(item: ReviewInboxItem): boolean {
  return item.publication_ids.length > 0 || item.thread.status === "published";
}

export function shortId(id?: string | null): string {
  if (!id) return "none";
  return id.length <= 8 ? id : id.slice(0, 8);
}

export function formatDate(value?: string): string {
  if (!value) return "unknown";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(parsed);
}
