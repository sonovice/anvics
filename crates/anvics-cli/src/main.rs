use anvics_api::{ApiMethod, ApiRequest, ApiResponse, ApiResult};
use anvics_store::{AnvicsStore, CommandEvidenceInput, StoreError};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

#[derive(Debug, Parser)]
#[command(name = "anvics")]
#[command(about = "Agent-native source control")]
struct Cli {
    #[arg(long, global = true, value_name = "DIR")]
    repo: Option<PathBuf>,

    #[arg(long, global = true)]
    use_daemon: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Repo {
        #[command(subcommand)]
        command: RepoCommand,
    },
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },
    Thread {
        #[command(subcommand)]
        command: ThreadCommand,
    },
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    Publish {
        #[command(subcommand)]
        command: PublishCommand,
    },
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Legacy {
        #[command(subcommand)]
        command: LegacyCommand,
    },
}

#[derive(Debug, Subcommand)]
enum RepoCommand {
    Init,
    Status,
}

#[derive(Debug, Subcommand)]
enum SnapshotCommand {
    Create {
        #[arg(short, long)]
        message: Option<String>,
    },
    List,
    Show {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum ThreadCommand {
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        task: String,
    },
    List,
    Show {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum WorkspaceCommand {
    Create {
        #[arg(long)]
        thread: String,
    },
    Snapshot {
        id: String,
        #[arg(short, long)]
        message: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    Attach {
        #[arg(long)]
        thread: String,
        #[arg(long)]
        command: String,
        #[arg(long)]
        exit_code: i32,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        artifact: Option<String>,
    },
    Command {
        #[arg(long)]
        thread: String,
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        command_file: Option<PathBuf>,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        exit_code: i32,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        artifact: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ReviewCommand {
    Create {
        #[arg(long)]
        thread: String,
    },
    Show {
        id: String,
        #[arg(long, value_enum, default_value_t = ReviewFormat::Json)]
        format: ReviewFormat,
    },
    Path {
        id: String,
    },
}

#[derive(Debug, Subcommand)]
enum PublishCommand {
    Create {
        #[arg(long)]
        thread: String,
        #[arg(long)]
        review: String,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum ReviewFormat {
    Json,
    Markdown,
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    Prepare {
        #[arg(long)]
        title: String,
        #[arg(long)]
        task: String,
    },
    Packet {
        #[arg(long)]
        thread: String,
    },
    Status {
        #[arg(long)]
        thread: String,
    },
    Finish {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        command_file: Option<PathBuf>,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        exit_code: i32,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        artifact: Option<String>,
    },
    Accept {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        command: Option<String>,
        #[arg(long)]
        command_file: Option<PathBuf>,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        exit_code: i32,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        artifact: Option<String>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum LegacyCommand {
    Git {
        #[command(subcommand)]
        command: LegacyGitCommand,
    },
}

#[derive(Debug, Subcommand)]
enum LegacyGitCommand {
    Export {
        #[arg(long)]
        publication: String,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug)]
struct CommandEvidenceOptions {
    command: Option<String>,
    command_file: Option<PathBuf>,
    label: Option<String>,
    cwd: Option<String>,
    exit_code: i32,
    summary: String,
    artifact: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.repo.unwrap_or(std::env::current_dir()?);
    let daemon = if cli.use_daemon || std::env::var_os("ANVICS_USE_DAEMON").is_some() {
        Some(daemon_socket()?)
    } else {
        None
    };

    match cli.command {
        Command::Repo {
            command: RepoCommand::Init,
        } => {
            if let Some(socket) = daemon {
                init_repo_via_daemon(root, socket)
            } else {
                init_repo(root)
            }
        }
        Command::Repo {
            command: RepoCommand::Status,
        } => {
            if let Some(socket) = daemon {
                repo_status_via_daemon(root, socket)
            } else {
                repo_status(root)
            }
        }
        Command::Snapshot {
            command: SnapshotCommand::Create { message },
        } => {
            if let Some(socket) = daemon {
                create_snapshot_via_daemon(root, socket, message)
            } else {
                create_snapshot(root, message)
            }
        }
        Command::Snapshot {
            command: SnapshotCommand::List,
        } => {
            if let Some(socket) = daemon {
                list_snapshots_via_daemon(root, socket)
            } else {
                list_snapshots(root)
            }
        }
        Command::Snapshot {
            command: SnapshotCommand::Show { id },
        } => {
            if let Some(socket) = daemon {
                show_snapshot_via_daemon(root, socket, id)
            } else {
                show_snapshot(root, &id)
            }
        }
        Command::Thread {
            command: ThreadCommand::Create { title, task },
        } => create_thread(root, title, task),
        Command::Thread {
            command: ThreadCommand::List,
        } => list_threads(root),
        Command::Thread {
            command: ThreadCommand::Show { id },
        } => show_thread(root, &id),
        Command::Workspace {
            command: WorkspaceCommand::Create { thread },
        } => create_workspace(root, &thread),
        Command::Workspace {
            command: WorkspaceCommand::Snapshot { id, message },
        } => snapshot_workspace(root, &id, message),
        Command::Evidence {
            command:
                EvidenceCommand::Attach {
                    thread,
                    command,
                    exit_code,
                    summary,
                    artifact,
                },
        } => attach_evidence(root, &thread, command, exit_code, summary, artifact),
        Command::Evidence {
            command:
                EvidenceCommand::Command {
                    thread,
                    command,
                    command_file,
                    label,
                    cwd,
                    exit_code,
                    summary,
                    artifact,
                },
        } => attach_command_evidence(
            root,
            &thread,
            CommandEvidenceOptions {
                command,
                command_file,
                label,
                cwd,
                exit_code,
                summary,
                artifact,
            },
        ),
        Command::Review {
            command: ReviewCommand::Create { thread },
        } => create_review(root, &thread),
        Command::Review {
            command: ReviewCommand::Show { id, format },
        } => show_review(root, &id, format),
        Command::Review {
            command: ReviewCommand::Path { id },
        } => show_review_path(root, &id),
        Command::Publish {
            command: PublishCommand::Create { thread, review },
        } => create_publication(root, &thread, &review),
        Command::Agent {
            command: AgentCommand::Prepare { title, task },
        } => prepare_agent(root, title, task),
        Command::Agent {
            command: AgentCommand::Packet { thread },
        } => show_agent_packet(root, &thread),
        Command::Agent {
            command: AgentCommand::Status { thread },
        } => {
            if let Some(socket) = daemon {
                show_agent_status_via_daemon(root, socket, thread)
            } else {
                show_agent_status(root, &thread)
            }
        }
        Command::Agent {
            command:
                AgentCommand::Finish {
                    workspace,
                    command,
                    command_file,
                    label,
                    cwd,
                    exit_code,
                    summary,
                    artifact,
                },
        } => finish_agent(
            root,
            &workspace,
            CommandEvidenceOptions {
                command,
                command_file,
                label,
                cwd,
                exit_code,
                summary,
                artifact,
            },
        ),
        Command::Agent {
            command:
                AgentCommand::Accept {
                    workspace,
                    command,
                    command_file,
                    label,
                    cwd,
                    exit_code,
                    summary,
                    artifact,
                    output,
                },
        } => accept_agent(
            root,
            &workspace,
            CommandEvidenceOptions {
                command,
                command_file,
                label,
                cwd,
                exit_code,
                summary,
                artifact,
            },
            output,
        ),
        Command::Legacy {
            command:
                LegacyCommand::Git {
                    command:
                        LegacyGitCommand::Export {
                            publication,
                            output,
                        },
                },
        } => export_legacy_git_patch(root, &publication, output),
    }
}

fn init_repo(root: PathBuf) -> Result<()> {
    match AnvicsStore::init(&root) {
        Ok(manifest) => {
            println!("Initialized Anvics repository");
            println!("repository: {}", manifest.id);
            println!("format: {}", manifest.format_version);
            Ok(())
        }
        Err(StoreError::AlreadyInitialized(_)) => {
            let manifest = AnvicsStore::open(&root)
                .and_then(|store| store.manifest())
                .context("failed to read existing Anvics repository")?;
            println!("Anvics repository already initialized");
            println!("repository: {}", manifest.id);
            println!("format: {}", manifest.format_version);
            Ok(())
        }
        Err(error) => Err(error).context("failed to initialize Anvics repository"),
    }
}

fn repo_status(root: PathBuf) -> Result<()> {
    match AnvicsStore::open(&root) {
        Ok(store) => {
            let manifest = store.manifest().context("failed to read repository")?;
            println!("Anvics repository initialized");
            println!("repository: {}", manifest.id);
            println!("format: {}", manifest.format_version);
            Ok(())
        }
        Err(StoreError::NotRepository(_)) => {
            println!("Anvics repository not initialized");
            Ok(())
        }
        Err(error) => Err(error).context("failed to read repository status"),
    }
}

fn init_repo_via_daemon(root: PathBuf, socket: PathBuf) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::RepoInit)? {
        ApiResult::RepoInit { manifest } => {
            println!("Initialized Anvics repository");
            println!("repository: {}", manifest.id);
            println!("format: {}", manifest.format_version);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn repo_status_via_daemon(root: PathBuf, socket: PathBuf) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::RepoStatus)? {
        ApiResult::RepoStatus {
            initialized: true,
            manifest: Some(manifest),
        } => {
            println!("Anvics repository initialized");
            println!("repository: {}", manifest.id);
            println!("format: {}", manifest.format_version);
            Ok(())
        }
        ApiResult::RepoStatus {
            initialized: false, ..
        } => {
            println!("Anvics repository not initialized");
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn create_snapshot(root: PathBuf, message: Option<String>) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let snapshot = store
        .create_snapshot(message)
        .context("failed to create snapshot")?;

    println!("Created snapshot {}", snapshot.id);
    println!("root_tree: {}", snapshot.root_tree);
    if let Some(message) = snapshot.message {
        println!("message: {message}");
    }
    Ok(())
}

fn create_snapshot_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    message: Option<String>,
) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::SnapshotCreate { message })? {
        ApiResult::SnapshotCreate { snapshot } => {
            println!("Created snapshot {}", snapshot.id);
            println!("root_tree: {}", snapshot.root_tree);
            if let Some(message) = snapshot.message {
                println!("message: {message}");
            }
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn list_snapshots(root: PathBuf) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let snapshots = store.list_snapshots().context("failed to list snapshots")?;

    print_snapshots(snapshots)
}

fn print_snapshots(snapshots: Vec<anvics_core::SourceSnapshot>) -> Result<()> {
    if snapshots.is_empty() {
        println!("No snapshots");
        return Ok(());
    }

    for snapshot in snapshots {
        let message = snapshot.message.unwrap_or_default();
        if message.is_empty() {
            println!(
                "{}  {}  {}",
                snapshot.id, snapshot.created_at, snapshot.root_tree
            );
        } else {
            println!(
                "{}  {}  {}  {}",
                snapshot.id, snapshot.created_at, snapshot.root_tree, message
            );
        }
    }

    Ok(())
}

fn list_snapshots_via_daemon(root: PathBuf, socket: PathBuf) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::SnapshotList)? {
        ApiResult::SnapshotList { snapshots } => print_snapshots(snapshots),
        result => unexpected_daemon_result(result),
    }
}

fn show_snapshot(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let snapshot = store
        .show_snapshot(id)
        .with_context(|| format!("failed to show snapshot {id}"))?;

    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
}

fn show_snapshot_via_daemon(root: PathBuf, socket: PathBuf, id: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::SnapshotShow { id })? {
        ApiResult::SnapshotShow { snapshot } => {
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn create_thread(root: PathBuf, title: String, task: String) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let thread = store
        .create_thread(title, task)
        .context("failed to create thread")?;

    println!("Created thread {}", thread.id);
    println!("base_snapshot: {}", thread.base_snapshot);
    println!("title: {}", thread.title);
    Ok(())
}

fn list_threads(root: PathBuf) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let threads = store.list_threads().context("failed to list threads")?;

    if threads.is_empty() {
        println!("No threads");
        return Ok(());
    }

    for thread in threads {
        println!(
            "{}  {:?}  {}  {}",
            thread.id, thread.status, thread.base_snapshot, thread.title
        );
    }
    Ok(())
}

fn show_thread(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let thread = store
        .show_thread(id)
        .with_context(|| format!("failed to show thread {id}"))?;

    println!("{}", serde_json::to_string_pretty(&thread)?);
    Ok(())
}

fn create_workspace(root: PathBuf, thread_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let workspace = store
        .create_workspace(thread_id)
        .context("failed to create workspace")?;

    println!("Created workspace {}", workspace.id);
    println!("thread: {}", workspace.thread_id);
    println!("path: {}", workspace.materialized_path);
    Ok(())
}

fn snapshot_workspace(root: PathBuf, workspace_id: &str, message: Option<String>) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let workspace = store
        .workspace_snapshot(workspace_id, message)
        .context("failed to snapshot workspace")?;

    println!("Snapshotted workspace {}", workspace.id);
    if let Some(snapshot) = workspace.latest_snapshot {
        println!("snapshot: {snapshot}");
    }
    Ok(())
}

fn attach_evidence(
    root: PathBuf,
    thread_id: &str,
    command: String,
    exit_code: i32,
    summary: String,
    artifact: Option<String>,
) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let evidence = store
        .attach_evidence(thread_id, command, exit_code, summary, artifact)
        .context("failed to attach evidence")?;

    println!("Attached evidence {}", evidence.id);
    println!("thread: {}", evidence.thread_id);
    Ok(())
}

fn attach_command_evidence(
    root: PathBuf,
    thread_id: &str,
    options: CommandEvidenceOptions,
) -> Result<()> {
    let input = command_input(options)?;
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let evidence = store
        .attach_command_evidence(thread_id, input)
        .context("failed to attach command evidence")?;

    println!("Attached command evidence {}", evidence.id);
    println!("thread: {}", evidence.thread_id);
    Ok(())
}

fn create_review(root: PathBuf, thread_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let review = store
        .create_review(thread_id)
        .context("failed to create review")?;

    println!("Created review {}", review.id);
    println!("thread: {}", review.thread_id);
    println!("changed_paths: {}", review.changed_paths.len());
    println!("overlap_notes: {}", review.overlap_notes.len());
    Ok(())
}

fn show_review(root: PathBuf, id: &str, format: ReviewFormat) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    match format {
        ReviewFormat::Json => {
            let review = store
                .show_review(id)
                .with_context(|| format!("failed to show review {id}"))?;
            println!("{}", serde_json::to_string_pretty(&review)?);
        }
        ReviewFormat::Markdown => {
            let markdown = store
                .review_markdown(id)
                .with_context(|| format!("failed to show review {id} as markdown"))?;
            println!("{markdown}");
        }
    }
    Ok(())
}

fn show_review_path(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let path = store
        .review_markdown_file_path(id)
        .with_context(|| format!("failed to find review {id} markdown path"))?;

    println!("{}", path.display());
    Ok(())
}

fn create_publication(root: PathBuf, thread_id: &str, review_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let publication = store
        .create_publication(thread_id, review_id)
        .context("failed to create publication")?;

    println!("Created publication {}", publication.id);
    println!("thread: {}", publication.thread_id);
    println!("accepted_snapshot: {}", publication.accepted_snapshot);
    println!(
        "legacy_export: anvics --repo {} legacy git export --publication {} --output accepted.patch",
        shell_quote(&display_path(&root)),
        publication.id
    );
    Ok(())
}

fn prepare_agent(root: PathBuf, title: String, task: String) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let preparation = store
        .prepare_agent(title, task)
        .context("failed to prepare agent task")?;

    println!("Prepared agent task");
    println!("thread: {}", preparation.thread.id);
    println!("workspace: {}", preparation.workspace.id);
    println!(
        "workspace_path: {}",
        preparation.workspace.materialized_path
    );
    println!("packet: {}", preparation.packet_path);
    println!(
        "accept: anvics --repo {} agent accept --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"",
        shell_quote(&display_path(&root)),
        preparation.workspace.id
    );
    println!(
        "finish: anvics --repo {} agent finish --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"",
        shell_quote(&display_path(&root)),
        preparation.workspace.id
    );
    Ok(())
}

fn show_agent_packet(root: PathBuf, thread_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let path = store
        .agent_packet_file_path(thread_id)
        .with_context(|| format!("failed to find agent packet for thread {thread_id}"))?;

    println!("{}", path.display());
    Ok(())
}

fn show_agent_status(root: PathBuf, thread_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let status = store
        .agent_status(thread_id)
        .with_context(|| format!("failed to show agent status for thread {thread_id}"))?;

    print_agent_status(status);
    Ok(())
}

fn show_agent_status_via_daemon(root: PathBuf, socket: PathBuf, thread: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::AgentStatus { thread })? {
        ApiResult::AgentStatus { status } => {
            print_agent_status(*status);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_agent_status(status: anvics_core::AgentStatus) {
    println!("thread: {}", status.thread.id);
    println!("title: {}", status.thread.title);
    println!("status: {:?}", status.thread.status);
    println!("base_snapshot: {}", status.thread.base_snapshot);
    println!("evidence_count: {}", status.evidence_count);
    if status.workspaces.is_empty() {
        println!("workspaces: none");
    } else {
        for workspace in status.workspaces {
            println!("workspace: {}", workspace.id);
            println!("workspace_path: {}", workspace.materialized_path);
            match workspace.latest_snapshot {
                Some(snapshot) => println!("latest_snapshot: {snapshot}"),
                None => println!("latest_snapshot: none"),
            }
        }
    }
    if status.review_ids.is_empty() {
        println!("reviews: none");
    } else {
        for review_id in status.review_ids {
            println!("review: {review_id}");
        }
    }
    if status.publication_ids.is_empty() {
        println!("publication_status: unpublished");
    } else {
        println!("publication_status: published");
        for publication_id in status.publication_ids {
            println!("publication: {publication_id}");
        }
    }
}

fn finish_agent(root: PathBuf, workspace_id: &str, options: CommandEvidenceOptions) -> Result<()> {
    let input = command_input(options)?;
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let finish = store
        .finish_agent_with_evidence(workspace_id, input)
        .context("failed to finish agent task")?;

    println!("Finished agent task");
    println!("thread: {}", finish.workspace.thread_id);
    println!("workspace: {}", finish.workspace.id);
    if let Some(snapshot) = finish.workspace.latest_snapshot {
        println!("snapshot: {snapshot}");
    }
    println!("evidence: {}", finish.evidence.id);
    println!("review: {}", finish.review.id);
    println!("review_markdown: {}", finish.review_markdown_path);
    Ok(())
}

fn accept_agent(
    root: PathBuf,
    workspace_id: &str,
    options: CommandEvidenceOptions,
    output: Option<PathBuf>,
) -> Result<()> {
    let input = command_input(options)?;
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let acceptance = store
        .accept_agent_with_evidence(workspace_id, input, output)
        .context("failed to accept agent workspace")?;

    println!("Accepted agent workspace");
    println!("thread: {}", acceptance.workspace.thread_id);
    println!("workspace: {}", acceptance.workspace.id);
    if let Some(snapshot) = acceptance.workspace.latest_snapshot {
        println!("snapshot: {snapshot}");
    }
    println!("evidence: {}", acceptance.evidence.id);
    println!("review: {}", acceptance.review.id);
    println!("review_markdown: {}", acceptance.review_markdown_path);
    println!("publication: {}", acceptance.publication.id);
    println!("patch: {}", acceptance.patch_path);
    println!(
        "git_apply: git apply {}",
        shell_quote(&acceptance.patch_path)
    );
    Ok(())
}

fn command_input(options: CommandEvidenceOptions) -> Result<CommandEvidenceInput> {
    let command_text = match (&options.command, &options.command_file) {
        (Some(command), _) => command.clone(),
        (None, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("failed to read command file {}", path.display()))?,
        (None, None) => anyhow::bail!("either --command or --command-file is required"),
    };
    let command_file = options
        .command_file
        .map(|path| path.to_string_lossy().to_string());

    Ok(CommandEvidenceInput {
        command: command_text,
        command_label: options.label,
        command_file,
        cwd: options.cwd,
        exit_code: options.exit_code,
        summary: options.summary,
        artifact_path: options.artifact,
    })
}

fn export_legacy_git_patch(root: PathBuf, publication_id: &str, output: PathBuf) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let output = store
        .export_legacy_git_patch(publication_id, output)
        .context("failed to export legacy Git patch")?;

    println!("Exported legacy Git patch");
    println!("path: {}", output.display());
    Ok(())
}

fn daemon_socket() -> Result<PathBuf> {
    std::env::var_os("ANVICS_DAEMON_SOCKET")
        .map(PathBuf::from)
        .context("ANVICS_DAEMON_SOCKET must be set when --use-daemon is used")
}

fn daemon_request(socket: &PathBuf, repo: PathBuf, method: ApiMethod) -> Result<ApiResult> {
    let mut stream = UnixStream::connect(socket)
        .with_context(|| format!("failed to connect to daemon socket {}", socket.display()))?;
    let request = ApiRequest {
        id: 1,
        repo: display_path(&repo),
        method,
    };
    serde_json::to_writer(&mut stream, &request)?;
    stream.write_all(b"\n")?;
    stream.flush()?;

    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let response: ApiResponse = serde_json::from_str(&line)
        .with_context(|| format!("failed to decode daemon response: {line}"))?;
    match response.result {
        ApiResult::Error { message } => anyhow::bail!("daemon error: {message}"),
        result => Ok(result),
    }
}

fn unexpected_daemon_result(result: ApiResult) -> Result<()> {
    anyhow::bail!("unexpected daemon response: {result:?}")
}

fn display_path(path: &std::path::Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
