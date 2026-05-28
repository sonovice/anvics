use anvics_core::{
    AgentAcceptance, AgentPreparation, AgentStatus, NativePublication, RepositoryManifest,
    ReviewProjection, SourceSnapshot,
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
    RepoInit,
    RepoStatus,
    SnapshotCreate {
        message: Option<String>,
    },
    SnapshotList,
    SnapshotShow {
        id: String,
    },
    AgentPrepare {
        title: String,
        task: String,
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
    },
    ReviewShow {
        id: String,
        format: ReviewFormat,
    },
    LegacyGitExport {
        publication: String,
        output: String,
    },
    EventsSince {
        sequence: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewFormat {
    Json,
    Markdown,
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
    RepoInit {
        manifest: RepositoryManifest,
    },
    RepoStatus {
        initialized: bool,
        manifest: Option<RepositoryManifest>,
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
    AgentPrepare {
        preparation: Box<AgentPreparation>,
    },
    AgentStatus {
        status: Box<AgentStatus>,
    },
    AgentAccept {
        acceptance: Box<AgentAcceptance>,
    },
    ReviewShowJson {
        review: Box<ReviewProjection>,
    },
    ReviewShowMarkdown {
        markdown: String,
    },
    LegacyGitExport {
        output: String,
    },
    EventsSince {
        events: Vec<anvics_core::RepositoryEvent>,
    },
    Publication {
        publication: NativePublication,
    },
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_request_round_trips_as_json() {
        let request = ApiRequest {
            id: 7,
            repo: "/tmp/repo".to_owned(),
            method: ApiMethod::AgentStatus {
                thread: "thread-1".to_owned(),
            },
        };

        assert_eq!(
            serde_json::from_str::<ApiRequest>(&serde_json::to_string(&request).unwrap()).unwrap(),
            request
        );
    }
}
