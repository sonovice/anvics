use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdError {
    #[error("id must not be empty")]
    Empty,
    #[error("object id must be 64 lowercase hex characters")]
    InvalidObjectId,
}

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.is_empty() {
                    return Err(IdError::Empty);
                }
                Ok(Self(value.to_owned()))
            }
        }
    };
}

opaque_id!(RepositoryId);
opaque_id!(SourceSnapshotId);
opaque_id!(WorkThreadId);
opaque_id!(WorkspaceViewId);
opaque_id!(EvidenceRecordId);
opaque_id!(CommandEventId);
opaque_id!(ReviewProjectionId);
opaque_id!(NativePublicationId);
opaque_id!(RepositoryEventId);
opaque_id!(AgentSessionId);
opaque_id!(AgentCheckpointId);
opaque_id!(RiskScanId);
opaque_id!(RiskFindingId);
opaque_id!(PolicyOverrideId);
opaque_id!(FileEffectSetId);
opaque_id!(ChangeUnitId);
opaque_id!(ConflictAnalysisId);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ObjectId(String);

impl ObjectId {
    pub fn new(hex: impl Into<String>) -> Result<Self, IdError> {
        let hex = hex.into();
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(IdError::InvalidObjectId);
        }
        Ok(Self(hex))
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes).to_hex().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ObjectId {
    type Err = IdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RepositoryManifest {
    pub id: RepositoryId,
    pub format_version: u32,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RepoDoctorReport {
    pub config_present: bool,
    pub config_path: Option<String>,
    pub generated_tracked: Vec<String>,
    pub generated_untracked: Vec<String>,
    pub ignore_paths: Vec<String>,
    pub evidence_candidate_paths: Vec<String>,
    pub classified_paths: Vec<FileEffectClassification>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct FileEffectClassification {
    pub path: String,
    pub labels: Vec<FileEffectLabel>,
    pub provenance: FileEffectProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SourceSnapshot {
    pub id: SourceSnapshotId,
    pub root_tree: ObjectId,
    pub created_at: String,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TreeEntry {
    pub name: String,
    pub kind: TreeEntryKind,
    pub object: ObjectId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeEntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct WorkThread {
    pub id: WorkThreadId,
    pub title: String,
    pub task: String,
    pub base_snapshot: SourceSnapshotId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_review_ids: Vec<ReviewProjectionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_analysis_id: Option<ConflictAnalysisId>,
    pub status: WorkThreadStatus,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkThreadStatus {
    Active,
    Published,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct WorkspaceView {
    pub id: WorkspaceViewId,
    pub thread_id: WorkThreadId,
    pub base_snapshot: SourceSnapshotId,
    pub materialized_path: String,
    pub latest_snapshot: Option<SourceSnapshotId>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct WorkspaceOverlay {
    pub workspace_id: WorkspaceViewId,
    pub base_snapshot: SourceSnapshotId,
    pub snapshot: SourceSnapshotId,
    pub entries: Vec<OverlayEntry>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct OverlayEntry {
    pub path: String,
    pub status: ChangeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionKind {
    MaterializedDir,
    FuseMount,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionRequest {
    #[default]
    MaterializedDir,
    FuseMount,
    Auto,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProjectionCapabilities {
    pub readable: bool,
    pub writable: bool,
    pub file_effects: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CommandRuntimeMetrics {
    pub projection_setup_ms: u64,
    pub command_ms: u64,
    pub reconcile_ms: u64,
    pub cleanup_ms: u64,
    pub projection_files: u64,
    pub projection_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandPolicyClass {
    ReadOnly,
    Mutating,
    Destructive,
    Networked,
    HostEscapeRisk,
    Interactive,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CommandPolicyDecision {
    pub policy_class: CommandPolicyClass,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_hint: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandExecutorKind {
    InProcess,
    Worker,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CommandWorkerRequest {
    pub argv: Vec<String>,
    pub cwd: String,
    pub timeout_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CommandWorkerResponse {
    pub exit_code: i32,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct EvidenceRecord {
    pub id: EvidenceRecordId,
    pub thread_id: WorkThreadId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_event_id: Option<CommandEventId>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub exit_code: i32,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_path: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct EvidenceSummary {
    pub id: EvidenceRecordId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_event_id: Option<CommandEventId>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub exit_code: i32,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_kind: Option<ProjectionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_metrics: Option<CommandRuntimeMetrics>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_executor: Option<CommandExecutorKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_policy_class: Option<CommandPolicyClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_policy_override_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_effects: Vec<ChangedPath>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CommandEvent {
    pub id: CommandEventId,
    pub workspace_id: WorkspaceViewId,
    pub thread_id: WorkThreadId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<AgentSessionId>,
    pub command_label: String,
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_file: Option<String>,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_kind: Option<ProjectionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_capabilities: Option<ProjectionCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_fallback_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_executor: Option<CommandExecutorKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_policy_class: Option<CommandPolicyClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_policy_override_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_metrics: Option<CommandRuntimeMetrics>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_effects: Vec<ChangedPath>,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ReviewProjection {
    pub id: ReviewProjectionId,
    pub thread_id: WorkThreadId,
    pub base_snapshot: SourceSnapshotId,
    pub final_snapshot: SourceSnapshotId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_review_ids: Vec<ReviewProjectionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_analysis_id: Option<ConflictAnalysisId>,
    pub changed_paths: Vec<ChangedPath>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_effects: Vec<FileEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub change_units: Vec<ChangeUnit>,
    pub overlap_notes: Vec<String>,
    pub evidence: Vec<EvidenceSummary>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RiskScan {
    pub id: RiskScanId,
    pub review_id: ReviewProjectionId,
    pub findings: Vec<RiskFinding>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RiskFinding {
    pub id: RiskFindingId,
    pub scan_id: RiskScanId,
    pub review_id: ReviewProjectionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_id: Option<EvidenceRecordId>,
    pub detector: String,
    pub target_kind: RiskTargetKind,
    pub target_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub severity: RiskSeverity,
    pub redacted_excerpt: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTargetKind {
    SourceFile,
    CommandStdout,
    CommandStderr,
    EvidenceArtifact,
    CommandFile,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskSeverity {
    SecretRisk,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct PolicyOverride {
    pub id: PolicyOverrideId,
    pub review_id: ReviewProjectionId,
    pub reason: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ConflictAnalysis {
    pub id: ConflictAnalysisId,
    pub base_snapshot: SourceSnapshotId,
    pub input_reviews: Vec<ConflictInputReview>,
    pub path_cases: Vec<ConflictPathCase>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ConflictInputReview {
    pub review_id: ReviewProjectionId,
    pub thread_id: WorkThreadId,
    pub title: String,
    pub final_snapshot: SourceSnapshotId,
    pub changed_paths: Vec<ChangedPath>,
    pub evidence_summaries: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ConflictPathCase {
    pub path: String,
    pub kind: ConflictCaseKind,
    pub safety: MergeSafety,
    pub review_ids: Vec<ReviewProjectionId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hunks: Vec<ConflictHunkCase>,
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ConflictHunkCase {
    pub review_id: ReviewProjectionId,
    pub base_start: u32,
    pub base_end: u32,
    pub final_start: u32,
    pub final_end: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCaseKind {
    IndependentPath,
    SamePathNonOverlappingHunks,
    SamePathOverlappingHunks,
    ModifyDelete,
    AddAdd,
    BinaryOrUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeSafety {
    AutoMergeable,
    NeedsResolution,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ResolutionVerification {
    pub workspace_id: WorkspaceViewId,
    pub thread_id: WorkThreadId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_analysis_id: Option<ConflictAnalysisId>,
    pub passed: bool,
    pub findings: Vec<String>,
    pub current_changed_paths: Vec<ChangedPath>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ConflictPreparation {
    pub analysis: ConflictAnalysis,
    pub analysis_markdown_path: String,
    pub preparation: AgentPreparation,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChangedPath {
    pub path: String,
    pub status: ChangeStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct FileEffectSet {
    pub id: FileEffectSetId,
    pub effects: Vec<FileEffect>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct FileEffect {
    pub path: String,
    pub status: ChangeStatus,
    pub labels: Vec<FileEffectLabel>,
    pub provenance: FileEffectProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChangeUnit {
    pub id: ChangeUnitId,
    pub path: String,
    pub status: ChangeStatus,
    pub labels: Vec<FileEffectLabel>,
    pub provenance: FileEffectProvenance,
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEffectLabel {
    Source,
    GeneratedTracked,
    GeneratedUntracked,
    EvidenceCandidate,
    Cache,
    Lockfile,
    Config,
    SecretRisk,
    Binary,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEffectProvenance {
    Policy,
    Heuristic,
    AgentClaim,
    Tool,
    Human,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct NativePublication {
    pub id: NativePublicationId,
    pub thread_id: WorkThreadId,
    pub accepted_snapshot: SourceSnapshotId,
    pub review_id: ReviewProjectionId,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RepositoryEvent {
    pub id: RepositoryEventId,
    pub sequence: u64,
    pub kind: RepositoryEventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryEventKind {
    RepositoryInitialized,
    SnapshotCreated,
    WorkThreadCreated,
    WorkspaceCreated,
    EvidenceAttached,
    EvidenceSuperseded,
    ReviewCreated,
    PublicationCreated,
    LegacyPatchExported,
    CommandStarted,
    CommandFinished,
    AgentSessionEntered,
    AgentSessionSeen,
    AgentSessionFinished,
    AgentCheckpointCreated,
    RiskScanCreated,
    SecretRiskDetected,
    PolicyOverrideRecorded,
    ConflictAnalysisCreated,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentSession {
    pub id: AgentSessionId,
    pub thread_id: WorkThreadId,
    pub workspace_id: WorkspaceViewId,
    pub agent_name: String,
    pub status: AgentSessionStatus,
    pub entered_at: String,
    pub last_seen_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionStatus {
    Active,
    Finished,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CoordinationStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_session: Option<AgentSession>,
    pub workspace: WorkspaceView,
    pub thread: WorkThread,
    pub known_changed_paths: Vec<String>,
    pub related_work: Vec<RelatedWork>,
    pub potential_clash_notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RelatedWork {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<AgentSessionId>,
    pub agent_name: String,
    pub thread_id: WorkThreadId,
    pub thread_title: String,
    pub workspace_id: WorkspaceViewId,
    pub known_changed_paths: Vec<String>,
    pub overlap_paths: Vec<String>,
    pub freshness_note: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentPreparation {
    pub thread: WorkThread,
    pub workspace: WorkspaceView,
    pub packet_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_command: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLaunchTool {
    Generic,
    Codex,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentInstructionTarget {
    Agents,
    Claude,
    All,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentInstructionFile {
    pub path: String,
    pub content: String,
    pub written: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentContextPack {
    pub thread_id: WorkThreadId,
    pub workspace_id: WorkspaceViewId,
    pub repo_path: String,
    pub workspace_path: String,
    pub packet_path: Option<String>,
    pub skill_path: Option<String>,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub written: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentLaunchPrompt {
    pub tool: AgentLaunchTool,
    pub thread_id: WorkThreadId,
    pub workspace_id: WorkspaceViewId,
    pub repo_path: String,
    pub workspace_path: String,
    pub packet_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill_path: Option<String>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentCheckpoint {
    pub id: AgentCheckpointId,
    pub thread_id: WorkThreadId,
    pub workspace_id: WorkspaceViewId,
    pub snapshot_id: SourceSnapshotId,
    pub summary: String,
    pub changed_paths: Vec<ChangedPath>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentRecovery {
    pub thread: WorkThread,
    pub workspace: WorkspaceView,
    pub current_changed_paths: Vec<ChangedPath>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_checkpoint: Option<AgentCheckpoint>,
    pub active_sessions: Vec<AgentSession>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentFinish {
    pub evidence: EvidenceRecord,
    pub workspace: WorkspaceView,
    pub review: ReviewProjection,
    pub review_markdown_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentAcceptance {
    pub evidence: EvidenceRecord,
    pub workspace: WorkspaceView,
    pub review: ReviewProjection,
    pub review_markdown_path: String,
    pub publication: NativePublication,
    pub patch_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentStatus {
    pub thread: WorkThread,
    pub workspaces: Vec<WorkspaceView>,
    pub evidence_count: usize,
    pub review_ids: Vec<ReviewProjectionId>,
    pub publication_ids: Vec<NativePublicationId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_ids_round_trip_as_strings() {
        let id = RepositoryId::new();
        let json = serde_json::to_string(&id).unwrap();
        let decoded: RepositoryId = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, id);
    }

    #[test]
    fn object_ids_are_validated() {
        assert!(ObjectId::new("abc").is_err());
        assert!(ObjectId::new("A".repeat(64)).is_err());
        assert!(ObjectId::new("a".repeat(64)).is_ok());
    }

    #[test]
    fn mvp_workflow_objects_round_trip_as_json() {
        let base_snapshot = SourceSnapshotId::new();
        let final_snapshot = SourceSnapshotId::new();
        let thread_id = WorkThreadId::new();
        let review_id = ReviewProjectionId::new();
        let evidence_id = EvidenceRecordId::new();
        let command_event_id = CommandEventId::new();

        let thread = WorkThread {
            id: thread_id.clone(),
            title: "Agent task".to_owned(),
            task: "Edit a file".to_owned(),
            base_snapshot: base_snapshot.clone(),
            source_review_ids: Vec::new(),
            conflict_analysis_id: None,
            status: WorkThreadStatus::Active,
            created_at: "2026-05-28T00:00:00Z".to_owned(),
        };
        let workspace = WorkspaceView {
            id: WorkspaceViewId::new(),
            thread_id: thread_id.clone(),
            base_snapshot: base_snapshot.clone(),
            materialized_path: ".anvics/workspaces/example/files".to_owned(),
            latest_snapshot: Some(final_snapshot.clone()),
            created_at: "2026-05-28T00:00:01Z".to_owned(),
        };
        let overlay = WorkspaceOverlay {
            workspace_id: workspace.id.clone(),
            base_snapshot: base_snapshot.clone(),
            snapshot: final_snapshot.clone(),
            entries: vec![OverlayEntry {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
                object: Some(ObjectId::new("a".repeat(64)).unwrap()),
                size: Some(12),
            }],
            created_at: "2026-05-28T00:00:01Z".to_owned(),
        };
        let evidence = EvidenceRecord {
            id: evidence_id.clone(),
            thread_id: thread_id.clone(),
            command_event_id: Some(command_event_id.clone()),
            command: "cargo test".to_owned(),
            command_label: Some("tests".to_owned()),
            command_file: Some("evidence/commands/test.sh".to_owned()),
            cwd: Some(".".to_owned()),
            exit_code: 0,
            summary: "Tests passed".to_owned(),
            artifact_path: Some("target/test.log".to_owned()),
            stdout_path: Some(".anvics/artifacts/commands/event/stdout.txt".to_owned()),
            stderr_path: Some(".anvics/artifacts/commands/event/stderr.txt".to_owned()),
            created_at: "2026-05-28T00:00:02Z".to_owned(),
            superseded_at: None,
            superseded_reason: None,
        };
        let command_event = CommandEvent {
            id: command_event_id.clone(),
            workspace_id: workspace.id.clone(),
            thread_id: thread_id.clone(),
            agent_session_id: None,
            command_label: "tests".to_owned(),
            argv: vec!["cargo".to_owned(), "test".to_owned()],
            command_file: None,
            cwd: ".anvics/workspaces/example/files".to_owned(),
            exit_code: Some(0),
            timed_out: false,
            duration_ms: 12,
            summary: "Tests passed".to_owned(),
            artifact_path: Some("target/test.log".to_owned()),
            stdout_path: Some(".anvics/artifacts/commands/event/stdout.txt".to_owned()),
            stderr_path: Some(".anvics/artifacts/commands/event/stderr.txt".to_owned()),
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
            runtime_metrics: Some(CommandRuntimeMetrics {
                projection_setup_ms: 1,
                command_ms: 12,
                reconcile_ms: 2,
                cleanup_ms: 0,
                projection_files: 1,
                projection_bytes: 12,
            }),
            command_policy_override_reason: Some("audited test override".to_owned()),
            file_effects: vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }],
            started_at: "2026-05-28T00:00:02Z".to_owned(),
            finished_at: Some("2026-05-28T00:00:03Z".to_owned()),
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
            overlap_notes: vec!["No path overlap detected.".to_owned()],
            evidence: vec![EvidenceSummary {
                id: evidence_id,
                command_event_id: Some(command_event_id),
                command: "cargo test".to_owned(),
                command_label: Some("tests".to_owned()),
                command_file: Some("evidence/commands/test.sh".to_owned()),
                cwd: Some(".".to_owned()),
                exit_code: 0,
                summary: "Tests passed".to_owned(),
                artifact_path: Some("target/test.log".to_owned()),
                stdout_path: Some(".anvics/artifacts/commands/event/stdout.txt".to_owned()),
                stderr_path: Some(".anvics/artifacts/commands/event/stderr.txt".to_owned()),
                projection_kind: Some(ProjectionKind::MaterializedDir),
                runtime_metrics: Some(CommandRuntimeMetrics {
                    projection_setup_ms: 1,
                    command_ms: 12,
                    reconcile_ms: 2,
                    cleanup_ms: 0,
                    projection_files: 1,
                    projection_bytes: 12,
                }),
                command_executor: Some(CommandExecutorKind::InProcess),
                command_policy_class: Some(CommandPolicyClass::ReadOnly),
                command_policy_override_reason: Some("audited test override".to_owned()),
                file_effects: vec![ChangedPath {
                    path: "app.txt".to_owned(),
                    status: ChangeStatus::Modified,
                }],
            }],
            created_at: "2026-05-28T00:00:03Z".to_owned(),
        };
        let risk_scan_id = RiskScanId::new();
        let risk_finding = RiskFinding {
            id: RiskFindingId::new(),
            scan_id: risk_scan_id.clone(),
            review_id: review_id.clone(),
            evidence_id: None,
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
            created_at: "2026-05-28T00:00:03Z".to_owned(),
        };
        let override_record = PolicyOverride {
            id: PolicyOverrideId::new(),
            review_id: review_id.clone(),
            reason: "fixture false positive".to_owned(),
            created_at: "2026-05-28T00:00:04Z".to_owned(),
        };
        let publication = NativePublication {
            id: NativePublicationId::new(),
            thread_id: thread_id.clone(),
            accepted_snapshot: final_snapshot,
            review_id,
            created_at: "2026-05-28T00:00:04Z".to_owned(),
        };
        let event = RepositoryEvent {
            id: RepositoryEventId::new(),
            sequence: 1,
            kind: RepositoryEventKind::PublicationCreated,
            subject_id: Some(publication.id.to_string()),
            created_at: "2026-05-28T00:00:05Z".to_owned(),
        };
        let session = AgentSession {
            id: AgentSessionId::new(),
            thread_id: thread_id.clone(),
            workspace_id: workspace.id.clone(),
            agent_name: "codex-cli".to_owned(),
            status: AgentSessionStatus::Active,
            entered_at: "2026-05-28T00:00:06Z".to_owned(),
            last_seen_at: "2026-05-28T00:00:07Z".to_owned(),
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
                thread_title: "Agent task".to_owned(),
                workspace_id: workspace.id.clone(),
                known_changed_paths: vec!["app.txt".to_owned()],
                overlap_paths: vec!["app.txt".to_owned()],
                freshness_note: "known changed paths from latest overlay".to_owned(),
            }],
            potential_clash_notes: vec!["Potential path overlap: app.txt".to_owned()],
        };
        let acceptance = AgentAcceptance {
            evidence: evidence.clone(),
            workspace: workspace.clone(),
            review: review.clone(),
            review_markdown_path: ".anvics/reviews/example.md".to_owned(),
            publication: publication.clone(),
            patch_path: "accepted.patch".to_owned(),
        };

        assert_eq!(
            serde_json::from_str::<WorkThread>(&serde_json::to_string(&thread).unwrap()).unwrap(),
            thread
        );
        assert_eq!(
            serde_json::from_str::<WorkspaceView>(&serde_json::to_string(&workspace).unwrap())
                .unwrap(),
            workspace
        );
        assert_eq!(
            serde_json::from_str::<WorkspaceOverlay>(&serde_json::to_string(&overlay).unwrap())
                .unwrap(),
            overlay
        );
        assert_eq!(
            serde_json::from_str::<EvidenceRecord>(&serde_json::to_string(&evidence).unwrap())
                .unwrap(),
            evidence
        );
        assert_eq!(
            serde_json::from_str::<CommandEvent>(&serde_json::to_string(&command_event).unwrap())
                .unwrap(),
            command_event
        );
        assert_eq!(
            serde_json::from_str::<ReviewProjection>(&serde_json::to_string(&review).unwrap())
                .unwrap(),
            review
        );
        assert_eq!(
            serde_json::from_str::<RiskScan>(&serde_json::to_string(&risk_scan).unwrap()).unwrap(),
            risk_scan
        );
        assert_eq!(
            serde_json::from_str::<RiskFinding>(&serde_json::to_string(&risk_finding).unwrap())
                .unwrap(),
            risk_finding
        );
        assert_eq!(
            serde_json::from_str::<PolicyOverride>(
                &serde_json::to_string(&override_record).unwrap()
            )
            .unwrap(),
            override_record
        );
        assert_eq!(
            serde_json::from_str::<NativePublication>(
                &serde_json::to_string(&publication).unwrap()
            )
            .unwrap(),
            publication
        );
        assert_eq!(
            serde_json::from_str::<RepositoryEvent>(&serde_json::to_string(&event).unwrap())
                .unwrap(),
            event
        );
        assert_eq!(
            serde_json::from_str::<AgentSession>(&serde_json::to_string(&session).unwrap())
                .unwrap(),
            session
        );
        let instruction_file = AgentInstructionFile {
            path: "AGENTS.md".to_owned(),
            content: "instructions".to_owned(),
            written: true,
        };
        assert_eq!(
            serde_json::from_str::<AgentInstructionFile>(
                &serde_json::to_string(&instruction_file).unwrap()
            )
            .unwrap(),
            instruction_file
        );
        let context_pack = AgentContextPack {
            thread_id: thread.id.clone(),
            workspace_id: workspace.id.clone(),
            repo_path: "/tmp/repo".to_owned(),
            workspace_path: "/tmp/repo/.anvics/workspaces/workspace/files".to_owned(),
            packet_path: Some(".anvics/agent-packets/thread.md".to_owned()),
            skill_path: None,
            content: "# Context".to_owned(),
            path: Some(".anvics/context-packs/workspace.md".to_owned()),
            written: true,
        };
        assert_eq!(
            serde_json::from_str::<AgentContextPack>(
                &serde_json::to_string(&context_pack).unwrap()
            )
            .unwrap(),
            context_pack
        );
        assert_eq!(
            serde_json::from_str::<CoordinationStatus>(
                &serde_json::to_string(&coordination).unwrap()
            )
            .unwrap(),
            coordination
        );
        assert_eq!(
            serde_json::from_str::<AgentAcceptance>(&serde_json::to_string(&acceptance).unwrap())
                .unwrap(),
            acceptance
        );
    }

    #[test]
    fn command_event_accepts_missing_projection_fields() {
        let json = format!(
            r#"{{
                "id": "{}",
                "workspace_id": "{}",
                "thread_id": "{}",
                "command_label": "verify",
                "argv": ["true"],
                "cwd": ".anvics/workspaces/example/files",
                "exit_code": 0,
                "timed_out": false,
                "duration_ms": 1,
                "summary": "ok",
                "started_at": "2026-05-28T00:00:00Z",
                "finished_at": "2026-05-28T00:00:01Z"
            }}"#,
            CommandEventId::new(),
            WorkspaceViewId::new(),
            WorkThreadId::new()
        );

        let event: CommandEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event.projection_kind, None);
        assert_eq!(event.projection_root, None);
        assert_eq!(event.projection_capabilities, None);
        assert_eq!(event.projection_fallback_reason, None);
        assert_eq!(event.command_executor, None);
        assert_eq!(event.command_policy_class, None);
        assert_eq!(event.command_policy_override_reason, None);
        assert_eq!(event.runtime_metrics, None);
        assert!(event.file_effects.is_empty());
    }

    #[test]
    fn projection_request_and_fuse_kind_round_trip_as_json() {
        assert_eq!(
            serde_json::from_str::<ProjectionRequest>("\"fuse_mount\"").unwrap(),
            ProjectionRequest::FuseMount
        );
        assert_eq!(
            serde_json::to_string(&ProjectionRequest::Auto).unwrap(),
            "\"auto\""
        );
        assert_eq!(
            serde_json::from_str::<ProjectionKind>("\"fuse_mount\"").unwrap(),
            ProjectionKind::FuseMount
        );
    }
}
