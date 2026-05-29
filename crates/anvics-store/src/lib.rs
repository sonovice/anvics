use anvics_core::{
    AgentAcceptance, AgentFinish, AgentLaunchPrompt, AgentLaunchTool, AgentPreparation,
    AgentSession, AgentSessionId, AgentSessionStatus, AgentStatus, ChangeStatus, ChangeUnit,
    ChangeUnitId, ChangedPath, CommandEvent, CommandEventId, CommandExecutorKind,
    CommandPolicyClass, CommandPolicyDecision, CommandRuntimeMetrics, CommandWorkerRequest,
    CommandWorkerResponse, CoordinationStatus, EvidenceRecord, EvidenceRecordId, EvidenceSummary,
    FileEffect, FileEffectClassification, FileEffectLabel, FileEffectProvenance, FileEffectSet,
    FileEffectSetId, NativePublication, NativePublicationId, ObjectId, OverlayEntry,
    PolicyOverride, PolicyOverrideId, ProjectionCapabilities, ProjectionKind, ProjectionRequest,
    RelatedWork, RepoDoctorReport, RepositoryEvent, RepositoryEventId, RepositoryEventKind,
    RepositoryId, RepositoryManifest, ReviewProjection, ReviewProjectionId, RiskFinding,
    RiskFindingId, RiskScan, RiskScanId, RiskSeverity, RiskTargetKind, SourceSnapshot,
    SourceSnapshotId, Tree, TreeEntry, TreeEntryKind, WorkThread, WorkThreadId, WorkThreadStatus,
    WorkspaceOverlay, WorkspaceView, WorkspaceViewId,
};
use ignore::WalkBuilder;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const ANVICS_DIR: &str = ".anvics";
const FORMAT_VERSION: u32 = 1;
const DEFAULT_COMMAND_TIMEOUT_SECONDS: u64 = 120;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("not an Anvics repository: {0}")]
    NotRepository(PathBuf),
    #[error("Anvics repository already exists: {0}")]
    AlreadyInitialized(PathBuf),
    #[error("snapshot does not exist: {0}")]
    SnapshotNotFound(String),
    #[error("thread does not exist: {0}")]
    ThreadNotFound(String),
    #[error("workspace does not exist: {0}")]
    WorkspaceNotFound(String),
    #[error("review does not exist: {0}")]
    ReviewNotFound(String),
    #[error("publication does not exist: {0}")]
    PublicationNotFound(String),
    #[error("agent session does not exist: {0}")]
    AgentSessionNotFound(String),
    #[error("command event does not exist: {0}")]
    CommandEventNotFound(String),
    #[error("risk finding does not exist: {0}")]
    RiskFindingNotFound(String),
    #[error("agent packet does not exist for thread: {0}")]
    AgentPacketNotFound(String),
    #[error("repository has no current snapshot")]
    NoHeadSnapshot,
    #[error("thread has no workspace snapshot yet: {0}")]
    MissingWorkspaceSnapshot(String),
    #[error("evidence summary must not be empty")]
    EmptyEvidenceSummary,
    #[error("evidence command must not be empty")]
    EmptyEvidenceCommand,
    #[error("agent name must not be empty")]
    EmptyAgentName,
    #[error("command label must not be empty")]
    EmptyCommandLabel,
    #[error("command argv must not be empty")]
    EmptyCommandArgv,
    #[error("command cwd must stay inside workspace: {0}")]
    InvalidCommandCwd(String),
    #[error("projection unavailable: {0}")]
    ProjectionUnavailable(String),
    #[error("command worker failed: {0}")]
    CommandWorkerFailed(String),
    #[error("command {id} failed with exit code {exit_code}")]
    CommandFailed { id: String, exit_code: i32 },
    #[error("command policy blocked {policy_class:?}; rerun with --allow-command-risk --command-risk-reason <reason> to proceed")]
    CommandPolicyBlocked { policy_class: CommandPolicyClass },
    #[error("publication blocked by {finding_count} unresolved secret-risk finding(s); rerun with --allow-secret-risk --override-reason <reason> to proceed")]
    PublicationBlockedSecretRisk { finding_count: usize },
    #[error("override reason must not be empty")]
    EmptyOverrideReason,
    #[error("command risk reason must not be empty")]
    EmptyCommandRiskReason,
    #[error("--command-risk-reason requires --allow-command-risk")]
    CommandRiskReasonWithoutOverride,
    #[error("review {review_id} does not belong to thread {thread_id}")]
    ReviewThreadMismatch {
        review_id: String,
        thread_id: String,
    },
    #[error("invalid repository path outside root: {0}")]
    OutsideRoot(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Time(#[from] time::error::Format),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Walk(#[from] ignore::Error),
    #[error(transparent)]
    Vfs(#[from] anvics_vfs::VfsError),
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Clone, Debug)]
pub struct AnvicsStore {
    root: PathBuf,
    anvics_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct CommandEvidenceInput {
    pub command: String,
    pub command_event_id: Option<CommandEventId>,
    pub command_label: Option<String>,
    pub command_file: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: i32,
    pub summary: String,
    pub artifact_path: Option<String>,
    pub stdout_path: Option<String>,
    pub stderr_path: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CommandRunInput {
    pub workspace_id: String,
    pub argv: Vec<String>,
    pub command_file: Option<String>,
    pub command_label: String,
    pub cwd: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub summary: String,
    pub artifact_path: Option<String>,
    pub projection: ProjectionRequest,
    pub mount_root: Option<String>,
    pub allow_command_risk: bool,
    pub command_risk_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CommandPolicyInput {
    pub argv: Vec<String>,
    pub command_file: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CommandRunResult {
    pub command_event: CommandEvent,
    pub evidence: EvidenceRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkspaceProjection {
    workspace_id: WorkspaceViewId,
    thread_id: WorkThreadId,
    kind: ProjectionKind,
    root_path: PathBuf,
    capabilities: ProjectionCapabilities,
    fallback_reason: Option<String>,
}

#[derive(Debug)]
struct ResolvedProjection {
    projection: WorkspaceProjection,
    #[allow(dead_code)]
    mounted_workspace: Option<anvics_vfs::MountedWorkspace>,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
struct RepositoryConfig {
    #[serde(default)]
    generated: GeneratedConfig,
    #[serde(default)]
    ignore: IgnoreConfig,
    #[serde(default)]
    evidence: EvidenceConfig,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
struct GeneratedConfig {
    #[serde(default)]
    tracked: Vec<String>,
    #[serde(default)]
    untracked: Vec<String>,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
struct IgnoreConfig {
    #[serde(default)]
    paths: Vec<String>,
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
struct EvidenceConfig {
    #[serde(default)]
    candidate_paths: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PublicationOptions {
    pub allow_secret_risk: bool,
    pub override_reason: Option<String>,
}

impl AnvicsStore {
    pub fn init(root: impl AsRef<Path>) -> Result<RepositoryManifest> {
        let root = root.as_ref();
        let anvics_dir = root.join(ANVICS_DIR);
        let repo_json = anvics_dir.join("repo.json");

        if repo_json.exists() {
            return Err(StoreError::AlreadyInitialized(anvics_dir));
        }

        fs::create_dir_all(anvics_dir.join("objects/blake3"))?;
        fs::create_dir_all(anvics_dir.join("snapshots"))?;
        fs::create_dir_all(anvics_dir.join("threads"))?;
        fs::create_dir_all(anvics_dir.join("workspaces"))?;
        fs::create_dir_all(anvics_dir.join("evidence"))?;
        fs::create_dir_all(anvics_dir.join("reviews"))?;
        fs::create_dir_all(anvics_dir.join("publications"))?;
        fs::create_dir_all(anvics_dir.join("agent-packets"))?;
        fs::create_dir_all(anvics_dir.join("events"))?;
        fs::create_dir_all(anvics_dir.join("sessions"))?;
        fs::create_dir_all(anvics_dir.join("command-events"))?;
        fs::create_dir_all(anvics_dir.join("artifacts/commands"))?;
        fs::create_dir_all(anvics_dir.join("mounts"))?;
        fs::create_dir_all(anvics_dir.join("risks"))?;
        fs::create_dir_all(anvics_dir.join("policy-overrides"))?;

        let manifest = RepositoryManifest {
            id: RepositoryId::new(),
            format_version: FORMAT_VERSION,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(&repo_json, &manifest)?;
        append_event_at(
            &anvics_dir,
            RepositoryEventKind::RepositoryInitialized,
            Some(manifest.id.to_string()),
        )?;
        Ok(manifest)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let anvics_dir = root.join(ANVICS_DIR);
        if !anvics_dir.join("repo.json").exists() {
            return Err(StoreError::NotRepository(root));
        }
        Ok(Self { root, anvics_dir })
    }

    pub fn manifest(&self) -> Result<RepositoryManifest> {
        read_json(self.anvics_dir.join("repo.json"))
    }

    pub fn repo_doctor(&self, paths: Vec<String>) -> Result<RepoDoctorReport> {
        let config_path = self.root.join("anvics.toml");
        let config_present = config_path.exists();
        let config = self.repository_config()?;
        let classified_paths = paths
            .into_iter()
            .map(|path| FileEffectClassification {
                labels: classify_file_effect_path(&path, &config),
                path,
                provenance: FileEffectProvenance::Heuristic,
            })
            .collect();
        let mut notes = Vec::new();
        if config_present {
            notes.push("Review classification uses the accepted repo-root anvics.toml.".to_owned());
            notes.push(
                "Workspace edits to anvics.toml are reviewed as config changes before affecting later work."
                    .to_owned(),
            );
        } else {
            notes.push(
                "No root anvics.toml found; Anvics will use built-in path heuristics.".to_owned(),
            );
        }

        Ok(RepoDoctorReport {
            config_present,
            config_path: config_present.then(|| config_path.to_string_lossy().to_string()),
            generated_tracked: config.generated.tracked,
            generated_untracked: config.generated.untracked,
            ignore_paths: config.ignore.paths,
            evidence_candidate_paths: config.evidence.candidate_paths,
            classified_paths,
            notes,
        })
    }

    fn repository_config(&self) -> Result<RepositoryConfig> {
        let path = self.root.join("anvics.toml");
        if !path.exists() {
            return Ok(RepositoryConfig::default());
        }
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn create_snapshot(&self, message: Option<String>) -> Result<SourceSnapshot> {
        self.create_snapshot_from_path(&self.root, message, true)
    }

    pub fn create_snapshot_from_path(
        &self,
        source_root: impl AsRef<Path>,
        message: Option<String>,
        update_head: bool,
    ) -> Result<SourceSnapshot> {
        let source_root = source_root.as_ref();
        let files = collect_files(source_root)?;
        let mut tree = TreeNode::default();

        for file in files {
            let bytes = fs::read(&file)?;
            let object = self.store_object(&bytes)?;
            let relative = file
                .strip_prefix(source_root)
                .map_err(|_| StoreError::OutsideRoot(file.clone()))?;
            tree.insert(relative, object, bytes.len() as u64);
        }

        let root_tree = self.store_tree(&tree)?;
        let snapshot = SourceSnapshot {
            id: SourceSnapshotId::new(),
            root_tree,
            created_at: now_rfc3339()?,
            message,
        };

        let snapshot_path = self.snapshot_path(snapshot.id.as_str());
        write_json_pretty(snapshot_path, &snapshot)?;
        if update_head {
            fs::write(self.anvics_dir.join("HEAD"), snapshot.id.as_str())?;
        }
        self.append_event(
            RepositoryEventKind::SnapshotCreated,
            Some(snapshot.id.to_string()),
        )?;

        Ok(snapshot)
    }

    pub fn list_snapshots(&self) -> Result<Vec<SourceSnapshot>> {
        let mut snapshots = Vec::new();
        let snapshots_dir = self.anvics_dir.join("snapshots");

        for entry in fs::read_dir(snapshots_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry.path().extension().is_some_and(|ext| ext == "json")
            {
                snapshots.push(read_json(entry.path())?);
            }
        }

        snapshots.sort_by(|left: &SourceSnapshot, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(snapshots)
    }

    pub fn show_snapshot(&self, id: &str) -> Result<SourceSnapshot> {
        let path = self.snapshot_path(id);
        if !path.exists() {
            return Err(StoreError::SnapshotNotFound(id.to_owned()));
        }
        read_json(path)
    }

    pub fn current_snapshot(&self) -> Result<SourceSnapshot> {
        let head_path = self.anvics_dir.join("HEAD");
        if !head_path.exists() {
            return Err(StoreError::NoHeadSnapshot);
        }
        let id = fs::read_to_string(head_path)?;
        self.show_snapshot(id.trim())
    }

    pub fn create_thread(&self, title: String, task: String) -> Result<WorkThread> {
        let base_snapshot = self.current_snapshot()?.id;
        let thread = WorkThread {
            id: WorkThreadId::new(),
            title,
            task,
            base_snapshot,
            status: WorkThreadStatus::Active,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.thread_path(thread.id.as_str()), &thread)?;
        self.append_event(
            RepositoryEventKind::WorkThreadCreated,
            Some(thread.id.to_string()),
        )?;
        Ok(thread)
    }

    pub fn list_threads(&self) -> Result<Vec<WorkThread>> {
        let mut threads: Vec<WorkThread> = read_json_dir(self.anvics_dir.join("threads"))?;
        threads.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(threads)
    }

    pub fn show_thread(&self, id: &str) -> Result<WorkThread> {
        let path = self.thread_path(id);
        if !path.exists() {
            return Err(StoreError::ThreadNotFound(id.to_owned()));
        }
        read_json(path)
    }

    pub fn create_workspace(&self, thread_id: &str) -> Result<WorkspaceView> {
        let thread = self.show_thread(thread_id)?;
        let id = WorkspaceViewId::new();
        let files_path = self.workspace_files_path(id.as_str());
        if files_path.exists() {
            fs::remove_dir_all(&files_path)?;
        }
        fs::create_dir_all(&files_path)?;
        self.restore_snapshot_to_path(thread.base_snapshot.as_str(), &files_path)?;

        let workspace = WorkspaceView {
            id,
            thread_id: thread.id.clone(),
            base_snapshot: thread.base_snapshot.clone(),
            materialized_path: files_path.to_string_lossy().to_string(),
            latest_snapshot: None,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.workspace_path(workspace.id.as_str()), &workspace)?;
        self.append_event(
            RepositoryEventKind::WorkspaceCreated,
            Some(workspace.id.to_string()),
        )?;
        Ok(workspace)
    }

    pub fn show_workspace(&self, id: &str) -> Result<WorkspaceView> {
        let path = self.workspace_path(id);
        if !path.exists() {
            return Err(StoreError::WorkspaceNotFound(id.to_owned()));
        }
        read_json(path)
    }

    pub fn workspace_changed_paths(&self, id: &str) -> Result<Option<Vec<ChangedPath>>> {
        let workspace = self.show_workspace(id)?;
        self.overlay_changed_paths(&workspace)
    }

    pub fn workspace_diff(&self, id: &str) -> Result<Vec<ChangedPath>> {
        let workspace = self.show_workspace(id)?;
        let base_files = self.snapshot_file_map(&workspace.base_snapshot)?;
        let workspace_files = workspace_file_map(Path::new(&workspace.materialized_path))?;
        Ok(diff_file_maps(&base_files, &workspace_files))
    }

    pub fn workspace_file_effects(&self, id: &str) -> Result<Vec<FileEffect>> {
        let changed_paths = self.workspace_diff(id)?;
        let config = self.repository_config()?;
        Ok(propose_file_effect_set(&changed_paths, &config)?.effects)
    }

    pub fn workspace_diff_patch(&self, id: &str) -> Result<String> {
        let workspace = self.show_workspace(id)?;
        let workspace_root = Path::new(&workspace.materialized_path);
        let base_files = self.snapshot_file_map(&workspace.base_snapshot)?;
        let workspace_files = workspace_file_map(workspace_root)?;
        let changed_paths = diff_file_maps(&base_files, &workspace_files);
        let mut patch = String::new();

        for changed in changed_paths {
            let old_content = base_files
                .get(&changed.path)
                .map(|object| self.read_object(object))
                .transpose()?;
            let new_content = match changed.status {
                ChangeStatus::Deleted => None,
                ChangeStatus::Added | ChangeStatus::Modified => {
                    Some(fs::read(workspace_root.join(&changed.path))?)
                }
            };
            patch.push_str(&render_unified_file_patch(
                &changed.path,
                &changed.status,
                old_content.as_deref().unwrap_or_default(),
                new_content.as_deref().unwrap_or_default(),
            ));
        }

        Ok(patch)
    }

    pub fn workspace_snapshot(&self, id: &str, message: Option<String>) -> Result<WorkspaceView> {
        let mut workspace = self.show_workspace(id)?;
        let snapshot =
            self.create_snapshot_from_path(&workspace.materialized_path, message, false)?;
        let overlay = self.build_workspace_overlay(&workspace, &snapshot)?;
        workspace.latest_snapshot = Some(snapshot.id.clone());
        self.write_workspace_overlay(&overlay)?;
        write_json_pretty(self.workspace_path(id), &workspace)?;
        Ok(workspace)
    }

    pub fn workspace_overlay(&self, id: &str) -> Result<Option<WorkspaceOverlay>> {
        let path = self.workspace_overlay_path(id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(read_json(path)?))
    }

    pub fn attach_evidence(
        &self,
        thread_id: &str,
        command: String,
        exit_code: i32,
        summary: String,
        artifact_path: Option<String>,
    ) -> Result<EvidenceRecord> {
        self.attach_command_evidence(
            thread_id,
            CommandEvidenceInput {
                command,
                command_event_id: None,
                command_label: None,
                command_file: None,
                cwd: None,
                exit_code,
                summary,
                artifact_path,
                stdout_path: None,
                stderr_path: None,
            },
        )
    }

    pub fn attach_command_evidence(
        &self,
        thread_id: &str,
        input: CommandEvidenceInput,
    ) -> Result<EvidenceRecord> {
        let thread = self.show_thread(thread_id)?;
        if input.summary.trim().is_empty() {
            return Err(StoreError::EmptyEvidenceSummary);
        }
        if input.command.trim().is_empty() {
            return Err(StoreError::EmptyEvidenceCommand);
        }

        let evidence = EvidenceRecord {
            id: EvidenceRecordId::new(),
            thread_id: thread.id,
            command_event_id: input.command_event_id,
            command: input.command,
            command_label: input.command_label,
            command_file: input.command_file,
            cwd: input.cwd,
            exit_code: input.exit_code,
            summary: input.summary,
            artifact_path: input.artifact_path,
            stdout_path: input.stdout_path,
            stderr_path: input.stderr_path,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.evidence_path(evidence.id.as_str()), &evidence)?;
        self.append_event(
            RepositoryEventKind::EvidenceAttached,
            Some(evidence.id.to_string()),
        )?;
        Ok(evidence)
    }

    pub fn run_command(&self, input: CommandRunInput) -> Result<CommandRunResult> {
        let workspace = self.show_workspace(&input.workspace_id)?;
        let thread = self.show_thread(workspace.thread_id.as_str())?;
        let command_label = input.command_label.trim().to_owned();
        if command_label.is_empty() {
            return Err(StoreError::EmptyCommandLabel);
        }
        if input.summary.trim().is_empty() {
            return Err(StoreError::EmptyEvidenceSummary);
        }

        let command_file = input.command_file.clone();
        let argv = if let Some(path) = &command_file {
            if !input.argv.is_empty() {
                return Err(StoreError::EmptyCommandArgv);
            }
            vec!["sh".to_owned(), path.clone()]
        } else {
            if input.argv.is_empty() || input.argv[0].trim().is_empty() {
                return Err(StoreError::EmptyCommandArgv);
            }
            input.argv.clone()
        };
        let command_policy_class = classify_command_policy(CommandPolicyInput {
            argv: input.argv.clone(),
            command_file: command_file.clone(),
        })?
        .policy_class;
        let command_policy_override_reason =
            command_policy_override_reason(&command_policy_class, &input)?;

        let command_event_id = CommandEventId::new();
        let projection_setup_started = Instant::now();
        let resolved_projection = self.resolve_workspace_projection(
            &workspace,
            &input.projection,
            input.mount_root.as_deref(),
            command_event_id.as_str(),
        )?;
        let projection_setup_ms = elapsed_ms(projection_setup_started);
        let projection = &resolved_projection.projection;
        debug_assert!(projection.capabilities.readable);
        debug_assert!(projection.capabilities.writable);
        debug_assert!(projection.capabilities.file_effects);
        let before_files = workspace_file_map(&projection.root_path)?;
        let projection_stats = workspace_file_stats(&projection.root_path)?;
        let cwd = self.resolve_projection_cwd(projection, input.cwd.as_deref())?;
        let cwd_display = cwd.to_string_lossy().to_string();
        let projection_root = projection.root_path.to_string_lossy().to_string();
        let timeout = Duration::from_secs(
            input
                .timeout_seconds
                .unwrap_or(DEFAULT_COMMAND_TIMEOUT_SECONDS),
        );
        let artifact_dir = self.command_artifact_dir(command_event_id.as_str());
        fs::create_dir_all(&artifact_dir)?;
        let stdout_path = artifact_dir.join("stdout.txt");
        let stderr_path = artifact_dir.join("stderr.txt");
        let started_at = now_rfc3339()?;
        let agent_session_id = self
            .latest_active_workspace_session(workspace.id.as_str())?
            .map(|session| session.id);

        let mut command_event = CommandEvent {
            id: command_event_id.clone(),
            workspace_id: projection.workspace_id.clone(),
            thread_id: projection.thread_id.clone(),
            agent_session_id,
            command_label: command_label.clone(),
            argv: argv.clone(),
            command_file: command_file.clone(),
            cwd: cwd_display.clone(),
            exit_code: None,
            timed_out: false,
            duration_ms: 0,
            summary: input.summary.clone(),
            artifact_path: input.artifact_path.clone(),
            stdout_path: Some(stdout_path.to_string_lossy().to_string()),
            stderr_path: Some(stderr_path.to_string_lossy().to_string()),
            projection_kind: Some(projection.kind.clone()),
            projection_root: Some(projection_root),
            projection_capabilities: Some(projection.capabilities.clone()),
            projection_fallback_reason: projection.fallback_reason.clone(),
            command_executor: None,
            command_policy_class: Some(command_policy_class.clone()),
            command_policy_override_reason: command_policy_override_reason.clone(),
            runtime_metrics: Some(CommandRuntimeMetrics {
                projection_setup_ms,
                command_ms: 0,
                reconcile_ms: 0,
                cleanup_ms: 0,
                projection_files: projection_stats.file_count,
                projection_bytes: projection_stats.byte_count,
            }),
            file_effects: Vec::new(),
            started_at,
            finished_at: None,
        };
        write_json_pretty(
            self.command_event_path(command_event.id.as_str()),
            &command_event,
        )?;
        self.append_event(
            RepositoryEventKind::CommandStarted,
            Some(command_event.id.to_string()),
        )?;

        let started = Instant::now();
        let execution = execute_command(&argv, &cwd, timeout)?;
        command_event.exit_code = Some(execution.exit_code);
        command_event.timed_out = execution.timed_out;
        command_event.duration_ms = execution.duration_ms.unwrap_or_else(|| elapsed_ms(started));
        command_event.command_executor = Some(execution.executor);
        command_event.finished_at = Some(now_rfc3339()?);
        let reconcile_started = Instant::now();
        let after_files = workspace_file_map(&projection.root_path)?;
        command_event.file_effects = diff_file_maps(&before_files, &after_files);
        if let Some(mounted_workspace) = &resolved_projection.mounted_workspace {
            mounted_workspace.persist_to_path(Path::new(&workspace.materialized_path))?;
            let mounted_effects = mounted_workspace.changed_paths();
            if !mounted_effects.is_empty() {
                command_event.file_effects = mounted_effects
                    .into_iter()
                    .map(|effect| ChangedPath {
                        path: effect.path,
                        status: match effect.status {
                            anvics_vfs::VfsFileEffectStatus::Added => ChangeStatus::Added,
                            anvics_vfs::VfsFileEffectStatus::Modified => ChangeStatus::Modified,
                            anvics_vfs::VfsFileEffectStatus::Deleted => ChangeStatus::Deleted,
                        },
                    })
                    .collect();
            }
        }
        let reconcile_ms = elapsed_ms(reconcile_started);
        fs::write(&stdout_path, execution.stdout)?;
        fs::write(&stderr_path, execution.stderr)?;

        let mount_cleanup_path = resolved_projection
            .mounted_workspace
            .as_ref()
            .map(|workspace| workspace.mount_path().to_path_buf());
        drop(resolved_projection);
        let cleanup_started = Instant::now();
        if let Some(path) = mount_cleanup_path {
            remove_empty_mount_dir(&path)?;
        }
        let cleanup_ms = elapsed_ms(cleanup_started);

        if let Some(metrics) = &mut command_event.runtime_metrics {
            metrics.command_ms = command_event.duration_ms;
            metrics.reconcile_ms = reconcile_ms;
            metrics.cleanup_ms = cleanup_ms;
        }

        write_json_pretty(
            self.command_event_path(command_event.id.as_str()),
            &command_event,
        )?;
        self.append_event(
            RepositoryEventKind::CommandFinished,
            Some(command_event.id.to_string()),
        )?;

        let evidence = self.attach_command_evidence(
            thread.id.as_str(),
            CommandEvidenceInput {
                command: shell_join(&argv),
                command_event_id: Some(command_event.id.clone()),
                command_label: Some(command_label),
                command_file,
                cwd: Some(cwd_display),
                exit_code: execution.exit_code,
                summary: input.summary,
                artifact_path: input.artifact_path,
                stdout_path: command_event.stdout_path.clone(),
                stderr_path: command_event.stderr_path.clone(),
            },
        )?;

        Ok(CommandRunResult {
            command_event,
            evidence,
        })
    }

    pub fn show_command_event(&self, id: &str) -> Result<CommandEvent> {
        let path = self.command_event_path(id);
        if !path.exists() {
            return Err(StoreError::CommandEventNotFound(id.to_owned()));
        }
        read_json(path)
    }

    pub fn create_review(&self, thread_id: &str) -> Result<ReviewProjection> {
        let thread = self.show_thread(thread_id)?;
        let workspace = self
            .latest_thread_workspace(&thread.id)?
            .ok_or_else(|| StoreError::MissingWorkspaceSnapshot(thread.id.to_string()))?;
        let final_snapshot = workspace
            .latest_snapshot
            .clone()
            .ok_or_else(|| StoreError::MissingWorkspaceSnapshot(thread.id.to_string()))?;
        let changed_paths = if let Some(paths) = self.overlay_changed_paths(&workspace)? {
            paths
        } else {
            self.diff_snapshots(&thread.base_snapshot, &final_snapshot)?
        };
        let evidence = self.thread_evidence(&thread.id)?;
        let overlap_notes = self.overlap_notes(&thread, &changed_paths)?;
        let repository_config = self.repository_config()?;
        let file_effect_set = propose_file_effect_set(&changed_paths, &repository_config)?;
        let file_effects = file_effect_set.effects.clone();
        let change_units = propose_change_units(&file_effect_set);

        let review = ReviewProjection {
            id: ReviewProjectionId::new(),
            thread_id: thread.id.clone(),
            base_snapshot: thread.base_snapshot.clone(),
            final_snapshot,
            changed_paths,
            file_effects,
            change_units,
            overlap_notes,
            evidence,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.review_path(review.id.as_str()), &review)?;
        fs::write(
            self.review_markdown_path(review.id.as_str()),
            render_review(&self.root, &review, &thread, &[], &[]),
        )?;
        self.append_event(
            RepositoryEventKind::ReviewCreated,
            Some(review.id.to_string()),
        )?;
        Ok(review)
    }

    pub fn show_review(&self, id: &str) -> Result<ReviewProjection> {
        let path = self.review_path(id);
        if !path.exists() {
            return Err(StoreError::ReviewNotFound(id.to_owned()));
        }
        read_json(path)
    }

    pub fn scan_review_risks(&self, review_id: &str) -> Result<RiskScan> {
        let review = self.show_review(review_id)?;
        let thread = self.show_thread(review.thread_id.as_str())?;
        let scan_id = RiskScanId::new();
        let mut findings = Vec::new();

        for changed in &review.changed_paths {
            if changed.status == ChangeStatus::Deleted {
                continue;
            }
            if let Some(bytes) =
                self.file_bytes_at_snapshot(&review.final_snapshot, &changed.path)?
            {
                let base_bytes =
                    self.file_bytes_at_snapshot(&review.base_snapshot, &changed.path)?;
                let final_text = String::from_utf8_lossy(&bytes);
                let introduced_lines = match (&changed.status, base_bytes) {
                    (ChangeStatus::Added, _) => all_numbered_lines(&final_text),
                    (ChangeStatus::Modified, Some(base_bytes)) => {
                        let base_text = String::from_utf8_lossy(&base_bytes);
                        introduced_numbered_lines(&base_text, &final_text)
                    }
                    (ChangeStatus::Modified, None) => all_numbered_lines(&final_text),
                    (ChangeStatus::Deleted, _) => Vec::new(),
                };
                collect_secret_findings_for_lines(
                    &mut findings,
                    &scan_id,
                    &review.id,
                    RiskTargetKind::SourceFile,
                    &changed.path,
                    introduced_lines,
                );
            }
        }

        for evidence in &review.evidence {
            for (kind, path) in [
                (
                    RiskTargetKind::CommandStdout,
                    evidence.stdout_path.as_deref(),
                ),
                (
                    RiskTargetKind::CommandStderr,
                    evidence.stderr_path.as_deref(),
                ),
                (
                    RiskTargetKind::EvidenceArtifact,
                    evidence.artifact_path.as_deref(),
                ),
                (
                    RiskTargetKind::CommandFile,
                    evidence.command_file.as_deref(),
                ),
            ] {
                let Some(path) = path.filter(|path| !path.trim().is_empty()) else {
                    continue;
                };
                if let Ok(bytes) = self.read_risk_target_bytes(path) {
                    let text = String::from_utf8_lossy(&bytes);
                    collect_secret_findings(&mut findings, &scan_id, &review.id, kind, path, &text);
                }
            }
        }

        let scan = RiskScan {
            id: scan_id,
            review_id: review.id.clone(),
            findings,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.risk_scan_path(scan.id.as_str()), &scan)?;
        self.append_event(
            RepositoryEventKind::RiskScanCreated,
            Some(scan.id.to_string()),
        )?;
        if !scan.findings.is_empty() {
            self.append_event(
                RepositoryEventKind::SecretRiskDetected,
                Some(scan.id.to_string()),
            )?;
        }
        self.write_review_markdown_with_risks(&review, &thread)?;
        Ok(scan)
    }

    pub fn list_review_risk_scans(&self, review_id: &str) -> Result<Vec<RiskScan>> {
        let review = self.show_review(review_id)?;
        let mut scans: Vec<RiskScan> = read_json_dir(self.anvics_dir.join("risks"))?;
        scans.retain(|scan| scan.review_id == review.id);
        scans.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(scans)
    }

    pub fn list_review_risk_findings(&self, review_id: &str) -> Result<Vec<RiskFinding>> {
        let mut findings = Vec::new();
        for scan in self.list_review_risk_scans(review_id)? {
            findings.extend(scan.findings);
        }
        findings.sort_by(|left, right| {
            left.target_path
                .cmp(&right.target_path)
                .then_with(|| left.line.cmp(&right.line))
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(findings)
    }

    pub fn show_risk_finding(&self, id: &str) -> Result<RiskFinding> {
        for scan in read_json_dir::<RiskScan>(self.anvics_dir.join("risks"))? {
            if let Some(finding) = scan
                .findings
                .into_iter()
                .find(|finding| finding.id.as_str() == id)
            {
                return Ok(finding);
            }
        }
        Err(StoreError::RiskFindingNotFound(id.to_owned()))
    }

    pub fn create_publication(
        &self,
        thread_id: &str,
        review_id: &str,
    ) -> Result<NativePublication> {
        self.create_publication_with_options(thread_id, review_id, PublicationOptions::default())
    }

    pub fn create_publication_with_options(
        &self,
        thread_id: &str,
        review_id: &str,
        options: PublicationOptions,
    ) -> Result<NativePublication> {
        let mut thread = self.show_thread(thread_id)?;
        let review = self.show_review(review_id)?;
        if review.thread_id != thread.id {
            return Err(StoreError::ReviewThreadMismatch {
                review_id: review_id.to_owned(),
                thread_id: thread_id.to_owned(),
            });
        }
        let scan = self.risk_scan_for_publication(review_id)?;
        if !scan.findings.is_empty() {
            if options.allow_secret_risk {
                self.record_policy_override(review_id, options.override_reason)?;
            } else {
                return Err(StoreError::PublicationBlockedSecretRisk {
                    finding_count: scan.findings.len(),
                });
            }
        }

        let publication = NativePublication {
            id: NativePublicationId::new(),
            thread_id: thread.id.clone(),
            accepted_snapshot: review.final_snapshot,
            review_id: review.id,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.publication_path(publication.id.as_str()), &publication)?;

        thread.status = WorkThreadStatus::Published;
        write_json_pretty(self.thread_path(thread.id.as_str()), &thread)?;
        self.append_event(
            RepositoryEventKind::PublicationCreated,
            Some(publication.id.to_string()),
        )?;

        Ok(publication)
    }

    pub fn show_publication(&self, id: &str) -> Result<NativePublication> {
        let path = self.publication_path(id);
        if !path.exists() {
            return Err(StoreError::PublicationNotFound(id.to_owned()));
        }
        read_json(path)
    }

    pub fn prepare_agent(&self, title: String, task: String) -> Result<AgentPreparation> {
        let thread = self.create_thread(title, task)?;
        let workspace = self.create_workspace(thread.id.as_str())?;
        let packet_path = self.agent_packet_path(thread.id.as_str());
        if let Some(parent) = packet_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &packet_path,
            render_agent_packet(&self.root, &thread, &workspace),
        )?;

        Ok(AgentPreparation {
            thread,
            workspace,
            packet_path: packet_path.to_string_lossy().to_string(),
        })
    }

    pub fn agent_launch_prompt(
        &self,
        workspace_id: &str,
        tool: AgentLaunchTool,
    ) -> Result<AgentLaunchPrompt> {
        let workspace = self.show_workspace(workspace_id)?;
        let thread = self.show_thread(workspace.thread_id.as_str())?;
        let packet_path = self.agent_packet_file_path(thread.id.as_str())?;
        let skill_path =
            Path::new(&workspace.materialized_path).join("skills/anvics-skill/SKILL.md");
        let skill_path = skill_path
            .exists()
            .then(|| skill_path.to_string_lossy().to_string());
        let prompt = render_agent_launch_prompt(
            &self.root,
            &thread,
            &workspace,
            &packet_path,
            skill_path.as_deref(),
        );
        let command = match tool {
            AgentLaunchTool::Generic => None,
            AgentLaunchTool::Codex => Some(render_codex_launch_command(&workspace, &prompt)),
        };

        Ok(AgentLaunchPrompt {
            tool,
            thread_id: thread.id,
            workspace_id: workspace.id,
            repo_path: self.root.to_string_lossy().to_string(),
            workspace_path: workspace.materialized_path,
            packet_path: packet_path.to_string_lossy().to_string(),
            skill_path,
            prompt,
            command,
        })
    }

    pub fn finish_agent(
        &self,
        workspace_id: &str,
        command: String,
        exit_code: i32,
        summary: String,
        artifact_path: Option<String>,
    ) -> Result<AgentFinish> {
        self.finish_agent_with_evidence(
            workspace_id,
            CommandEvidenceInput {
                command,
                command_event_id: None,
                command_label: None,
                command_file: None,
                cwd: None,
                exit_code,
                summary,
                artifact_path,
                stdout_path: None,
                stderr_path: None,
            },
        )
    }

    pub fn finish_agent_with_evidence(
        &self,
        workspace_id: &str,
        evidence_input: CommandEvidenceInput,
    ) -> Result<AgentFinish> {
        let workspace = self.show_workspace(workspace_id)?;
        let evidence =
            self.attach_command_evidence(workspace.thread_id.as_str(), evidence_input)?;
        self.finish_agent_with_existing_evidence(workspace_id, evidence)
    }

    fn finish_agent_with_existing_evidence(
        &self,
        workspace_id: &str,
        evidence: EvidenceRecord,
    ) -> Result<AgentFinish> {
        let workspace =
            self.workspace_snapshot(workspace_id, Some("agent finish result".to_owned()))?;
        let review = self.create_review(workspace.thread_id.as_str())?;
        let review_markdown_path = self
            .review_markdown_path(review.id.as_str())
            .to_string_lossy()
            .to_string();
        self.finish_workspace_sessions(workspace_id)?;

        Ok(AgentFinish {
            evidence,
            workspace,
            review,
            review_markdown_path,
        })
    }

    pub fn accept_agent(
        &self,
        workspace_id: &str,
        command: String,
        exit_code: i32,
        summary: String,
        artifact_path: Option<String>,
        output_path: Option<PathBuf>,
    ) -> Result<AgentAcceptance> {
        self.accept_agent_with_evidence(
            workspace_id,
            CommandEvidenceInput {
                command,
                command_event_id: None,
                command_label: None,
                command_file: None,
                cwd: None,
                exit_code,
                summary,
                artifact_path,
                stdout_path: None,
                stderr_path: None,
            },
            output_path,
        )
    }

    pub fn accept_agent_with_evidence(
        &self,
        workspace_id: &str,
        evidence_input: CommandEvidenceInput,
        output_path: Option<PathBuf>,
    ) -> Result<AgentAcceptance> {
        self.accept_agent_with_evidence_and_options(
            workspace_id,
            evidence_input,
            output_path,
            PublicationOptions::default(),
        )
    }

    pub fn accept_agent_with_evidence_and_options(
        &self,
        workspace_id: &str,
        evidence_input: CommandEvidenceInput,
        output_path: Option<PathBuf>,
        options: PublicationOptions,
    ) -> Result<AgentAcceptance> {
        let finish = self.finish_agent_with_evidence(workspace_id, evidence_input)?;
        let publication = self.create_publication_with_options(
            finish.workspace.thread_id.as_str(),
            finish.review.id.as_str(),
            options,
        )?;
        let output = output_path.unwrap_or_else(|| self.root.join("accepted.patch"));
        let patch_path = self.export_legacy_git_patch(publication.id.as_str(), output)?;

        Ok(AgentAcceptance {
            evidence: finish.evidence,
            workspace: finish.workspace,
            review: finish.review,
            review_markdown_path: finish.review_markdown_path,
            publication,
            patch_path: patch_path.to_string_lossy().to_string(),
        })
    }

    pub fn accept_agent_with_command_run(
        &self,
        input: CommandRunInput,
        output_path: Option<PathBuf>,
    ) -> Result<AgentAcceptance> {
        self.accept_agent_with_command_run_and_options(
            input,
            output_path,
            PublicationOptions::default(),
        )
    }

    pub fn accept_agent_with_command_run_and_options(
        &self,
        input: CommandRunInput,
        output_path: Option<PathBuf>,
        options: PublicationOptions,
    ) -> Result<AgentAcceptance> {
        let workspace_id = input.workspace_id.clone();
        let command = self.run_command(input)?;
        let exit_code = command.command_event.exit_code.unwrap_or(-1);
        if exit_code != 0 {
            return Err(StoreError::CommandFailed {
                id: command.command_event.id.to_string(),
                exit_code,
            });
        }
        let finish = self.finish_agent_with_existing_evidence(&workspace_id, command.evidence)?;
        let publication = self.create_publication_with_options(
            finish.workspace.thread_id.as_str(),
            finish.review.id.as_str(),
            options,
        )?;
        let output = output_path.unwrap_or_else(|| self.root.join("accepted.patch"));
        let patch_path = self.export_legacy_git_patch(publication.id.as_str(), output)?;

        Ok(AgentAcceptance {
            evidence: finish.evidence,
            workspace: finish.workspace,
            review: finish.review,
            review_markdown_path: finish.review_markdown_path,
            publication,
            patch_path: patch_path.to_string_lossy().to_string(),
        })
    }

    pub fn agent_packet_file_path(&self, thread_id: &str) -> Result<PathBuf> {
        self.show_thread(thread_id)?;
        let path = self.agent_packet_path(thread_id);
        if !path.exists() {
            return Err(StoreError::AgentPacketNotFound(thread_id.to_owned()));
        }
        Ok(path)
    }

    pub fn agent_status(&self, thread_id: &str) -> Result<AgentStatus> {
        let thread = self.show_thread(thread_id)?;
        let workspaces: Vec<WorkspaceView> = self
            .list_workspaces()?
            .into_iter()
            .filter(|workspace| workspace.thread_id == thread.id)
            .collect();
        let evidence_count = self.thread_evidence(&thread.id)?.len();
        let review_ids = self
            .thread_reviews(&thread.id)?
            .into_iter()
            .map(|review| review.id)
            .collect();
        let publication_ids = self
            .thread_publications(&thread.id)?
            .into_iter()
            .map(|publication| publication.id)
            .collect();

        Ok(AgentStatus {
            thread,
            workspaces,
            evidence_count,
            review_ids,
            publication_ids,
        })
    }

    pub fn review_markdown(&self, id: &str) -> Result<String> {
        let path = self.review_markdown_path(id);
        if !path.exists() {
            return Err(StoreError::ReviewNotFound(id.to_owned()));
        }
        Ok(fs::read_to_string(path)?)
    }

    pub fn review_markdown_file_path(&self, id: &str) -> Result<PathBuf> {
        let path = self.review_markdown_path(id);
        if !path.exists() {
            return Err(StoreError::ReviewNotFound(id.to_owned()));
        }
        Ok(path)
    }

    pub fn export_legacy_git_patch(
        &self,
        publication_id: &str,
        output: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let publication = self.show_publication(publication_id)?;
        let review = self.show_review(publication.review_id.as_str())?;
        let thread = self.show_thread(publication.thread_id.as_str())?;
        let patch = self.render_legacy_git_patch(&publication, &review, &thread)?;
        let output = output.as_ref();
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output, patch)?;
        self.append_event(
            RepositoryEventKind::LegacyPatchExported,
            Some(publication.id.to_string()),
        )?;
        Ok(output.to_path_buf())
    }

    pub fn events_since(&self, sequence: u64) -> Result<Vec<RepositoryEvent>> {
        let mut events: Vec<RepositoryEvent> = read_json_dir(self.anvics_dir.join("events"))?;
        events.retain(|event| event.sequence > sequence);
        events.sort_by(|left, right| {
            left.sequence
                .cmp(&right.sequence)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(events)
    }

    pub fn enter_agent_session(
        &self,
        workspace_id: &str,
        agent_name: String,
    ) -> Result<CoordinationStatus> {
        let agent_name = agent_name.trim().to_owned();
        if agent_name.is_empty() {
            return Err(StoreError::EmptyAgentName);
        }
        let workspace = self.show_workspace(workspace_id)?;
        let now = now_rfc3339()?;

        let mut matching: Vec<AgentSession> = self
            .list_sessions()?
            .into_iter()
            .filter(|session| {
                session.workspace_id == workspace.id
                    && session.agent_name == agent_name
                    && session.status == AgentSessionStatus::Active
            })
            .collect();
        matching.sort_by(|left, right| {
            left.entered_at
                .cmp(&right.entered_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });

        let session = if let Some(mut session) = matching.pop() {
            session.last_seen_at = now;
            write_json_pretty(self.session_path(session.id.as_str()), &session)?;
            self.append_event(
                RepositoryEventKind::AgentSessionSeen,
                Some(session.id.to_string()),
            )?;
            session
        } else {
            let session = AgentSession {
                id: AgentSessionId::new(),
                thread_id: workspace.thread_id.clone(),
                workspace_id: workspace.id,
                agent_name,
                status: AgentSessionStatus::Active,
                entered_at: now.clone(),
                last_seen_at: now,
                finished_at: None,
            };
            write_json_pretty(self.session_path(session.id.as_str()), &session)?;
            self.append_event(
                RepositoryEventKind::AgentSessionEntered,
                Some(session.id.to_string()),
            )?;
            session
        };

        self.coordination_status_with_session(workspace_id, Some(session))
    }

    pub fn leave_agent_session(&self, session_id: &str) -> Result<AgentSession> {
        let mut session = self.show_session(session_id)?;
        self.finish_session(&mut session)
    }

    pub fn list_agent_sessions(
        &self,
        thread_id: Option<&str>,
        workspace_id: Option<&str>,
    ) -> Result<Vec<AgentSession>> {
        let mut sessions = self.list_sessions()?;
        if let Some(thread_id) = thread_id {
            let thread = self.show_thread(thread_id)?;
            sessions.retain(|session| session.thread_id == thread.id);
        }
        if let Some(workspace_id) = workspace_id {
            let workspace = self.show_workspace(workspace_id)?;
            sessions.retain(|session| session.workspace_id == workspace.id);
        }
        Ok(sessions)
    }

    pub fn coordination_status(&self, workspace_id: &str) -> Result<CoordinationStatus> {
        let current_session = self.latest_active_workspace_session(workspace_id)?;
        if let Some(mut session) = current_session {
            session.last_seen_at = now_rfc3339()?;
            write_json_pretty(self.session_path(session.id.as_str()), &session)?;
            self.append_event(
                RepositoryEventKind::AgentSessionSeen,
                Some(session.id.to_string()),
            )?;
            self.coordination_status_with_session(workspace_id, Some(session))
        } else {
            self.coordination_status_with_session(workspace_id, None)
        }
    }

    pub fn restore_snapshot_to_path(
        &self,
        snapshot_id: &str,
        target: impl AsRef<Path>,
    ) -> Result<()> {
        let snapshot = self.show_snapshot(snapshot_id)?;
        let target = target.as_ref();
        fs::create_dir_all(target)?;
        self.restore_tree(&snapshot.root_tree, target)
    }

    pub fn restore_workspace_from_overlay(
        &self,
        workspace_id: &str,
        target: impl AsRef<Path>,
    ) -> Result<()> {
        let workspace = self.show_workspace(workspace_id)?;
        let target = target.as_ref();
        if target.exists() {
            fs::remove_dir_all(target)?;
        }
        fs::create_dir_all(target)?;

        if let Some(overlay) = self.workspace_overlay(workspace_id)? {
            self.restore_snapshot_to_path(overlay.base_snapshot.as_str(), target)?;
            self.apply_overlay_to_path(&overlay, target)?;
        } else if let Some(snapshot) = workspace.latest_snapshot {
            self.restore_snapshot_to_path(snapshot.as_str(), target)?;
        } else {
            self.restore_snapshot_to_path(workspace.base_snapshot.as_str(), target)?;
        }

        Ok(())
    }

    pub fn diff_snapshots(
        &self,
        base: &SourceSnapshotId,
        final_snapshot: &SourceSnapshotId,
    ) -> Result<Vec<ChangedPath>> {
        let base_snapshot = self.show_snapshot(base.as_str())?;
        let final_snapshot = self.show_snapshot(final_snapshot.as_str())?;
        let base_files = self.flatten_tree(&base_snapshot.root_tree, "")?;
        let final_files = self.flatten_tree(&final_snapshot.root_tree, "")?;
        Ok(diff_file_maps(&base_files, &final_files))
    }

    pub fn object_exists(&self, object: &ObjectId) -> bool {
        self.object_path(object).exists()
    }

    fn store_object(&self, bytes: &[u8]) -> Result<ObjectId> {
        let object = ObjectId::from_bytes(bytes);
        let path = self.object_path(&object);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, bytes)?;
        }
        Ok(object)
    }

    fn read_object(&self, object: &ObjectId) -> Result<Vec<u8>> {
        Ok(fs::read(self.object_path(object))?)
    }

    fn store_tree(&self, node: &TreeNode) -> Result<ObjectId> {
        let mut entries = Vec::new();

        for (name, child) in &node.dirs {
            entries.push(TreeEntry {
                name: name.clone(),
                kind: TreeEntryKind::Directory,
                object: self.store_tree(child)?,
                size: None,
            });
        }

        for (name, object, size) in &node.files {
            entries.push(TreeEntry {
                name: name.clone(),
                kind: TreeEntryKind::File,
                object: object.clone(),
                size: Some(*size),
            });
        }

        entries.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| tree_kind_order(&left.kind).cmp(&tree_kind_order(&right.kind)))
        });

        let tree = Tree { entries };
        let bytes = serde_json::to_vec(&tree)?;
        self.store_object(&bytes)
    }

    fn object_path(&self, object: &ObjectId) -> PathBuf {
        let hex = object.as_str();
        self.anvics_dir
            .join("objects/blake3")
            .join(&hex[..2])
            .join(hex)
    }

    fn snapshot_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("snapshots").join(format!("{id}.json"))
    }

    fn thread_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("threads").join(format!("{id}.json"))
    }

    fn workspace_path(&self, id: &str) -> PathBuf {
        self.anvics_dir
            .join("workspaces")
            .join(format!("{id}.json"))
    }

    fn workspace_files_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("workspaces").join(id).join("files")
    }

    fn workspace_overlay_path(&self, id: &str) -> PathBuf {
        self.anvics_dir
            .join("workspaces")
            .join(id)
            .join("overlay.json")
    }

    fn evidence_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("evidence").join(format!("{id}.json"))
    }

    fn review_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("reviews").join(format!("{id}.json"))
    }

    fn review_markdown_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("reviews").join(format!("{id}.md"))
    }

    fn publication_path(&self, id: &str) -> PathBuf {
        self.anvics_dir
            .join("publications")
            .join(format!("{id}.json"))
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("sessions").join(format!("{id}.json"))
    }

    fn command_event_path(&self, id: &str) -> PathBuf {
        self.anvics_dir
            .join("command-events")
            .join(format!("{id}.json"))
    }

    fn command_artifact_dir(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("artifacts").join("commands").join(id)
    }

    fn risk_scan_path(&self, id: &str) -> PathBuf {
        self.anvics_dir.join("risks").join(format!("{id}.json"))
    }

    fn policy_override_path(&self, id: &str) -> PathBuf {
        self.anvics_dir
            .join("policy-overrides")
            .join(format!("{id}.json"))
    }

    fn agent_packet_path(&self, thread_id: &str) -> PathBuf {
        self.anvics_dir
            .join("agent-packets")
            .join(format!("{thread_id}.md"))
    }

    fn append_event(
        &self,
        kind: RepositoryEventKind,
        subject_id: Option<String>,
    ) -> Result<RepositoryEvent> {
        append_event_at(&self.anvics_dir, kind, subject_id)
    }

    fn resolve_workspace_projection(
        &self,
        workspace: &WorkspaceView,
        request: &ProjectionRequest,
        mount_root: Option<&str>,
        command_event_id: &str,
    ) -> Result<ResolvedProjection> {
        match request {
            ProjectionRequest::MaterializedDir => self
                .resolve_materialized_projection(workspace, None)
                .map(|projection| ResolvedProjection {
                    projection,
                    mounted_workspace: None,
                }),
            ProjectionRequest::FuseMount => {
                self.resolve_fuse_projection(workspace, mount_root, command_event_id, false)
            }
            ProjectionRequest::Auto => {
                match self.resolve_fuse_projection(workspace, mount_root, command_event_id, true) {
                    Ok(projection) => Ok(projection),
                    Err(error) => self
                        .resolve_materialized_projection(workspace, Some(error.to_string()))
                        .map(|projection| ResolvedProjection {
                            projection,
                            mounted_workspace: None,
                        }),
                }
            }
        }
    }

    fn resolve_materialized_projection(
        &self,
        workspace: &WorkspaceView,
        fallback_reason: Option<String>,
    ) -> Result<WorkspaceProjection> {
        let root_path = PathBuf::from(&workspace.materialized_path).canonicalize()?;
        Ok(WorkspaceProjection {
            workspace_id: workspace.id.clone(),
            thread_id: workspace.thread_id.clone(),
            kind: ProjectionKind::MaterializedDir,
            root_path,
            capabilities: ProjectionCapabilities {
                readable: true,
                writable: true,
                file_effects: true,
            },
            fallback_reason,
        })
    }

    fn resolve_fuse_projection(
        &self,
        workspace: &WorkspaceView,
        mount_root: Option<&str>,
        command_event_id: &str,
        _allow_auto_fallback: bool,
    ) -> Result<ResolvedProjection> {
        let workspace_root = PathBuf::from(&workspace.materialized_path).canonicalize()?;
        let mount_base = match mount_root {
            Some(root) if !root.trim().is_empty() => PathBuf::from(root),
            _ => self.anvics_dir.join("mounts"),
        };
        fs::create_dir_all(&mount_base)?;
        let mount_path = mount_base.join(command_event_id);
        let mounted_workspace = anvics_vfs::mount_workspace(&workspace_root, &mount_path)
            .map_err(|error| StoreError::ProjectionUnavailable(error.to_string()))?;
        let root_path = mounted_workspace.mount_path().canonicalize()?;

        Ok(ResolvedProjection {
            projection: WorkspaceProjection {
                workspace_id: workspace.id.clone(),
                thread_id: workspace.thread_id.clone(),
                kind: ProjectionKind::FuseMount,
                root_path,
                capabilities: ProjectionCapabilities {
                    readable: true,
                    writable: true,
                    file_effects: true,
                },
                fallback_reason: None,
            },
            mounted_workspace: Some(mounted_workspace),
        })
    }

    fn resolve_projection_cwd(
        &self,
        projection: &WorkspaceProjection,
        cwd: Option<&str>,
    ) -> Result<PathBuf> {
        let workspace_root = &projection.root_path;
        let candidate = match cwd.filter(|cwd| !cwd.trim().is_empty()) {
            Some(cwd) => {
                let path = PathBuf::from(cwd);
                if path.is_absolute() {
                    path
                } else {
                    workspace_root.join(path)
                }
            }
            None => workspace_root.to_path_buf(),
        };
        let candidate = candidate.canonicalize()?;
        if !candidate.starts_with(workspace_root) {
            return Err(StoreError::InvalidCommandCwd(
                candidate.to_string_lossy().to_string(),
            ));
        }
        Ok(candidate)
    }

    fn file_bytes_at_snapshot(
        &self,
        snapshot_id: &SourceSnapshotId,
        path: &str,
    ) -> Result<Option<Vec<u8>>> {
        let files = self.snapshot_file_map(snapshot_id)?;
        files
            .get(path)
            .map(|object| self.read_object(object))
            .transpose()
    }

    fn write_review_markdown_with_risks(
        &self,
        review: &ReviewProjection,
        thread: &WorkThread,
    ) -> Result<()> {
        let scans = self.list_review_risk_scans(review.id.as_str())?;
        let overrides = self.list_review_policy_overrides(review.id.as_str())?;
        fs::write(
            self.review_markdown_path(review.id.as_str()),
            render_review(&self.root, review, thread, &scans, &overrides),
        )?;
        Ok(())
    }

    fn risk_scan_for_publication(&self, review_id: &str) -> Result<RiskScan> {
        let scans = self.list_review_risk_scans(review_id)?;
        match scans.into_iter().last() {
            Some(scan) => Ok(scan),
            None => self.scan_review_risks(review_id),
        }
    }

    fn list_review_policy_overrides(&self, review_id: &str) -> Result<Vec<PolicyOverride>> {
        let review = self.show_review(review_id)?;
        let mut overrides: Vec<PolicyOverride> =
            read_json_dir(self.anvics_dir.join("policy-overrides"))?;
        overrides.retain(|override_record| override_record.review_id == review.id);
        overrides.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(overrides)
    }

    fn read_risk_target_bytes(&self, path: &str) -> std::io::Result<Vec<u8>> {
        let path = Path::new(path);
        if path.is_absolute() {
            fs::read(path)
        } else {
            fs::read(self.root.join(path))
        }
    }

    fn record_policy_override(
        &self,
        review_id: &str,
        reason: Option<String>,
    ) -> Result<PolicyOverride> {
        let reason = reason.unwrap_or_default().trim().to_owned();
        if reason.is_empty() {
            return Err(StoreError::EmptyOverrideReason);
        }
        let review = self.show_review(review_id)?;
        let override_record = PolicyOverride {
            id: PolicyOverrideId::new(),
            review_id: review.id.clone(),
            reason,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(
            self.policy_override_path(override_record.id.as_str()),
            &override_record,
        )?;
        self.append_event(
            RepositoryEventKind::PolicyOverrideRecorded,
            Some(override_record.id.to_string()),
        )?;
        let thread = self.show_thread(review.thread_id.as_str())?;
        self.write_review_markdown_with_risks(&review, &thread)?;
        Ok(override_record)
    }

    fn restore_tree(&self, tree_id: &ObjectId, target: &Path) -> Result<()> {
        let tree: Tree = serde_json::from_slice(&self.read_object(tree_id)?)?;
        for entry in tree.entries {
            let path = target.join(&entry.name);
            match entry.kind {
                TreeEntryKind::Directory => {
                    fs::create_dir_all(&path)?;
                    self.restore_tree(&entry.object, &path)?;
                }
                TreeEntryKind::File => {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(path, self.read_object(&entry.object)?)?;
                }
            }
        }
        Ok(())
    }

    fn flatten_tree(&self, tree_id: &ObjectId, prefix: &str) -> Result<BTreeMap<String, ObjectId>> {
        let tree: Tree = serde_json::from_slice(&self.read_object(tree_id)?)?;
        let mut files = BTreeMap::new();

        for entry in tree.entries {
            let path = if prefix.is_empty() {
                entry.name
            } else {
                format!("{prefix}/{}", entry.name)
            };
            match entry.kind {
                TreeEntryKind::Directory => {
                    files.extend(self.flatten_tree(&entry.object, &path)?);
                }
                TreeEntryKind::File => {
                    files.insert(path, entry.object);
                }
            }
        }

        Ok(files)
    }

    fn build_workspace_overlay(
        &self,
        workspace: &WorkspaceView,
        snapshot: &SourceSnapshot,
    ) -> Result<WorkspaceOverlay> {
        let base_files = self.snapshot_file_map(&workspace.base_snapshot)?;
        let final_files = self.flatten_tree(&snapshot.root_tree, "")?;
        let entries = diff_file_maps(&base_files, &final_files)
            .into_iter()
            .map(|changed| {
                let object = final_files.get(&changed.path).cloned();
                let size = object
                    .as_ref()
                    .map(|object| self.read_object(object).map(|bytes| bytes.len() as u64))
                    .transpose()?;
                Ok(OverlayEntry {
                    path: changed.path,
                    status: changed.status,
                    object,
                    size,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(WorkspaceOverlay {
            workspace_id: workspace.id.clone(),
            base_snapshot: workspace.base_snapshot.clone(),
            snapshot: snapshot.id.clone(),
            entries,
            created_at: now_rfc3339()?,
        })
    }

    fn write_workspace_overlay(&self, overlay: &WorkspaceOverlay) -> Result<()> {
        let path = self.workspace_overlay_path(overlay.workspace_id.as_str());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_json_pretty(path, overlay)
    }

    fn apply_overlay_to_path(&self, overlay: &WorkspaceOverlay, target: &Path) -> Result<()> {
        for entry in &overlay.entries {
            let path = target.join(&entry.path);
            match entry.status {
                ChangeStatus::Added | ChangeStatus::Modified => {
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let Some(object) = &entry.object else {
                        continue;
                    };
                    fs::write(path, self.read_object(object)?)?;
                }
                ChangeStatus::Deleted => {
                    if path.exists() {
                        fs::remove_file(path)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn overlay_changed_paths(&self, workspace: &WorkspaceView) -> Result<Option<Vec<ChangedPath>>> {
        let Some(overlay) = self.workspace_overlay(workspace.id.as_str())? else {
            return Ok(None);
        };
        Ok(Some(
            overlay
                .entries
                .into_iter()
                .map(|entry| ChangedPath {
                    path: entry.path,
                    status: entry.status,
                })
                .collect(),
        ))
    }

    fn overlay_changed_path_names(&self, workspace: &WorkspaceView) -> Result<Option<Vec<String>>> {
        Ok(self
            .overlay_changed_paths(workspace)?
            .map(|paths| paths.into_iter().map(|path| path.path).collect::<Vec<_>>()))
    }

    fn snapshot_file_map(
        &self,
        snapshot_id: &SourceSnapshotId,
    ) -> Result<BTreeMap<String, ObjectId>> {
        let snapshot = self.show_snapshot(snapshot_id.as_str())?;
        self.flatten_tree(&snapshot.root_tree, "")
    }

    fn list_workspaces(&self) -> Result<Vec<WorkspaceView>> {
        let mut workspaces: Vec<WorkspaceView> = read_json_dir(self.anvics_dir.join("workspaces"))?;
        workspaces.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(workspaces)
    }

    fn show_session(&self, id: &str) -> Result<AgentSession> {
        let path = self.session_path(id);
        if !path.exists() {
            return Err(StoreError::AgentSessionNotFound(id.to_owned()));
        }
        read_json(path)
    }

    fn list_sessions(&self) -> Result<Vec<AgentSession>> {
        let mut sessions: Vec<AgentSession> = read_json_dir(self.anvics_dir.join("sessions"))?;
        sessions.sort_by(|left, right| {
            left.entered_at
                .cmp(&right.entered_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(sessions)
    }

    fn latest_active_workspace_session(&self, workspace_id: &str) -> Result<Option<AgentSession>> {
        let workspace = self.show_workspace(workspace_id)?;
        Ok(self.list_sessions()?.into_iter().rfind(|session| {
            session.workspace_id == workspace.id && session.status == AgentSessionStatus::Active
        }))
    }

    fn finish_session(&self, session: &mut AgentSession) -> Result<AgentSession> {
        if session.status == AgentSessionStatus::Finished {
            return Ok(session.clone());
        }
        let now = now_rfc3339()?;
        session.status = AgentSessionStatus::Finished;
        session.last_seen_at = now.clone();
        session.finished_at = Some(now);
        write_json_pretty(self.session_path(session.id.as_str()), session)?;
        self.append_event(
            RepositoryEventKind::AgentSessionFinished,
            Some(session.id.to_string()),
        )?;
        Ok(session.clone())
    }

    fn finish_workspace_sessions(&self, workspace_id: &str) -> Result<()> {
        let workspace = self.show_workspace(workspace_id)?;
        for mut session in self.list_sessions()?.into_iter().filter(|session| {
            session.workspace_id == workspace.id && session.status == AgentSessionStatus::Active
        }) {
            self.finish_session(&mut session)?;
        }
        Ok(())
    }

    fn coordination_status_with_session(
        &self,
        workspace_id: &str,
        current_session: Option<AgentSession>,
    ) -> Result<CoordinationStatus> {
        let workspace = self.show_workspace(workspace_id)?;
        let thread = self.show_thread(workspace.thread_id.as_str())?;
        let known_changed_paths = self
            .overlay_changed_path_names(&workspace)?
            .unwrap_or_default();
        let changed_set: BTreeSet<String> = known_changed_paths.iter().cloned().collect();
        let sessions = self.list_sessions()?;
        let mut related_work = Vec::new();

        for other_workspace in self.list_workspaces()? {
            if other_workspace.id == workspace.id
                || other_workspace.base_snapshot != workspace.base_snapshot
            {
                continue;
            }
            let other_thread = self.show_thread(other_workspace.thread_id.as_str())?;
            if other_thread.status != WorkThreadStatus::Active {
                continue;
            }
            let active_sessions: Vec<AgentSession> = sessions
                .iter()
                .filter(|session| {
                    session.workspace_id == other_workspace.id
                        && session.status == AgentSessionStatus::Active
                })
                .cloned()
                .collect();

            if active_sessions.is_empty() {
                related_work.push(self.related_work_for_workspace(
                    &other_workspace,
                    &other_thread,
                    None,
                    &changed_set,
                )?);
            } else {
                for session in active_sessions {
                    related_work.push(self.related_work_for_workspace(
                        &other_workspace,
                        &other_thread,
                        Some(session),
                        &changed_set,
                    )?);
                }
            }
        }

        related_work.sort_by(|left, right| {
            left.thread_title
                .cmp(&right.thread_title)
                .then_with(|| left.workspace_id.as_str().cmp(right.workspace_id.as_str()))
                .then_with(|| left.agent_name.cmp(&right.agent_name))
        });

        let mut potential_clash_notes = Vec::new();
        for related in &related_work {
            if !related.overlap_paths.is_empty() {
                potential_clash_notes.push(format!(
                    "Potential path overlap with {} on workspace {}: {}",
                    related.thread_title,
                    related.workspace_id,
                    related.overlap_paths.join(", ")
                ));
            } else if related.known_changed_paths.is_empty()
                && related.freshness_note == "unknown changes possible until snapshot/finish"
            {
                potential_clash_notes.push(format!(
                    "Workspace {} ({}) has unknown changes possible until snapshot/finish",
                    related.workspace_id, related.thread_title
                ));
            }
        }

        Ok(CoordinationStatus {
            current_session,
            workspace,
            thread,
            known_changed_paths,
            related_work,
            potential_clash_notes,
        })
    }

    fn related_work_for_workspace(
        &self,
        workspace: &WorkspaceView,
        thread: &WorkThread,
        session: Option<AgentSession>,
        changed_set: &BTreeSet<String>,
    ) -> Result<RelatedWork> {
        let maybe_paths = self.overlay_changed_path_names(workspace)?;
        let known_changed_paths = maybe_paths.clone().unwrap_or_default();
        let other_set: BTreeSet<String> = known_changed_paths.iter().cloned().collect();
        let overlap_paths: Vec<String> = changed_set.intersection(&other_set).cloned().collect();
        let freshness_note = match maybe_paths {
            Some(paths) if paths.is_empty() => "known no changes from latest overlay".to_owned(),
            Some(_) => "known changed paths from latest overlay".to_owned(),
            None => "unknown changes possible until snapshot/finish".to_owned(),
        };
        let (session_id, agent_name) = match session {
            Some(session) => (Some(session.id), session.agent_name),
            None => (None, "unregistered agent or human".to_owned()),
        };

        Ok(RelatedWork {
            session_id,
            agent_name,
            thread_id: thread.id.clone(),
            thread_title: thread.title.clone(),
            workspace_id: workspace.id.clone(),
            known_changed_paths,
            overlap_paths,
            freshness_note,
        })
    }

    fn latest_thread_workspace(&self, thread_id: &WorkThreadId) -> Result<Option<WorkspaceView>> {
        Ok(self
            .list_workspaces()?
            .into_iter()
            .filter(|workspace| &workspace.thread_id == thread_id)
            .rfind(|workspace| workspace.latest_snapshot.is_some()))
    }

    fn thread_evidence(&self, thread_id: &WorkThreadId) -> Result<Vec<EvidenceSummary>> {
        let mut records: Vec<EvidenceRecord> = read_json_dir(self.anvics_dir.join("evidence"))?;
        records.retain(|record| &record.thread_id == thread_id);
        records.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        let mut summaries = Vec::new();
        for record in records {
            let command_event = match &record.command_event_id {
                Some(command_event_id) => self.show_command_event(command_event_id.as_str()).ok(),
                None => None,
            };
            let command_policy_class = command_event
                .as_ref()
                .and_then(|event| event.command_policy_class.clone());
            let projection_kind = command_event
                .as_ref()
                .and_then(|event| event.projection_kind.clone());
            let runtime_metrics = command_event
                .as_ref()
                .and_then(|event| event.runtime_metrics.clone());
            let command_executor = command_event
                .as_ref()
                .and_then(|event| event.command_executor.clone());
            let command_policy_override_reason = command_event
                .as_ref()
                .and_then(|event| event.command_policy_override_reason.clone());
            let file_effects = command_event
                .map(|event| event.file_effects)
                .unwrap_or_default();
            summaries.push(EvidenceSummary {
                id: record.id,
                command_event_id: record.command_event_id,
                command: record.command,
                command_label: record.command_label,
                command_file: record.command_file,
                cwd: record.cwd,
                exit_code: record.exit_code,
                summary: record.summary,
                artifact_path: record.artifact_path,
                stdout_path: record.stdout_path,
                stderr_path: record.stderr_path,
                projection_kind,
                runtime_metrics,
                command_executor,
                command_policy_class,
                command_policy_override_reason,
                file_effects,
            });
        }
        Ok(summaries)
    }

    fn thread_reviews(&self, thread_id: &WorkThreadId) -> Result<Vec<ReviewProjection>> {
        let mut reviews: Vec<ReviewProjection> = read_json_dir(self.anvics_dir.join("reviews"))?;
        reviews.retain(|review| &review.thread_id == thread_id);
        reviews.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(reviews)
    }

    fn thread_publications(&self, thread_id: &WorkThreadId) -> Result<Vec<NativePublication>> {
        let mut publications: Vec<NativePublication> =
            read_json_dir(self.anvics_dir.join("publications"))?;
        publications.retain(|publication| &publication.thread_id == thread_id);
        publications.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(publications)
    }

    fn overlap_notes(
        &self,
        thread: &WorkThread,
        changed_paths: &[ChangedPath],
    ) -> Result<Vec<String>> {
        let changed: BTreeSet<&str> = changed_paths
            .iter()
            .map(|path| path.path.as_str())
            .collect();
        if changed.is_empty() {
            return Ok(Vec::new());
        }

        let mut notes = Vec::new();
        for other in self.list_threads()? {
            if other.id == thread.id || other.base_snapshot != thread.base_snapshot {
                continue;
            }
            let Some(other_workspace) = self.latest_thread_workspace(&other.id)? else {
                continue;
            };
            let other_final = other_workspace
                .latest_snapshot
                .clone()
                .ok_or_else(|| StoreError::MissingWorkspaceSnapshot(other.id.to_string()))?;
            let other_changed = if let Some(paths) = self.overlay_changed_paths(&other_workspace)? {
                paths
            } else {
                self.diff_snapshots(&other.base_snapshot, &other_final)?
            };
            let overlap: Vec<String> = other_changed
                .into_iter()
                .filter(|path| changed.contains(path.path.as_str()))
                .map(|path| path.path)
                .collect();
            if !overlap.is_empty() {
                notes.push(format!(
                    "Thread {} also changed: {}",
                    other.id,
                    overlap.join(", ")
                ));
            }
        }
        Ok(notes)
    }

    fn render_legacy_git_patch(
        &self,
        publication: &NativePublication,
        review: &ReviewProjection,
        thread: &WorkThread,
    ) -> Result<String> {
        let base_files = self.snapshot_file_map(&review.base_snapshot)?;
        let final_files = self.snapshot_file_map(&review.final_snapshot)?;
        let mut patch = format!(
            "From anvics {}\nSubject: [PATCH] {}\nAnvics-Publication: {}\nAnvics-Review: {}\nAnvics-Thread: {}\n\n---\n",
            publication.id, thread.title, publication.id, review.id, thread.id
        );

        for changed in &review.changed_paths {
            patch.push_str(&self.render_file_patch(
                &changed.path,
                &changed.status,
                &base_files,
                &final_files,
            )?);
        }

        Ok(patch)
    }

    fn render_file_patch(
        &self,
        path: &str,
        status: &ChangeStatus,
        base_files: &BTreeMap<String, ObjectId>,
        final_files: &BTreeMap<String, ObjectId>,
    ) -> Result<String> {
        let old_content = base_files
            .get(path)
            .map(|object| self.read_object(object))
            .transpose()?;
        let new_content = final_files
            .get(path)
            .map(|object| self.read_object(object))
            .transpose()?;
        Ok(render_unified_file_patch(
            path,
            status,
            old_content.as_deref().unwrap_or_default(),
            new_content.as_deref().unwrap_or_default(),
        ))
    }
}

#[derive(Default)]
struct TreeNode {
    dirs: BTreeMap<String, TreeNode>,
    files: Vec<(String, ObjectId, u64)>,
}

impl TreeNode {
    fn insert(&mut self, path: &Path, object: ObjectId, size: u64) {
        let mut components = path.components().peekable();
        let mut node = self;

        while let Some(component) = components.next() {
            let Component::Normal(name) = component else {
                continue;
            };
            let name = name.to_string_lossy().to_string();
            if components.peek().is_none() {
                node.files.push((name, object, size));
                return;
            }
            node = node.dirs.entry(name).or_default();
        }
    }
}

fn now_rfc3339() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn write_json_pretty(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes)?;
    Ok(())
}

fn remove_empty_mount_dir(path: &Path) -> Result<()> {
    match fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(error) => Err(StoreError::Io(error)),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u64::MAX as u128) as u64
}

fn read_json<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_json_dir<T: serde::de::DeserializeOwned>(path: impl AsRef<Path>) -> Result<Vec<T>> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut values = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().extension().is_some_and(|ext| ext == "json")
        {
            values.push(read_json(entry.path())?);
        }
    }
    Ok(values)
}

fn collect_secret_findings(
    findings: &mut Vec<RiskFinding>,
    scan_id: &RiskScanId,
    review_id: &ReviewProjectionId,
    target_kind: RiskTargetKind,
    target_path: &str,
    text: &str,
) {
    for (index, line) in text.lines().enumerate() {
        for detector in secret_detectors_for_line(line) {
            findings.push(RiskFinding {
                id: RiskFindingId::new(),
                scan_id: scan_id.clone(),
                review_id: review_id.clone(),
                detector,
                target_kind: target_kind.clone(),
                target_path: target_path.to_owned(),
                line: Some((index + 1) as u32),
                severity: RiskSeverity::SecretRisk,
                redacted_excerpt: redact_line(line),
            });
        }
    }
}

fn collect_secret_findings_for_lines<'a>(
    findings: &mut Vec<RiskFinding>,
    scan_id: &RiskScanId,
    review_id: &ReviewProjectionId,
    target_kind: RiskTargetKind,
    target_path: &str,
    lines: impl IntoIterator<Item = (u32, &'a str)>,
) {
    for (line_number, line) in lines {
        for detector in secret_detectors_for_line(line) {
            findings.push(RiskFinding {
                id: RiskFindingId::new(),
                scan_id: scan_id.clone(),
                review_id: review_id.clone(),
                detector,
                target_kind: target_kind.clone(),
                target_path: target_path.to_owned(),
                line: Some(line_number),
                severity: RiskSeverity::SecretRisk,
                redacted_excerpt: redact_line(line),
            });
        }
    }
}

fn all_numbered_lines(text: &str) -> Vec<(u32, &str)> {
    text.lines()
        .enumerate()
        .map(|(index, line)| ((index + 1) as u32, line))
        .collect()
}

fn introduced_numbered_lines<'a>(base_text: &str, final_text: &'a str) -> Vec<(u32, &'a str)> {
    let mut remaining_base_lines: BTreeMap<&str, usize> = BTreeMap::new();
    for line in base_text.lines() {
        *remaining_base_lines.entry(line).or_default() += 1;
    }

    final_text
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            if let Some(count) = remaining_base_lines.get_mut(line) {
                if *count > 0 {
                    *count -= 1;
                    return None;
                }
            }
            Some(((index + 1) as u32, line))
        })
        .collect()
}

fn secret_detectors_for_line(line: &str) -> Vec<String> {
    let mut detectors = Vec::new();
    let trimmed = line.trim();
    if trimmed.contains("-----BEGIN ") && trimmed.contains("PRIVATE KEY-----") {
        detectors.push("private_key_block".to_owned());
    }
    if contains_prefixed_token(trimmed, "ghp_", 36)
        || contains_prefixed_token(trimmed, "gho_", 36)
        || contains_prefixed_token(trimmed, "github_pat_", 40)
    {
        detectors.push("github_token".to_owned());
    }
    if contains_prefixed_token(trimmed, "sk-", 24) {
        detectors.push("openai_token".to_owned());
    }
    if contains_aws_access_key_id(trimmed) {
        detectors.push("aws_access_key_id".to_owned());
    }
    if looks_like_env_secret_assignment(trimmed) {
        detectors.push("env_secret_assignment".to_owned());
    }
    if contains_suspicious_high_entropy_value(trimmed) {
        detectors.push("high_entropy_secret_like_value".to_owned());
    }
    detectors.sort();
    detectors.dedup();
    detectors
}

fn contains_prefixed_token(line: &str, prefix: &str, min_len: usize) -> bool {
    line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    })
    .any(|part| part.starts_with(prefix) && part.len() >= min_len)
}

fn contains_aws_access_key_id(line: &str) -> bool {
    line.split(|character: char| !character.is_ascii_alphanumeric())
        .any(|part| {
            part.len() == 20
                && (part.starts_with("AKIA") || part.starts_with("ASIA"))
                && part
                    .chars()
                    .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
        })
}

fn looks_like_env_secret_assignment(line: &str) -> bool {
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    let key = key.trim().to_ascii_uppercase();
    let value = value.trim().trim_matches('"').trim_matches('\'');
    value.len() >= 8
        && ["SECRET", "TOKEN", "API_KEY", "PASSWORD", "PRIVATE_KEY"]
            .iter()
            .any(|marker| key.contains(marker))
}

fn contains_suspicious_high_entropy_value(line: &str) -> bool {
    let suspicious_context = ["SECRET", "TOKEN", "API_KEY", "PASSWORD", "KEY"]
        .iter()
        .any(|marker| line.to_ascii_uppercase().contains(marker));
    suspicious_context
        && line
            .split(|character: char| {
                !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '/'))
            })
            .any(|part| part.len() >= 32 && unique_ascii_count(part) >= 16)
}

fn unique_ascii_count(value: &str) -> usize {
    let mut bytes = BTreeSet::new();
    for byte in value.bytes() {
        bytes.insert(byte);
    }
    bytes.len()
}

fn redact_line(line: &str) -> String {
    let mut redacted = line.trim().to_owned();
    if redacted.len() > 120 {
        redacted.truncate(120);
        redacted.push_str("...");
    }
    let Some((key, value)) = redacted.split_once('=') else {
        return "<redacted secret-like content>".to_owned();
    };
    let value = value.trim();
    if value.is_empty() {
        return format!("{}=<redacted>", key.trim());
    }
    format!("{}=<redacted:{} chars>", key.trim(), value.len())
}

struct LocalCommandOutput {
    exit_code: i32,
    timed_out: bool,
    duration_ms: Option<u64>,
    executor: CommandExecutorKind,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn execute_command(argv: &[String], cwd: &Path, timeout: Duration) -> Result<LocalCommandOutput> {
    if std::env::var("ANVICS_COMMAND_EXECUTOR").ok().as_deref() == Some("worker") {
        execute_worker_command(argv, cwd, timeout)
    } else {
        execute_local_command(argv, cwd, timeout)
    }
}

fn execute_local_command(
    argv: &[String],
    cwd: &Path,
    timeout: Duration,
) -> Result<LocalCommandOutput> {
    let mut child = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let started = Instant::now();
    let mut timed_out = false;

    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            child.kill()?;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let output = child.wait_with_output()?;
    let exit_code = if timed_out {
        -1
    } else {
        output.status.code().unwrap_or(-1)
    };
    Ok(LocalCommandOutput {
        exit_code,
        timed_out,
        duration_ms: Some(elapsed_ms(started)),
        executor: CommandExecutorKind::InProcess,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

fn execute_worker_command(
    argv: &[String],
    cwd: &Path,
    timeout: Duration,
) -> Result<LocalCommandOutput> {
    let worker = std::env::var("ANVICS_WORKER_BIN").unwrap_or_else(|_| "anvics-worker".to_owned());
    let request = CommandWorkerRequest {
        argv: argv.to_vec(),
        cwd: cwd.to_string_lossy().to_string(),
        timeout_seconds: timeout.as_secs(),
    };
    let mut child = Command::new(worker)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        let mut stdin = child.stdin.take().ok_or_else(|| {
            StoreError::CommandWorkerFailed("worker stdin was unavailable".to_owned())
        })?;
        serde_json::to_writer(&mut stdin, &request)?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(StoreError::CommandWorkerFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let response: CommandWorkerResponse = serde_json::from_slice(&output.stdout)?;
    Ok(LocalCommandOutput {
        exit_code: response.exit_code,
        timed_out: response.timed_out,
        duration_ms: Some(response.duration_ms),
        executor: CommandExecutorKind::Worker,
        stdout: response.stdout,
        stderr: response.stderr,
    })
}

fn shell_join(argv: &[String]) -> String {
    argv.iter()
        .map(|part| {
            if part.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/' | b'-' | b'_')
            }) {
                part.clone()
            } else {
                shell_quote(part)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn classify_command_policy(input: CommandPolicyInput) -> Result<CommandPolicyDecision> {
    let policy_class = if let Some(path) = input.command_file {
        let command = fs::read_to_string(path)?;
        classify_shell_command(&command)
    } else {
        classify_argv_policy(&input.argv)
    };
    let blocked = command_policy_requires_override(&policy_class);
    Ok(CommandPolicyDecision {
        policy_class,
        blocked,
        override_hint: blocked
            .then(|| "--allow-command-risk --command-risk-reason <reason>".to_owned()),
    })
}

fn classify_argv_policy(argv: &[String]) -> CommandPolicyClass {
    let Some(program) = argv.first().map(|value| value.as_str()) else {
        return CommandPolicyClass::Unknown;
    };
    let program_name = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    let shell_command = if matches!(program_name, "sh" | "bash" | "zsh") && argv.len() >= 3 {
        argv.last().map(String::as_str)
    } else {
        None
    };

    if let Some(command) = shell_command {
        return classify_shell_command(command);
    }

    if matches!(
        program_name,
        "rm" | "rmdir" | "unlink" | "trash" | "shred" | "dd"
    ) {
        return CommandPolicyClass::Destructive;
    }
    if matches!(
        program_name,
        "sudo" | "su" | "doas" | "docker" | "podman" | "osascript" | "open"
    ) {
        return CommandPolicyClass::HostEscapeRisk;
    }
    if matches!(
        program_name,
        "curl" | "wget" | "ssh" | "scp" | "sftp" | "nc" | "netcat" | "gh" | "git"
    ) {
        return CommandPolicyClass::Networked;
    }
    if matches!(
        program_name,
        "vim" | "vi" | "nano" | "emacs" | "less" | "more" | "top" | "htop"
    ) {
        return CommandPolicyClass::Interactive;
    }
    if matches!(
        program_name,
        "touch" | "mkdir" | "cp" | "mv" | "tee" | "sed" | "perl" | "python" | "python3" | "node"
    ) {
        return CommandPolicyClass::Mutating;
    }
    if matches!(
        program_name,
        "cat"
            | "grep"
            | "rg"
            | "ls"
            | "pwd"
            | "head"
            | "tail"
            | "wc"
            | "find"
            | "test"
            | "true"
            | "false"
    ) {
        return CommandPolicyClass::ReadOnly;
    }

    CommandPolicyClass::Unknown
}

fn command_policy_override_reason(
    policy_class: &CommandPolicyClass,
    input: &CommandRunInput,
) -> Result<Option<String>> {
    if !input.allow_command_risk {
        if input.command_risk_reason.is_some() {
            return Err(StoreError::CommandRiskReasonWithoutOverride);
        }
        if command_policy_requires_override(policy_class) {
            return Err(StoreError::CommandPolicyBlocked {
                policy_class: policy_class.clone(),
            });
        }
        return Ok(None);
    }

    let reason = input
        .command_risk_reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .ok_or(StoreError::EmptyCommandRiskReason)?;
    Ok(Some(reason.to_owned()))
}

fn command_policy_requires_override(policy_class: &CommandPolicyClass) -> bool {
    matches!(
        policy_class,
        CommandPolicyClass::Networked
            | CommandPolicyClass::HostEscapeRisk
            | CommandPolicyClass::Interactive
    )
}

fn classify_shell_command(command: &str) -> CommandPolicyClass {
    let command = command.trim();
    if contains_shell_word(command, "rm")
        || contains_shell_word(command, "rmdir")
        || contains_shell_word(command, "unlink")
    {
        return CommandPolicyClass::Destructive;
    }
    if contains_shell_word(command, "sudo")
        || contains_shell_word(command, "docker")
        || contains_shell_word(command, "podman")
        || contains_shell_word(command, "osascript")
    {
        return CommandPolicyClass::HostEscapeRisk;
    }
    if contains_shell_word(command, "curl")
        || contains_shell_word(command, "wget")
        || contains_shell_word(command, "ssh")
        || contains_shell_word(command, "scp")
        || contains_shell_word(command, "gh")
        || contains_shell_word(command, "git")
    {
        return CommandPolicyClass::Networked;
    }
    if contains_shell_word(command, "vim")
        || contains_shell_word(command, "nano")
        || contains_shell_word(command, "less")
        || contains_shell_word(command, "more")
    {
        return CommandPolicyClass::Interactive;
    }
    if command.contains('>')
        || command.contains(">>")
        || contains_shell_word(command, "touch")
        || contains_shell_word(command, "mkdir")
        || contains_shell_word(command, "cp")
        || contains_shell_word(command, "mv")
        || contains_shell_word(command, "tee")
        || command.contains(" -i")
    {
        return CommandPolicyClass::Mutating;
    }
    if contains_shell_word(command, "cat")
        || contains_shell_word(command, "grep")
        || contains_shell_word(command, "rg")
        || contains_shell_word(command, "ls")
        || contains_shell_word(command, "pwd")
        || contains_shell_word(command, "test")
    {
        return CommandPolicyClass::ReadOnly;
    }

    CommandPolicyClass::Unknown
}

fn contains_shell_word(command: &str, word: &str) -> bool {
    command
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .any(|part| part == word)
}

fn append_event_at(
    anvics_dir: &Path,
    kind: RepositoryEventKind,
    subject_id: Option<String>,
) -> Result<RepositoryEvent> {
    let events_dir = anvics_dir.join("events");
    fs::create_dir_all(&events_dir)?;
    let sequence_path = events_dir.join("SEQ");
    let current_sequence = if sequence_path.exists() {
        fs::read_to_string(&sequence_path)?
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
    } else {
        0
    };
    let sequence = current_sequence + 1;
    let event = RepositoryEvent {
        id: RepositoryEventId::new(),
        sequence,
        kind,
        subject_id,
        created_at: now_rfc3339()?,
    };
    write_json_pretty(events_dir.join(format!("{sequence:020}.json")), &event)?;
    fs::write(sequence_path, sequence.to_string())?;
    Ok(event)
}

fn collect_files(source_root: &Path) -> Result<Vec<PathBuf>> {
    let source_root = source_root.to_path_buf();
    let mut builder = WalkBuilder::new(&source_root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .ignore(true)
        .parents(true)
        .filter_entry({
            let source_root = source_root.clone();
            move |entry| {
                let relative = entry
                    .path()
                    .strip_prefix(&source_root)
                    .unwrap_or(entry.path());
                !is_skipped(relative)
            }
        });

    let mut files = Vec::new();
    for result in builder.build() {
        let entry = result?;
        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file())
        {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn workspace_file_map(source_root: &Path) -> Result<BTreeMap<String, ObjectId>> {
    let mut files = BTreeMap::new();
    for file in collect_files(source_root)? {
        let bytes = fs::read(&file)?;
        let relative = file
            .strip_prefix(source_root)
            .map_err(|_| StoreError::OutsideRoot(file.clone()))?;
        files.insert(
            relative.to_string_lossy().to_string(),
            ObjectId::from_bytes(&bytes),
        );
    }
    Ok(files)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WorkspaceFileStats {
    file_count: u64,
    byte_count: u64,
}

fn workspace_file_stats(source_root: &Path) -> Result<WorkspaceFileStats> {
    let mut stats = WorkspaceFileStats {
        file_count: 0,
        byte_count: 0,
    };
    for file in collect_files(source_root)? {
        stats.file_count += 1;
        stats.byte_count = stats.byte_count.saturating_add(fs::metadata(file)?.len());
    }
    Ok(stats)
}

fn is_skipped(path: &Path) -> bool {
    path.components().any(|component| {
        let Component::Normal(name) = component else {
            return false;
        };
        matches!(
            name.to_str(),
            Some(".git" | ".anvics" | ".DS_Store" | "target")
        )
    })
}

fn tree_kind_order(kind: &TreeEntryKind) -> u8 {
    match kind {
        TreeEntryKind::Directory => 0,
        TreeEntryKind::File => 1,
    }
}

fn diff_file_maps(
    base_files: &BTreeMap<String, ObjectId>,
    final_files: &BTreeMap<String, ObjectId>,
) -> Vec<ChangedPath> {
    let mut paths: BTreeSet<&str> = BTreeSet::new();
    paths.extend(base_files.keys().map(String::as_str));
    paths.extend(final_files.keys().map(String::as_str));

    paths
        .into_iter()
        .filter_map(|path| match (base_files.get(path), final_files.get(path)) {
            (None, Some(_)) => Some(ChangedPath {
                path: path.to_owned(),
                status: ChangeStatus::Added,
            }),
            (Some(_), None) => Some(ChangedPath {
                path: path.to_owned(),
                status: ChangeStatus::Deleted,
            }),
            (Some(base), Some(final_object)) if base != final_object => Some(ChangedPath {
                path: path.to_owned(),
                status: ChangeStatus::Modified,
            }),
            _ => None,
        })
        .collect()
}

fn propose_file_effect_set(
    changed_paths: &[ChangedPath],
    config: &RepositoryConfig,
) -> Result<FileEffectSet> {
    Ok(FileEffectSet {
        id: FileEffectSetId::new(),
        effects: changed_paths
            .iter()
            .map(|path| FileEffect {
                path: path.path.clone(),
                status: path.status.clone(),
                labels: classify_file_effect_path(&path.path, config),
                provenance: FileEffectProvenance::Heuristic,
            })
            .collect(),
        created_at: now_rfc3339()?,
    })
}

fn propose_change_units(effect_set: &FileEffectSet) -> Vec<ChangeUnit> {
    effect_set
        .effects
        .iter()
        .filter(|effect| file_effect_becomes_change_unit(effect))
        .map(|effect| ChangeUnit {
            id: ChangeUnitId::new(),
            path: effect.path.clone(),
            status: effect.status.clone(),
            labels: effect.labels.clone(),
            provenance: effect.provenance.clone(),
            summary: format!("{} {}", change_status_label(&effect.status), effect.path),
        })
        .collect()
}

fn file_effect_becomes_change_unit(effect: &FileEffect) -> bool {
    effect.labels.iter().any(|label| {
        matches!(
            label,
            FileEffectLabel::Source
                | FileEffectLabel::GeneratedTracked
                | FileEffectLabel::Lockfile
                | FileEffectLabel::Config
                | FileEffectLabel::Binary
                | FileEffectLabel::Unknown
        )
    }) && !effect.labels.iter().any(|label| {
        matches!(
            label,
            FileEffectLabel::Cache
                | FileEffectLabel::GeneratedUntracked
                | FileEffectLabel::EvidenceCandidate
        )
    })
}

fn classify_file_effect_path(path: &str, config: &RepositoryConfig) -> Vec<FileEffectLabel> {
    let lower = path.to_ascii_lowercase();
    let file_name = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_ascii_lowercase();
    if config_patterns_match(&config.ignore.paths, path) {
        return vec![FileEffectLabel::Cache];
    }
    if config_patterns_match(&config.generated.untracked, path) {
        return vec![FileEffectLabel::GeneratedUntracked];
    }
    if config_patterns_match(&config.evidence.candidate_paths, path) {
        return vec![FileEffectLabel::EvidenceCandidate];
    }
    if config_patterns_match(&config.generated.tracked, path) {
        return vec![FileEffectLabel::GeneratedTracked];
    }
    if lower.starts_with("target/")
        || lower.starts_with(".cache/")
        || lower.starts_with("node_modules/")
        || lower.contains("/target/")
        || lower.contains("/.cache/")
        || lower.contains("/node_modules/")
    {
        return vec![FileEffectLabel::Cache];
    }
    if lower.starts_with(".anvics/artifacts/")
        || lower.starts_with("artifacts/")
        || lower.starts_with("evidence/")
        || lower.ends_with(".log")
    {
        return vec![FileEffectLabel::EvidenceCandidate];
    }
    if matches!(
        file_name.as_str(),
        "cargo.lock" | "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock" | "bun.lockb"
    ) {
        return vec![FileEffectLabel::Lockfile];
    }
    if matches!(
        file_name.as_str(),
        "anvics.toml"
            | "package.json"
            | "cargo.toml"
            | "tsconfig.json"
            | "deno.json"
            | "pyproject.toml"
    ) || lower.starts_with(".github/")
    {
        return vec![FileEffectLabel::Config];
    }
    if lower.contains("/generated/")
        || lower.contains("/dist/")
        || lower.starts_with("dist/")
        || lower.ends_with(".generated.rs")
        || lower.ends_with(".pb.rs")
    {
        return vec![FileEffectLabel::GeneratedTracked];
    }
    if matches!(
        Path::new(&lower)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "pdf" | "zip" | "gz" | "wasm")
    ) {
        return vec![FileEffectLabel::Binary];
    }
    if matches!(
        Path::new(&lower)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some(
            "rs" | "js"
                | "ts"
                | "tsx"
                | "jsx"
                | "py"
                | "go"
                | "java"
                | "c"
                | "cc"
                | "cpp"
                | "h"
                | "hpp"
                | "md"
                | "txt"
                | "sh"
                | "html"
                | "css"
                | "sql"
        )
    ) {
        return vec![FileEffectLabel::Source];
    }
    vec![FileEffectLabel::Unknown]
}

fn config_patterns_match(patterns: &[String], path: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| path_matches_config_pattern(pattern, path))
}

fn path_matches_config_pattern(pattern: &str, path: &str) -> bool {
    let pattern = normalize_config_pattern(pattern);
    let path = normalize_config_pattern(path);
    if pattern.is_empty() {
        return false;
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if pattern.contains('*') || pattern.contains('?') {
        return wildcard_match(pattern.as_bytes(), path.as_bytes());
    }
    path == pattern || path.starts_with(&format!("{pattern}/"))
}

fn normalize_config_pattern(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("./")
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn wildcard_match(pattern: &[u8], value: &[u8]) -> bool {
    let (mut pattern_index, mut value_index) = (0, 0);
    let mut star_index = None;
    let mut star_value_index = 0;

    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            star_value_index = value_index;
            pattern_index += 1;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

fn render_agent_packet(repo_root: &Path, thread: &WorkThread, workspace: &WorkspaceView) -> String {
    let repo = shell_quote(&display_path(repo_root));
    let workspace_path = shell_quote(&workspace.materialized_path);
    let skill_path = Path::new(&workspace.materialized_path).join("skills/anvics-skill/SKILL.md");
    let skill_section = if skill_path.exists() {
        format!(
            "\n## Anvics Skill\n\nBefore editing, read and follow the Anvics skill:\n\n```sh\nsed -n '1,220p' {}\n```\n",
            shell_quote(&display_path(&skill_path))
        )
    } else {
        "\n## Anvics Skill\n\nIf this repository provides `skills/anvics-skill/SKILL.md`, read it before editing and follow it as the source-control workflow guide.\n".to_owned()
    };
    format!(
        "# Anvics Agent Task\n\nThread: `{}`\nWorkspace: `{}`\nRepository: `{}`\nWorkspace path: `{}`\n{skill_section}\n## Task\n\n{}\n\n## Instructions\n\n- Read the Anvics skill above before editing when it is available.\n- Before editing, run the agent enter command below and read the coordination output.\n- Work only inside the workspace path above. This workspace is the only editable area for this task.\n- This workspace may not be a Git repository. Use Anvics commands even if your CLI normally expects Git.\n- Use `anvics --repo {repo} workspace diff {}` to inspect workspace changes; do not use Git status or Git diff inside the workspace.\n- Do not edit the repository root, `.anvics/` metadata, another workspace, a Git branch, a Git worktree, or a Git commit.\n- Keep command output compact, and do not paste secrets or tokens into evidence summaries.\n- Before finishing, run `anvics --repo {repo} coordination status --workspace {}` and summarize any potential clashes.\n- If you spawn subagents, give them this packet, the Anvics skill path, the repository path, the workspace id/path, and these same agent-run commands.\n- Do not run operator-only commands such as `agent accept`, `publish create`, or `legacy git export` unless the operator explicitly asks you to accept, publish, or export.\n\n## Workspace\n\n```sh\ncd {workspace_path}\n```\n\n## Agent-Run Commands\n\nRun these commands as the working agent.\n\n### Enter The Workspace\n\n```sh\nanvics --repo {repo} agent enter --workspace {} --name \"<agent-name>\"\n```\n\n### Inspect Workspace Changes\n\n```sh\nanvics --repo {repo} workspace diff {}\n```\n\n### Check Coordination Before Finishing\n\n```sh\nanvics --repo {repo} coordination status --workspace {}\n```\n\n### Optional Agent Finish\n\nRun this only when asked to record self-reported evidence before operator acceptance.\n\n```sh\nanvics --repo {repo} agent finish --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"\n```\n\nAdd `--artifact <path>` only when you created a compact artifact worth linking.\n\n## Operator-Run Commands\n\nThese commands accept, publish, or export work. Do not run them as an agent unless the operator explicitly asks you to.\n\n### Launch Prompt For External Agent CLIs\n\n```sh\nanvics --repo {repo} agent launch-prompt --workspace {} --tool codex\n```\n\n### Accept With Anvics-Run Verification\n\n```sh\nanvics --repo {repo} agent accept --workspace {} --run-label \"<short label>\" --run-summary \"<short summary>\" -- <program> [args...]\n```\n\n### Accept With Externally-Run Verification\n\n```sh\nanvics --repo {repo} agent accept --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"\n```\n",
        thread.id,
        workspace.id,
        repo_root.display(),
        workspace.materialized_path,
        thread.task,
        workspace.id,
        workspace.id,
        workspace.id,
        workspace.id,
        workspace.id,
        workspace.id,
        workspace.id,
        workspace.id,
        workspace.id
    )
}

fn render_agent_launch_prompt(
    repo_root: &Path,
    thread: &WorkThread,
    workspace: &WorkspaceView,
    packet_path: &Path,
    skill_path: Option<&str>,
) -> String {
    let skill_instruction = match skill_path {
        Some(path) => format!("Read the Anvics skill before editing:\n{path}\n"),
        None => {
            "If this workspace contains `skills/anvics-skill/SKILL.md`, read it before editing.\n"
                .to_owned()
        }
    };

    format!(
        "You are working inside an Anvics task packet.\n\n\
Read the packet at:\n{packet_path}\n\n\
{skill_instruction}\n\
Repository path:\n{repo}\n\n\
Thread id:\n{thread_id}\n\n\
Task title:\n{title}\n\n\
Workspace id:\n{workspace_id}\n\n\
Workspace path:\n{workspace_path}\n\n\
Anvics workspaces are not Git worktrees and may not contain a `.git` directory. \
Use the Anvics commands in the packet instead of Git status, Git diff, branches, worktrees, commits, or pushes.\n\n\
Follow the skill and packet exactly. Work only inside the workspace path above. \
Run the packet's `agent enter` command before editing. Use `workspace diff` to inspect changes. \
Run `coordination status` before finishing and report potential clashes.\n\n\
When done, report whether you read the skill, whether you used `workspace diff`, \
what verification command you ran, its exit code, a one-sentence summary, and any compact artifact path.",
        packet_path = packet_path.display(),
        repo = repo_root.display(),
        thread_id = thread.id,
        title = thread.title,
        workspace_id = workspace.id,
        workspace_path = workspace.materialized_path,
    )
}

fn render_codex_launch_command(workspace: &WorkspaceView, prompt: &str) -> String {
    format!(
        "cat <<'ANVICS_PROMPT' | codex exec --skip-git-repo-check --cd {} -\n{}\nANVICS_PROMPT",
        shell_quote(&workspace.materialized_path),
        prompt
    )
}

fn render_review(
    repo_root: &Path,
    review: &ReviewProjection,
    thread: &WorkThread,
    scans: &[RiskScan],
    overrides: &[PolicyOverride],
) -> String {
    let repo = shell_quote(&display_path(repo_root));
    let mut markdown = format!(
        "# Review {}\n\n- Thread: {}\n- Title: {}\n- Base snapshot: {}\n- Final snapshot: {}\n\n## Task\n\n{}\n\n",
        review.id,
        review.thread_id,
        thread.title,
        review.base_snapshot,
        review.final_snapshot,
        thread.task
    );

    markdown.push_str("## Changed Paths\n\n");
    if review.changed_paths.is_empty() {
        markdown.push_str("- No source changes detected.\n");
    } else {
        for path in &review.changed_paths {
            markdown.push_str(&format!("- {:?}: `{}`\n", path.status, path.path));
        }
    }

    markdown.push_str("\n## File Effects\n\n");
    if review.file_effects.is_empty() {
        markdown.push_str("- No classified file effects recorded.\n");
    } else {
        for effect in &review.file_effects {
            let labels = effect
                .labels
                .iter()
                .map(file_effect_label)
                .collect::<Vec<_>>()
                .join(", ");
            let note = file_effect_review_note(effect);
            markdown.push_str(&format!(
                "- {:?}: `{}` ({labels}) - {note}\n",
                effect.status, effect.path
            ));
        }
    }

    markdown.push_str("\n## Evidence\n\n");
    if review.evidence.is_empty() {
        markdown.push_str("- No evidence attached.\n");
    } else {
        for evidence in &review.evidence {
            markdown.push_str(&format!("- {}\n", render_evidence_summary(evidence)));
        }
    }

    markdown.push_str("\n## Change Units\n\n");
    if review.change_units.is_empty() {
        markdown.push_str("- No source-relevant change units proposed.\n");
    } else {
        for unit in &review.change_units {
            let labels = unit
                .labels
                .iter()
                .map(file_effect_label)
                .collect::<Vec<_>>()
                .join(", ");
            markdown.push_str(&format!(
                "- {} {:?}: `{}` ({labels}) - {}\n",
                unit.id, unit.status, unit.path, unit.summary
            ));
        }
    }

    markdown.push_str("\n## Overlap Notes\n\n");
    if review.overlap_notes.is_empty() {
        markdown.push_str("- No path overlap detected.\n");
    } else {
        for note in &review.overlap_notes {
            markdown.push_str(&format!("- {note}\n"));
        }
    }

    markdown.push_str("\n## Risk Notes\n\n");
    let findings: Vec<&RiskFinding> = scans.iter().flat_map(|scan| scan.findings.iter()).collect();
    if findings.is_empty() {
        if scans.is_empty() {
            markdown.push_str("- No risk scan has run for this review.\n");
        } else {
            markdown.push_str("- No secret-risk findings detected.\n");
        }
    } else {
        markdown.push_str(&format!(
            "- Publication blocked by default: {} secret-risk finding(s).\n",
            findings.len()
        ));
        for finding in findings {
            let line = finding
                .line
                .map(|line| format!(":{line}"))
                .unwrap_or_default();
            markdown.push_str(&format!(
                "- {:?} `{}`{} via `{}`: {}\n",
                finding.target_kind,
                finding.target_path,
                line,
                finding.detector,
                finding.redacted_excerpt
            ));
        }
    }
    if overrides.is_empty() {
        markdown.push_str("- Override: none.\n");
    } else {
        for override_record in overrides {
            markdown.push_str(&format!(
                "- Override `{}` recorded: {}\n",
                override_record.id, override_record.reason
            ));
        }
    }

    markdown.push_str("\n## Next Commands\n\n");
    markdown.push_str("Shortest path for an unaccepted workspace:\n\n");
    markdown.push_str(&format!(
        "```sh\nanvics --repo {repo} agent accept --workspace <workspace-id> --run-label \"<short label>\" --run-summary \"<short summary>\" -- <program> [args...]\n```\n\n"
    ));
    markdown.push_str("Manual path from this review:\n\n");
    markdown.push_str(&format!(
        "```sh\nanvics --repo {repo} review show {} --format markdown\nanvics --repo {repo} publish create --thread {} --review {}\nanvics --repo {repo} legacy git export --publication <publication-id> --output accepted.patch\n```\n",
        review.id,
        review.thread_id, review.id
    ));

    markdown
}

fn file_effect_review_note(effect: &FileEffect) -> &'static str {
    if effect
        .labels
        .iter()
        .any(|label| matches!(label, FileEffectLabel::EvidenceCandidate))
    {
        "evidence candidate; attach compactly if useful"
    } else if effect.labels.iter().any(|label| {
        matches!(
            label,
            FileEffectLabel::Cache | FileEffectLabel::GeneratedUntracked
        )
    }) {
        "excluded from ChangeUnits"
    } else if effect
        .labels
        .iter()
        .any(|label| matches!(label, FileEffectLabel::GeneratedTracked))
    {
        "proposed ChangeUnit; generated source may need rationale"
    } else {
        "proposed ChangeUnit"
    }
}

fn render_evidence_summary(evidence: &EvidenceSummary) -> String {
    let label = evidence
        .command_label
        .as_deref()
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| compact_command_label(&evidence.command));
    let mut text = format!(
        "`{label}` exited {}: {}",
        evidence.exit_code, evidence.summary
    );
    if let Some(command_event_id) = &evidence.command_event_id {
        text.push_str(&format!(" (anvics-run: `{command_event_id}`)"));
    }
    if let Some(cwd) = evidence.cwd.as_deref().filter(|cwd| !cwd.trim().is_empty()) {
        text.push_str(&format!(" (cwd: `{cwd}`)"));
    }
    if let Some(command_file) = evidence
        .command_file
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        text.push_str(&format!(" (command file: `{command_file}`)"));
    }
    if let Some(artifact) = evidence
        .artifact_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        text.push_str(&format!(" (artifact: `{artifact}`)"));
    }
    if let Some(stdout) = evidence
        .stdout_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        text.push_str(&format!(" (stdout: `{stdout}`)"));
    }
    if let Some(stderr) = evidence
        .stderr_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
    {
        text.push_str(&format!(" (stderr: `{stderr}`)"));
    }
    if let Some(policy_class) = &evidence.command_policy_class {
        text.push_str(&format!(
            " (policy: {})",
            command_policy_class_label(policy_class)
        ));
    }
    if let Some(executor) = &evidence.command_executor {
        text.push_str(&format!(
            " (executor: {})",
            command_executor_label(executor)
        ));
    }
    if let Some(reason) = evidence
        .command_policy_override_reason
        .as_deref()
        .filter(|reason| !reason.trim().is_empty())
    {
        text.push_str(&format!(" (command policy override: {reason})"));
    }
    if let Some(projection_kind) = &evidence.projection_kind {
        text.push_str(&format!(
            " (projection: {})",
            projection_kind_label(projection_kind)
        ));
    }
    if let Some(metrics) = &evidence.runtime_metrics {
        text.push_str(&format!(
            " (runtime: setup={}ms command={}ms reconcile={}ms cleanup={}ms files={} bytes={})",
            metrics.projection_setup_ms,
            metrics.command_ms,
            metrics.reconcile_ms,
            metrics.cleanup_ms,
            metrics.projection_files,
            metrics.projection_bytes
        ));
    }
    if !evidence.file_effects.is_empty() {
        let effects = evidence
            .file_effects
            .iter()
            .map(|effect| format!("{} `{}`", change_status_label(&effect.status), effect.path))
            .collect::<Vec<_>>()
            .join(", ");
        text.push_str(&format!(" (file effects: {effects})"));
    }
    text
}

fn projection_kind_label(kind: &ProjectionKind) -> &'static str {
    match kind {
        ProjectionKind::MaterializedDir => "materialized_dir",
        ProjectionKind::FuseMount => "fuse_mount",
    }
}

fn command_executor_label(executor: &CommandExecutorKind) -> &'static str {
    match executor {
        CommandExecutorKind::InProcess => "in_process",
        CommandExecutorKind::Worker => "worker",
    }
}

fn file_effect_label(label: &FileEffectLabel) -> &'static str {
    match label {
        FileEffectLabel::Source => "source",
        FileEffectLabel::GeneratedTracked => "generated_tracked",
        FileEffectLabel::GeneratedUntracked => "generated_untracked",
        FileEffectLabel::EvidenceCandidate => "evidence_candidate",
        FileEffectLabel::Cache => "cache",
        FileEffectLabel::Lockfile => "lockfile",
        FileEffectLabel::Config => "config",
        FileEffectLabel::SecretRisk => "secret_risk",
        FileEffectLabel::Binary => "binary",
        FileEffectLabel::Unknown => "unknown",
    }
}

fn command_policy_class_label(policy_class: &CommandPolicyClass) -> &'static str {
    match policy_class {
        CommandPolicyClass::ReadOnly => "read_only",
        CommandPolicyClass::Mutating => "mutating",
        CommandPolicyClass::Destructive => "destructive",
        CommandPolicyClass::Networked => "networked",
        CommandPolicyClass::HostEscapeRisk => "host_escape_risk",
        CommandPolicyClass::Interactive => "interactive",
        CommandPolicyClass::Unknown => "unknown",
    }
}

fn change_status_label(status: &ChangeStatus) -> &'static str {
    match status {
        ChangeStatus::Added => "added",
        ChangeStatus::Modified => "modified",
        ChangeStatus::Deleted => "deleted",
    }
}

fn compact_command_label(command: &str) -> &str {
    const MAX_INLINE_COMMAND: usize = 80;
    if command.len() > MAX_INLINE_COMMAND || command.lines().count() > 1 {
        "command"
    } else {
        command
    }
}

fn display_path(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn render_unified_file_patch(
    path: &str,
    status: &ChangeStatus,
    old_content: &[u8],
    new_content: &[u8],
) -> String {
    let old_text = String::from_utf8_lossy(old_content);
    let new_text = String::from_utf8_lossy(new_content);
    let old_lines = split_patch_lines(&old_text);
    let new_lines = split_patch_lines(&new_text);
    let old_count = old_lines.len();
    let new_count = new_lines.len();
    let old_range = if old_count == 0 {
        "0,0".to_owned()
    } else {
        format!("1,{old_count}")
    };
    let new_range = if new_count == 0 {
        "0,0".to_owned()
    } else {
        format!("1,{new_count}")
    };
    let old_header = match status {
        ChangeStatus::Added => "/dev/null".to_owned(),
        ChangeStatus::Modified | ChangeStatus::Deleted => format!("a/{path}"),
    };
    let new_header = match status {
        ChangeStatus::Deleted => "/dev/null".to_owned(),
        ChangeStatus::Added | ChangeStatus::Modified => format!("b/{path}"),
    };
    let mut patch = format!("diff --git a/{path} b/{path}\n");
    match status {
        ChangeStatus::Added => patch.push_str("new file mode 100644\n"),
        ChangeStatus::Deleted => patch.push_str("deleted file mode 100644\n"),
        ChangeStatus::Modified => {}
    }
    patch.push_str(&format!(
        "--- {old_header}\n+++ {new_header}\n@@ -{old_range} +{new_range} @@\n"
    ));

    for line in old_lines {
        patch.push('-');
        patch.push_str(line);
        patch.push('\n');
    }
    for line in new_lines {
        patch.push('+');
        patch.push_str(line);
        patch.push('\n');
    }
    patch
}

fn split_patch_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.strip_suffix('\n')
            .unwrap_or(text)
            .split('\n')
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;
    use tempfile::tempdir;

    #[test]
    fn init_creates_repo_layout() {
        let dir = tempdir().unwrap();

        let manifest = AnvicsStore::init(dir.path()).unwrap();

        assert_eq!(manifest.format_version, FORMAT_VERSION);
        assert!(dir.path().join(".anvics/repo.json").exists());
        assert!(dir.path().join(".anvics/objects/blake3").exists());
        assert!(dir.path().join(".anvics/snapshots").exists());
        assert!(dir.path().join(".anvics/agent-packets").exists());
        assert!(dir.path().join(".anvics/events").exists());
        assert!(dir.path().join(".anvics/sessions").exists());
        assert!(dir.path().join(".anvics/command-events").exists());
        assert!(dir.path().join(".anvics/artifacts/commands").exists());
        assert!(dir.path().join(".anvics/risks").exists());
        assert!(dir.path().join(".anvics/policy-overrides").exists());
        assert_eq!(
            AnvicsStore::open(dir.path())
                .unwrap()
                .events_since(0)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn snapshot_stores_blobs_and_manifest() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "hello").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();

        let snapshot = store.create_snapshot(Some("initial".to_owned())).unwrap();

        assert!(store.object_exists(&snapshot.root_tree));
        assert!(dir
            .path()
            .join(format!(".anvics/snapshots/{}.json", snapshot.id))
            .exists());
        assert_eq!(
            fs::read_to_string(dir.path().join(".anvics/HEAD")).unwrap(),
            snapshot.id.as_str()
        );
    }

    #[test]
    fn unchanged_tree_has_stable_root_tree() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "same").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();

        let first = store.create_snapshot(None).unwrap();
        let second = store.create_snapshot(None).unwrap();

        assert_eq!(first.root_tree, second.root_tree);
        assert_ne!(first.id, second.id);
    }

    #[test]
    fn snapshot_skips_internal_and_junk_paths() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("keep.txt"), "keep").unwrap();
        fs::write(dir.path().join(".DS_Store"), "junk").unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::write(dir.path().join(".git/config"), "git").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();

        let snapshot = store.create_snapshot(None).unwrap();
        let tree_bytes = fs::read(store.object_path(&snapshot.root_tree)).unwrap();
        let tree: Tree = serde_json::from_slice(&tree_bytes).unwrap();

        assert_eq!(tree.entries.len(), 1);
        assert_eq!(tree.entries[0].name, "keep.txt");
    }

    #[test]
    fn object_ids_deduplicate_same_content() {
        let dir = tempdir().unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();

        let first = store.store_object(b"same").unwrap();
        let second = store.store_object(b"same").unwrap();

        assert_eq!(first, second);
        assert!(store.object_exists(&first));
    }

    #[test]
    fn mutation_events_are_append_only() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();

        store.create_snapshot(Some("base".to_owned())).unwrap();
        let thread = store
            .create_thread("events".to_owned(), "record mutations".to_owned())
            .unwrap();
        let workspace = store.create_workspace(thread.id.as_str()).unwrap();

        let events = store.events_since(0).unwrap();

        assert_eq!(events.len(), 4);
        assert_eq!(
            events
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(events[3].subject_id, Some(workspace.id.to_string()));
        assert!(store
            .events_since(2)
            .unwrap()
            .iter()
            .all(|event| event.sequence > 2));
    }

    #[test]
    fn agent_enter_refreshes_existing_session_and_records_events() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Agent".to_owned(), "Edit app".to_owned())
            .unwrap();

        let first = store
            .enter_agent_session(preparation.workspace.id.as_str(), "codex-cli".to_owned())
            .unwrap();
        let second = store
            .enter_agent_session(preparation.workspace.id.as_str(), "codex-cli".to_owned())
            .unwrap();

        let first_session = first.current_session.unwrap();
        let second_session = second.current_session.unwrap();
        assert_eq!(first_session.id, second_session.id);
        assert_eq!(
            store
                .list_agent_sessions(None, Some(preparation.workspace.id.as_str()))
                .unwrap()
                .len(),
            1
        );
        let events = store.events_since(0).unwrap();
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::AgentSessionEntered));
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::AgentSessionSeen));
    }

    #[test]
    fn coordination_status_reports_unknown_related_workspace() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let first = store
            .prepare_agent("Agent A".to_owned(), "Change app".to_owned())
            .unwrap();
        let second = store
            .prepare_agent("Agent B".to_owned(), "Also change app".to_owned())
            .unwrap();
        store
            .enter_agent_session(first.workspace.id.as_str(), "codex-a".to_owned())
            .unwrap();
        store
            .enter_agent_session(second.workspace.id.as_str(), "codex-b".to_owned())
            .unwrap();

        let status = store
            .coordination_status(first.workspace.id.as_str())
            .unwrap();

        assert_eq!(status.related_work.len(), 1);
        assert_eq!(
            status.related_work[0].freshness_note,
            "unknown changes possible until snapshot/finish"
        );
        assert!(status
            .potential_clash_notes
            .iter()
            .any(|note| note.contains("unknown changes possible")));
    }

    #[test]
    fn coordination_status_reports_known_path_overlap() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let first = store
            .prepare_agent("Agent A".to_owned(), "Change app".to_owned())
            .unwrap();
        let second = store
            .prepare_agent("Agent B".to_owned(), "Also change app".to_owned())
            .unwrap();
        store
            .enter_agent_session(first.workspace.id.as_str(), "codex-a".to_owned())
            .unwrap();
        store
            .enter_agent_session(second.workspace.id.as_str(), "codex-b".to_owned())
            .unwrap();
        fs::write(
            Path::new(&first.workspace.materialized_path).join("app.txt"),
            "agent a\n",
        )
        .unwrap();
        fs::write(
            Path::new(&second.workspace.materialized_path).join("app.txt"),
            "agent b\n",
        )
        .unwrap();
        store
            .workspace_snapshot(first.workspace.id.as_str(), Some("a".to_owned()))
            .unwrap();
        store
            .workspace_snapshot(second.workspace.id.as_str(), Some("b".to_owned()))
            .unwrap();

        let status = store
            .coordination_status(first.workspace.id.as_str())
            .unwrap();

        assert_eq!(status.known_changed_paths, vec!["app.txt".to_owned()]);
        assert_eq!(
            status.related_work[0].overlap_paths,
            vec!["app.txt".to_owned()]
        );
        assert!(status
            .potential_clash_notes
            .iter()
            .any(|note| note.contains("app.txt")));
    }

    #[test]
    fn restore_snapshot_recreates_files() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        let snapshot = store.create_snapshot(Some("base".to_owned())).unwrap();
        let restored = dir.path().join("restored");

        store
            .restore_snapshot_to_path(snapshot.id.as_str(), &restored)
            .unwrap();

        assert_eq!(
            fs::read_to_string(restored.join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );
    }

    #[test]
    fn diff_snapshots_reports_added_modified_deleted_paths() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("modified.txt"), "before").unwrap();
        fs::write(dir.path().join("deleted.txt"), "gone soon").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        let base = store.create_snapshot(Some("base".to_owned())).unwrap();
        fs::write(dir.path().join("modified.txt"), "after").unwrap();
        fs::remove_file(dir.path().join("deleted.txt")).unwrap();
        fs::write(dir.path().join("added.txt"), "new").unwrap();
        let final_snapshot = store.create_snapshot(Some("final".to_owned())).unwrap();

        let diff = store.diff_snapshots(&base.id, &final_snapshot.id).unwrap();

        assert!(diff.contains(&ChangedPath {
            path: "added.txt".to_owned(),
            status: ChangeStatus::Added,
        }));
        assert!(diff.contains(&ChangedPath {
            path: "modified.txt".to_owned(),
            status: ChangeStatus::Modified,
        }));
        assert!(diff.contains(&ChangedPath {
            path: "deleted.txt".to_owned(),
            status: ChangeStatus::Deleted,
        }));
    }

    #[test]
    fn workspace_diff_reports_current_changes_without_snapshot() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
        fs::write(dir.path().join("deleted.txt"), "gone\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Workspace diff".to_owned(), "Change files".to_owned())
            .unwrap();
        let workspace_path = Path::new(&preparation.workspace.materialized_path);
        fs::write(workspace_path.join("modified.txt"), "after\n").unwrap();
        fs::remove_file(workspace_path.join("deleted.txt")).unwrap();
        fs::write(workspace_path.join("added.txt"), "new\n").unwrap();

        let diff = store
            .workspace_diff(preparation.workspace.id.as_str())
            .unwrap();

        assert_eq!(
            diff,
            vec![
                ChangedPath {
                    path: "added.txt".to_owned(),
                    status: ChangeStatus::Added,
                },
                ChangedPath {
                    path: "deleted.txt".to_owned(),
                    status: ChangeStatus::Deleted,
                },
                ChangedPath {
                    path: "modified.txt".to_owned(),
                    status: ChangeStatus::Modified,
                },
            ]
        );
        assert!(store
            .show_workspace(preparation.workspace.id.as_str())
            .unwrap()
            .latest_snapshot
            .is_none());
    }

    #[test]
    fn workspace_diff_patch_applies_to_clean_base() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
        fs::write(dir.path().join("deleted.txt"), "gone\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Workspace patch".to_owned(), "Change files".to_owned())
            .unwrap();
        let workspace_path = Path::new(&preparation.workspace.materialized_path);
        fs::write(workspace_path.join("modified.txt"), "after\n").unwrap();
        fs::remove_file(workspace_path.join("deleted.txt")).unwrap();
        fs::write(workspace_path.join("added.txt"), "new\n").unwrap();
        let patch = store
            .workspace_diff_patch(preparation.workspace.id.as_str())
            .unwrap();
        let patch_path = dir.path().join("workspace.patch");
        fs::write(&patch_path, patch).unwrap();

        let clean = tempdir().unwrap();
        fs::write(clean.path().join("modified.txt"), "before\n").unwrap();
        fs::write(clean.path().join("deleted.txt"), "gone\n").unwrap();
        StdCommand::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(clean.path())
            .status()
            .unwrap();
        let apply = StdCommand::new("git")
            .args(["apply", patch_path.to_str().unwrap()])
            .current_dir(clean.path())
            .output()
            .unwrap();
        assert!(
            apply.status.success(),
            "git apply failed: {}",
            String::from_utf8_lossy(&apply.stderr)
        );
        assert_eq!(
            fs::read_to_string(clean.path().join("modified.txt")).unwrap(),
            "after\n"
        );
        assert_eq!(
            fs::read_to_string(clean.path().join("added.txt")).unwrap(),
            "new\n"
        );
        assert!(!clean.path().join("deleted.txt").exists());
    }

    #[test]
    fn workspace_snapshot_writes_overlay_manifest() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
        fs::write(dir.path().join("deleted.txt"), "remove\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Overlay".to_owned(), "Change files".to_owned())
            .unwrap();
        let workspace = Path::new(&preparation.workspace.materialized_path);
        fs::write(workspace.join("modified.txt"), "after\n").unwrap();
        fs::remove_file(workspace.join("deleted.txt")).unwrap();
        fs::write(workspace.join("added.txt"), "new\n").unwrap();

        let workspace = store
            .workspace_snapshot(preparation.workspace.id.as_str(), Some("result".to_owned()))
            .unwrap();
        let overlay = store
            .workspace_overlay(preparation.workspace.id.as_str())
            .unwrap()
            .unwrap();

        assert_eq!(overlay.workspace_id, workspace.id);
        assert_eq!(overlay.base_snapshot, preparation.thread.base_snapshot);
        assert_eq!(overlay.snapshot, workspace.latest_snapshot.unwrap());
        assert_eq!(
            overlay
                .entries
                .iter()
                .map(|entry| (entry.path.clone(), entry.status.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("added.txt".to_owned(), ChangeStatus::Added),
                ("deleted.txt".to_owned(), ChangeStatus::Deleted),
                ("modified.txt".to_owned(), ChangeStatus::Modified),
            ]
        );
        assert!(overlay.entries.iter().any(|entry| {
            entry.path == "added.txt" && entry.object.is_some() && entry.size == Some(4)
        }));
        assert!(overlay.entries.iter().any(|entry| {
            entry.path == "deleted.txt" && entry.object.is_none() && entry.size.is_none()
        }));
    }

    #[test]
    fn file_effects_propose_source_relevant_change_units() {
        let effects = propose_file_effect_set(
            &[
                ChangedPath {
                    path: "src/lib.rs".to_owned(),
                    status: ChangeStatus::Modified,
                },
                ChangedPath {
                    path: "target/debug/anvics".to_owned(),
                    status: ChangeStatus::Added,
                },
                ChangedPath {
                    path: "Cargo.lock".to_owned(),
                    status: ChangeStatus::Modified,
                },
                ChangedPath {
                    path: "evidence/verify.log".to_owned(),
                    status: ChangeStatus::Added,
                },
                ChangedPath {
                    path: "assets/logo.png".to_owned(),
                    status: ChangeStatus::Modified,
                },
            ],
            &RepositoryConfig::default(),
        )
        .unwrap();

        assert_eq!(effects.effects.len(), 5);
        assert_eq!(effects.effects[0].labels, vec![FileEffectLabel::Source]);
        assert_eq!(effects.effects[1].labels, vec![FileEffectLabel::Cache]);
        assert_eq!(effects.effects[2].labels, vec![FileEffectLabel::Lockfile]);
        assert_eq!(
            effects.effects[3].labels,
            vec![FileEffectLabel::EvidenceCandidate]
        );
        assert_eq!(effects.effects[4].labels, vec![FileEffectLabel::Binary]);

        let change_units = propose_change_units(&effects);
        assert_eq!(
            change_units
                .iter()
                .map(|unit| (unit.path.as_str(), unit.labels.as_slice()))
                .collect::<Vec<_>>(),
            vec![
                ("src/lib.rs", &[FileEffectLabel::Source][..]),
                ("Cargo.lock", &[FileEffectLabel::Lockfile][..]),
                ("assets/logo.png", &[FileEffectLabel::Binary][..]),
            ]
        );
        assert!(change_units
            .iter()
            .all(|unit| unit.provenance == FileEffectProvenance::Heuristic));
    }

    #[test]
    fn anvics_toml_classifies_review_change_units_without_self_reinterpretation() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("anvics.toml"),
            "[generated]\ntracked = [\"src/generated/**\"]\nuntracked = [\"dist/**\"]\n\n[ignore]\npaths = [\"cache/**\"]\n\n[evidence]\ncandidate_paths = [\"reports/**\"]\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("src/generated")).unwrap();
        fs::write(
            dir.path().join("src/generated/client.rs"),
            "old generated\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join("dist")).unwrap();
        fs::write(dir.path().join("dist/bundle.js"), "old bundle\n").unwrap();
        fs::create_dir_all(dir.path().join("cache")).unwrap();
        fs::write(dir.path().join("cache/result.txt"), "old cache\n").unwrap();
        fs::create_dir_all(dir.path().join("reports")).unwrap();
        fs::write(dir.path().join("reports/test.txt"), "old report\n").unwrap();
        fs::write(dir.path().join("src.rs"), "old source\n").unwrap();

        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent(
                "Config labels".to_owned(),
                "Exercise anvics.toml".to_owned(),
            )
            .unwrap();
        let workspace = Path::new(&preparation.workspace.materialized_path);
        fs::write(workspace.join("src/generated/client.rs"), "new generated\n").unwrap();
        fs::write(workspace.join("dist/bundle.js"), "new bundle\n").unwrap();
        fs::write(workspace.join("cache/result.txt"), "new cache\n").unwrap();
        fs::write(workspace.join("reports/test.txt"), "new report\n").unwrap();
        fs::write(workspace.join("src.rs"), "new source\n").unwrap();
        fs::write(
            workspace.join("anvics.toml"),
            "[ignore]\npaths = [\"src.rs\"]\n",
        )
        .unwrap();

        store
            .workspace_snapshot(preparation.workspace.id.as_str(), Some("result".to_owned()))
            .unwrap();
        let review = store.create_review(preparation.thread.id.as_str()).unwrap();

        assert!(review
            .changed_paths
            .iter()
            .any(|path| path.path == "dist/bundle.js"));
        assert!(review
            .changed_paths
            .iter()
            .any(|path| path.path == "reports/test.txt"));
        assert!(review.file_effects.iter().any(|effect| {
            effect.path == "dist/bundle.js"
                && effect.labels == vec![FileEffectLabel::GeneratedUntracked]
        }));
        assert!(review.file_effects.iter().any(|effect| {
            effect.path == "reports/test.txt"
                && effect.labels == vec![FileEffectLabel::EvidenceCandidate]
        }));

        let units = review
            .change_units
            .iter()
            .map(|unit| (unit.path.as_str(), unit.labels.as_slice()))
            .collect::<Vec<_>>();
        assert!(units.contains(&("src.rs", &[FileEffectLabel::Source][..])));
        assert!(units.contains(&(
            "src/generated/client.rs",
            &[FileEffectLabel::GeneratedTracked][..]
        )));
        assert!(units.contains(&("anvics.toml", &[FileEffectLabel::Config][..])));
        assert!(!review
            .change_units
            .iter()
            .any(|unit| unit.path == "dist/bundle.js"));
        assert!(!review
            .change_units
            .iter()
            .any(|unit| unit.path == "cache/result.txt"));
        assert!(!review
            .change_units
            .iter()
            .any(|unit| unit.path == "reports/test.txt"));

        let markdown = store.review_markdown(review.id.as_str()).unwrap();
        assert!(markdown.contains("## File Effects"));
        assert!(markdown.contains("`src/generated/client.rs` (generated_tracked)"));
        assert!(markdown.contains("generated source may need rationale"));
        assert!(markdown.contains("`reports/test.txt` (evidence_candidate)"));
        assert!(markdown.contains("evidence candidate; attach compactly if useful"));
        assert!(markdown.contains("`dist/bundle.js` (generated_untracked)"));
        assert!(markdown.contains("excluded from ChangeUnits"));
        assert!(markdown.contains("`src.rs` (source)"));
        assert!(
            !markdown.contains("`dist/bundle.js` (generated_untracked) - modified dist/bundle.js")
        );
    }

    #[test]
    fn repo_doctor_reports_config_and_classifies_paths() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("anvics.toml"),
            "[generated]\ntracked = [\"src/generated/**\"]\nuntracked = [\"dist/**\"]\n\n[ignore]\npaths = [\"cache/**\"]\n\n[evidence]\ncandidate_paths = [\"reports/**\"]\n",
        )
        .unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();

        let report = store
            .repo_doctor(vec![
                "src/generated/client.rs".to_owned(),
                "dist/bundle.js".to_owned(),
                "reports/test.txt".to_owned(),
                "src/lib.rs".to_owned(),
            ])
            .unwrap();

        assert!(report.config_present);
        assert!(report
            .config_path
            .as_deref()
            .unwrap()
            .ends_with("anvics.toml"));
        assert_eq!(report.generated_tracked, vec!["src/generated/**"]);
        assert_eq!(report.generated_untracked, vec!["dist/**"]);
        assert_eq!(report.ignore_paths, vec!["cache/**"]);
        assert_eq!(report.evidence_candidate_paths, vec!["reports/**"]);
        assert_eq!(
            report
                .classified_paths
                .iter()
                .map(|path| (path.path.as_str(), path.labels.as_slice()))
                .collect::<Vec<_>>(),
            vec![
                (
                    "src/generated/client.rs",
                    &[FileEffectLabel::GeneratedTracked][..]
                ),
                ("dist/bundle.js", &[FileEffectLabel::GeneratedUntracked][..]),
                (
                    "reports/test.txt",
                    &[FileEffectLabel::EvidenceCandidate][..]
                ),
                ("src/lib.rs", &[FileEffectLabel::Source][..]),
            ]
        );
        assert!(report
            .notes
            .iter()
            .any(|note| note.contains("accepted repo-root anvics.toml")));
    }

    #[test]
    fn restore_workspace_from_overlay_recreates_composed_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
        fs::write(dir.path().join("deleted.txt"), "remove\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Overlay restore".to_owned(), "Change files".to_owned())
            .unwrap();
        let workspace = Path::new(&preparation.workspace.materialized_path);
        fs::write(workspace.join("modified.txt"), "after\n").unwrap();
        fs::remove_file(workspace.join("deleted.txt")).unwrap();
        fs::write(workspace.join("added.txt"), "new\n").unwrap();
        store
            .workspace_snapshot(preparation.workspace.id.as_str(), Some("result".to_owned()))
            .unwrap();
        let restored = dir.path().join("restored");

        store
            .restore_workspace_from_overlay(preparation.workspace.id.as_str(), &restored)
            .unwrap();

        assert_eq!(
            fs::read_to_string(restored.join("modified.txt")).unwrap(),
            "after\n"
        );
        assert_eq!(
            fs::read_to_string(restored.join("added.txt")).unwrap(),
            "new\n"
        );
        assert!(!restored.join("deleted.txt").exists());
    }

    #[test]
    fn evidence_rejects_empty_summary() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "hello").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let thread = store
            .create_thread("test".to_owned(), "do a thing".to_owned())
            .unwrap();

        let err = store
            .attach_evidence(
                thread.id.as_str(),
                "true".to_owned(),
                0,
                "   ".to_owned(),
                None,
            )
            .unwrap_err();

        assert!(matches!(err, StoreError::EmptyEvidenceSummary));
    }

    #[test]
    fn secret_detectors_redact_without_storing_raw_values() {
        let openai_line = "OPENAI_API_KEY=sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
        let github_line = "token=github_pat_1234567890abcdefghijklmnopqrstuvwxyz";
        let aws_line = "AWS_ACCESS_KEY_ID=AKIA1234567890ABCDEF";
        let env_line = "DATABASE_PASSWORD=correct-horse-battery";
        let private_key_line = "-----BEGIN PRIVATE KEY-----";

        assert!(secret_detectors_for_line(openai_line).contains(&"openai_token".to_owned()));
        assert!(secret_detectors_for_line(github_line).contains(&"github_token".to_owned()));
        assert!(secret_detectors_for_line(aws_line).contains(&"aws_access_key_id".to_owned()));
        assert!(secret_detectors_for_line(env_line).contains(&"env_secret_assignment".to_owned()));
        assert!(
            secret_detectors_for_line(private_key_line).contains(&"private_key_block".to_owned())
        );

        let redacted = redact_line(openai_line);
        assert!(redacted.contains("<redacted:"));
        assert!(!redacted.contains("sk-proj-1234567890abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn command_run_records_event_artifacts_and_evidence() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Command run".to_owned(), "Verify app".to_owned())
            .unwrap();

        let result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["sh".to_owned(), "-c".to_owned(), "cat app.txt".to_owned()],
                command_file: None,
                command_label: "verify app".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Read app.txt".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(result.command_event.exit_code, Some(0));
        assert_eq!(
            result.evidence.command_event_id,
            Some(result.command_event.id.clone())
        );
        assert!(result.command_event.stdout_path.is_some());
        assert_eq!(
            fs::read_to_string(result.command_event.stdout_path.unwrap()).unwrap(),
            "base\n"
        );
        let event = store
            .show_command_event(result.command_event.id.as_str())
            .unwrap();
        assert_eq!(event.command_label, "verify app");
        let events = store.events_since(0).unwrap();
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::CommandStarted));
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::CommandFinished));
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::EvidenceAttached));
    }

    #[test]
    fn command_run_rejects_cwd_outside_workspace() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Command run".to_owned(), "Verify app".to_owned())
            .unwrap();

        let err = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["true".to_owned()],
                command_file: None,
                command_label: "verify".to_owned(),
                cwd: Some(dir.path().to_string_lossy().to_string()),
                timeout_seconds: Some(10),
                summary: "ok".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap_err();

        assert!(matches!(err, StoreError::InvalidCommandCwd(_)));
    }

    #[test]
    fn projection_resolution_returns_materialized_workspace() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Projection".to_owned(), "Resolve workspace".to_owned())
            .unwrap();

        let projection = store
            .resolve_workspace_projection(
                &preparation.workspace,
                &ProjectionRequest::MaterializedDir,
                None,
                "test-command",
            )
            .unwrap();
        let projection = projection.projection;

        assert_eq!(projection.workspace_id, preparation.workspace.id);
        assert_eq!(projection.thread_id, preparation.thread.id);
        assert_eq!(projection.kind, ProjectionKind::MaterializedDir);
        assert_eq!(
            projection.root_path,
            Path::new(&preparation.workspace.materialized_path)
                .canonicalize()
                .unwrap()
        );
        assert_eq!(
            projection.capabilities,
            ProjectionCapabilities {
                readable: true,
                writable: true,
                file_effects: true,
            }
        );

        let missing = store.run_command(CommandRunInput {
            workspace_id: "missing-workspace".to_owned(),
            argv: vec!["true".to_owned()],
            command_file: None,
            command_label: "missing".to_owned(),
            cwd: None,
            timeout_seconds: Some(10),
            summary: "Missing workspace".to_owned(),
            artifact_path: None,
            projection: ProjectionRequest::MaterializedDir,
            mount_root: None,
            allow_command_risk: false,
            command_risk_reason: None,
        });
        assert!(matches!(missing, Err(StoreError::WorkspaceNotFound(_))));
    }

    #[test]
    fn auto_projection_falls_back_without_fuse_feature() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Auto projection".to_owned(), "Read file".to_owned())
            .unwrap();

        let result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["cat".to_owned(), "app.txt".to_owned()],
                command_file: None,
                command_label: "read app".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Read app through auto projection".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::Auto,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        #[cfg(not(feature = "vfs-fuse"))]
        {
            assert_eq!(
                result.command_event.projection_kind,
                Some(ProjectionKind::MaterializedDir)
            );
            assert!(result
                .command_event
                .projection_fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("FUSE support is not compiled"));
            assert!(result.command_event.runtime_metrics.is_some());
        }
        #[cfg(feature = "vfs-fuse")]
        {
            assert!(result.command_event.projection_kind.is_some());
        }
    }

    #[test]
    fn command_run_records_projection_and_file_effects() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        fs::write(dir.path().join("delete.txt"), "delete\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Projection effects".to_owned(), "Edit files".to_owned())
            .unwrap();

        let result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec![
                    "sh".to_owned(),
                    "-c".to_owned(),
                    "printf 'changed\\n' > app.txt && rm delete.txt && printf 'new\\n' > added.txt"
                        .to_owned(),
                ],
                command_file: None,
                command_label: "edit files".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Edited files through projection".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(
            result.command_event.projection_kind,
            Some(ProjectionKind::MaterializedDir)
        );
        assert_eq!(
            result.command_event.projection_root,
            Some(
                Path::new(&preparation.workspace.materialized_path)
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            )
        );
        assert_eq!(
            result.command_event.file_effects,
            vec![
                ChangedPath {
                    path: "added.txt".to_owned(),
                    status: ChangeStatus::Added,
                },
                ChangedPath {
                    path: "app.txt".to_owned(),
                    status: ChangeStatus::Modified,
                },
                ChangedPath {
                    path: "delete.txt".to_owned(),
                    status: ChangeStatus::Deleted,
                },
            ]
        );
        assert_eq!(
            result.command_event.projection_capabilities,
            Some(ProjectionCapabilities {
                readable: true,
                writable: true,
                file_effects: true,
            })
        );
        assert_eq!(
            result.command_event.command_policy_class,
            Some(CommandPolicyClass::Destructive)
        );
        let metrics = result.command_event.runtime_metrics.as_ref().unwrap();
        assert_eq!(metrics.projection_files, 2);
        assert_eq!(metrics.projection_bytes, 12);
        assert_eq!(metrics.command_ms, result.command_event.duration_ms);
        assert_eq!(
            result.evidence.command_event_id,
            Some(result.command_event.id)
        );
    }

    #[test]
    fn failed_command_run_records_projection_file_effects_and_evidence() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent(
                "Failed projection effects".to_owned(),
                "Edit then fail".to_owned(),
            )
            .unwrap();

        let result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec![
                    "sh".to_owned(),
                    "-c".to_owned(),
                    "printf 'changed\\n' > app.txt; exit 7".to_owned(),
                ],
                command_file: None,
                command_label: "edit then fail".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Edited app.txt before failing".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(result.command_event.exit_code, Some(7));
        assert_eq!(
            result.command_event.projection_kind,
            Some(ProjectionKind::MaterializedDir)
        );
        assert_eq!(
            result.command_event.projection_capabilities,
            Some(ProjectionCapabilities {
                readable: true,
                writable: true,
                file_effects: true,
            })
        );
        assert_eq!(
            result.command_event.command_policy_class,
            Some(CommandPolicyClass::Mutating)
        );
        let metrics = result.command_event.runtime_metrics.as_ref().unwrap();
        assert_eq!(metrics.projection_files, 1);
        assert_eq!(metrics.projection_bytes, 5);
        assert_eq!(metrics.command_ms, result.command_event.duration_ms);
        assert_eq!(
            result.command_event.file_effects,
            vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }]
        );
        assert_eq!(result.evidence.exit_code, 7);
    }

    #[test]
    fn command_policy_classifier_reports_coarse_classes() {
        assert_eq!(
            classify_argv_policy(&["cat".to_owned(), "app.txt".to_owned()]),
            CommandPolicyClass::ReadOnly
        );
        assert_eq!(
            classify_argv_policy(&[
                "sh".to_owned(),
                "-c".to_owned(),
                "printf x > app.txt".to_owned()
            ]),
            CommandPolicyClass::Mutating
        );
        assert_eq!(
            classify_argv_policy(&["rm".to_owned(), "app.txt".to_owned()]),
            CommandPolicyClass::Destructive
        );
        assert_eq!(
            classify_argv_policy(&["mystery-tool".to_owned()]),
            CommandPolicyClass::Unknown
        );
    }

    #[test]
    fn command_policy_blocks_risky_classes_without_artifacts() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Policy".to_owned(), "Check risky command".to_owned())
            .unwrap();

        for (argv, expected) in [
            (
                vec!["curl".to_owned(), "https://example.com".to_owned()],
                CommandPolicyClass::Networked,
            ),
            (
                vec!["docker".to_owned(), "ps".to_owned()],
                CommandPolicyClass::HostEscapeRisk,
            ),
            (vec!["vim".to_owned()], CommandPolicyClass::Interactive),
        ] {
            let err = store
                .run_command(CommandRunInput {
                    workspace_id: preparation.workspace.id.to_string(),
                    argv,
                    command_file: None,
                    command_label: "risky".to_owned(),
                    cwd: None,
                    timeout_seconds: Some(10),
                    summary: "Risky command".to_owned(),
                    artifact_path: None,
                    projection: ProjectionRequest::MaterializedDir,
                    mount_root: None,
                    allow_command_risk: false,
                    command_risk_reason: None,
                })
                .unwrap_err();
            assert!(matches!(
                err,
                StoreError::CommandPolicyBlocked { policy_class } if policy_class == expected
            ));
        }

        assert!(
            read_json_dir::<CommandEvent>(store.anvics_dir.join("command-events"))
                .unwrap()
                .is_empty()
        );
        assert!(
            read_json_dir::<EvidenceRecord>(store.anvics_dir.join("evidence"))
                .unwrap()
                .is_empty()
        );
        assert!(store.events_since(0).unwrap().iter().all(|event| {
            event.kind != RepositoryEventKind::CommandStarted
                && event.kind != RepositoryEventKind::EvidenceAttached
        }));
    }

    #[test]
    fn command_file_policy_blocks_risky_contents_without_artifacts() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        let command_file = dir.path().join("verify.sh");
        fs::write(&command_file, "curl https://example.com\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Policy".to_owned(), "Check command file".to_owned())
            .unwrap();

        let decision = classify_command_policy(CommandPolicyInput {
            argv: Vec::new(),
            command_file: Some(command_file.to_string_lossy().to_string()),
        })
        .unwrap();
        assert_eq!(decision.policy_class, CommandPolicyClass::Networked);
        assert!(decision.blocked);
        assert_eq!(
            decision.override_hint.as_deref(),
            Some("--allow-command-risk --command-risk-reason <reason>")
        );

        let err = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: Vec::new(),
                command_file: Some(command_file.to_string_lossy().to_string()),
                command_label: "risky file".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Risky command file".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap_err();
        assert!(matches!(
            err,
            StoreError::CommandPolicyBlocked {
                policy_class: CommandPolicyClass::Networked
            }
        ));
        assert!(
            read_json_dir::<CommandEvent>(store.anvics_dir.join("command-events"))
                .unwrap()
                .is_empty()
        );
        assert!(
            read_json_dir::<EvidenceRecord>(store.anvics_dir.join("evidence"))
                .unwrap()
                .is_empty()
        );
        let command_artifact_root = store.anvics_dir.join("artifacts/commands");
        assert!(
            !command_artifact_root.exists()
                || fs::read_dir(command_artifact_root)
                    .unwrap()
                    .next()
                    .is_none()
        );
    }

    #[test]
    fn command_policy_override_requires_reason_and_records_metadata() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Policy".to_owned(), "Allow risky command".to_owned())
            .unwrap();

        let empty_reason = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["git".to_owned(), "--version".to_owned()],
                command_file: None,
                command_label: "git version".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Allowed git version".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: true,
                command_risk_reason: Some("   ".to_owned()),
            })
            .unwrap_err();
        assert!(matches!(empty_reason, StoreError::EmptyCommandRiskReason));

        let reason_without_flag = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["true".to_owned()],
                command_file: None,
                command_label: "true".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Reason without flag".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: Some("not needed".to_owned()),
            })
            .unwrap_err();
        assert!(matches!(
            reason_without_flag,
            StoreError::CommandRiskReasonWithoutOverride
        ));

        let result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["git".to_owned(), "--version".to_owned()],
                command_file: None,
                command_label: "git version".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Allowed git version".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: true,
                command_risk_reason: Some("Operator approved version check".to_owned()),
            })
            .unwrap();

        assert_eq!(
            result.command_event.command_policy_class,
            Some(CommandPolicyClass::Networked)
        );
        assert_eq!(
            result
                .command_event
                .command_policy_override_reason
                .as_deref(),
            Some("Operator approved version check")
        );

        let command_file = dir.path().join("git-version.sh");
        fs::write(&command_file, "git --version\n").unwrap();
        let command_file_result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: Vec::new(),
                command_file: Some(command_file.to_string_lossy().to_string()),
                command_label: "git version file".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Allowed git version command file".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: true,
                command_risk_reason: Some("Operator approved command file".to_owned()),
            })
            .unwrap();
        assert_eq!(
            command_file_result.command_event.command_policy_class,
            Some(CommandPolicyClass::Networked)
        );
        assert_eq!(
            command_file_result
                .command_event
                .command_policy_override_reason
                .as_deref(),
            Some("Operator approved command file")
        );
    }

    #[test]
    fn command_run_timeout_records_failed_command_event() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Command run".to_owned(), "Verify app".to_owned())
            .unwrap();

        let result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["sh".to_owned(), "-c".to_owned(), "sleep 2".to_owned()],
                command_file: None,
                command_label: "slow".to_owned(),
                cwd: None,
                timeout_seconds: Some(0),
                summary: "Timed out".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::MaterializedDir,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(result.command_event.exit_code, Some(-1));
        assert!(result.command_event.timed_out);
    }

    #[cfg(feature = "vfs-fuse")]
    #[test]
    fn fuse_projection_remounts_readback_and_cleans_up_mount_dir() {
        if std::env::var("ANVICS_RUN_FUSE_TESTS").ok().as_deref() != Some("1") {
            eprintln!("skipping real FUSE store test; set ANVICS_RUN_FUSE_TESTS=1 to run it");
            return;
        }

        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        fs::write(dir.path().join("delete.txt"), "delete\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Fuse remount".to_owned(), "Edit through mount".to_owned())
            .unwrap();

        let first = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec![
                    "sh".to_owned(),
                    "-c".to_owned(),
                    "printf 'changed\\n' > app.txt && printf 'new\\n' > added.txt && rm delete.txt"
                        .to_owned(),
                ],
                command_file: None,
                command_label: "fuse edit".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Edited through FUSE".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::FuseMount,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(
            first.command_event.projection_kind,
            Some(ProjectionKind::FuseMount)
        );
        assert_eq!(first.command_event.projection_fallback_reason, None);
        assert!(!Path::new(first.command_event.projection_root.as_ref().unwrap()).exists());
        assert_eq!(
            fs::read_to_string(Path::new(&preparation.workspace.materialized_path).join("app.txt"))
                .unwrap(),
            "changed\n"
        );
        assert!(Path::new(&preparation.workspace.materialized_path)
            .join("added.txt")
            .exists());
        assert!(!Path::new(&preparation.workspace.materialized_path)
            .join("delete.txt")
            .exists());

        let second = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec![
                    "sh".to_owned(),
                    "-c".to_owned(),
                    "cat app.txt && test -f added.txt && test ! -e delete.txt".to_owned(),
                ],
                command_file: None,
                command_label: "fuse readback".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Read persisted FUSE changes".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::FuseMount,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(second.command_event.exit_code, Some(0));
        assert!(second.command_event.file_effects.is_empty());
        assert_eq!(
            fs::read_to_string(second.command_event.stdout_path.as_ref().unwrap()).unwrap(),
            "changed\n"
        );
        assert!(!Path::new(second.command_event.projection_root.as_ref().unwrap()).exists());

        let auto = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec!["cat".to_owned(), "app.txt".to_owned()],
                command_file: None,
                command_label: "auto fuse readback".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "Auto used FUSE when available".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::Auto,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(
            auto.command_event.projection_kind,
            Some(ProjectionKind::FuseMount)
        );
        assert_eq!(auto.command_event.projection_fallback_reason, None);
        assert!(auto.command_event.runtime_metrics.is_some());
        assert!(!Path::new(auto.command_event.projection_root.as_ref().unwrap()).exists());
    }

    #[cfg(feature = "vfs-fuse")]
    #[test]
    fn failed_fuse_command_persists_writes_and_records_evidence() {
        if std::env::var("ANVICS_RUN_FUSE_TESTS").ok().as_deref() != Some("1") {
            eprintln!("skipping real FUSE store test; set ANVICS_RUN_FUSE_TESTS=1 to run it");
            return;
        }

        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Fuse failure".to_owned(), "Edit then fail".to_owned())
            .unwrap();

        let result = store
            .run_command(CommandRunInput {
                workspace_id: preparation.workspace.id.to_string(),
                argv: vec![
                    "sh".to_owned(),
                    "-c".to_owned(),
                    "printf 'changed before failure\\n' > app.txt; exit 7".to_owned(),
                ],
                command_file: None,
                command_label: "fuse edit then fail".to_owned(),
                cwd: None,
                timeout_seconds: Some(10),
                summary: "FUSE command wrote before failing".to_owned(),
                artifact_path: None,
                projection: ProjectionRequest::FuseMount,
                mount_root: None,
                allow_command_risk: false,
                command_risk_reason: None,
            })
            .unwrap();

        assert_eq!(result.command_event.exit_code, Some(7));
        assert_eq!(
            result.command_event.projection_kind,
            Some(ProjectionKind::FuseMount)
        );
        assert_eq!(
            result.command_event.file_effects,
            vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }]
        );
        assert_eq!(result.evidence.exit_code, 7);
        assert!(result.command_event.runtime_metrics.is_some());
        assert_eq!(
            fs::read_to_string(Path::new(&preparation.workspace.materialized_path).join("app.txt"))
                .unwrap(),
            "changed before failure\n"
        );
        assert!(!Path::new(result.command_event.projection_root.as_ref().unwrap()).exists());
    }

    #[test]
    fn review_reports_path_overlap_between_threads() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let first = store
            .create_thread("first".to_owned(), "change app".to_owned())
            .unwrap();
        let second = store
            .create_thread("second".to_owned(), "also change app".to_owned())
            .unwrap();
        let first_workspace = store.create_workspace(first.id.as_str()).unwrap();
        let second_workspace = store.create_workspace(second.id.as_str()).unwrap();

        fs::write(
            Path::new(&first_workspace.materialized_path).join("app.txt"),
            "first\n",
        )
        .unwrap();
        fs::write(
            Path::new(&second_workspace.materialized_path).join("app.txt"),
            "second\n",
        )
        .unwrap();
        store
            .workspace_snapshot(first_workspace.id.as_str(), Some("first result".to_owned()))
            .unwrap();
        store
            .workspace_snapshot(
                second_workspace.id.as_str(),
                Some("second result".to_owned()),
            )
            .unwrap();

        let review = store.create_review(first.id.as_str()).unwrap();

        assert_eq!(review.changed_paths.len(), 1);
        assert!(review
            .overlap_notes
            .iter()
            .any(|note| note.contains("app.txt")));
    }

    #[test]
    fn publication_marks_thread_published() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let thread = store
            .create_thread("publish".to_owned(), "change app".to_owned())
            .unwrap();
        let workspace = store.create_workspace(thread.id.as_str()).unwrap();
        fs::write(
            Path::new(&workspace.materialized_path).join("app.txt"),
            "published\n",
        )
        .unwrap();
        store
            .workspace_snapshot(workspace.id.as_str(), Some("result".to_owned()))
            .unwrap();
        let review = store.create_review(thread.id.as_str()).unwrap();

        let publication = store
            .create_publication(thread.id.as_str(), review.id.as_str())
            .unwrap();

        assert_eq!(publication.review_id, review.id);
        assert_eq!(
            store.show_thread(thread.id.as_str()).unwrap().status,
            WorkThreadStatus::Published
        );
    }

    #[test]
    fn risk_scan_finds_changed_source_and_blocks_publication_until_override() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Risky source".to_owned(), "Add config".to_owned())
            .unwrap();
        let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("config.env"),
            format!("OPENAI_API_KEY={secret}\n"),
        )
        .unwrap();
        let finish = store
            .finish_agent(
                preparation.workspace.id.as_str(),
                "manual check".to_owned(),
                0,
                "Created config fixture".to_owned(),
                None,
            )
            .unwrap();

        let scan = store.scan_review_risks(finish.review.id.as_str()).unwrap();
        assert!(!scan.findings.is_empty());
        assert!(scan
            .findings
            .iter()
            .any(|finding| finding.detector == "openai_token"));
        assert!(scan.findings.iter().all(|finding| {
            finding.target_kind == RiskTargetKind::SourceFile
                && finding.target_path == "config.env"
                && !finding.redacted_excerpt.contains(secret)
        }));

        let err = store
            .create_publication(preparation.thread.id.as_str(), finish.review.id.as_str())
            .unwrap_err();
        assert!(matches!(
            err,
            StoreError::PublicationBlockedSecretRisk { finding_count } if finding_count >= 1
        ));
        let status = store.agent_status(preparation.thread.id.as_str()).unwrap();
        assert!(status.publication_ids.is_empty());

        let publication = store
            .create_publication_with_options(
                preparation.thread.id.as_str(),
                finish.review.id.as_str(),
                PublicationOptions {
                    allow_secret_risk: true,
                    override_reason: Some("fixture secret is intentional".to_owned()),
                },
            )
            .unwrap();
        assert_eq!(publication.review_id, finish.review.id);
        let markdown = store.review_markdown(finish.review.id.as_str()).unwrap();
        assert!(markdown.contains("Risk Notes"));
        assert!(markdown.contains("openai_token"));
        assert!(markdown.contains("fixture secret is intentional"));
        assert!(!markdown.contains(secret));
        let events = store.events_since(0).unwrap();
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::RiskScanCreated));
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::SecretRiskDetected));
        assert!(events
            .iter()
            .any(|event| event.kind == RepositoryEventKind::PolicyOverrideRecorded));
    }

    #[test]
    fn unchanged_source_secret_fixture_does_not_block_unrelated_modified_line() {
        let dir = tempdir().unwrap();
        let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
        fs::write(
            dir.path().join("config.rs"),
            format!("const FIXTURE: &str = \"{secret}\";\nfn marker() {{}}\n"),
        )
        .unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Safe fixture edit".to_owned(), "Edit marker".to_owned())
            .unwrap();
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("config.rs"),
            format!("const FIXTURE: &str = \"{secret}\";\nfn marker() {{ println!(\"safe\"); }}\n"),
        )
        .unwrap();
        let finish = store
            .finish_agent(
                preparation.workspace.id.as_str(),
                "manual check".to_owned(),
                0,
                "Changed non-secret code".to_owned(),
                None,
            )
            .unwrap();

        let scan = store.scan_review_risks(finish.review.id.as_str()).unwrap();

        assert!(scan.findings.is_empty());
        let publication = store
            .create_publication(preparation.thread.id.as_str(), finish.review.id.as_str())
            .unwrap();
        assert_eq!(publication.review_id, finish.review.id);
    }

    #[test]
    fn introduced_secret_in_modified_source_still_blocks_publication() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("config.rs"), "fn marker() {}\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Risky modified source".to_owned(), "Add fixture".to_owned())
            .unwrap();
        let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("config.rs"),
            format!("fn marker() {{}}\nconst LEAK: &str = \"{secret}\";\n"),
        )
        .unwrap();
        let finish = store
            .finish_agent(
                preparation.workspace.id.as_str(),
                "manual check".to_owned(),
                0,
                "Added secret fixture".to_owned(),
                None,
            )
            .unwrap();

        let err = store
            .create_publication(preparation.thread.id.as_str(), finish.review.id.as_str())
            .unwrap_err();

        assert!(matches!(
            err,
            StoreError::PublicationBlockedSecretRisk { finding_count } if finding_count >= 1
        ));
        let findings = store
            .list_review_risk_findings(finish.review.id.as_str())
            .unwrap();
        assert!(findings
            .iter()
            .any(|finding| finding.detector == "openai_token" && finding.line == Some(2)));
    }

    #[test]
    fn secret_in_added_source_file_still_blocks_publication() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Risky added source".to_owned(), "Add config".to_owned())
            .unwrap();
        let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("config.env"),
            format!("OPENAI_API_KEY={secret}\n"),
        )
        .unwrap();
        let finish = store
            .finish_agent(
                preparation.workspace.id.as_str(),
                "manual check".to_owned(),
                0,
                "Added config fixture".to_owned(),
                None,
            )
            .unwrap();

        let err = store
            .create_publication(preparation.thread.id.as_str(), finish.review.id.as_str())
            .unwrap_err();

        assert!(matches!(
            err,
            StoreError::PublicationBlockedSecretRisk { finding_count } if finding_count >= 1
        ));
    }

    #[test]
    fn external_evidence_artifact_secret_blocks_publication() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        fs::create_dir(dir.path().join("artifacts")).unwrap();
        fs::write(
            dir.path().join("artifacts/leak.txt"),
            "GH_TOKEN=ghp_1234567890abcdefghijklmnopqrstuvwxyz\n",
        )
        .unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Artifact risk".to_owned(), "Edit app".to_owned())
            .unwrap();
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("app.txt"),
            "safe change\n",
        )
        .unwrap();
        let finish = store
            .finish_agent(
                preparation.workspace.id.as_str(),
                "cat artifacts/leak.txt".to_owned(),
                0,
                "Linked verification artifact".to_owned(),
                Some("artifacts/leak.txt".to_owned()),
            )
            .unwrap();

        let err = store
            .create_publication(preparation.thread.id.as_str(), finish.review.id.as_str())
            .unwrap_err();

        assert!(matches!(
            err,
            StoreError::PublicationBlockedSecretRisk { finding_count } if finding_count >= 1
        ));
        let findings = store
            .list_review_risk_findings(finish.review.id.as_str())
            .unwrap();
        assert!(findings
            .iter()
            .any(|finding| finding.target_kind == RiskTargetKind::EvidenceArtifact));
    }

    #[test]
    fn agent_prepare_writes_task_packet() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();

        let preparation = store
            .prepare_agent("Live agent".to_owned(), "Edit app.txt".to_owned())
            .unwrap();

        let packet = fs::read_to_string(&preparation.packet_path).unwrap();
        assert!(packet.contains(preparation.thread.id.as_str()));
        assert!(packet.contains(preparation.workspace.id.as_str()));
        assert!(packet.contains(&preparation.workspace.materialized_path));
        assert!(packet.contains("Edit app.txt"));
        assert!(packet.contains("only editable area"));
        assert!(packet.contains("anvics --repo"));
        assert!(packet.contains("agent enter"));
        assert!(packet.contains("coordination status"));
        assert!(packet.contains("agent accept"));
        assert!(packet.contains("agent finish"));
        assert!(packet.contains("If you spawn subagents"));
        assert!(packet.contains("## Agent-Run Commands"));
        assert!(packet.contains("## Operator-Run Commands"));
        assert!(packet.contains("Do not run them as an agent"));
    }

    #[test]
    fn agent_finish_attaches_evidence_snapshots_and_reviews() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Live agent".to_owned(), "Edit app.txt".to_owned())
            .unwrap();
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("app.txt"),
            "live agent\n",
        )
        .unwrap();

        let finish = store
            .finish_agent(
                preparation.workspace.id.as_str(),
                "manual agent".to_owned(),
                0,
                "Edited app.txt".to_owned(),
                Some("artifacts/test.log".to_owned()),
            )
            .unwrap();

        assert_eq!(finish.evidence.thread_id, preparation.thread.id);
        assert_eq!(
            finish.evidence.artifact_path,
            Some("artifacts/test.log".to_owned())
        );
        assert_eq!(finish.review.thread_id, preparation.thread.id);
        assert!(finish.workspace.latest_snapshot.is_some());
        let markdown = fs::read_to_string(finish.review_markdown_path).unwrap();
        assert!(markdown.contains("Live agent"));
        assert!(markdown.contains("Edit app.txt"));
        assert!(markdown.contains("Edited app.txt"));
        assert!(markdown.contains("anvics --repo"));
        assert!(markdown.contains("agent accept"));
        assert!(markdown.contains("publish create"));

        let status = store.agent_status(preparation.thread.id.as_str()).unwrap();
        assert_eq!(status.evidence_count, 1);
        assert_eq!(status.review_ids, vec![finish.review.id]);
        assert!(status.publication_ids.is_empty());
    }

    #[test]
    fn legacy_git_patch_export_covers_added_modified_deleted_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
        fs::write(dir.path().join("deleted.txt"), "delete me\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Patch export".to_owned(), "Change three files".to_owned())
            .unwrap();
        let workspace = Path::new(&preparation.workspace.materialized_path);
        fs::write(workspace.join("modified.txt"), "after\n").unwrap();
        fs::remove_file(workspace.join("deleted.txt")).unwrap();
        fs::write(workspace.join("added.txt"), "new\n").unwrap();
        let finish = store
            .finish_agent(
                preparation.workspace.id.as_str(),
                "script".to_owned(),
                0,
                "Changed three files".to_owned(),
                None,
            )
            .unwrap();
        let publication = store
            .create_publication(preparation.thread.id.as_str(), finish.review.id.as_str())
            .unwrap();
        let output = dir.path().join("accepted.patch");

        store
            .export_legacy_git_patch(publication.id.as_str(), &output)
            .unwrap();

        let patch = fs::read_to_string(output).unwrap();
        assert!(patch.contains("Anvics-Publication"));
        assert!(patch.contains("diff --git a/added.txt b/added.txt"));
        assert!(patch.contains("diff --git a/modified.txt b/modified.txt"));
        assert!(patch.contains("diff --git a/deleted.txt b/deleted.txt"));
    }

    #[test]
    fn agent_accept_stores_review_publication_and_patch() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Accept agent".to_owned(), "Edit app.txt".to_owned())
            .unwrap();
        let session = store
            .enter_agent_session(preparation.workspace.id.as_str(), "codex-cli".to_owned())
            .unwrap()
            .current_session
            .unwrap();
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("app.txt"),
            "accepted\n",
        )
        .unwrap();

        let acceptance = store
            .accept_agent(
                preparation.workspace.id.as_str(),
                "cat app.txt".to_owned(),
                0,
                "Verified accepted app.txt".to_owned(),
                Some("artifacts/accept.log".to_owned()),
                None,
            )
            .unwrap();

        assert_eq!(acceptance.evidence.thread_id, preparation.thread.id);
        assert_eq!(
            acceptance.evidence.artifact_path,
            Some("artifacts/accept.log".to_owned())
        );
        assert_eq!(acceptance.review.thread_id, preparation.thread.id);
        assert_eq!(acceptance.publication.review_id, acceptance.review.id);
        assert_eq!(
            acceptance.patch_path,
            dir.path().join("accepted.patch").display().to_string()
        );
        assert!(dir.path().join("accepted.patch").exists());
        assert_eq!(
            store
                .show_thread(preparation.thread.id.as_str())
                .unwrap()
                .status,
            WorkThreadStatus::Published
        );
        let sessions = store
            .list_agent_sessions(None, Some(preparation.workspace.id.as_str()))
            .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session.id);
        assert_eq!(sessions[0].status, AgentSessionStatus::Finished);
    }

    #[test]
    fn agent_accept_with_command_run_publishes_only_on_success() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Accept command run".to_owned(), "Edit app.txt".to_owned())
            .unwrap();
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("app.txt"),
            "accepted\n",
        )
        .unwrap();

        let acceptance = store
            .accept_agent_with_command_run(
                CommandRunInput {
                    workspace_id: preparation.workspace.id.to_string(),
                    argv: vec![
                        "sh".to_owned(),
                        "-c".to_owned(),
                        "grep accepted app.txt".to_owned(),
                    ],
                    command_file: None,
                    command_label: "verify accepted".to_owned(),
                    cwd: None,
                    timeout_seconds: Some(10),
                    summary: "Verified accepted app.txt".to_owned(),
                    artifact_path: None,
                    projection: ProjectionRequest::MaterializedDir,
                    mount_root: None,
                    allow_command_risk: false,
                    command_risk_reason: None,
                },
                None,
            )
            .unwrap();

        assert!(acceptance.evidence.command_event_id.is_some());
        assert_eq!(acceptance.publication.review_id, acceptance.review.id);
        let markdown = fs::read_to_string(acceptance.review_markdown_path).unwrap();
        assert!(markdown.contains("anvics-run:"));
        assert!(markdown.contains("stdout:"));
        assert!(dir.path().join("accepted.patch").exists());
    }

    #[test]
    fn agent_accept_with_failed_command_run_records_evidence_without_publication() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Accept command run".to_owned(), "Edit app.txt".to_owned())
            .unwrap();

        let err = store
            .accept_agent_with_command_run(
                CommandRunInput {
                    workspace_id: preparation.workspace.id.to_string(),
                    argv: vec!["false".to_owned()],
                    command_file: None,
                    command_label: "verify failure".to_owned(),
                    cwd: None,
                    timeout_seconds: Some(10),
                    summary: "Verification failed".to_owned(),
                    artifact_path: None,
                    projection: ProjectionRequest::MaterializedDir,
                    mount_root: None,
                    allow_command_risk: false,
                    command_risk_reason: None,
                },
                None,
            )
            .unwrap_err();

        assert!(matches!(err, StoreError::CommandFailed { .. }));
        let status = store.agent_status(preparation.thread.id.as_str()).unwrap();
        assert_eq!(status.evidence_count, 1);
        assert!(status.review_ids.is_empty());
        assert!(status.publication_ids.is_empty());
    }

    #[test]
    fn agent_accept_with_command_run_blocks_on_secret_stdout() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("app.txt"), "base\n").unwrap();
        AnvicsStore::init(dir.path()).unwrap();
        let store = AnvicsStore::open(dir.path()).unwrap();
        store.create_snapshot(Some("base".to_owned())).unwrap();
        let preparation = store
            .prepare_agent("Accept command run".to_owned(), "Edit app.txt".to_owned())
            .unwrap();
        fs::write(
            Path::new(&preparation.workspace.materialized_path).join("app.txt"),
            "accepted\n",
        )
        .unwrap();
        let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";

        let err = store
            .accept_agent_with_command_run_and_options(
                CommandRunInput {
                    workspace_id: preparation.workspace.id.to_string(),
                    argv: vec![
                        "sh".to_owned(),
                        "-c".to_owned(),
                        format!("printf 'OPENAI_API_KEY={secret}\\n'"),
                    ],
                    command_file: None,
                    command_label: "leaky verify".to_owned(),
                    cwd: None,
                    timeout_seconds: Some(10),
                    summary: "Command emitted fixture secret".to_owned(),
                    artifact_path: None,
                    projection: ProjectionRequest::MaterializedDir,
                    mount_root: None,
                    allow_command_risk: false,
                    command_risk_reason: None,
                },
                None,
                PublicationOptions::default(),
            )
            .unwrap_err();

        assert!(matches!(
            err,
            StoreError::PublicationBlockedSecretRisk { finding_count } if finding_count >= 1
        ));
        let status = store.agent_status(preparation.thread.id.as_str()).unwrap();
        assert_eq!(status.evidence_count, 1);
        assert_eq!(status.review_ids.len(), 1);
        assert!(status.publication_ids.is_empty());
        let markdown = store
            .review_markdown(status.review_ids[0].as_str())
            .unwrap();
        assert!(markdown.contains("CommandStdout"));
        assert!(markdown.contains("openai_token"));
        assert!(!markdown.contains(secret));
    }
}
