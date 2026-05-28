use anvics_api::{ApiMethod, ApiRequest, ApiResponse, ApiResult, ReviewFormat};
use anvics_store::{AnvicsStore, CommandEvidenceInput, StoreError};
use anyhow::{Context, Result};
use clap::Parser;
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
};

#[derive(Debug, Parser)]
#[command(name = "anvicsd")]
#[command(about = "Local Anvics JSON-RPC daemon")]
struct Cli {
    #[arg(long, value_name = "PATH")]
    socket: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.socket.exists() {
        fs::remove_file(&cli.socket)
            .with_context(|| format!("failed to remove stale socket {}", cli.socket.display()))?;
    }
    let listener = UnixListener::bind(&cli.socket)
        .with_context(|| format!("failed to bind socket {}", cli.socket.display()))?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_client(stream) {
                    eprintln!("anvicsd client error: {error:?}");
                }
            }
            Err(error) => eprintln!("anvicsd accept error: {error:?}"),
        }
    }

    Ok(())
}

fn handle_client(stream: UnixStream) -> Result<()> {
    let mut writer = stream.try_clone().context("failed to clone socket")?;
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<ApiRequest>(&line) {
            Ok(request) => handle_request(request),
            Err(error) => ApiResponse::error(0, format!("invalid request: {error}")),
        };
        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }

    Ok(())
}

fn handle_request(request: ApiRequest) -> ApiResponse {
    let id = request.id;
    match run_request(request) {
        Ok(result) => ApiResponse::ok(id, result),
        Err(error) => ApiResponse::error(id, format!("{error:?}")),
    }
}

fn run_request(request: ApiRequest) -> Result<ApiResult> {
    let repo = PathBuf::from(&request.repo);
    match request.method {
        ApiMethod::RepoInit => {
            let manifest = match AnvicsStore::init(&repo) {
                Ok(manifest) => manifest,
                Err(StoreError::AlreadyInitialized(_)) => AnvicsStore::open(&repo)?.manifest()?,
                Err(error) => return Err(error.into()),
            };
            Ok(ApiResult::RepoInit { manifest })
        }
        ApiMethod::RepoStatus => {
            let status = match AnvicsStore::open(&repo) {
                Ok(store) => ApiResult::RepoStatus {
                    initialized: true,
                    manifest: Some(store.manifest()?),
                },
                Err(StoreError::NotRepository(_)) => ApiResult::RepoStatus {
                    initialized: false,
                    manifest: None,
                },
                Err(error) => return Err(error.into()),
            };
            Ok(status)
        }
        ApiMethod::SnapshotCreate { message } => {
            let snapshot = AnvicsStore::open(&repo)?.create_snapshot(message)?;
            Ok(ApiResult::SnapshotCreate { snapshot })
        }
        ApiMethod::SnapshotList => {
            let snapshots = AnvicsStore::open(&repo)?.list_snapshots()?;
            Ok(ApiResult::SnapshotList { snapshots })
        }
        ApiMethod::SnapshotShow { id } => {
            let snapshot = AnvicsStore::open(&repo)?.show_snapshot(&id)?;
            Ok(ApiResult::SnapshotShow { snapshot })
        }
        ApiMethod::AgentPrepare { title, task } => {
            let preparation = AnvicsStore::open(&repo)?.prepare_agent(title, task)?;
            Ok(ApiResult::AgentPrepare {
                preparation: Box::new(preparation),
            })
        }
        ApiMethod::AgentStatus { thread } => {
            let status = AnvicsStore::open(&repo)?.agent_status(&thread)?;
            Ok(ApiResult::AgentStatus {
                status: Box::new(status),
            })
        }
        ApiMethod::AgentAccept {
            workspace,
            command,
            command_file,
            command_label,
            cwd,
            exit_code,
            summary,
            artifact_path,
            output_path,
        } => {
            let command_text = match (&command, &command_file) {
                (Some(command), _) => command.clone(),
                (None, Some(path)) => fs::read_to_string(path)
                    .with_context(|| format!("failed to read command file {path}"))?,
                (None, None) => anyhow::bail!("either command or command_file is required"),
            };
            let acceptance = AnvicsStore::open(&repo)?.accept_agent_with_evidence(
                &workspace,
                CommandEvidenceInput {
                    command: command_text,
                    command_label,
                    command_file,
                    cwd,
                    exit_code,
                    summary,
                    artifact_path,
                },
                output_path.map(PathBuf::from),
            )?;
            Ok(ApiResult::AgentAccept {
                acceptance: Box::new(acceptance),
            })
        }
        ApiMethod::ReviewShow { id, format } => match format {
            ReviewFormat::Json => {
                let review = AnvicsStore::open(&repo)?.show_review(&id)?;
                Ok(ApiResult::ReviewShowJson {
                    review: Box::new(review),
                })
            }
            ReviewFormat::Markdown => {
                let markdown = AnvicsStore::open(&repo)?.review_markdown(&id)?;
                Ok(ApiResult::ReviewShowMarkdown { markdown })
            }
        },
        ApiMethod::LegacyGitExport {
            publication,
            output,
        } => {
            let output = AnvicsStore::open(&repo)?
                .export_legacy_git_patch(&publication, output)?
                .to_string_lossy()
                .to_string();
            Ok(ApiResult::LegacyGitExport { output })
        }
        ApiMethod::EventsSince { sequence } => {
            let events = AnvicsStore::open(&repo)?.events_since(sequence)?;
            Ok(ApiResult::EventsSince { events })
        }
    }
}
