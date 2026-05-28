use anvics_api::{ApiMethod, ApiRequest, ApiResponse, ApiResult, ReviewFormat as ApiReviewFormat};
use anvics_store::{
    AnvicsStore, CommandEvidenceInput, CommandRunInput, PublicationOptions, StoreError,
};
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
    command: CliCommand,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
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
    Command {
        #[command(subcommand)]
        command: CommandRunCommand,
    },
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    Risk {
        #[command(subcommand)]
        command: RiskCommand,
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
    Events {
        #[command(subcommand)]
        command: EventsCommand,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    Coordination {
        #[command(subcommand)]
        command: CoordinationCommand,
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
    Show {
        id: String,
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
enum CommandRunCommand {
    Run {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        label: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        cwd: Option<String>,
        #[arg(long)]
        timeout_seconds: Option<u64>,
        #[arg(long)]
        artifact: Option<String>,
        #[arg(long)]
        command_file: Option<PathBuf>,
        #[arg(last = true)]
        argv: Vec<String>,
    },
    Show {
        id: String,
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
        #[arg(long)]
        allow_secret_risk: bool,
        #[arg(long)]
        override_reason: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum RiskCommand {
    Scan {
        #[arg(long)]
        review: String,
    },
    List {
        #[arg(long)]
        review: String,
    },
    Show {
        id: String,
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
    Enter {
        #[arg(long)]
        workspace: String,
        #[arg(long)]
        name: String,
    },
    Leave {
        #[arg(long)]
        session: String,
    },
    Sessions {
        #[arg(long)]
        thread: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
    },
    Packet {
        #[arg(long)]
        thread: String,
    },
    Status {
        #[arg(long)]
        thread: Option<String>,
        #[arg(long)]
        workspace: Option<String>,
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
        exit_code: Option<i32>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        artifact: Option<String>,
        #[arg(long)]
        run_label: Option<String>,
        #[arg(long)]
        run_summary: Option<String>,
        #[arg(long)]
        run_timeout_seconds: Option<u64>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        allow_secret_risk: bool,
        #[arg(long)]
        override_reason: Option<String>,
        #[arg(last = true)]
        argv: Vec<String>,
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

#[derive(Debug, Subcommand)]
enum EventsCommand {
    List {
        #[arg(long, default_value_t = 0)]
        since: u64,
    },
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    Ping {
        #[arg(long)]
        socket: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum CoordinationCommand {
    Status {
        #[arg(long)]
        workspace: String,
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

#[derive(Debug)]
struct CommandRunOptions {
    workspace: String,
    argv: Vec<String>,
    command_file: Option<PathBuf>,
    label: String,
    cwd: Option<String>,
    timeout_seconds: Option<u64>,
    summary: String,
    artifact: Option<String>,
}

#[derive(Debug)]
struct AgentAcceptOptions {
    command: Option<String>,
    command_file: Option<PathBuf>,
    label: Option<String>,
    cwd: Option<String>,
    exit_code: Option<i32>,
    summary: Option<String>,
    artifact: Option<String>,
    run_label: Option<String>,
    run_summary: Option<String>,
    run_timeout_seconds: Option<u64>,
    allow_secret_risk: bool,
    override_reason: Option<String>,
    argv: Vec<String>,
}

impl AgentAcceptOptions {
    fn publication_options(&self) -> PublicationOptions {
        PublicationOptions {
            allow_secret_risk: self.allow_secret_risk,
            override_reason: self.override_reason.clone(),
        }
    }
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
        CliCommand::Repo {
            command: RepoCommand::Init,
        } => {
            if let Some(socket) = daemon {
                init_repo_via_daemon(root, socket)
            } else {
                init_repo(root)
            }
        }
        CliCommand::Repo {
            command: RepoCommand::Status,
        } => {
            if let Some(socket) = daemon {
                repo_status_via_daemon(root, socket)
            } else {
                repo_status(root)
            }
        }
        CliCommand::Snapshot {
            command: SnapshotCommand::Create { message },
        } => {
            if let Some(socket) = daemon {
                create_snapshot_via_daemon(root, socket, message)
            } else {
                create_snapshot(root, message)
            }
        }
        CliCommand::Snapshot {
            command: SnapshotCommand::List,
        } => {
            if let Some(socket) = daemon {
                list_snapshots_via_daemon(root, socket)
            } else {
                list_snapshots(root)
            }
        }
        CliCommand::Snapshot {
            command: SnapshotCommand::Show { id },
        } => {
            if let Some(socket) = daemon {
                show_snapshot_via_daemon(root, socket, id)
            } else {
                show_snapshot(root, &id)
            }
        }
        CliCommand::Thread {
            command: ThreadCommand::Create { title, task },
        } => {
            if let Some(socket) = daemon {
                create_thread_via_daemon(root, socket, title, task)
            } else {
                create_thread(root, title, task)
            }
        }
        CliCommand::Thread {
            command: ThreadCommand::List,
        } => {
            if let Some(socket) = daemon {
                list_threads_via_daemon(root, socket)
            } else {
                list_threads(root)
            }
        }
        CliCommand::Thread {
            command: ThreadCommand::Show { id },
        } => {
            if let Some(socket) = daemon {
                show_thread_via_daemon(root, socket, id)
            } else {
                show_thread(root, &id)
            }
        }
        CliCommand::Workspace {
            command: WorkspaceCommand::Create { thread },
        } => {
            if let Some(socket) = daemon {
                create_workspace_via_daemon(root, socket, thread)
            } else {
                create_workspace(root, &thread)
            }
        }
        CliCommand::Workspace {
            command: WorkspaceCommand::Show { id },
        } => {
            if let Some(socket) = daemon {
                show_workspace_via_daemon(root, socket, id)
            } else {
                show_workspace(root, &id)
            }
        }
        CliCommand::Workspace {
            command: WorkspaceCommand::Snapshot { id, message },
        } => {
            if let Some(socket) = daemon {
                snapshot_workspace_via_daemon(root, socket, id, message)
            } else {
                snapshot_workspace(root, &id, message)
            }
        }
        CliCommand::Evidence {
            command:
                EvidenceCommand::Attach {
                    thread,
                    command,
                    exit_code,
                    summary,
                    artifact,
                },
        } => {
            if let Some(socket) = daemon {
                attach_evidence_via_daemon(
                    root, socket, thread, command, exit_code, summary, artifact,
                )
            } else {
                attach_evidence(root, &thread, command, exit_code, summary, artifact)
            }
        }
        CliCommand::Evidence {
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
        } => {
            let options = CommandEvidenceOptions {
                command,
                command_file,
                label,
                cwd,
                exit_code,
                summary,
                artifact,
            };
            if let Some(socket) = daemon {
                attach_command_evidence_via_daemon(root, socket, thread, options)
            } else {
                attach_command_evidence(root, &thread, options)
            }
        }
        CliCommand::Command {
            command:
                CommandRunCommand::Run {
                    workspace,
                    label,
                    summary,
                    cwd,
                    timeout_seconds,
                    artifact,
                    command_file,
                    argv,
                },
        } => {
            let options = CommandRunOptions {
                workspace,
                argv,
                command_file,
                label,
                cwd,
                timeout_seconds,
                summary,
                artifact,
            };
            if let Some(socket) = daemon {
                run_command_via_daemon(root, socket, options)
            } else {
                run_command(root, options)
            }
        }
        CliCommand::Command {
            command: CommandRunCommand::Show { id },
        } => {
            if let Some(socket) = daemon {
                show_command_event_via_daemon(root, socket, id)
            } else {
                show_command_event(root, &id)
            }
        }
        CliCommand::Review {
            command: ReviewCommand::Create { thread },
        } => {
            if let Some(socket) = daemon {
                create_review_via_daemon(root, socket, thread)
            } else {
                create_review(root, &thread)
            }
        }
        CliCommand::Review {
            command: ReviewCommand::Show { id, format },
        } => {
            if let Some(socket) = daemon {
                show_review_via_daemon(root, socket, id, format)
            } else {
                show_review(root, &id, format)
            }
        }
        CliCommand::Review {
            command: ReviewCommand::Path { id },
        } => {
            if let Some(socket) = daemon {
                show_review_path_via_daemon(root, socket, id)
            } else {
                show_review_path(root, &id)
            }
        }
        CliCommand::Risk {
            command: RiskCommand::Scan { review },
        } => {
            if let Some(socket) = daemon {
                scan_risks_via_daemon(root, socket, review)
            } else {
                scan_risks(root, &review)
            }
        }
        CliCommand::Risk {
            command: RiskCommand::List { review },
        } => {
            if let Some(socket) = daemon {
                list_risks_via_daemon(root, socket, review)
            } else {
                list_risks(root, &review)
            }
        }
        CliCommand::Risk {
            command: RiskCommand::Show { id },
        } => {
            if let Some(socket) = daemon {
                show_risk_via_daemon(root, socket, id)
            } else {
                show_risk(root, &id)
            }
        }
        CliCommand::Publish {
            command:
                PublishCommand::Create {
                    thread,
                    review,
                    allow_secret_risk,
                    override_reason,
                },
        } => {
            let options = PublicationOptions {
                allow_secret_risk,
                override_reason,
            };
            if let Some(socket) = daemon {
                create_publication_via_daemon(root, socket, thread, review, options)
            } else {
                create_publication(root, &thread, &review, options)
            }
        }
        CliCommand::Agent {
            command: AgentCommand::Prepare { title, task },
        } => {
            if let Some(socket) = daemon {
                prepare_agent_via_daemon(root, socket, title, task)
            } else {
                prepare_agent(root, title, task)
            }
        }
        CliCommand::Agent {
            command: AgentCommand::Enter { workspace, name },
        } => {
            if let Some(socket) = daemon {
                enter_agent_via_daemon(root, socket, workspace, name)
            } else {
                enter_agent(root, &workspace, name)
            }
        }
        CliCommand::Agent {
            command: AgentCommand::Leave { session },
        } => {
            if let Some(socket) = daemon {
                leave_agent_via_daemon(root, socket, session)
            } else {
                leave_agent(root, &session)
            }
        }
        CliCommand::Agent {
            command: AgentCommand::Sessions { thread, workspace },
        } => {
            if let Some(socket) = daemon {
                list_agent_sessions_via_daemon(root, socket, thread, workspace)
            } else {
                list_agent_sessions(root, thread.as_deref(), workspace.as_deref())
            }
        }
        CliCommand::Agent {
            command: AgentCommand::Packet { thread },
        } => {
            if let Some(socket) = daemon {
                show_agent_packet_via_daemon(root, socket, thread)
            } else {
                show_agent_packet(root, &thread)
            }
        }
        CliCommand::Agent {
            command: AgentCommand::Status { thread, workspace },
        } => {
            if let Some(socket) = daemon {
                show_agent_status_via_daemon(root, socket, thread, workspace)
            } else {
                show_agent_status(root, thread.as_deref(), workspace.as_deref())
            }
        }
        CliCommand::Agent {
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
        } => {
            let options = CommandEvidenceOptions {
                command,
                command_file,
                label,
                cwd,
                exit_code,
                summary,
                artifact,
            };
            if let Some(socket) = daemon {
                finish_agent_via_daemon(root, socket, workspace, options)
            } else {
                finish_agent(root, &workspace, options)
            }
        }
        CliCommand::Agent {
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
                    run_label,
                    run_summary,
                    run_timeout_seconds,
                    output,
                    allow_secret_risk,
                    override_reason,
                    argv,
                },
        } => {
            let options = AgentAcceptOptions {
                command,
                command_file,
                label,
                cwd,
                exit_code,
                summary,
                artifact,
                run_label,
                run_summary,
                run_timeout_seconds,
                allow_secret_risk,
                override_reason,
                argv,
            };
            if let Some(socket) = daemon {
                accept_agent_via_daemon(root, socket, workspace, options, output)
            } else {
                accept_agent(root, &workspace, options, output)
            }
        }
        CliCommand::Legacy {
            command:
                LegacyCommand::Git {
                    command:
                        LegacyGitCommand::Export {
                            publication,
                            output,
                        },
                },
        } => {
            if let Some(socket) = daemon {
                export_legacy_git_patch_via_daemon(root, socket, publication, output)
            } else {
                export_legacy_git_patch(root, &publication, output)
            }
        }
        CliCommand::Events {
            command: EventsCommand::List { since },
        } => {
            if let Some(socket) = daemon {
                list_events_via_daemon(root, socket, since)
            } else {
                list_events(root, since)
            }
        }
        CliCommand::Daemon {
            command: DaemonCommand::Ping { socket },
        } => ping_daemon(socket.or(daemon)),
        CliCommand::Coordination {
            command: CoordinationCommand::Status { workspace },
        } => {
            if let Some(socket) = daemon {
                coordination_status_via_daemon(root, socket, workspace)
            } else {
                coordination_status(root, &workspace)
            }
        }
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

    print_thread_created(thread);
    Ok(())
}

fn create_thread_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    title: String,
    task: String,
) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::ThreadCreate { title, task })? {
        ApiResult::ThreadCreate { thread } => {
            print_thread_created(*thread);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_thread_created(thread: anvics_core::WorkThread) {
    println!("Created thread {}", thread.id);
    println!("base_snapshot: {}", thread.base_snapshot);
    println!("title: {}", thread.title);
}

fn list_threads(root: PathBuf) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let threads = store.list_threads().context("failed to list threads")?;

    print_threads(threads)
}

fn list_threads_via_daemon(root: PathBuf, socket: PathBuf) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::ThreadList)? {
        ApiResult::ThreadList { threads } => print_threads(threads),
        result => unexpected_daemon_result(result),
    }
}

fn print_threads(threads: Vec<anvics_core::WorkThread>) -> Result<()> {
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

fn show_thread_via_daemon(root: PathBuf, socket: PathBuf, id: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::ThreadShow { id })? {
        ApiResult::ThreadShow { thread } => {
            println!("{}", serde_json::to_string_pretty(&thread)?);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn create_workspace(root: PathBuf, thread_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let workspace = store
        .create_workspace(thread_id)
        .context("failed to create workspace")?;

    print_workspace_created(workspace);
    Ok(())
}

fn create_workspace_via_daemon(root: PathBuf, socket: PathBuf, thread: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::WorkspaceCreate { thread })? {
        ApiResult::WorkspaceCreate { workspace } => {
            print_workspace_created(*workspace);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_workspace_created(workspace: anvics_core::WorkspaceView) {
    println!("Created workspace {}", workspace.id);
    println!("thread: {}", workspace.thread_id);
    println!("path: {}", workspace.materialized_path);
}

fn show_workspace(root: PathBuf, workspace_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let workspace = store
        .show_workspace(workspace_id)
        .with_context(|| format!("failed to show workspace {workspace_id}"))?;
    let changed_paths = store
        .workspace_changed_paths(workspace_id)
        .with_context(|| format!("failed to read overlay for workspace {workspace_id}"))?;

    print_workspace_show(&workspace, changed_paths.as_deref());
    Ok(())
}

fn show_workspace_via_daemon(root: PathBuf, socket: PathBuf, id: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::WorkspaceShow { id })? {
        ApiResult::WorkspaceShow {
            workspace,
            changed_paths,
        } => {
            print_workspace_show(&workspace, changed_paths.as_deref());
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_workspace_show(
    workspace: &anvics_core::WorkspaceView,
    changed_paths: Option<&[anvics_core::ChangedPath]>,
) {
    println!("workspace: {}", workspace.id);
    println!("thread: {}", workspace.thread_id);
    println!("base_snapshot: {}", workspace.base_snapshot);
    println!("workspace_path: {}", workspace.materialized_path);
    match &workspace.latest_snapshot {
        Some(snapshot) => println!("latest_snapshot: {snapshot}"),
        None => println!("latest_snapshot: none"),
    }
    match changed_paths {
        Some([]) => println!("overlay_changed_paths: none"),
        Some(paths) => {
            println!("overlay_changed_paths:");
            for path in paths {
                println!("- {:?}: {}", path.status, path.path);
            }
        }
        None => println!("overlay_changed_paths: unknown"),
    }
}

fn snapshot_workspace(root: PathBuf, workspace_id: &str, message: Option<String>) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let workspace = store
        .workspace_snapshot(workspace_id, message)
        .context("failed to snapshot workspace")?;

    print_workspace_snapshot(workspace);
    Ok(())
}

fn snapshot_workspace_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    id: String,
    message: Option<String>,
) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::WorkspaceSnapshot { id, message })? {
        ApiResult::WorkspaceSnapshot { workspace } => {
            print_workspace_snapshot(*workspace);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_workspace_snapshot(workspace: anvics_core::WorkspaceView) {
    println!("Snapshotted workspace {}", workspace.id);
    if let Some(snapshot) = workspace.latest_snapshot {
        println!("snapshot: {snapshot}");
    }
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

    print_evidence_attached("Attached evidence", evidence);
    Ok(())
}

fn attach_evidence_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    thread: String,
    command: String,
    exit_code: i32,
    summary: String,
    artifact_path: Option<String>,
) -> Result<()> {
    match daemon_request(
        &socket,
        root,
        ApiMethod::EvidenceAttach {
            thread,
            command,
            exit_code,
            summary,
            artifact_path,
        },
    )? {
        ApiResult::EvidenceAttached { evidence } => {
            print_evidence_attached("Attached evidence", evidence);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_evidence_attached(prefix: &str, evidence: anvics_core::EvidenceRecord) {
    println!("{prefix} {}", evidence.id);
    println!("thread: {}", evidence.thread_id);
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

    print_evidence_attached("Attached command evidence", evidence);
    Ok(())
}

fn attach_command_evidence_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    thread: String,
    options: CommandEvidenceOptions,
) -> Result<()> {
    match daemon_request(
        &socket,
        root,
        ApiMethod::EvidenceCommand {
            thread,
            command: options.command,
            command_file: options
                .command_file
                .map(|path| path.to_string_lossy().to_string()),
            command_label: options.label,
            cwd: options.cwd,
            exit_code: options.exit_code,
            summary: options.summary,
            artifact_path: options.artifact,
        },
    )? {
        ApiResult::EvidenceAttached { evidence } => {
            print_evidence_attached("Attached command evidence", evidence);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn run_command(root: PathBuf, options: CommandRunOptions) -> Result<()> {
    let input = command_run_input(options)?;
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let result = store
        .run_command(input)
        .context("failed to run command through Anvics")?;

    print_command_run(result.command_event, result.evidence);
    Ok(())
}

fn run_command_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    options: CommandRunOptions,
) -> Result<()> {
    let input = command_run_input(options)?;
    match daemon_request(
        &socket,
        root,
        ApiMethod::CommandRun {
            workspace: input.workspace_id,
            argv: input.argv,
            command_file: input.command_file,
            command_label: input.command_label,
            cwd: input.cwd,
            timeout_seconds: input.timeout_seconds,
            summary: input.summary,
            artifact_path: input.artifact_path,
        },
    )? {
        ApiResult::CommandRun {
            command_event,
            evidence,
        } => {
            print_command_run(*command_event, evidence);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn show_command_event(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let command_event = store
        .show_command_event(id)
        .with_context(|| format!("failed to show command event {id}"))?;

    println!("{}", serde_json::to_string_pretty(&command_event)?);
    Ok(())
}

fn show_command_event_via_daemon(root: PathBuf, socket: PathBuf, id: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::CommandShow { id })? {
        ApiResult::CommandShow { command_event } => {
            println!("{}", serde_json::to_string_pretty(&*command_event)?);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_command_run(
    command_event: anvics_core::CommandEvent,
    evidence: anvics_core::EvidenceRecord,
) {
    println!("Ran command {}", command_event.id);
    println!("workspace: {}", command_event.workspace_id);
    println!("thread: {}", command_event.thread_id);
    println!("label: {}", command_event.command_label);
    println!("exit_code: {}", command_event.exit_code.unwrap_or(-1));
    println!("timed_out: {}", command_event.timed_out);
    println!("evidence: {}", evidence.id);
    if let Some(stdout) = command_event.stdout_path {
        println!("stdout: {stdout}");
    }
    if let Some(stderr) = command_event.stderr_path {
        println!("stderr: {stderr}");
    }
}

fn create_review(root: PathBuf, thread_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let review = store
        .create_review(thread_id)
        .context("failed to create review")?;

    print_review_created(review);
    Ok(())
}

fn create_review_via_daemon(root: PathBuf, socket: PathBuf, thread: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::ReviewCreate { thread })? {
        ApiResult::ReviewCreate { review } => {
            print_review_created(*review);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_review_created(review: anvics_core::ReviewProjection) {
    println!("Created review {}", review.id);
    println!("thread: {}", review.thread_id);
    println!("changed_paths: {}", review.changed_paths.len());
    println!("overlap_notes: {}", review.overlap_notes.len());
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

fn show_review_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    id: String,
    format: ReviewFormat,
) -> Result<()> {
    match daemon_request(
        &socket,
        root,
        ApiMethod::ReviewShow {
            id,
            format: api_review_format(format),
        },
    )? {
        ApiResult::ReviewShowJson { review } => {
            println!("{}", serde_json::to_string_pretty(&review)?);
            Ok(())
        }
        ApiResult::ReviewShowMarkdown { markdown } => {
            println!("{markdown}");
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn show_review_path(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let path = store
        .review_markdown_file_path(id)
        .with_context(|| format!("failed to find review {id} markdown path"))?;

    println!("{}", path.display());
    Ok(())
}

fn show_review_path_via_daemon(root: PathBuf, socket: PathBuf, id: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::ReviewPath { id })? {
        ApiResult::ReviewPath { path } => {
            println!("{path}");
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn scan_risks(root: PathBuf, review_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let scan = store
        .scan_review_risks(review_id)
        .context("failed to scan review risks")?;

    print_risk_scan(&scan);
    Ok(())
}

fn scan_risks_via_daemon(root: PathBuf, socket: PathBuf, review: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::RiskScan { review })? {
        ApiResult::RiskScan { scan } => {
            print_risk_scan(&scan);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn list_risks(root: PathBuf, review_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let findings = store
        .list_review_risk_findings(review_id)
        .context("failed to list review risks")?;

    print_risk_findings(findings);
    Ok(())
}

fn list_risks_via_daemon(root: PathBuf, socket: PathBuf, review: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::RiskList { review })? {
        ApiResult::RiskList { findings } => {
            print_risk_findings(findings);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn show_risk(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let finding = store
        .show_risk_finding(id)
        .context("failed to show risk finding")?;

    print_risk_finding(&finding);
    Ok(())
}

fn show_risk_via_daemon(root: PathBuf, socket: PathBuf, id: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::RiskShow { id })? {
        ApiResult::RiskShow { finding } => {
            print_risk_finding(&finding);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_risk_scan(scan: &anvics_core::RiskScan) {
    println!("Risk scan {}", scan.id);
    println!("review: {}", scan.review_id);
    println!("findings: {}", scan.findings.len());
    print_risk_findings(scan.findings.clone());
}

fn print_risk_findings(findings: Vec<anvics_core::RiskFinding>) {
    if findings.is_empty() {
        println!("No risk findings");
        return;
    }
    for finding in findings {
        print_risk_finding(&finding);
    }
}

fn print_risk_finding(finding: &anvics_core::RiskFinding) {
    let line = finding
        .line
        .map(|line| format!(":{line}"))
        .unwrap_or_default();
    println!("finding: {}", finding.id);
    println!("review: {}", finding.review_id);
    println!("severity: {:?}", finding.severity);
    println!("detector: {}", finding.detector);
    println!(
        "target: {:?} {}{}",
        finding.target_kind, finding.target_path, line
    );
    println!("excerpt: {}", finding.redacted_excerpt);
}

fn create_publication(
    root: PathBuf,
    thread_id: &str,
    review_id: &str,
    options: PublicationOptions,
) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let publication = store
        .create_publication_with_options(thread_id, review_id, options)
        .context("failed to create publication")?;

    print_publication_created(&root, publication);
    Ok(())
}

fn create_publication_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    thread: String,
    review: String,
    options: PublicationOptions,
) -> Result<()> {
    match daemon_request(
        &socket,
        root.clone(),
        ApiMethod::PublishCreate {
            thread,
            review,
            allow_secret_risk: options.allow_secret_risk,
            override_reason: options.override_reason,
        },
    )? {
        ApiResult::PublishCreate { publication } => {
            print_publication_created(&root, publication);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_publication_created(root: &std::path::Path, publication: anvics_core::NativePublication) {
    println!("Created publication {}", publication.id);
    println!("thread: {}", publication.thread_id);
    println!("accepted_snapshot: {}", publication.accepted_snapshot);
    println!(
        "legacy_export: anvics --repo {} legacy git export --publication {} --output accepted.patch",
        shell_quote(&display_path(root)),
        publication.id
    );
}

fn prepare_agent(root: PathBuf, title: String, task: String) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let preparation = store
        .prepare_agent(title, task)
        .context("failed to prepare agent task")?;

    print_agent_preparation(&root, preparation);
    Ok(())
}

fn prepare_agent_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    title: String,
    task: String,
) -> Result<()> {
    match daemon_request(
        &socket,
        root.clone(),
        ApiMethod::AgentPrepare { title, task },
    )? {
        ApiResult::AgentPrepare { preparation } => {
            print_agent_preparation(&root, *preparation);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_agent_preparation(root: &std::path::Path, preparation: anvics_core::AgentPreparation) {
    println!("Prepared agent task");
    println!("thread: {}", preparation.thread.id);
    println!("workspace: {}", preparation.workspace.id);
    println!(
        "workspace_path: {}",
        preparation.workspace.materialized_path
    );
    println!("packet: {}", preparation.packet_path);
    println!(
        "accept_run: anvics --repo {} agent accept --workspace {} --run-label \"<short label>\" --run-summary \"<short summary>\" -- <program> [args...]",
        shell_quote(&display_path(root)),
        preparation.workspace.id
    );
    println!(
        "accept: anvics --repo {} agent accept --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"",
        shell_quote(&display_path(root)),
        preparation.workspace.id
    );
    println!(
        "finish: anvics --repo {} agent finish --workspace {} --command \"<command>\" --exit-code <code> --summary \"<short summary>\"",
        shell_quote(&display_path(root)),
        preparation.workspace.id
    );
}

fn show_agent_packet(root: PathBuf, thread_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let path = store
        .agent_packet_file_path(thread_id)
        .with_context(|| format!("failed to find agent packet for thread {thread_id}"))?;

    println!("{}", path.display());
    Ok(())
}

fn show_agent_packet_via_daemon(root: PathBuf, socket: PathBuf, thread: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::AgentPacket { thread })? {
        ApiResult::AgentPacket { path } => {
            println!("{path}");
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn enter_agent(root: PathBuf, workspace_id: &str, name: String) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let status = store
        .enter_agent_session(workspace_id, name)
        .context("failed to enter agent workspace")?;

    print_agent_enter(&status);
    Ok(())
}

fn enter_agent_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    workspace: String,
    name: String,
) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::AgentEnter { workspace, name })? {
        ApiResult::AgentEnter { status } => {
            print_agent_enter(&status);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn leave_agent(root: PathBuf, session_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let session = store
        .leave_agent_session(session_id)
        .context("failed to leave agent session")?;

    print_agent_session("Left agent session", &session);
    Ok(())
}

fn leave_agent_via_daemon(root: PathBuf, socket: PathBuf, session: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::AgentLeave { session })? {
        ApiResult::AgentLeave { session } => {
            print_agent_session("Left agent session", &session);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn list_agent_sessions(root: PathBuf, thread: Option<&str>, workspace: Option<&str>) -> Result<()> {
    if thread.is_none() && workspace.is_none() {
        anyhow::bail!("agent sessions requires --thread or --workspace");
    }
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let sessions = store
        .list_agent_sessions(thread, workspace)
        .context("failed to list agent sessions")?;

    print_agent_sessions(sessions);
    Ok(())
}

fn list_agent_sessions_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    thread: Option<String>,
    workspace: Option<String>,
) -> Result<()> {
    if thread.is_none() && workspace.is_none() {
        anyhow::bail!("agent sessions requires --thread or --workspace");
    }
    match daemon_request(
        &socket,
        root,
        ApiMethod::AgentSessions { thread, workspace },
    )? {
        ApiResult::AgentSessions { sessions } => {
            print_agent_sessions(sessions);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn coordination_status(root: PathBuf, workspace_id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let status = store
        .coordination_status(workspace_id)
        .context("failed to compute coordination status")?;

    print_coordination_status(&status);
    Ok(())
}

fn coordination_status_via_daemon(root: PathBuf, socket: PathBuf, workspace: String) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::CoordinationStatus { workspace })? {
        ApiResult::CoordinationStatus { status } => {
            print_coordination_status(&status);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_agent_enter(status: &anvics_core::CoordinationStatus) {
    if let Some(session) = &status.current_session {
        print_agent_session("Entered agent session", session);
    }
    print_coordination_status(status);
}

fn print_agent_session(prefix: &str, session: &anvics_core::AgentSession) {
    println!("{prefix} {}", session.id);
    println!("agent: {}", session.agent_name);
    println!("thread: {}", session.thread_id);
    println!("workspace: {}", session.workspace_id);
    println!("status: {:?}", session.status);
    println!("last_seen_at: {}", session.last_seen_at);
    if let Some(finished_at) = &session.finished_at {
        println!("finished_at: {finished_at}");
    }
}

fn print_agent_sessions(sessions: Vec<anvics_core::AgentSession>) {
    if sessions.is_empty() {
        println!("No agent sessions");
        return;
    }
    for session in sessions {
        println!(
            "{}  {:?}  {}  thread={}  workspace={}  last_seen={}",
            session.id,
            session.status,
            session.agent_name,
            session.thread_id,
            session.workspace_id,
            session.last_seen_at
        );
    }
}

fn print_coordination_status(status: &anvics_core::CoordinationStatus) {
    println!("coordination_workspace: {}", status.workspace.id);
    println!("thread: {}", status.thread.id);
    println!("title: {}", status.thread.title);
    if status.known_changed_paths.is_empty() {
        println!("known_changed_paths: none");
    } else {
        println!(
            "known_changed_paths: {}",
            status.known_changed_paths.join(", ")
        );
    }

    if status.related_work.is_empty() {
        println!("related_work: none");
    } else {
        println!("related_work:");
        for related in &status.related_work {
            println!(
                "- agent={} thread=\"{}\" workspace={}",
                related.agent_name, related.thread_title, related.workspace_id
            );
            if let Some(session_id) = &related.session_id {
                println!("  session: {session_id}");
            }
            if related.known_changed_paths.is_empty() {
                println!("  known_changed_paths: none");
            } else {
                println!(
                    "  known_changed_paths: {}",
                    related.known_changed_paths.join(", ")
                );
            }
            if related.overlap_paths.is_empty() {
                println!("  overlap_paths: none");
            } else {
                println!("  overlap_paths: {}", related.overlap_paths.join(", "));
            }
            println!("  freshness: {}", related.freshness_note);
        }
    }

    if status.potential_clash_notes.is_empty() {
        println!("potential_clashes: none");
    } else {
        println!("potential_clashes:");
        for note in &status.potential_clash_notes {
            println!("- {note}");
        }
    }
}

fn show_agent_status(
    root: PathBuf,
    thread_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let thread_id = resolve_status_thread(&store, thread_id, workspace_id)?;
    let status = store
        .agent_status(&thread_id)
        .with_context(|| format!("failed to show agent status for thread {thread_id}"))?;

    print_agent_status(status);
    Ok(())
}

fn show_agent_status_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    thread: Option<String>,
    workspace: Option<String>,
) -> Result<()> {
    let thread = if let Some(thread) = thread {
        thread
    } else if let Some(workspace) = workspace {
        match daemon_request(
            &socket,
            root.clone(),
            ApiMethod::WorkspaceShow { id: workspace },
        )? {
            ApiResult::WorkspaceShow { workspace, .. } => workspace.thread_id.to_string(),
            result => return unexpected_daemon_result(result),
        }
    } else {
        anyhow::bail!("agent status requires --thread or --workspace");
    };
    match daemon_request(&socket, root, ApiMethod::AgentStatus { thread })? {
        ApiResult::AgentStatus { status } => {
            print_agent_status(*status);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn resolve_status_thread(
    store: &AnvicsStore,
    thread_id: Option<&str>,
    workspace_id: Option<&str>,
) -> Result<String> {
    match (thread_id, workspace_id) {
        (Some(thread), None) => Ok(thread.to_owned()),
        (None, Some(workspace)) => Ok(store
            .show_workspace(workspace)
            .with_context(|| format!("failed to show workspace {workspace}"))?
            .thread_id
            .to_string()),
        (Some(_), Some(_)) => {
            anyhow::bail!("agent status accepts --thread or --workspace, not both")
        }
        (None, None) => anyhow::bail!("agent status requires --thread or --workspace"),
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

    print_agent_finish(finish);
    Ok(())
}

fn finish_agent_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    workspace: String,
    options: CommandEvidenceOptions,
) -> Result<()> {
    match daemon_request(
        &socket,
        root,
        ApiMethod::AgentFinish {
            workspace,
            command: options.command,
            command_file: options
                .command_file
                .map(|path| path.to_string_lossy().to_string()),
            command_label: options.label,
            cwd: options.cwd,
            exit_code: options.exit_code,
            summary: options.summary,
            artifact_path: options.artifact,
        },
    )? {
        ApiResult::AgentFinish { finish } => {
            print_agent_finish(*finish);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_agent_finish(finish: anvics_core::AgentFinish) {
    println!("Finished agent task");
    println!("thread: {}", finish.workspace.thread_id);
    println!("workspace: {}", finish.workspace.id);
    if let Some(snapshot) = finish.workspace.latest_snapshot {
        println!("snapshot: {snapshot}");
    }
    println!("evidence: {}", finish.evidence.id);
    println!("review: {}", finish.review.id);
    println!("review_markdown: {}", finish.review_markdown_path);
}

fn accept_agent(
    root: PathBuf,
    workspace_id: &str,
    options: AgentAcceptOptions,
    output: Option<PathBuf>,
) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let publication_options = options.publication_options();
    let result = if options.run_label.is_some() || !options.argv.is_empty() {
        let input = agent_accept_run_input(workspace_id.to_owned(), options)?;
        store
            .accept_agent_with_command_run_and_options(input, output, publication_options)
            .map_err(|error| (error, "failed to accept agent workspace with command run"))
    } else {
        let input = accept_command_input(options)?;
        store
            .accept_agent_with_evidence_and_options(
                workspace_id,
                input,
                output,
                publication_options,
            )
            .map_err(|error| (error, "failed to accept agent workspace"))
    };
    let acceptance = match result {
        Ok(acceptance) => acceptance,
        Err((error @ StoreError::PublicationBlockedSecretRisk { .. }, context)) => {
            if let Err(hint_error) = print_accept_recovery_hint(&root, &store, workspace_id) {
                eprintln!("Recovery hint unavailable: {hint_error:#}");
            }
            return Err(error).context(context);
        }
        Err((error, context)) => return Err(error).context(context),
    };

    print_agent_acceptance(acceptance);
    Ok(())
}

fn print_accept_recovery_hint(
    root: &std::path::Path,
    store: &AnvicsStore,
    workspace_id: &str,
) -> Result<()> {
    let workspace = store
        .show_workspace(workspace_id)
        .with_context(|| format!("failed to show workspace {workspace_id}"))?;
    let status = store
        .agent_status(workspace.thread_id.as_str())
        .with_context(|| {
            format!(
                "failed to show agent status for thread {}",
                workspace.thread_id
            )
        })?;
    eprintln!("Recovery hint: Anvics preserved the evidence, snapshot, and review.");
    eprintln!(
        "  anvics --repo {} agent status --workspace {}",
        shell_quote(&display_path(root)),
        workspace.id
    );
    if let Some(review_id) = status.review_ids.last() {
        eprintln!(
            "  anvics --repo {} risk list --review {}",
            shell_quote(&display_path(root)),
            review_id
        );
        eprintln!(
            "  anvics --repo {} publish create --thread {} --review {} --allow-secret-risk --override-reason \"<audited reason>\"",
            shell_quote(&display_path(root)),
            workspace.thread_id,
            review_id
        );
    }
    Ok(())
}

fn accept_agent_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    workspace: String,
    options: AgentAcceptOptions,
    output: Option<PathBuf>,
) -> Result<()> {
    let allow_secret_risk = options.allow_secret_risk;
    let override_reason = options.override_reason.clone();
    let method = if options.run_label.is_some() || !options.argv.is_empty() {
        let input = agent_accept_run_input(workspace, options)?;
        ApiMethod::AgentAcceptRun {
            workspace: input.workspace_id,
            argv: input.argv,
            command_file: input.command_file,
            command_label: input.command_label,
            cwd: input.cwd,
            timeout_seconds: input.timeout_seconds,
            summary: input.summary,
            artifact_path: input.artifact_path,
            output_path: output.map(|path| path.to_string_lossy().to_string()),
            allow_secret_risk,
            override_reason,
        }
    } else {
        let input = accept_command_input(options)?;
        ApiMethod::AgentAccept {
            workspace,
            command: Some(input.command),
            command_file: input.command_file,
            command_label: input.command_label,
            cwd: input.cwd,
            exit_code: input.exit_code,
            summary: input.summary,
            artifact_path: input.artifact_path,
            output_path: output.map(|path| path.to_string_lossy().to_string()),
            allow_secret_risk,
            override_reason,
        }
    };

    match daemon_request(&socket, root, method)? {
        ApiResult::AgentAccept { acceptance } => {
            print_agent_acceptance(*acceptance);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_agent_acceptance(acceptance: anvics_core::AgentAcceptance) {
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
        command_event_id: None,
        command_label: options.label,
        command_file,
        cwd: options.cwd,
        exit_code: options.exit_code,
        summary: options.summary,
        artifact_path: options.artifact,
        stdout_path: None,
        stderr_path: None,
    })
}

fn accept_command_input(options: AgentAcceptOptions) -> Result<CommandEvidenceInput> {
    let exit_code = options
        .exit_code
        .ok_or_else(|| anyhow::anyhow!("--exit-code is required unless using --run-label"))?;
    let summary = options
        .summary
        .ok_or_else(|| anyhow::anyhow!("--summary is required unless using --run-summary"))?;
    command_input(CommandEvidenceOptions {
        command: options.command,
        command_file: options.command_file,
        label: options.label,
        cwd: options.cwd,
        exit_code,
        summary,
        artifact: options.artifact,
    })
}

fn command_run_input(options: CommandRunOptions) -> Result<CommandRunInput> {
    if options.command_file.is_none() && options.argv.is_empty() {
        anyhow::bail!("command run requires --command-file or command argv after --");
    }
    Ok(CommandRunInput {
        workspace_id: options.workspace,
        argv: options.argv,
        command_file: options
            .command_file
            .map(|path| path.to_string_lossy().to_string()),
        command_label: options.label,
        cwd: options.cwd,
        timeout_seconds: options.timeout_seconds,
        summary: options.summary,
        artifact_path: options.artifact,
    })
}

fn agent_accept_run_input(
    workspace: String,
    options: AgentAcceptOptions,
) -> Result<CommandRunInput> {
    let label = options
        .run_label
        .ok_or_else(|| anyhow::anyhow!("--run-label is required for command-run accept"))?;
    let summary = options
        .run_summary
        .ok_or_else(|| anyhow::anyhow!("--run-summary is required for command-run accept"))?;
    if options.argv.is_empty() {
        anyhow::bail!("command-run accept requires command argv after --");
    }
    Ok(CommandRunInput {
        workspace_id: workspace,
        argv: options.argv,
        command_file: None,
        command_label: label,
        cwd: options.cwd,
        timeout_seconds: options.run_timeout_seconds,
        summary,
        artifact_path: options.artifact,
    })
}

fn export_legacy_git_patch(root: PathBuf, publication_id: &str, output: PathBuf) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let output = store
        .export_legacy_git_patch(publication_id, output)
        .context("failed to export legacy Git patch")?;

    print_legacy_git_export(output);
    Ok(())
}

fn export_legacy_git_patch_via_daemon(
    root: PathBuf,
    socket: PathBuf,
    publication: String,
    output: PathBuf,
) -> Result<()> {
    match daemon_request(
        &socket,
        root,
        ApiMethod::LegacyGitExport {
            publication,
            output: output.to_string_lossy().to_string(),
        },
    )? {
        ApiResult::LegacyGitExport { output } => {
            print_legacy_git_export(PathBuf::from(output));
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_legacy_git_export(output: PathBuf) {
    println!("Exported legacy Git patch");
    println!("path: {}", output.display());
}

fn list_events(root: PathBuf, since: u64) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let events = store
        .events_since(since)
        .context("failed to list repository events")?;

    print_events(events);
    Ok(())
}

fn list_events_via_daemon(root: PathBuf, socket: PathBuf, since: u64) -> Result<()> {
    match daemon_request(&socket, root, ApiMethod::EventsSince { sequence: since })? {
        ApiResult::EventsSince { events } => {
            print_events(events);
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn print_events(events: Vec<anvics_core::RepositoryEvent>) {
    if events.is_empty() {
        println!("No events");
        return;
    }
    for event in events {
        let subject = event.subject_id.unwrap_or_default();
        if subject.is_empty() {
            println!("{}  {:?}  {}", event.sequence, event.kind, event.created_at);
        } else {
            println!(
                "{}  {:?}  {}  {}",
                event.sequence, event.kind, subject, event.created_at
            );
        }
    }
}

fn ping_daemon(socket: Option<PathBuf>) -> Result<()> {
    let socket = socket
        .or_else(|| std::env::var_os("ANVICS_DAEMON_SOCKET").map(PathBuf::from))
        .context("daemon socket required: pass --socket or set ANVICS_DAEMON_SOCKET")?;
    match daemon_request(&socket, std::env::current_dir()?, ApiMethod::Ping)? {
        ApiResult::Pong => {
            println!("daemon: ok");
            println!("socket: {}", socket.display());
            Ok(())
        }
        result => unexpected_daemon_result(result),
    }
}

fn api_review_format(format: ReviewFormat) -> ApiReviewFormat {
    match format {
        ReviewFormat::Json => ApiReviewFormat::Json,
        ReviewFormat::Markdown => ApiReviewFormat::Markdown,
    }
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
