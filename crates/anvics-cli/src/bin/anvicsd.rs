use anvics_api::{
    ApiMethod, ApiRequest, ApiResponse, ApiResult, ReviewFormat, ReviewInboxItem, API_VERSION,
};
use anvics_store::{
    classify_command_policy, AnvicsStore, CommandEvidenceInput, CommandPolicyInput,
    CommandRunInput, PublicationOptions, StoreError,
};
use anyhow::{Context, Result};
use clap::Parser;
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    thread,
};

#[derive(Debug, Parser)]
#[command(name = "anvicsd")]
#[command(about = "Local Anvics JSON-RPC daemon")]
struct Cli {
    #[arg(long, value_name = "PATH")]
    socket: PathBuf,

    #[arg(long, value_name = "HOST:PORT")]
    http: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.socket.exists() {
        if UnixStream::connect(&cli.socket).is_ok() {
            anyhow::bail!(
                "daemon socket {} is already serving a live daemon",
                cli.socket.display()
            );
        }
        fs::remove_file(&cli.socket).with_context(|| {
            format!(
                "failed to remove non-responsive stale socket {}",
                cli.socket.display()
            )
        })?;
    }
    let listener = UnixListener::bind(&cli.socket)
        .with_context(|| format!("failed to bind socket {}", cli.socket.display()))?;

    if let Some(http_addr) = cli.http.clone() {
        thread::spawn(move || {
            if let Err(error) = run_http_server(http_addr) {
                eprintln!("anvicsd http error: {error:?}");
            }
        });
    }

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

fn run_http_server(addr: String) -> Result<()> {
    let listener =
        TcpListener::bind(&addr).with_context(|| format!("failed to bind HTTP bridge {addr}"))?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_http_client(stream) {
                    eprintln!("anvicsd http client error: {error:?}");
                }
            }
            Err(error) => eprintln!("anvicsd http accept error: {error:?}"),
        }
    }
    Ok(())
}

fn handle_http_client(mut stream: TcpStream) -> Result<()> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(());
        }
        buffer.extend_from_slice(&chunk[..read]);
        if let Some(index) = find_header_end(&buffer) {
            break index;
        }
        if buffer.len() > 1024 * 1024 {
            anyhow::bail!("HTTP request headers too large");
        }
    };

    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_text.lines();
    let Some(request_line) = lines.next() else {
        write_http_json(
            &mut stream,
            400,
            None,
            br#"{"error":"missing request line"}"#,
        )?;
        return Ok(());
    };
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_owned();
    let target = request_parts.next().unwrap_or_default().to_owned();

    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let origin = headers
        .get("origin")
        .and_then(|origin| allowed_localhost_origin(origin));

    if method == "OPTIONS" {
        write_http_empty(&mut stream, 204, origin.as_deref())?;
        return Ok(());
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    let body = &buffer[body_start..buffer.len().min(body_start + content_length)];

    match (method.as_str(), target.split('?').next().unwrap_or(&target)) {
        ("GET", "/api/health") => {
            let payload = serde_json::json!({
                "ok": true,
                "version": API_VERSION,
            });
            let body = serde_json::to_vec(&payload)?;
            write_http_json(&mut stream, 200, origin.as_deref(), &body)?;
        }
        ("GET", "/api/review-inbox") => {
            let query = target.split_once('?').map(|(_, query)| query).unwrap_or("");
            let params = parse_query(query);
            let Some(repo) = params.get("repo").filter(|repo| !repo.is_empty()) else {
                write_http_json(
                    &mut stream,
                    400,
                    origin.as_deref(),
                    br#"{"error":"missing repo query parameter"}"#,
                )?;
                return Ok(());
            };
            let response = handle_request(ApiRequest {
                id: 1,
                repo: repo.clone(),
                method: ApiMethod::ReviewInbox,
            });
            let body = serde_json::to_vec(&response)?;
            write_http_json(&mut stream, 200, origin.as_deref(), &body)?;
        }
        ("POST", "/api/rpc") => {
            let response = match serde_json::from_slice::<ApiRequest>(body) {
                Ok(request) => handle_request(request),
                Err(error) => ApiResponse::error(0, format!("invalid request: {error}")),
            };
            let body = serde_json::to_vec(&response)?;
            write_http_json(&mut stream, 200, origin.as_deref(), &body)?;
        }
        ("GET", "/") => {
            let body = br#"{"ok":true,"message":"Anvics local HTTP bridge is running","endpoints":["/api/health","/api/review-inbox?repo=<path>","/api/rpc"]}"#;
            write_http_json(&mut stream, 200, origin.as_deref(), body)?;
        }
        _ => {
            write_http_json(
                &mut stream,
                404,
                origin.as_deref(),
                br#"{"error":"not found"}"#,
            )?;
        }
    }

    Ok(())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn allowed_localhost_origin(origin: &str) -> Option<String> {
    if origin.starts_with("http://localhost:")
        || origin.starts_with("http://127.0.0.1:")
        || origin.starts_with("http://[::1]:")
    {
        Some(origin.to_owned())
    } else {
        None
    }
}

fn parse_query(query: &str) -> BTreeMap<String, String> {
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            (percent_decode(key), percent_decode(value))
        })
        .collect()
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &value[index + 1..index + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte);
                    index += 3;
                } else {
                    decoded.push(bytes[index]);
                    index += 1;
                }
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn write_http_empty(stream: &mut TcpStream, status: u16, origin: Option<&str>) -> Result<()> {
    write_http_response(stream, status, origin, None, &[])
}

fn write_http_json(
    stream: &mut TcpStream,
    status: u16,
    origin: Option<&str>,
    body: &[u8],
) -> Result<()> {
    write_http_response(stream, status, origin, Some("application/json"), body)
}

fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    origin: Option<&str>,
    content_type: Option<&str>,
    body: &[u8],
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };
    write!(stream, "HTTP/1.1 {status} {reason}\r\n")?;
    if let Some(content_type) = content_type {
        write!(stream, "Content-Type: {content_type}\r\n")?;
    }
    if let Some(origin) = origin {
        write!(stream, "Access-Control-Allow-Origin: {origin}\r\n")?;
        write!(
            stream,
            "Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n"
        )?;
        write!(stream, "Access-Control-Allow-Headers: Content-Type\r\n")?;
    }
    write!(stream, "Content-Length: {}\r\n", body.len())?;
    write!(stream, "Connection: close\r\n\r\n")?;
    stream.write_all(body)?;
    stream.flush()?;
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
        Err(error) => ApiResponse::error(id, format!("{error:#}")),
    }
}

fn run_request(request: ApiRequest) -> Result<ApiResult> {
    let repo = PathBuf::from(&request.repo);
    match request.method {
        ApiMethod::Ping => Ok(ApiResult::Pong),
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
        ApiMethod::RepoDoctor { paths } => {
            let report = AnvicsStore::open(&repo)?.repo_doctor(paths)?;
            Ok(ApiResult::RepoDoctor { report })
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
        ApiMethod::ThreadCreate { title, task } => {
            let thread = AnvicsStore::open(&repo)?.create_thread(title, task)?;
            Ok(ApiResult::ThreadCreate {
                thread: Box::new(thread),
            })
        }
        ApiMethod::ThreadList => {
            let threads = AnvicsStore::open(&repo)?.list_threads()?;
            Ok(ApiResult::ThreadList { threads })
        }
        ApiMethod::ThreadShow { id } => {
            let thread = AnvicsStore::open(&repo)?.show_thread(&id)?;
            Ok(ApiResult::ThreadShow {
                thread: Box::new(thread),
            })
        }
        ApiMethod::WorkspaceCreate { thread } => {
            let workspace = AnvicsStore::open(&repo)?.create_workspace(&thread)?;
            Ok(ApiResult::WorkspaceCreate {
                workspace: Box::new(workspace),
            })
        }
        ApiMethod::WorkspaceShow { id } => {
            let store = AnvicsStore::open(&repo)?;
            let workspace = store.show_workspace(&id)?;
            let changed_paths = store.workspace_changed_paths(&id)?;
            Ok(ApiResult::WorkspaceShow {
                workspace: Box::new(workspace),
                changed_paths,
            })
        }
        ApiMethod::WorkspaceDiff {
            id,
            format,
            classify,
        } => {
            let store = AnvicsStore::open(&repo)?;
            let changed_paths = store.workspace_diff(&id)?;
            let file_effects = if classify {
                store.workspace_file_effects(&id)?
            } else {
                Vec::new()
            };
            let patch = match format {
                anvics_api::WorkspaceDiffFormat::Summary => None,
                anvics_api::WorkspaceDiffFormat::Patch => Some(store.workspace_diff_patch(&id)?),
            };
            Ok(ApiResult::WorkspaceDiff {
                changed_paths,
                file_effects,
                patch,
            })
        }
        ApiMethod::WorkspaceSnapshot { id, message } => {
            let workspace = AnvicsStore::open(&repo)?.workspace_snapshot(&id, message)?;
            Ok(ApiResult::WorkspaceSnapshot {
                workspace: Box::new(workspace),
            })
        }
        ApiMethod::WorkspaceRestore {
            id,
            source,
            paths,
            reason,
            dry_run,
        } => {
            let restore = AnvicsStore::open(&repo)?
                .workspace_restore(&id, &source, paths, reason, dry_run)?;
            Ok(ApiResult::WorkspaceRestore {
                restore: Box::new(restore),
            })
        }
        ApiMethod::EvidenceAttach {
            thread,
            command,
            exit_code,
            summary,
            artifact_path,
        } => {
            let evidence = AnvicsStore::open(&repo)?.attach_evidence(
                &thread,
                command,
                exit_code,
                summary,
                artifact_path,
            )?;
            Ok(ApiResult::EvidenceAttached { evidence })
        }
        ApiMethod::EvidenceCommand {
            thread,
            command,
            command_file,
            command_label,
            cwd,
            exit_code,
            summary,
            artifact_path,
        } => {
            let input = command_input(
                command,
                command_file,
                command_label,
                cwd,
                exit_code,
                summary,
                artifact_path,
            )?;
            let evidence = AnvicsStore::open(&repo)?.attach_command_evidence(&thread, input)?;
            Ok(ApiResult::EvidenceAttached { evidence })
        }
        ApiMethod::EvidenceList {
            thread,
            include_superseded,
        } => {
            let evidence =
                AnvicsStore::open(&repo)?.list_thread_evidence(&thread, include_superseded)?;
            Ok(ApiResult::EvidenceList { evidence })
        }
        ApiMethod::EvidenceShow { id } => {
            let evidence = AnvicsStore::open(&repo)?.show_evidence(&id)?;
            Ok(ApiResult::EvidenceShow { evidence })
        }
        ApiMethod::EvidenceSupersede { id, reason } => {
            let evidence = AnvicsStore::open(&repo)?.supersede_evidence(&id, reason)?;
            Ok(ApiResult::EvidenceSuperseded { evidence })
        }
        ApiMethod::CommandRun {
            workspace,
            argv,
            command_file,
            command_label,
            cwd,
            timeout_seconds,
            summary,
            artifact_path,
            projection,
            mount_root,
            allow_command_risk,
            command_risk_reason,
        } => {
            let result = AnvicsStore::open(&repo)?.run_command(CommandRunInput {
                workspace_id: workspace,
                argv,
                command_file,
                command_label,
                cwd,
                timeout_seconds,
                summary,
                artifact_path,
                projection,
                mount_root,
                allow_command_risk,
                command_risk_reason,
            })?;
            Ok(ApiResult::CommandRun {
                command_event: Box::new(result.command_event),
                evidence: result.evidence,
            })
        }
        ApiMethod::CommandClassify { argv, command_file } => {
            let decision = classify_command_policy(CommandPolicyInput { argv, command_file })?;
            Ok(ApiResult::CommandClassify { decision })
        }
        ApiMethod::CommandShow { id } => {
            let command_event = AnvicsStore::open(&repo)?.show_command_event(&id)?;
            Ok(ApiResult::CommandShow {
                command_event: Box::new(command_event),
            })
        }
        ApiMethod::ReviewCreate { thread } => {
            let review = AnvicsStore::open(&repo)?.create_review(&thread)?;
            Ok(ApiResult::ReviewCreate {
                review: Box::new(review),
            })
        }
        ApiMethod::ReviewInbox => {
            let store = AnvicsStore::open(&repo)?;
            let mut items = Vec::new();
            for thread in store.list_threads()? {
                let status = store.agent_status(thread.id.as_str())?;
                let latest_review = status
                    .review_ids
                    .last()
                    .map(|review_id| store.show_review(review_id.as_str()))
                    .transpose()?;
                let latest_risk_findings = latest_review
                    .as_ref()
                    .map(|review| store.list_review_risk_findings(review.id.as_str()))
                    .transpose()?
                    .unwrap_or_default();
                items.push(ReviewInboxItem {
                    thread,
                    workspaces: status.workspaces,
                    evidence_count: status.evidence_count,
                    review_ids: status.review_ids,
                    publication_ids: status.publication_ids,
                    latest_review,
                    latest_risk_findings,
                });
            }
            Ok(ApiResult::ReviewInbox { items })
        }
        ApiMethod::AgentPrepare {
            title,
            task,
            agent_command,
        } => {
            let preparation =
                AnvicsStore::open(&repo)?.prepare_agent_with_command(title, task, agent_command)?;
            Ok(ApiResult::AgentPrepare {
                preparation: Box::new(preparation),
            })
        }
        ApiMethod::AgentResolve {
            reviews,
            title,
            task,
            agent_command,
        } => {
            let preparation = AnvicsStore::open(&repo)?.prepare_resolution_agent(
                reviews,
                title,
                task,
                agent_command,
            )?;
            Ok(ApiResult::AgentPrepare {
                preparation: Box::new(preparation),
            })
        }
        ApiMethod::ConflictAnalyze { reviews } => {
            let store = AnvicsStore::open(&repo)?;
            let analysis = store.create_conflict_analysis(reviews)?;
            let markdown = store.conflict_analysis_markdown(analysis.id.as_str())?;
            let markdown_path = repo
                .join(".anvics")
                .join("conflicts")
                .join(format!("{}.md", analysis.id))
                .to_string_lossy()
                .to_string();
            Ok(ApiResult::ConflictAnalyze {
                analysis: Box::new(analysis),
                markdown_path,
                markdown,
            })
        }
        ApiMethod::ConflictPrepare {
            reviews,
            title,
            task,
            agent_command,
        } => {
            let preparation = AnvicsStore::open(&repo)?.prepare_conflict_resolution(
                reviews,
                title,
                task,
                agent_command,
            )?;
            Ok(ApiResult::ConflictPrepare {
                preparation: Box::new(preparation),
            })
        }
        ApiMethod::ConflictStatus { workspace } => {
            let verification = AnvicsStore::open(&repo)?.conflict_status(&workspace)?;
            Ok(ApiResult::ConflictStatus {
                verification: Box::new(verification),
            })
        }
        ApiMethod::ConflictVerify { workspace } => {
            let verification = AnvicsStore::open(&repo)?.conflict_verify(&workspace)?;
            Ok(ApiResult::ConflictVerify {
                verification: Box::new(verification),
            })
        }
        ApiMethod::AgentEnter { workspace, name } => {
            let status = AnvicsStore::open(&repo)?.enter_agent_session(&workspace, name)?;
            Ok(ApiResult::AgentEnter {
                status: Box::new(status),
            })
        }
        ApiMethod::AgentLeave { session } => {
            let session = AnvicsStore::open(&repo)?.leave_agent_session(&session)?;
            Ok(ApiResult::AgentLeave { session })
        }
        ApiMethod::AgentSessions { thread, workspace } => {
            let sessions = AnvicsStore::open(&repo)?
                .list_agent_sessions(thread.as_deref(), workspace.as_deref())?;
            Ok(ApiResult::AgentSessions { sessions })
        }
        ApiMethod::AgentStatus { thread } => {
            let status = AnvicsStore::open(&repo)?.agent_status(&thread)?;
            Ok(ApiResult::AgentStatus {
                status: Box::new(status),
            })
        }
        ApiMethod::AgentPacket { thread } => {
            let path = AnvicsStore::open(&repo)?
                .agent_packet_file_path(&thread)?
                .to_string_lossy()
                .to_string();
            Ok(ApiResult::AgentPacket { path })
        }
        ApiMethod::AgentLaunchPrompt { workspace, tool } => {
            let prompt = AnvicsStore::open(&repo)?.agent_launch_prompt(&workspace, tool)?;
            Ok(ApiResult::AgentLaunchPrompt {
                prompt: Box::new(prompt),
            })
        }
        ApiMethod::AgentInstructions {
            target,
            install,
            force,
        } => {
            let files =
                AnvicsStore::open(&repo)?.agent_instruction_files(target, install, force)?;
            Ok(ApiResult::AgentInstructions { files })
        }
        ApiMethod::AgentContextPack { workspace, write } => {
            let pack = AnvicsStore::open(&repo)?.agent_context_pack(&workspace, write)?;
            Ok(ApiResult::AgentContextPack {
                pack: Box::new(pack),
            })
        }
        ApiMethod::AgentCheckpoint { workspace, summary } => {
            let checkpoint = AnvicsStore::open(&repo)?.agent_checkpoint(&workspace, summary)?;
            Ok(ApiResult::AgentCheckpoint {
                checkpoint: Box::new(checkpoint),
            })
        }
        ApiMethod::AgentCheckpointList { workspace } => {
            let checkpoints = AnvicsStore::open(&repo)?.list_agent_checkpoints(&workspace)?;
            Ok(ApiResult::AgentCheckpointList { checkpoints })
        }
        ApiMethod::AgentCheckpointShow { id } => {
            let checkpoint = AnvicsStore::open(&repo)?.show_agent_checkpoint(&id)?;
            Ok(ApiResult::AgentCheckpointShow {
                checkpoint: Box::new(checkpoint),
            })
        }
        ApiMethod::AgentCheckpointRestore {
            workspace,
            checkpoint,
            reason,
        } => {
            let restore = AnvicsStore::open(&repo)?.restore_agent_checkpoint(
                &workspace,
                &checkpoint,
                reason,
            )?;
            Ok(ApiResult::WorkspaceRestore {
                restore: Box::new(restore),
            })
        }
        ApiMethod::AgentRecover { workspace } => {
            let recovery = AnvicsStore::open(&repo)?.agent_recovery(&workspace)?;
            Ok(ApiResult::AgentRecover {
                recovery: Box::new(recovery),
            })
        }
        ApiMethod::AgentFinish {
            workspace,
            command,
            command_file,
            command_label,
            cwd,
            exit_code,
            summary,
            artifact_path,
        } => {
            let input = command_input(
                command,
                command_file,
                command_label,
                cwd,
                exit_code,
                summary,
                artifact_path,
            )?;
            let finish = AnvicsStore::open(&repo)?.finish_agent_with_evidence(&workspace, input)?;
            Ok(ApiResult::AgentFinish {
                finish: Box::new(finish),
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
            allow_secret_risk,
            override_reason,
            allow_resolution_risk,
            resolution_risk_reason,
        } => {
            let input = command_input(
                command,
                command_file,
                command_label,
                cwd,
                exit_code,
                summary,
                artifact_path,
            )?;
            let acceptance = AnvicsStore::open(&repo)?.accept_agent_with_evidence_and_options(
                &workspace,
                input,
                output_path.map(PathBuf::from),
                PublicationOptions {
                    allow_secret_risk,
                    override_reason,
                    allow_resolution_risk,
                    resolution_risk_reason,
                },
            )?;
            Ok(ApiResult::AgentAccept {
                acceptance: Box::new(acceptance),
            })
        }
        ApiMethod::AgentAcceptRun {
            workspace,
            argv,
            command_file,
            command_label,
            cwd,
            timeout_seconds,
            summary,
            artifact_path,
            projection,
            mount_root,
            output_path,
            allow_secret_risk,
            override_reason,
            allow_resolution_risk,
            resolution_risk_reason,
            allow_command_risk,
            command_risk_reason,
        } => {
            let acceptance = AnvicsStore::open(&repo)?.accept_agent_with_command_run_and_options(
                CommandRunInput {
                    workspace_id: workspace,
                    argv,
                    command_file,
                    command_label,
                    cwd,
                    timeout_seconds,
                    summary,
                    artifact_path,
                    projection,
                    mount_root,
                    allow_command_risk,
                    command_risk_reason,
                },
                output_path.map(PathBuf::from),
                PublicationOptions {
                    allow_secret_risk,
                    override_reason,
                    allow_resolution_risk,
                    resolution_risk_reason,
                },
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
        ApiMethod::ReviewPath { id } => {
            let path = AnvicsStore::open(&repo)?
                .review_markdown_file_path(&id)?
                .to_string_lossy()
                .to_string();
            Ok(ApiResult::ReviewPath { path })
        }
        ApiMethod::PublishCreate {
            thread,
            review,
            allow_secret_risk,
            override_reason,
            allow_resolution_risk,
            resolution_risk_reason,
        } => {
            let publication = AnvicsStore::open(&repo)?.create_publication_with_options(
                &thread,
                &review,
                PublicationOptions {
                    allow_secret_risk,
                    override_reason,
                    allow_resolution_risk,
                    resolution_risk_reason,
                },
            )?;
            Ok(ApiResult::PublishCreate { publication })
        }
        ApiMethod::PublishRevertPrepare {
            publication,
            base_snapshot,
            reason,
        } => {
            let revert = AnvicsStore::open(&repo)?.prepare_publication_revert(
                &publication,
                base_snapshot,
                reason,
            )?;
            Ok(ApiResult::PublishRevertPrepare {
                revert: Box::new(revert),
            })
        }
        ApiMethod::RiskScan { review } => {
            let scan = AnvicsStore::open(&repo)?.scan_review_risks(&review)?;
            Ok(ApiResult::RiskScan {
                scan: Box::new(scan),
            })
        }
        ApiMethod::RiskList { review } => {
            let findings = AnvicsStore::open(&repo)?.list_review_risk_findings(&review)?;
            Ok(ApiResult::RiskList { findings })
        }
        ApiMethod::RiskShow { id } => {
            let finding = AnvicsStore::open(&repo)?.show_risk_finding(&id)?;
            Ok(ApiResult::RiskShow { finding })
        }
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
        ApiMethod::CoordinationStatus { workspace } => {
            let status = AnvicsStore::open(&repo)?.coordination_status(&workspace)?;
            Ok(ApiResult::CoordinationStatus {
                status: Box::new(status),
            })
        }
    }
}

fn command_input(
    command: Option<String>,
    command_file: Option<String>,
    command_label: Option<String>,
    cwd: Option<String>,
    exit_code: i32,
    summary: String,
    artifact_path: Option<String>,
) -> Result<CommandEvidenceInput> {
    let command_text = match (&command, &command_file) {
        (Some(command), _) => command.clone(),
        (None, Some(path)) => fs::read_to_string(path)
            .with_context(|| format!("failed to read command file {path}"))?,
        (None, None) => anyhow::bail!("either command or command_file is required"),
    };
    Ok(CommandEvidenceInput {
        command: command_text,
        command_event_id: None,
        command_label,
        command_file,
        cwd,
        exit_code,
        summary,
        artifact_path,
        stdout_path: None,
        stderr_path: None,
    })
}
