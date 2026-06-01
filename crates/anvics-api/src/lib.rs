use anvics_core::{
    AgentAcceptance, AgentCheckpoint, AgentContextPack, AgentFinish, AgentInstructionFile,
    AgentInstructionTarget, AgentLaunchPrompt, AgentLaunchTool, AgentPreparation, AgentRecovery,
    AgentSession, AgentStatus, ChangedPath, CommandEvent, CommandPolicyDecision, ConflictAnalysis,
    ConflictPreparation, CoordinationStatus, EvidenceRecord, FileEffect, NativePublication,
    ProjectionRequest, RepoDoctorReport, RepositoryEvent, RepositoryManifest,
    ResolutionVerification, ReviewProjection, RiskFinding, RiskScan, SourceSnapshot, WorkThread,
    WorkspaceView,
};
use serde::{Deserialize, Serialize};

pub const API_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ApiRequest {
    pub id: u64,
    pub repo: String,
    pub method: ApiMethod,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum ApiMethod {
    Ping,
    RepoInit,
    RepoStatus,
    RepoDoctor {
        paths: Vec<String>,
    },
    SnapshotCreate {
        message: Option<String>,
    },
    SnapshotList,
    SnapshotShow {
        id: String,
    },
    ThreadCreate {
        title: String,
        task: String,
    },
    ThreadList,
    ThreadShow {
        id: String,
    },
    WorkspaceCreate {
        thread: String,
    },
    WorkspaceShow {
        id: String,
    },
    WorkspaceDiff {
        id: String,
        format: WorkspaceDiffFormat,
        #[serde(default)]
        classify: bool,
    },
    WorkspaceSnapshot {
        id: String,
        message: Option<String>,
    },
    EvidenceAttach {
        thread: String,
        command: String,
        exit_code: i32,
        summary: String,
        artifact_path: Option<String>,
    },
    EvidenceCommand {
        thread: String,
        command: Option<String>,
        command_file: Option<String>,
        command_label: Option<String>,
        cwd: Option<String>,
        exit_code: i32,
        summary: String,
        artifact_path: Option<String>,
    },
    EvidenceList {
        thread: String,
        include_superseded: bool,
    },
    EvidenceShow {
        id: String,
    },
    EvidenceSupersede {
        id: String,
        reason: String,
    },
    CommandRun {
        workspace: String,
        argv: Vec<String>,
        command_file: Option<String>,
        command_label: String,
        cwd: Option<String>,
        timeout_seconds: Option<u64>,
        summary: String,
        artifact_path: Option<String>,
        #[serde(default)]
        projection: ProjectionRequest,
        mount_root: Option<String>,
        #[serde(default)]
        allow_command_risk: bool,
        command_risk_reason: Option<String>,
    },
    CommandClassify {
        argv: Vec<String>,
        command_file: Option<String>,
    },
    CommandShow {
        id: String,
    },
    ReviewCreate {
        thread: String,
    },
    AgentPrepare {
        title: String,
        task: String,
        agent_command: Option<String>,
    },
    AgentResolve {
        reviews: Vec<String>,
        title: Option<String>,
        task: Option<String>,
        agent_command: Option<String>,
    },
    ConflictAnalyze {
        reviews: Vec<String>,
    },
    ConflictPrepare {
        reviews: Vec<String>,
        title: Option<String>,
        task: Option<String>,
        agent_command: Option<String>,
    },
    ConflictStatus {
        workspace: String,
    },
    ConflictVerify {
        workspace: String,
    },
    AgentEnter {
        workspace: String,
        name: String,
    },
    AgentLeave {
        session: String,
    },
    AgentSessions {
        thread: Option<String>,
        workspace: Option<String>,
    },
    AgentStatus {
        thread: String,
    },
    AgentAccept {
        workspace: String,
        command: Option<String>,
        command_file: Option<String>,
        command_label: Option<String>,
        cwd: Option<String>,
        exit_code: i32,
        summary: String,
        artifact_path: Option<String>,
        output_path: Option<String>,
        allow_secret_risk: bool,
        override_reason: Option<String>,
        #[serde(default)]
        allow_resolution_risk: bool,
        resolution_risk_reason: Option<String>,
    },
    AgentAcceptRun {
        workspace: String,
        argv: Vec<String>,
        command_file: Option<String>,
        command_label: String,
        cwd: Option<String>,
        timeout_seconds: Option<u64>,
        summary: String,
        artifact_path: Option<String>,
        #[serde(default)]
        projection: ProjectionRequest,
        mount_root: Option<String>,
        output_path: Option<String>,
        allow_secret_risk: bool,
        override_reason: Option<String>,
        #[serde(default)]
        allow_resolution_risk: bool,
        resolution_risk_reason: Option<String>,
        #[serde(default)]
        allow_command_risk: bool,
        command_risk_reason: Option<String>,
    },
    AgentPacket {
        thread: String,
    },
    AgentLaunchPrompt {
        workspace: String,
        tool: AgentLaunchTool,
    },
    AgentInstructions {
        target: AgentInstructionTarget,
        install: bool,
        force: bool,
    },
    AgentContextPack {
        workspace: String,
        write: bool,
    },
    AgentCheckpoint {
        workspace: String,
        summary: String,
    },
    AgentRecover {
        workspace: String,
    },
    AgentFinish {
        workspace: String,
        command: Option<String>,
        command_file: Option<String>,
        command_label: Option<String>,
        cwd: Option<String>,
        exit_code: i32,
        summary: String,
        artifact_path: Option<String>,
    },
    ReviewShow {
        id: String,
        format: ReviewFormat,
    },
    ReviewPath {
        id: String,
    },
    PublishCreate {
        thread: String,
        review: String,
        allow_secret_risk: bool,
        override_reason: Option<String>,
        #[serde(default)]
        allow_resolution_risk: bool,
        resolution_risk_reason: Option<String>,
    },
    RiskScan {
        review: String,
    },
    RiskList {
        review: String,
    },
    RiskShow {
        id: String,
    },
    LegacyGitExport {
        publication: String,
        output: String,
    },
    EventsSince {
        sequence: u64,
    },
    CoordinationStatus {
        workspace: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFormat {
    Json,
    Markdown,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDiffFormat {
    Summary,
    Patch,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ApiResponse {
    pub id: u64,
    pub version: u32,
    pub result: ApiResult,
}

impl ApiResponse {
    pub fn ok(id: u64, result: ApiResult) -> Self {
        Self {
            id,
            version: API_VERSION,
            result,
        }
    }

    pub fn error(id: u64, message: impl Into<String>) -> Self {
        Self::ok(
            id,
            ApiResult::Error {
                message: message.into(),
            },
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiResult {
    Pong,
    RepoInit {
        manifest: RepositoryManifest,
    },
    RepoStatus {
        initialized: bool,
        manifest: Option<RepositoryManifest>,
    },
    RepoDoctor {
        report: RepoDoctorReport,
    },
    SnapshotCreate {
        snapshot: SourceSnapshot,
    },
    SnapshotList {
        snapshots: Vec<SourceSnapshot>,
    },
    SnapshotShow {
        snapshot: SourceSnapshot,
    },
    ThreadCreate {
        thread: Box<WorkThread>,
    },
    ThreadList {
        threads: Vec<WorkThread>,
    },
    ThreadShow {
        thread: Box<WorkThread>,
    },
    WorkspaceCreate {
        workspace: Box<WorkspaceView>,
    },
    WorkspaceShow {
        workspace: Box<WorkspaceView>,
        changed_paths: Option<Vec<ChangedPath>>,
    },
    WorkspaceDiff {
        changed_paths: Vec<ChangedPath>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        file_effects: Vec<FileEffect>,
        patch: Option<String>,
    },
    WorkspaceSnapshot {
        workspace: Box<WorkspaceView>,
    },
    EvidenceAttached {
        evidence: EvidenceRecord,
    },
    EvidenceList {
        evidence: Vec<EvidenceRecord>,
    },
    EvidenceShow {
        evidence: EvidenceRecord,
    },
    EvidenceSuperseded {
        evidence: EvidenceRecord,
    },
    CommandRun {
        command_event: Box<CommandEvent>,
        evidence: EvidenceRecord,
    },
    CommandClassify {
        decision: CommandPolicyDecision,
    },
    CommandShow {
        command_event: Box<CommandEvent>,
    },
    ReviewCreate {
        review: Box<ReviewProjection>,
    },
    AgentPrepare {
        preparation: Box<AgentPreparation>,
    },
    ConflictAnalyze {
        analysis: Box<ConflictAnalysis>,
        markdown_path: String,
        markdown: String,
    },
    ConflictPrepare {
        preparation: Box<ConflictPreparation>,
    },
    ConflictStatus {
        verification: Box<ResolutionVerification>,
    },
    ConflictVerify {
        verification: Box<ResolutionVerification>,
    },
    AgentEnter {
        status: Box<CoordinationStatus>,
    },
    AgentLeave {
        session: AgentSession,
    },
    AgentSessions {
        sessions: Vec<AgentSession>,
    },
    AgentStatus {
        status: Box<AgentStatus>,
    },
    AgentAccept {
        acceptance: Box<AgentAcceptance>,
    },
    AgentPacket {
        path: String,
    },
    AgentLaunchPrompt {
        prompt: Box<AgentLaunchPrompt>,
    },
    AgentInstructions {
        files: Vec<AgentInstructionFile>,
    },
    AgentContextPack {
        pack: Box<AgentContextPack>,
    },
    AgentCheckpoint {
        checkpoint: Box<AgentCheckpoint>,
    },
    AgentRecover {
        recovery: Box<AgentRecovery>,
    },
    AgentFinish {
        finish: Box<AgentFinish>,
    },
    ReviewShowJson {
        review: Box<ReviewProjection>,
    },
    ReviewShowMarkdown {
        markdown: String,
    },
    ReviewPath {
        path: String,
    },
    PublishCreate {
        publication: NativePublication,
    },
    RiskScan {
        scan: Box<RiskScan>,
    },
    RiskList {
        findings: Vec<RiskFinding>,
    },
    RiskShow {
        finding: RiskFinding,
    },
    LegacyGitExport {
        output: String,
    },
    EventsSince {
        events: Vec<RepositoryEvent>,
    },
    CoordinationStatus {
        status: Box<CoordinationStatus>,
    },
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use anvics_core::*;

    #[test]
    fn api_request_round_trips_as_json() {
        let requests = vec![
            ApiMethod::Ping,
            ApiMethod::RepoInit,
            ApiMethod::RepoStatus,
            ApiMethod::RepoDoctor {
                paths: vec!["src/lib.rs".to_owned()],
            },
            ApiMethod::SnapshotCreate {
                message: Some("base".to_owned()),
            },
            ApiMethod::SnapshotList,
            ApiMethod::SnapshotShow {
                id: "snapshot-1".to_owned(),
            },
            ApiMethod::ThreadCreate {
                title: "title".to_owned(),
                task: "task".to_owned(),
            },
            ApiMethod::ThreadList,
            ApiMethod::ThreadShow {
                id: "thread-1".to_owned(),
            },
            ApiMethod::WorkspaceCreate {
                thread: "thread-1".to_owned(),
            },
            ApiMethod::WorkspaceShow {
                id: "workspace-1".to_owned(),
            },
            ApiMethod::WorkspaceDiff {
                id: "workspace-1".to_owned(),
                format: WorkspaceDiffFormat::Patch,
                classify: true,
            },
            ApiMethod::WorkspaceSnapshot {
                id: "workspace-1".to_owned(),
                message: None,
            },
            ApiMethod::EvidenceAttach {
                thread: "thread-1".to_owned(),
                command: "true".to_owned(),
                exit_code: 0,
                summary: "ok".to_owned(),
                artifact_path: None,
            },
            ApiMethod::EvidenceCommand {
                thread: "thread-1".to_owned(),
                command: None,
                command_file: Some("verify.sh".to_owned()),
                command_label: Some("verify".to_owned()),
                cwd: Some(".".to_owned()),
                exit_code: 0,
                summary: "ok".to_owned(),
                artifact_path: None,
            },
            ApiMethod::EvidenceList {
                thread: "thread-1".to_owned(),
                include_superseded: false,
            },
            ApiMethod::EvidenceShow {
                id: "evidence-1".to_owned(),
            },
            ApiMethod::EvidenceSupersede {
                id: "evidence-1".to_owned(),
                reason: "obsolete verification".to_owned(),
            },
            ApiMethod::CommandRun {
                workspace: "workspace-1".to_owned(),
                argv: vec!["true".to_owned()],
                command_file: None,
                command_label: "verify".to_owned(),
                cwd: None,
                timeout_seconds: Some(120),
                summary: "ok".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            },
            ApiMethod::CommandClassify {
                argv: vec!["curl".to_owned(), "https://example.com".to_owned()],
                command_file: None,
            },
            ApiMethod::CommandShow {
                id: "command-1".to_owned(),
            },
            ApiMethod::ReviewCreate {
                thread: "thread-1".to_owned(),
            },
            ApiMethod::ReviewShow {
                id: "review-1".to_owned(),
                format: ReviewFormat::Markdown,
            },
            ApiMethod::ReviewPath {
                id: "review-1".to_owned(),
            },
            ApiMethod::PublishCreate {
                thread: "thread-1".to_owned(),
                review: "review-1".to_owned(),
                allow_secret_risk: false,
                override_reason: None,
                allow_resolution_risk: false,
                resolution_risk_reason: None,
            },
            ApiMethod::RiskScan {
                review: "review-1".to_owned(),
            },
            ApiMethod::RiskList {
                review: "review-1".to_owned(),
            },
            ApiMethod::RiskShow {
                id: "finding-1".to_owned(),
            },
            ApiMethod::AgentPrepare {
                title: "title".to_owned(),
                task: "task".to_owned(),
                agent_command: None,
            },
            ApiMethod::AgentResolve {
                reviews: vec!["review-1".to_owned(), "review-2".to_owned()],
                title: Some("Resolve".to_owned()),
                task: Some("Combine candidates".to_owned()),
                agent_command: Some("cargo run -q -p anvics-cli --bin anvics --".to_owned()),
            },
            ApiMethod::ConflictAnalyze {
                reviews: vec!["review-1".to_owned(), "review-2".to_owned()],
            },
            ApiMethod::ConflictPrepare {
                reviews: vec!["review-1".to_owned(), "review-2".to_owned()],
                title: Some("Resolve".to_owned()),
                task: Some("Combine candidates".to_owned()),
                agent_command: None,
            },
            ApiMethod::ConflictStatus {
                workspace: "workspace-1".to_owned(),
            },
            ApiMethod::ConflictVerify {
                workspace: "workspace-1".to_owned(),
            },
            ApiMethod::AgentEnter {
                workspace: "workspace-1".to_owned(),
                name: "codex-cli".to_owned(),
            },
            ApiMethod::AgentLeave {
                session: "session-1".to_owned(),
            },
            ApiMethod::AgentSessions {
                thread: Some("thread-1".to_owned()),
                workspace: None,
            },
            ApiMethod::AgentStatus {
                thread: "thread-1".to_owned(),
            },
            ApiMethod::AgentPacket {
                thread: "thread-1".to_owned(),
            },
            ApiMethod::AgentLaunchPrompt {
                workspace: "workspace-1".to_owned(),
                tool: AgentLaunchTool::Codex,
            },
            ApiMethod::AgentInstructions {
                target: AgentInstructionTarget::All,
                install: false,
                force: false,
            },
            ApiMethod::AgentContextPack {
                workspace: "workspace-1".to_owned(),
                write: false,
            },
            ApiMethod::AgentCheckpoint {
                workspace: "workspace-1".to_owned(),
                summary: "salvage docs edits".to_owned(),
            },
            ApiMethod::AgentRecover {
                workspace: "workspace-1".to_owned(),
            },
            ApiMethod::AgentFinish {
                workspace: "workspace-1".to_owned(),
                command: Some("true".to_owned()),
                command_file: None,
                command_label: None,
                cwd: None,
                exit_code: 0,
                summary: "ok".to_owned(),
                artifact_path: None,
            },
            ApiMethod::AgentAccept {
                workspace: "workspace-1".to_owned(),
                command: Some("true".to_owned()),
                command_file: None,
                command_label: None,
                cwd: None,
                exit_code: 0,
                summary: "ok".to_owned(),
                artifact_path: None,
                output_path: Some("accepted.patch".to_owned()),
                allow_secret_risk: false,
                override_reason: None,
                allow_resolution_risk: false,
                resolution_risk_reason: None,
            },
            ApiMethod::AgentAcceptRun {
                workspace: "workspace-1".to_owned(),
                argv: vec!["true".to_owned()],
                command_file: None,
                command_label: "verify".to_owned(),
                cwd: None,
                timeout_seconds: Some(120),
                summary: "ok".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::Auto,
                mount_root: Some("/tmp/anvics-mounts".to_owned()),
                allow_command_risk: true,
                command_risk_reason: Some("operator approved network verification".to_owned()),
                output_path: Some("accepted.patch".to_owned()),
                allow_secret_risk: true,
                override_reason: Some("fixture false positive".to_owned()),
                allow_resolution_risk: true,
                resolution_risk_reason: Some("operator accepted tradeoff".to_owned()),
            },
            ApiMethod::LegacyGitExport {
                publication: "publication-1".to_owned(),
                output: "accepted.patch".to_owned(),
            },
            ApiMethod::EventsSince { sequence: 42 },
            ApiMethod::CoordinationStatus {
                workspace: "workspace-1".to_owned(),
            },
        ];

        for method in requests {
            let request = ApiRequest {
                id: 7,
                repo: "/tmp/repo".to_owned(),
                method,
            };
            assert_eq!(
                serde_json::from_str::<ApiRequest>(&serde_json::to_string(&request).unwrap())
                    .unwrap(),
                request
            );
        }
    }

    #[test]
    fn command_run_request_defaults_projection_for_old_clients() {
        let json = r#"{
            "id": 7,
            "repo": "/tmp/repo",
            "method": {
                "method": "command_run",
                "workspace": "workspace-1",
                "argv": ["true"],
                "command_file": null,
                "command_label": "verify",
                "cwd": null,
                "timeout_seconds": null,
                "summary": "ok",
                "artifact_path": null,
                "mount_root": null
            }
        }"#;

        let request: ApiRequest = serde_json::from_str(json).unwrap();
        match request.method {
            ApiMethod::CommandRun { projection, .. } => {
                assert_eq!(projection, ProjectionRequest::MaterializedDir);
            }
            _ => panic!("expected command run request"),
        }
    }

    #[test]
    fn api_response_round_trips_for_simple_results() {
        let base_snapshot = SourceSnapshotId::new();
        let final_snapshot = SourceSnapshotId::new();
        let thread_id = WorkThreadId::new();
        let workspace_id = WorkspaceViewId::new();
        let evidence_id = EvidenceRecordId::new();
        let command_event_id = CommandEventId::new();
        let review_id = ReviewProjectionId::new();
        let publication_id = NativePublicationId::new();
        let risk_scan_id = RiskScanId::new();
        let risk_finding_id = RiskFindingId::new();
        let object = ObjectId::new("a".repeat(64)).unwrap();
        let manifest = RepositoryManifest {
            id: RepositoryId::new(),
            format_version: 1,
            created_at: "2026-05-28T00:00:00Z".to_owned(),
        };
        let snapshot = SourceSnapshot {
            id: base_snapshot.clone(),
            root_tree: object,
            created_at: "2026-05-28T00:00:01Z".to_owned(),
            message: Some("base".to_owned()),
        };
        let thread = WorkThread {
            id: thread_id.clone(),
            title: "title".to_owned(),
            task: "task".to_owned(),
            base_snapshot: base_snapshot.clone(),
            source_review_ids: Vec::new(),
            conflict_analysis_id: None,
            status: WorkThreadStatus::Active,
            created_at: "2026-05-28T00:00:02Z".to_owned(),
        };
        let workspace = WorkspaceView {
            id: workspace_id,
            thread_id: thread_id.clone(),
            base_snapshot: base_snapshot.clone(),
            materialized_path: ".anvics/workspaces/example/files".to_owned(),
            latest_snapshot: Some(final_snapshot.clone()),
            created_at: "2026-05-28T00:00:03Z".to_owned(),
        };
        let evidence = EvidenceRecord {
            id: evidence_id.clone(),
            thread_id: thread_id.clone(),
            command_event_id: Some(command_event_id.clone()),
            command: "true".to_owned(),
            command_label: Some("verify".to_owned()),
            command_file: None,
            cwd: None,
            exit_code: 0,
            summary: "ok".to_owned(),
            artifact_path: None,
            stdout_path: Some(".anvics/artifacts/commands/command/stdout.txt".to_owned()),
            stderr_path: Some(".anvics/artifacts/commands/command/stderr.txt".to_owned()),
            created_at: "2026-05-28T00:00:04Z".to_owned(),
            superseded_at: None,
            superseded_reason: None,
        };
        let command_event = CommandEvent {
            id: command_event_id.clone(),
            workspace_id: workspace.id.clone(),
            thread_id: thread_id.clone(),
            agent_session_id: None,
            command_label: "verify".to_owned(),
            argv: vec!["true".to_owned()],
            command_file: None,
            cwd: ".anvics/workspaces/example/files".to_owned(),
            exit_code: Some(0),
            timed_out: false,
            duration_ms: 3,
            summary: "ok".to_owned(),
            artifact_path: None,
            stdout_path: Some(".anvics/artifacts/commands/command/stdout.txt".to_owned()),
            stderr_path: Some(".anvics/artifacts/commands/command/stderr.txt".to_owned()),
            projection_kind: Some(ProjectionKind::MaterializedDir),
            projection_root: Some(".anvics/workspaces/example/files".to_owned()),
            projection_capabilities: Some(ProjectionCapabilities {
                readable: true,
                writable: true,
                file_effects: true,
            }),
            projection_fallback_reason: None,
            command_executor: Some(CommandExecutorKind::InProcess),
            command_policy_class: Some(CommandPolicyClass::ReadOnly),
            command_policy_override_reason: None,
            runtime_metrics: Some(CommandRuntimeMetrics {
                projection_setup_ms: 1,
                command_ms: 3,
                reconcile_ms: 1,
                cleanup_ms: 0,
                projection_files: 1,
                projection_bytes: 12,
            }),
            file_effects: vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }],
            started_at: "2026-05-28T00:00:04Z".to_owned(),
            finished_at: Some("2026-05-28T00:00:05Z".to_owned()),
        };
        let review = ReviewProjection {
            id: review_id.clone(),
            thread_id: thread_id.clone(),
            base_snapshot: base_snapshot.clone(),
            final_snapshot: final_snapshot.clone(),
            source_review_ids: Vec::new(),
            conflict_analysis_id: None,
            changed_paths: vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }],
            file_effects: vec![FileEffect {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
                labels: vec![FileEffectLabel::Source],
                provenance: FileEffectProvenance::Heuristic,
            }],
            change_units: vec![ChangeUnit {
                id: ChangeUnitId::new(),
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
                labels: vec![FileEffectLabel::Source],
                provenance: FileEffectProvenance::Heuristic,
                summary: "modified app.txt".to_owned(),
            }],
            overlap_notes: Vec::new(),
            evidence: vec![EvidenceSummary {
                id: evidence_id.clone(),
                command_event_id: Some(command_event_id),
                command: "true".to_owned(),
                command_label: Some("verify".to_owned()),
                command_file: None,
                cwd: None,
                exit_code: 0,
                summary: "ok".to_owned(),
                artifact_path: None,
                stdout_path: Some(".anvics/artifacts/commands/command/stdout.txt".to_owned()),
                stderr_path: Some(".anvics/artifacts/commands/command/stderr.txt".to_owned()),
                projection_kind: Some(ProjectionKind::MaterializedDir),
                runtime_metrics: Some(CommandRuntimeMetrics {
                    projection_setup_ms: 1,
                    command_ms: 3,
                    reconcile_ms: 1,
                    cleanup_ms: 0,
                    projection_files: 1,
                    projection_bytes: 12,
                }),
                command_executor: Some(CommandExecutorKind::InProcess),
                command_policy_class: Some(CommandPolicyClass::ReadOnly),
                command_policy_override_reason: None,
                file_effects: vec![ChangedPath {
                    path: "app.txt".to_owned(),
                    status: ChangeStatus::Modified,
                }],
            }],
            created_at: "2026-05-28T00:00:05Z".to_owned(),
        };
        let risk_finding = RiskFinding {
            id: risk_finding_id,
            scan_id: risk_scan_id.clone(),
            review_id: review_id.clone(),
            evidence_id: Some(evidence_id.clone()),
            detector: "openai_token".to_owned(),
            target_kind: RiskTargetKind::SourceFile,
            target_path: "app.txt".to_owned(),
            line: Some(1),
            severity: RiskSeverity::SecretRisk,
            redacted_excerpt: "OPENAI_API_KEY=<redacted:51 chars>".to_owned(),
        };
        let risk_scan = RiskScan {
            id: risk_scan_id,
            review_id: review_id.clone(),
            findings: vec![risk_finding.clone()],
            created_at: "2026-05-28T00:00:05Z".to_owned(),
        };
        let publication = NativePublication {
            id: publication_id,
            thread_id: thread_id.clone(),
            accepted_snapshot: final_snapshot.clone(),
            review_id: review_id.clone(),
            created_at: "2026-05-28T00:00:06Z".to_owned(),
        };
        let preparation = AgentPreparation {
            thread: thread.clone(),
            workspace: workspace.clone(),
            packet_path: ".anvics/agent-packets/thread.md".to_owned(),
            agent_command: Some("anvics".to_owned()),
        };
        let conflict_analysis = ConflictAnalysis {
            id: anvics_core::ConflictAnalysisId::new(),
            base_snapshot: base_snapshot.clone(),
            input_reviews: Vec::new(),
            path_cases: Vec::new(),
            created_at: "2026-05-28T00:00:06Z".to_owned(),
        };
        let conflict_preparation = ConflictPreparation {
            analysis: conflict_analysis.clone(),
            analysis_markdown_path: ".anvics/conflicts/conflict.md".to_owned(),
            preparation: preparation.clone(),
        };
        let resolution_verification = ResolutionVerification {
            workspace_id: workspace.id.clone(),
            thread_id: thread.id.clone(),
            conflict_analysis_id: Some(conflict_analysis.id.clone()),
            passed: true,
            findings: Vec::new(),
            current_changed_paths: vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }],
            created_at: "2026-05-28T00:00:06Z".to_owned(),
        };
        let launch_prompt = AgentLaunchPrompt {
            tool: AgentLaunchTool::Codex,
            thread_id: thread.id.clone(),
            workspace_id: workspace.id.clone(),
            repo_path: "/tmp/repo".to_owned(),
            workspace_path: "/tmp/repo/.anvics/workspaces/workspace/files".to_owned(),
            packet_path: ".anvics/agent-packets/thread.md".to_owned(),
            skill_path: Some(
                "/tmp/repo/.anvics/workspaces/workspace/files/skills/anvics-skill/SKILL.md"
                    .to_owned(),
            ),
            prompt: "Read the packet.".to_owned(),
            command: Some("codex exec --skip-git-repo-check".to_owned()),
        };
        let instruction_file = AgentInstructionFile {
            path: "/tmp/repo/AGENTS.md".to_owned(),
            content: "# Anvics Agent Instructions\n".to_owned(),
            written: false,
        };
        let context_pack = AgentContextPack {
            thread_id: thread.id.clone(),
            workspace_id: workspace.id.clone(),
            repo_path: "/tmp/repo".to_owned(),
            workspace_path: "/tmp/repo/.anvics/workspaces/workspace/files".to_owned(),
            packet_path: Some(".anvics/agent-packets/thread.md".to_owned()),
            skill_path: None,
            content: "# Anvics Context Pack".to_owned(),
            path: None,
            written: false,
        };
        let checkpoint = AgentCheckpoint {
            id: anvics_core::AgentCheckpointId::new(),
            thread_id: thread.id.clone(),
            workspace_id: workspace.id.clone(),
            snapshot_id: final_snapshot.clone(),
            summary: "salvage docs edits".to_owned(),
            changed_paths: vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }],
            created_at: "2026-05-28T00:00:09Z".to_owned(),
        };
        let finish = AgentFinish {
            evidence: evidence.clone(),
            workspace: workspace.clone(),
            review: review.clone(),
            review_markdown_path: ".anvics/reviews/review.md".to_owned(),
        };
        let acceptance = AgentAcceptance {
            evidence: evidence.clone(),
            workspace: workspace.clone(),
            review: review.clone(),
            review_markdown_path: ".anvics/reviews/review.md".to_owned(),
            publication: publication.clone(),
            patch_path: "accepted.patch".to_owned(),
        };
        let session = AgentSession {
            id: AgentSessionId::new(),
            thread_id: thread_id.clone(),
            workspace_id: workspace.id.clone(),
            agent_name: "codex-cli".to_owned(),
            status: AgentSessionStatus::Active,
            entered_at: "2026-05-28T00:00:07Z".to_owned(),
            last_seen_at: "2026-05-28T00:00:08Z".to_owned(),
            finished_at: None,
        };
        let coordination = CoordinationStatus {
            current_session: Some(session.clone()),
            workspace: workspace.clone(),
            thread: thread.clone(),
            known_changed_paths: vec!["app.txt".to_owned()],
            related_work: vec![RelatedWork {
                session_id: Some(session.id.clone()),
                agent_name: "codex-cli".to_owned(),
                thread_id: thread_id.clone(),
                thread_title: "title".to_owned(),
                workspace_id: workspace.id.clone(),
                known_changed_paths: vec!["app.txt".to_owned()],
                overlap_paths: vec!["app.txt".to_owned()],
                freshness_note: "known changed paths from latest overlay".to_owned(),
            }],
            potential_clash_notes: vec!["Potential path overlap with title: app.txt".to_owned()],
        };
        let recovery = AgentRecovery {
            thread: thread.clone(),
            workspace: workspace.clone(),
            current_changed_paths: vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }],
            latest_checkpoint: Some(checkpoint.clone()),
            active_sessions: vec![session.clone()],
            notes: vec!["Current workspace has 1 recoverable changed path(s).".to_owned()],
        };
        let results = vec![
            ApiResult::Pong,
            ApiResult::RepoInit {
                manifest: manifest.clone(),
            },
            ApiResult::RepoStatus {
                initialized: true,
                manifest: Some(manifest),
            },
            ApiResult::RepoDoctor {
                report: RepoDoctorReport {
                    config_present: true,
                    config_path: Some("anvics.toml".to_owned()),
                    generated_tracked: vec!["src/generated/**".to_owned()],
                    generated_untracked: vec!["dist/**".to_owned()],
                    ignore_paths: vec!["target/**".to_owned()],
                    evidence_candidate_paths: vec!["reports/**".to_owned()],
                    classified_paths: vec![FileEffectClassification {
                        path: "src/lib.rs".to_owned(),
                        labels: vec![FileEffectLabel::Source],
                        provenance: FileEffectProvenance::Heuristic,
                    }],
                    notes: vec!["ok".to_owned()],
                },
            },
            ApiResult::SnapshotCreate {
                snapshot: snapshot.clone(),
            },
            ApiResult::SnapshotList {
                snapshots: vec![snapshot.clone()],
            },
            ApiResult::SnapshotShow { snapshot },
            ApiResult::ThreadCreate {
                thread: Box::new(thread.clone()),
            },
            ApiResult::ThreadList {
                threads: vec![thread.clone()],
            },
            ApiResult::ThreadShow {
                thread: Box::new(thread.clone()),
            },
            ApiResult::WorkspaceCreate {
                workspace: Box::new(workspace.clone()),
            },
            ApiResult::WorkspaceShow {
                workspace: Box::new(workspace.clone()),
                changed_paths: Some(vec![ChangedPath {
                    path: "app.txt".to_owned(),
                    status: ChangeStatus::Modified,
                }]),
            },
            ApiResult::WorkspaceDiff {
                changed_paths: vec![ChangedPath {
                    path: "app.txt".to_owned(),
                    status: ChangeStatus::Modified,
                }],
                file_effects: vec![FileEffect {
                    path: "app.txt".to_owned(),
                    status: ChangeStatus::Modified,
                    labels: vec![FileEffectLabel::Source],
                    provenance: FileEffectProvenance::Heuristic,
                }],
                patch: Some("diff --git a/app.txt b/app.txt\n".to_owned()),
            },
            ApiResult::WorkspaceSnapshot {
                workspace: Box::new(workspace.clone()),
            },
            ApiResult::EvidenceAttached {
                evidence: evidence.clone(),
            },
            ApiResult::EvidenceList {
                evidence: vec![evidence.clone()],
            },
            ApiResult::EvidenceShow {
                evidence: evidence.clone(),
            },
            ApiResult::EvidenceSuperseded {
                evidence: evidence.clone(),
            },
            ApiResult::CommandRun {
                command_event: Box::new(command_event.clone()),
                evidence: evidence.clone(),
            },
            ApiResult::CommandClassify {
                decision: CommandPolicyDecision {
                    policy_class: CommandPolicyClass::Networked,
                    blocked: true,
                    override_hint: Some(
                        "--allow-command-risk --command-risk-reason <reason>".to_owned(),
                    ),
                },
            },
            ApiResult::CommandShow {
                command_event: Box::new(command_event.clone()),
            },
            ApiResult::ReviewCreate {
                review: Box::new(review.clone()),
            },
            ApiResult::AgentPrepare {
                preparation: Box::new(preparation),
            },
            ApiResult::ConflictAnalyze {
                analysis: Box::new(conflict_analysis),
                markdown_path: ".anvics/conflicts/conflict.md".to_owned(),
                markdown: "# Conflict Analysis".to_owned(),
            },
            ApiResult::ConflictPrepare {
                preparation: Box::new(conflict_preparation),
            },
            ApiResult::ConflictStatus {
                verification: Box::new(resolution_verification.clone()),
            },
            ApiResult::ConflictVerify {
                verification: Box::new(resolution_verification),
            },
            ApiResult::AgentEnter {
                status: Box::new(coordination.clone()),
            },
            ApiResult::AgentLeave {
                session: session.clone(),
            },
            ApiResult::AgentSessions {
                sessions: vec![session],
            },
            ApiResult::AgentStatus {
                status: Box::new(AgentStatus {
                    thread: thread.clone(),
                    workspaces: vec![workspace],
                    evidence_count: 1,
                    review_ids: vec![review_id],
                    publication_ids: vec![publication.id.clone()],
                }),
            },
            ApiResult::AgentAccept {
                acceptance: Box::new(acceptance),
            },
            ApiResult::AgentPacket {
                path: ".anvics/agent-packets/thread.md".to_owned(),
            },
            ApiResult::AgentLaunchPrompt {
                prompt: Box::new(launch_prompt),
            },
            ApiResult::AgentInstructions {
                files: vec![instruction_file],
            },
            ApiResult::AgentContextPack {
                pack: Box::new(context_pack),
            },
            ApiResult::AgentCheckpoint {
                checkpoint: Box::new(checkpoint),
            },
            ApiResult::AgentRecover {
                recovery: Box::new(recovery),
            },
            ApiResult::AgentFinish {
                finish: Box::new(finish),
            },
            ApiResult::ReviewShowJson {
                review: Box::new(review),
            },
            ApiResult::ReviewShowMarkdown {
                markdown: "# Review".to_owned(),
            },
            ApiResult::ReviewPath {
                path: ".anvics/reviews/review.md".to_owned(),
            },
            ApiResult::PublishCreate { publication },
            ApiResult::RiskScan {
                scan: Box::new(risk_scan),
            },
            ApiResult::RiskList {
                findings: vec![risk_finding.clone()],
            },
            ApiResult::RiskShow {
                finding: risk_finding,
            },
            ApiResult::LegacyGitExport {
                output: "accepted.patch".to_owned(),
            },
            ApiResult::EventsSince {
                events: vec![RepositoryEvent {
                    id: RepositoryEventId::new(),
                    sequence: 1,
                    kind: RepositoryEventKind::RepositoryInitialized,
                    subject_id: None,
                    created_at: "2026-05-28T00:00:09Z".to_owned(),
                }],
            },
            ApiResult::CoordinationStatus {
                status: Box::new(coordination),
            },
            ApiResult::Error {
                message: "missing thread".to_owned(),
            },
        ];

        for result in results {
            let response = ApiResponse::ok(7, result);
            assert_eq!(
                serde_json::from_str::<ApiResponse>(&serde_json::to_string(&response).unwrap())
                    .unwrap(),
                response
            );
        }
    }
}
