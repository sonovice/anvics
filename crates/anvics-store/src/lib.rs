use anvics_core::{
    AgentAcceptance, AgentFinish, AgentPreparation, AgentStatus, ChangeStatus, ChangedPath,
    EvidenceRecord, EvidenceRecordId, EvidenceSummary, NativePublication, NativePublicationId,
    ObjectId, RepositoryId, RepositoryManifest, ReviewProjection, ReviewProjectionId,
    SourceSnapshot, SourceSnapshotId, Tree, TreeEntry, TreeEntryKind, WorkThread, WorkThreadId,
    WorkThreadStatus, WorkspaceView, WorkspaceViewId,
};
use ignore::WalkBuilder;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

const ANVICS_DIR: &str = ".anvics";
const FORMAT_VERSION: u32 = 1;

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
    Walk(#[from] ignore::Error),
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
    pub command_label: Option<String>,
    pub command_file: Option<String>,
    pub cwd: Option<String>,
    pub exit_code: i32,
    pub summary: String,
    pub artifact_path: Option<String>,
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

        let manifest = RepositoryManifest {
            id: RepositoryId::new(),
            format_version: FORMAT_VERSION,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(&repo_json, &manifest)?;
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
        Ok(workspace)
    }

    pub fn show_workspace(&self, id: &str) -> Result<WorkspaceView> {
        let path = self.workspace_path(id);
        if !path.exists() {
            return Err(StoreError::WorkspaceNotFound(id.to_owned()));
        }
        read_json(path)
    }

    pub fn workspace_snapshot(&self, id: &str, message: Option<String>) -> Result<WorkspaceView> {
        let mut workspace = self.show_workspace(id)?;
        let snapshot =
            self.create_snapshot_from_path(&workspace.materialized_path, message, false)?;
        workspace.latest_snapshot = Some(snapshot.id);
        write_json_pretty(self.workspace_path(id), &workspace)?;
        Ok(workspace)
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
                command_label: None,
                command_file: None,
                cwd: None,
                exit_code,
                summary,
                artifact_path,
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
            command: input.command,
            command_label: input.command_label,
            command_file: input.command_file,
            cwd: input.cwd,
            exit_code: input.exit_code,
            summary: input.summary,
            artifact_path: input.artifact_path,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.evidence_path(evidence.id.as_str()), &evidence)?;
        Ok(evidence)
    }

    pub fn create_review(&self, thread_id: &str) -> Result<ReviewProjection> {
        let thread = self.show_thread(thread_id)?;
        let final_snapshot = self
            .latest_thread_snapshot(&thread.id)?
            .ok_or_else(|| StoreError::MissingWorkspaceSnapshot(thread.id.to_string()))?;
        let changed_paths = self.diff_snapshots(&thread.base_snapshot, &final_snapshot)?;
        let evidence = self.thread_evidence(&thread.id)?;
        let overlap_notes = self.overlap_notes(&thread, &changed_paths)?;

        let review = ReviewProjection {
            id: ReviewProjectionId::new(),
            thread_id: thread.id.clone(),
            base_snapshot: thread.base_snapshot.clone(),
            final_snapshot,
            changed_paths,
            overlap_notes,
            evidence,
            created_at: now_rfc3339()?,
        };
        write_json_pretty(self.review_path(review.id.as_str()), &review)?;
        fs::write(
            self.review_markdown_path(review.id.as_str()),
            render_review(&self.root, &review, &thread),
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

    pub fn create_publication(
        &self,
        thread_id: &str,
        review_id: &str,
    ) -> Result<NativePublication> {
        let mut thread = self.show_thread(thread_id)?;
        let review = self.show_review(review_id)?;
        if review.thread_id != thread.id {
            return Err(StoreError::ReviewThreadMismatch {
                review_id: review_id.to_owned(),
                thread_id: thread_id.to_owned(),
            });
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
                command_label: None,
                command_file: None,
                cwd: None,
                exit_code,
                summary,
                artifact_path,
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
        let workspace =
            self.workspace_snapshot(workspace_id, Some("agent finish result".to_owned()))?;
        let review = self.create_review(workspace.thread_id.as_str())?;
        let review_markdown_path = self
            .review_markdown_path(review.id.as_str())
            .to_string_lossy()
            .to_string();

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
                command_label: None,
                command_file: None,
                cwd: None,
                exit_code,
                summary,
                artifact_path,
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
        let finish = self.finish_agent_with_evidence(workspace_id, evidence_input)?;
        let publication = self.create_publication(
            finish.workspace.thread_id.as_str(),
            finish.review.id.as_str(),
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
        Ok(output.to_path_buf())
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

    fn agent_packet_path(&self, thread_id: &str) -> PathBuf {
        self.anvics_dir
            .join("agent-packets")
            .join(format!("{thread_id}.md"))
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

    fn latest_thread_snapshot(&self, thread_id: &WorkThreadId) -> Result<Option<SourceSnapshotId>> {
        Ok(self
            .list_workspaces()?
            .into_iter()
            .filter(|workspace| &workspace.thread_id == thread_id)
            .filter_map(|workspace| workspace.latest_snapshot)
            .next_back())
    }

    fn thread_evidence(&self, thread_id: &WorkThreadId) -> Result<Vec<EvidenceSummary>> {
        let mut records: Vec<EvidenceRecord> = read_json_dir(self.anvics_dir.join("evidence"))?;
        records.retain(|record| &record.thread_id == thread_id);
        records.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(records
            .into_iter()
            .map(|record| EvidenceSummary {
                id: record.id,
                command: record.command,
                command_label: record.command_label,
                command_file: record.command_file,
                cwd: record.cwd,
                exit_code: record.exit_code,
                summary: record.summary,
                artifact_path: record.artifact_path,
            })
            .collect())
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
            let Some(other_final) = self.latest_thread_snapshot(&other.id)? else {
                continue;
            };
            let other_changed = self.diff_snapshots(&other.base_snapshot, &other_final)?;
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

fn render_agent_packet(repo_root: &Path, thread: &WorkThread, workspace: &WorkspaceView) -> String {
    let repo = shell_quote(&display_path(repo_root));
    let workspace_path = shell_quote(&workspace.materialized_path);
    format!(
        "# Anvics Agent Task\n\nThread: `{}`\nWorkspace: `{}`\nRepository: `{}`\nWorkspace path: `{}`\n\n## Task\n\n{}\n\n## Instructions\n\n- Work only inside the workspace path above. This workspace is the only editable area for this task.\n- Do not edit the repository root, `.anvics/` metadata, another workspace, a Git branch, a Git worktree, or a Git commit.\n- Keep command output compact.\n- When finished, report a concise command label, exit code, short summary, and optional command/evidence file path.\n\n## Workspace\n\n```sh\ncd {workspace_path}\n```\n\n## Operator Acceptance Command Template\n\n```sh\nanvics --repo {repo} agent accept --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"\n```\n\nFor multi-command verification, write the commands to a small file and use `--command-file <path> --label \"<short label>\"` instead of a long inline command.\n\n## Manual Finish Command Template\n\n```sh\nanvics --repo {repo} agent finish --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"\n```\n\nAdd `--artifact <path>` only when you created a compact artifact worth linking.\n",
        thread.id,
        workspace.id,
        repo_root.display(),
        workspace.materialized_path,
        thread.task,
        workspace.id,
        workspace.id
    )
}

fn render_review(repo_root: &Path, review: &ReviewProjection, thread: &WorkThread) -> String {
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

    markdown.push_str("\n## Evidence\n\n");
    if review.evidence.is_empty() {
        markdown.push_str("- No evidence attached.\n");
    } else {
        for evidence in &review.evidence {
            markdown.push_str(&format!("- {}\n", render_evidence_summary(evidence)));
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

    markdown.push_str("\n## Next Commands\n\n");
    markdown.push_str("Shortest path for an unaccepted workspace:\n\n");
    markdown.push_str(&format!(
        "```sh\nanvics --repo {repo} agent accept --workspace <workspace-id> --command \"<command>\" --exit-code <code> --summary \"<short summary>\"\n```\n\n"
    ));
    markdown.push_str("Manual path from this review:\n\n");
    markdown.push_str(&format!(
        "```sh\nanvics --repo {repo} review show {} --format markdown\nanvics --repo {repo} publish create --thread {} --review {}\nanvics --repo {repo} legacy git export --publication <publication-id> --output accepted.patch\n```\n",
        review.id,
        review.thread_id, review.id
    ));

    markdown
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
    text
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
        assert!(packet.contains("agent accept"));
        assert!(packet.contains("agent finish"));
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
    }
}
