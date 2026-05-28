use anvics_store::{AnvicsStore, StoreError};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "anvics")]
#[command(about = "Agent-native source control")]
struct Cli {
    #[arg(long, global = true, value_name = "DIR")]
    repo: Option<PathBuf>,

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
}

#[derive(Debug, Subcommand)]
enum RepoCommand {
    Init,
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
}

#[derive(Debug, Subcommand)]
enum ReviewCommand {
    Create {
        #[arg(long)]
        thread: String,
    },
    Show {
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = cli.repo.unwrap_or(std::env::current_dir()?);

    match cli.command {
        Command::Repo {
            command: RepoCommand::Init,
        } => init_repo(root),
        Command::Snapshot {
            command: SnapshotCommand::Create { message },
        } => create_snapshot(root, message),
        Command::Snapshot {
            command: SnapshotCommand::List,
        } => list_snapshots(root),
        Command::Snapshot {
            command: SnapshotCommand::Show { id },
        } => show_snapshot(root, &id),
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
        Command::Review {
            command: ReviewCommand::Create { thread },
        } => create_review(root, &thread),
        Command::Review {
            command: ReviewCommand::Show { id },
        } => show_review(root, &id),
        Command::Publish {
            command: PublishCommand::Create { thread, review },
        } => create_publication(root, &thread, &review),
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

fn list_snapshots(root: PathBuf) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let snapshots = store.list_snapshots().context("failed to list snapshots")?;

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

fn show_snapshot(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let snapshot = store
        .show_snapshot(id)
        .with_context(|| format!("failed to show snapshot {id}"))?;

    println!("{}", serde_json::to_string_pretty(&snapshot)?);
    Ok(())
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

fn show_review(root: PathBuf, id: &str) -> Result<()> {
    let store = AnvicsStore::open(&root).context("failed to open Anvics repository")?;
    let review = store
        .show_review(id)
        .with_context(|| format!("failed to show review {id}"))?;

    println!("{}", serde_json::to_string_pretty(&review)?);
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
    Ok(())
}
