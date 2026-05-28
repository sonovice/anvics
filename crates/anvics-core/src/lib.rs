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
opaque_id!(ReviewProjectionId);
opaque_id!(NativePublicationId);

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
pub struct EvidenceRecord {
    pub id: EvidenceRecordId,
    pub thread_id: WorkThreadId,
    pub command: String,
    pub exit_code: i32,
    pub summary: String,
    pub artifact_path: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct EvidenceSummary {
    pub id: EvidenceRecordId,
    pub command: String,
    pub exit_code: i32,
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ReviewProjection {
    pub id: ReviewProjectionId,
    pub thread_id: WorkThreadId,
    pub base_snapshot: SourceSnapshotId,
    pub final_snapshot: SourceSnapshotId,
    pub changed_paths: Vec<ChangedPath>,
    pub overlap_notes: Vec<String>,
    pub evidence: Vec<EvidenceSummary>,
    pub created_at: String,
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
pub struct NativePublication {
    pub id: NativePublicationId,
    pub thread_id: WorkThreadId,
    pub accepted_snapshot: SourceSnapshotId,
    pub review_id: ReviewProjectionId,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AgentPreparation {
    pub thread: WorkThread,
    pub workspace: WorkspaceView,
    pub packet_path: String,
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

        let thread = WorkThread {
            id: thread_id.clone(),
            title: "Agent task".to_owned(),
            task: "Edit a file".to_owned(),
            base_snapshot: base_snapshot.clone(),
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
        let evidence = EvidenceRecord {
            id: evidence_id.clone(),
            thread_id: thread_id.clone(),
            command: "cargo test".to_owned(),
            exit_code: 0,
            summary: "Tests passed".to_owned(),
            artifact_path: None,
            created_at: "2026-05-28T00:00:02Z".to_owned(),
        };
        let review = ReviewProjection {
            id: review_id.clone(),
            thread_id: thread_id.clone(),
            base_snapshot: base_snapshot.clone(),
            final_snapshot: final_snapshot.clone(),
            changed_paths: vec![ChangedPath {
                path: "app.txt".to_owned(),
                status: ChangeStatus::Modified,
            }],
            overlap_notes: vec!["No path overlap detected.".to_owned()],
            evidence: vec![EvidenceSummary {
                id: evidence_id,
                command: "cargo test".to_owned(),
                exit_code: 0,
                summary: "Tests passed".to_owned(),
            }],
            created_at: "2026-05-28T00:00:03Z".to_owned(),
        };
        let publication = NativePublication {
            id: NativePublicationId::new(),
            thread_id: thread_id.clone(),
            accepted_snapshot: final_snapshot,
            review_id,
            created_at: "2026-05-28T00:00:04Z".to_owned(),
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
            serde_json::from_str::<EvidenceRecord>(&serde_json::to_string(&evidence).unwrap())
                .unwrap(),
            evidence
        );
        assert_eq!(
            serde_json::from_str::<ReviewProjection>(&serde_json::to_string(&review).unwrap())
                .unwrap(),
            review
        );
        assert_eq!(
            serde_json::from_str::<NativePublication>(
                &serde_json::to_string(&publication).unwrap()
            )
            .unwrap(),
            publication
        );
        assert_eq!(
            serde_json::from_str::<AgentAcceptance>(&serde_json::to_string(&acceptance).unwrap())
                .unwrap(),
            acceptance
        );
    }
}
