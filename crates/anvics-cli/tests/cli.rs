use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::tempdir;

#[test]
fn repo_init_creates_layout() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("anvics")
        .unwrap()
        .args(["--repo", dir.path().to_str().unwrap(), "repo", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Anvics repository"));

    assert!(dir.path().join(".anvics/repo.json").exists());
}

#[test]
fn repo_doctor_reports_config_classification_and_daemon_matches() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("anvics.toml"),
        "[generated]\ntracked = [\"src/generated/**\"]\nuntracked = [\"dist/**\"]\n\n[ignore]\npaths = [\"cache/**\"]\n\n[evidence]\ncandidate_paths = [\"reports/**\"]\n",
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("src/generated")).unwrap();
    fs::write(dir.path().join("src/generated/client.rs"), "old\n").unwrap();
    fs::create_dir_all(dir.path().join("dist")).unwrap();
    fs::write(dir.path().join("dist/bundle.js"), "old\n").unwrap();
    anvics(dir.path(), &["repo", "init"]).assert().success();

    anvics(
        dir.path(),
        &[
            "repo",
            "doctor",
            "--path",
            "src/generated/client.rs",
            "--path",
            "dist/bundle.js",
            "--path",
            "reports/test.txt",
            "--path",
            "src/lib.rs",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Anvics repo doctor"))
    .stdout(predicate::str::contains("config_present: true"))
    .stdout(predicate::str::contains(
        "generated_tracked: src/generated/**",
    ))
    .stdout(predicate::str::contains(
        "- src/generated/client.rs: generated_tracked",
    ))
    .stdout(predicate::str::contains(
        "- dist/bundle.js: generated_untracked",
    ))
    .stdout(predicate::str::contains(
        "- reports/test.txt: evidence_candidate",
    ))
    .stdout(predicate::str::contains("- src/lib.rs: source"))
    .stdout(predicate::str::contains("accepted repo-root anvics.toml"));

    let socket_dir = tempdir().unwrap();
    let socket = socket_dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    daemon_anvics(
        dir.path(),
        &socket,
        &["repo", "doctor", "--path", "src/generated/client.rs"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "- src/generated/client.rs: generated_tracked",
    ));

    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Classified diff",
            "--task",
            "Edit generated and dist files",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    fs::write(format!("{workspace_path}/src/generated/client.rs"), "new\n").unwrap();
    fs::write(format!("{workspace_path}/dist/bundle.js"), "new\n").unwrap();

    anvics(dir.path(), &["workspace", "diff", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Modified: src/generated/client.rs (generated_tracked)",
        ))
        .stdout(predicate::str::contains(
            "Modified: dist/bundle.js (generated_untracked)",
        ));
    daemon_anvics(dir.path(), &socket, &["workspace", "diff", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Modified: src/generated/client.rs (generated_tracked)",
        ))
        .stdout(predicate::str::contains(
            "Modified: dist/bundle.js (generated_untracked)",
        ));
    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn snapshot_create_list_show() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("README.md"), "hello").unwrap();

    Command::cargo_bin("anvics")
        .unwrap()
        .args(["--repo", dir.path().to_str().unwrap(), "repo", "init"])
        .assert()
        .success();

    let create = Command::cargo_bin("anvics")
        .unwrap()
        .args([
            "--repo",
            dir.path().to_str().unwrap(),
            "snapshot",
            "create",
            "--message",
            "initial",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created snapshot"))
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(create).unwrap();
    let id = output
        .lines()
        .find_map(|line| line.strip_prefix("Created snapshot "))
        .unwrap()
        .to_owned();

    Command::cargo_bin("anvics")
        .unwrap()
        .args(["--repo", dir.path().to_str().unwrap(), "snapshot", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("initial"));

    Command::cargo_bin("anvics")
        .unwrap()
        .args([
            "--repo",
            dir.path().to_str().unwrap(),
            "snapshot",
            "show",
            &id,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"message\": \"initial\""));
}

#[test]
fn agent_thread_workspace_review_publish_flow() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();

    let first_thread = created_id(
        &anvics(
            dir.path(),
            &[
                "thread",
                "create",
                "--title",
                "Agent A",
                "--task",
                "Change app text",
            ],
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("Created thread"))
        .get_output()
        .stdout,
        "Created thread ",
    );
    let second_thread = created_id(
        &anvics(
            dir.path(),
            &[
                "thread",
                "create",
                "--title",
                "Agent B",
                "--task",
                "Also change app text",
            ],
        )
        .assert()
        .success()
        .get_output()
        .stdout,
        "Created thread ",
    );

    let first_workspace_output = anvics(
        dir.path(),
        &["workspace", "create", "--thread", &first_thread],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let first_workspace = created_id(&first_workspace_output, "Created workspace ");
    let first_path = value_after_prefix(&first_workspace_output, "path: ");

    let second_workspace_output = anvics(
        dir.path(),
        &["workspace", "create", "--thread", &second_thread],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let second_workspace = created_id(&second_workspace_output, "Created workspace ");
    let second_path = value_after_prefix(&second_workspace_output, "path: ");

    fs::write(format!("{first_path}/app.txt"), "agent a\n").unwrap();
    fs::write(format!("{second_path}/app.txt"), "agent b\n").unwrap();

    anvics(
        dir.path(),
        &[
            "evidence",
            "attach",
            "--thread",
            &first_thread,
            "--command",
            "scripted-agent-a",
            "--exit-code",
            "0",
            "--summary",
            "Agent A changed app.txt",
        ],
    )
    .assert()
    .success();
    anvics(
        dir.path(),
        &[
            "evidence",
            "attach",
            "--thread",
            &second_thread,
            "--command",
            "scripted-agent-b",
            "--exit-code",
            "0",
            "--summary",
            "Agent B changed app.txt",
        ],
    )
    .assert()
    .success();

    anvics(
        dir.path(),
        &[
            "workspace",
            "snapshot",
            &first_workspace,
            "--message",
            "Agent A result",
        ],
    )
    .assert()
    .success();
    anvics(
        dir.path(),
        &[
            "workspace",
            "snapshot",
            &second_workspace,
            "--message",
            "Agent B result",
        ],
    )
    .assert()
    .success();

    let review_output = anvics(dir.path(), &["review", "create", "--thread", &first_thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("overlap_notes: 1"))
        .get_output()
        .stdout
        .clone();
    let review = created_id(&review_output, "Created review ");

    anvics(dir.path(), &["review", "show", &review])
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent A changed app.txt"))
        .stdout(predicate::str::contains("app.txt"))
        .stdout(predicate::str::contains("\"status\": \"modified\""));

    anvics(
        dir.path(),
        &[
            "publish",
            "create",
            "--thread",
            &first_thread,
            "--review",
            &review,
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Created publication"));
}

#[test]
fn agent_prepare_finish_and_legacy_patch_export_flow() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
    fs::write(dir.path().join("deleted.txt"), "delete me\n").unwrap();
    fs::create_dir_all(dir.path().join("skills/anvics-skill")).unwrap();
    fs::write(
        dir.path().join("skills/anvics-skill/SKILL.md"),
        "# Anvics\n",
    )
    .unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();

    let prepare_output = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Live Agent",
            "--task",
            "Modify, add, and delete files",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Prepared agent task"))
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare_output, "thread: ");
    let workspace = value_after_prefix(&prepare_output, "workspace: ");
    let workspace_path = value_after_prefix(&prepare_output, "workspace_path: ");
    let packet = value_after_prefix(&prepare_output, "packet: ");
    let packet_text = fs::read_to_string(packet).unwrap();
    assert!(packet_text.contains(&thread));
    assert!(packet_text.contains(&workspace));
    assert!(packet_text.contains("Modify, add, and delete files"));
    assert!(packet_text.contains("anvics --repo"));
    assert!(packet_text.contains("## Anvics CLI"));
    assert!(packet_text.contains("Use this command prefix"));
    assert!(packet_text.contains("If the command is unavailable"));
    assert!(packet_text.contains("only editable area"));
    assert!(packet_text.contains("## Anvics Skill"));
    assert!(packet_text.contains("skills/anvics-skill/SKILL.md"));
    assert!(packet_text.contains("Before editing, read and follow the Anvics skill"));
    assert!(packet_text.contains("agent enter"));
    assert!(packet_text.contains("agent context-pack"));
    assert!(packet_text.contains("agent checkpoint"));
    assert!(packet_text.contains("agent recover"));
    assert!(packet_text.contains("coordination status"));
    assert!(packet_text.contains("workspace diff"));
    assert!(packet_text.contains("If you spawn subagents"));
    assert!(packet_text.contains("## Agent-Run Commands"));
    assert!(packet_text.contains("## Operator-Run Commands"));
    assert!(packet_text.contains("Do not run them as an agent"));
    anvics(dir.path(), &["agent", "packet", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains(".anvics/agent-packets"));
    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("evidence_count: 0"))
        .stdout(predicate::str::contains("publication_status: unpublished"))
        .stdout(predicate::str::contains(&workspace));

    fs::write(format!("{workspace_path}/modified.txt"), "after\n").unwrap();
    fs::remove_file(format!("{workspace_path}/deleted.txt")).unwrap();
    fs::write(format!("{workspace_path}/added.txt"), "new\n").unwrap();
    let artifact = dir.path().join("agent-summary.txt");
    fs::write(&artifact, "compact result\n").unwrap();

    let finish_output = anvics(
        dir.path(),
        &[
            "agent",
            "finish",
            "--workspace",
            &workspace,
            "--command",
            "scripted-live-agent",
            "--exit-code",
            "0",
            "--summary",
            "Scripted live agent changed three files",
            "--artifact",
            artifact.to_str().unwrap(),
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Finished agent task"))
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&finish_output, "review: ");
    let review_markdown = value_after_prefix(&finish_output, "review_markdown: ");

    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "Scripted live agent changed three files",
    ))
    .stdout(predicate::str::contains("Modify, add, and delete files"))
    .stdout(predicate::str::contains("anvics --repo"))
    .stdout(predicate::str::contains("anvics --repo").count(4))
    .stdout(predicate::str::contains("agent accept"))
    .stdout(predicate::str::contains("publish create"));
    anvics(dir.path(), &["review", "path", &review])
        .assert()
        .success()
        .stdout(predicate::str::contains(review_markdown));
    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("evidence_count: 1"))
        .stdout(predicate::str::contains(&review));

    let publish_output = anvics(
        dir.path(),
        &[
            "publish", "create", "--thread", &thread, "--review", &review,
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Created publication"))
    .stdout(predicate::str::contains("legacy_export: anvics --repo"))
    .get_output()
    .stdout
    .clone();
    let publication = value_after_prefix(&publish_output, "Created publication ");
    let patch_path = dir.path().join("accepted.patch");

    anvics(
        dir.path(),
        &[
            "legacy",
            "git",
            "export",
            "--publication",
            &publication,
            "--output",
            patch_path.to_str().unwrap(),
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Exported legacy Git patch"));

    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("publication_status: published"))
        .stdout(predicate::str::contains(&publication));

    let clean = tempdir().unwrap();
    fs::write(clean.path().join("modified.txt"), "before\n").unwrap();
    fs::write(clean.path().join("deleted.txt"), "delete me\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(clean.path())
        .output()
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
fn agent_launch_prompt_includes_codex_flags_and_daemon_matches() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    fs::create_dir_all(dir.path().join("skills/anvics-skill")).unwrap();
    fs::write(
        dir.path().join("skills/anvics-skill/SKILL.md"),
        "# Anvics\n",
    )
    .unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare_output = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Launch Prompt",
            "--task",
            "Edit app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let workspace = value_after_prefix(&prepare_output, "workspace: ");
    let workspace_path = value_after_prefix(&prepare_output, "workspace_path: ");
    let packet = value_after_prefix(&prepare_output, "packet: ");

    anvics(
        dir.path(),
        &[
            "agent",
            "launch-prompt",
            "--workspace",
            &workspace,
            "--tool",
            "codex",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("--skip-git-repo-check"))
    .stdout(predicate::str::contains("--cd"))
    .stdout(predicate::str::contains(&workspace_path))
    .stdout(predicate::str::contains(&packet))
    .stdout(predicate::str::contains("skills/anvics-skill/SKILL.md"))
    .stdout(predicate::str::contains("not Git worktrees"))
    .stdout(predicate::str::contains("Anvics command prefix"))
    .stdout(predicate::str::contains("write-enabled sandbox mode"))
    .stdout(predicate::str::contains("agent context-pack"))
    .stdout(predicate::str::contains("agent checkpoint"))
    .stdout(predicate::str::contains("agent recover"))
    .stdout(predicate::str::contains("workspace diff"));

    anvics(
        dir.path(),
        &["agent", "launch-prompt", "--workspace", &workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("## Prompt"))
    .stdout(predicate::str::contains("--skip-git-repo-check").not());

    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "launch-prompt",
            "--workspace",
            &workspace,
            "--tool",
            "codex",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("--skip-git-repo-check"))
    .stdout(predicate::str::contains(&workspace_path))
    .stdout(predicate::str::contains(&packet))
    .stdout(predicate::str::contains("agent context-pack"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn agent_checkpoint_and_recover_report_salvage_state() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare_output = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Crash Recovery",
            "--task",
            "Edit app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let workspace = value_after_prefix(&prepare_output, "workspace: ");
    let workspace_path = value_after_prefix(&prepare_output, "workspace_path: ");
    fs::write(format!("{workspace_path}/app.txt"), "salvaged\n").unwrap();
    anvics(
        dir.path(),
        &[
            "agent",
            "enter",
            "--workspace",
            &workspace,
            "--name",
            "codex",
        ],
    )
    .assert()
    .success();

    anvics(dir.path(), &["agent", "recover", "--workspace", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains("Agent recovery"))
        .stdout(predicate::str::contains("current_changed_paths:"))
        .stdout(predicate::str::contains("Modified: app.txt"))
        .stdout(predicate::str::contains("latest_checkpoint: none"))
        .stdout(predicate::str::contains("active_sessions:"))
        .stdout(predicate::str::contains("agent checkpoint"));

    let checkpoint_output = anvics(
        dir.path(),
        &[
            "agent",
            "checkpoint",
            "--workspace",
            &workspace,
            "--summary",
            "salvaged app edit",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Created agent checkpoint"))
    .stdout(predicate::str::contains("snapshot:"))
    .stdout(predicate::str::contains("summary: salvaged app edit"))
    .stdout(predicate::str::contains("Modified: app.txt"))
    .get_output()
    .stdout
    .clone();
    let checkpoint = created_id(&checkpoint_output, "Created agent checkpoint ");

    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    daemon_anvics(
        dir.path(),
        &socket,
        &["agent", "recover", "--workspace", &workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains(format!(
        "latest_checkpoint: {checkpoint}"
    )))
    .stdout(predicate::str::contains("checkpoint_changed_paths:"))
    .stdout(predicate::str::contains("next_commands:"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn agent_instructions_render_install_and_daemon_matches() {
    let dir = tempdir().unwrap();
    anvics(dir.path(), &["repo", "init"]).assert().success();

    anvics(dir.path(), &["agent", "instructions", "--target", "agents"])
        .assert()
        .success()
        .stdout(predicate::str::contains("path:"))
        .stdout(predicate::str::contains("AGENTS.md"))
        .stdout(predicate::str::contains("Anvics Agent Instructions"))
        .stdout(predicate::str::contains("workspace diff"))
        .stdout(predicate::str::contains("Do not create Git branches"));
    assert!(!dir.path().join("AGENTS.md").exists());

    anvics(dir.path(), &["agent", "instructions", "--install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote:"))
        .stdout(predicate::str::contains("AGENTS.md"))
        .stdout(predicate::str::contains("CLAUDE.md"));
    assert!(dir.path().join("AGENTS.md").exists());
    assert!(dir.path().join("CLAUDE.md").exists());

    anvics(dir.path(), &["agent", "instructions", "--install"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));

    anvics(
        dir.path(),
        &[
            "agent",
            "instructions",
            "--target",
            "agents",
            "--install",
            "--force",
        ],
    )
    .assert()
    .success();

    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    daemon_anvics(
        dir.path(),
        &socket,
        &["agent", "instructions", "--target", "claude"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("CLAUDE.md"))
    .stdout(predicate::str::contains("Anvics Agent Instructions"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn agent_context_pack_renders_write_and_daemon_matches() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    fs::create_dir_all(dir.path().join("skills/anvics-skill")).unwrap();
    fs::write(
        dir.path().join("skills/anvics-skill/SKILL.md"),
        "# Anvics\n",
    )
    .unwrap();
    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Context Pack",
            "--task",
            "Edit app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    fs::write(format!("{workspace_path}/app.txt"), "changed\n").unwrap();

    anvics(
        dir.path(),
        &["agent", "context-pack", "--workspace", &workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("# Anvics Context Pack"))
    .stdout(predicate::str::contains("Edit app.txt"))
    .stdout(predicate::str::contains("workspace diff"))
    .stdout(predicate::str::contains("Modified: `app.txt` (source)"))
    .stdout(predicate::str::contains("Potential clashes: none"));

    let write_output = anvics(
        dir.path(),
        &[
            "agent",
            "context-pack",
            "--workspace",
            &workspace,
            "--write",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("context_pack:"))
    .get_output()
    .stdout
    .clone();
    let pack_path = value_after_prefix(&write_output, "context_pack: ");
    assert!(std::path::Path::new(&pack_path).exists());

    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    daemon_anvics(
        dir.path(),
        &socket,
        &["agent", "context-pack", "--workspace", &workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("# Anvics Context Pack"))
    .stdout(predicate::str::contains("Modified: `app.txt` (source)"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn agent_accept_publishes_and_exports_patch() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();

    let prepare_output = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Accept Agent",
            "--task",
            "Edit app.txt",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("agent accept"))
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare_output, "thread: ");
    let workspace = value_after_prefix(&prepare_output, "workspace: ");
    let workspace_path = value_after_prefix(&prepare_output, "workspace_path: ");
    anvics(
        dir.path(),
        &[
            "agent",
            "enter",
            "--workspace",
            &workspace,
            "--name",
            "codex-cli",
        ],
    )
    .assert()
    .success();
    fs::write(format!("{workspace_path}/app.txt"), "accepted\n").unwrap();
    let artifact = dir.path().join("accept-artifact.txt");
    fs::write(&artifact, "compact accept artifact\n").unwrap();
    let command_file = dir.path().join("verify.sh");
    fs::write(&command_file, "cat app.txt\n").unwrap();
    let patch_path = dir.path().join("custom.patch");

    let accept_output = anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--command-file",
            command_file.to_str().unwrap(),
            "--label",
            "verify accepted app",
            "--cwd",
            &workspace_path,
            "--exit-code",
            "0",
            "--summary",
            "Accepted app.txt change",
            "--artifact",
            artifact.to_str().unwrap(),
            "--output",
            patch_path.to_str().unwrap(),
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Accepted agent workspace"))
    .stdout(predicate::str::contains("snapshot: "))
    .stdout(predicate::str::contains("evidence: "))
    .stdout(predicate::str::contains("review: "))
    .stdout(predicate::str::contains("review_markdown: "))
    .stdout(predicate::str::contains("publication: "))
    .stdout(predicate::str::contains("patch: "))
    .stdout(predicate::str::contains("git_apply: git apply"))
    .get_output()
    .stdout
    .clone();
    let publication = value_after_prefix(&accept_output, "publication: ");

    assert!(patch_path.exists());
    let review = value_after_prefix(&accept_output, "review: ");
    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("verify accepted app"))
    .stdout(predicate::str::contains("command file:"))
    .stdout(predicate::str::contains("artifact:"));
    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("publication_status: published"))
        .stdout(predicate::str::contains(&publication));
    anvics(
        dir.path(),
        &["agent", "sessions", "--workspace", &workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Finished"))
    .stdout(predicate::str::contains("codex-cli"));

    let clean = tempdir().unwrap();
    fs::write(clean.path().join("app.txt"), "base\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(clean.path())
        .output()
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
        fs::read_to_string(clean.path().join("app.txt")).unwrap(),
        "accepted\n"
    );
}

#[test]
fn workspace_show_and_agent_status_by_workspace_report_overlay_state() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare_output = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Workspace UX",
            "--task",
            "Edit app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare_output, "thread: ");
    let workspace = value_after_prefix(&prepare_output, "workspace: ");
    let workspace_path = value_after_prefix(&prepare_output, "workspace_path: ");

    anvics(dir.path(), &["workspace", "show", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains(&thread))
        .stdout(predicate::str::contains(&workspace_path))
        .stdout(predicate::str::contains("latest_snapshot: none"))
        .stdout(predicate::str::contains("overlay_changed_paths: unknown"));
    anvics(dir.path(), &["agent", "status", "--workspace", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains(&thread))
        .stdout(predicate::str::contains(&workspace))
        .stdout(predicate::str::contains(&workspace_path))
        .stdout(predicate::str::contains("latest_snapshot: none"))
        .stdout(predicate::str::contains("publication_status: unpublished"));
    anvics(
        dir.path(),
        &[
            "agent",
            "status",
            "--thread",
            &thread,
            "--workspace",
            &workspace,
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("--thread or --workspace"));

    fs::write(format!("{workspace_path}/app.txt"), "changed\n").unwrap();
    anvics(dir.path(), &["workspace", "diff", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified: app.txt"));
    anvics(
        dir.path(),
        &["workspace", "diff", &workspace, "--format", "patch"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("diff --git a/app.txt b/app.txt"))
    .stdout(predicate::str::contains("-base"))
    .stdout(predicate::str::contains("+changed"));
    let snapshot_output = anvics(
        dir.path(),
        &["workspace", "snapshot", &workspace, "--message", "changed"],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let latest_snapshot = value_after_prefix(&snapshot_output, "snapshot: ");
    anvics(dir.path(), &["workspace", "show", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "latest_snapshot: {latest_snapshot}"
        )))
        .stdout(predicate::str::contains("overlay_changed_paths:"))
        .stdout(predicate::str::contains("Modified: app.txt"));
    anvics(dir.path(), &["agent", "status", "--workspace", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "latest_snapshot: {latest_snapshot}"
        )));
}

#[test]
fn evidence_command_attaches_file_backed_evidence() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    let command_file = dir.path().join("verify.sh");
    fs::write(&command_file, "cat app.txt\ncargo test\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let thread_output = anvics(
        dir.path(),
        &[
            "thread",
            "create",
            "--title",
            "Evidence",
            "--task",
            "Attach command evidence",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&thread_output, "Created thread ");

    anvics(
        dir.path(),
        &[
            "evidence",
            "command",
            "--thread",
            &thread,
            "--command-file",
            command_file.to_str().unwrap(),
            "--label",
            "verify script",
            "--cwd",
            ".",
            "--exit-code",
            "0",
            "--summary",
            "Verification script passed",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Attached command evidence"));
}

#[test]
fn evidence_list_show_and_supersede_work_direct_and_daemon() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let thread_output = anvics(
        dir.path(),
        &[
            "thread",
            "create",
            "--title",
            "Evidence",
            "--task",
            "Attach command evidence",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&thread_output, "Created thread ");
    let evidence_output = anvics(
        dir.path(),
        &[
            "evidence",
            "attach",
            "--thread",
            &thread,
            "--command",
            "true",
            "--exit-code",
            "0",
            "--summary",
            "Verification passed",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let evidence = value_after_prefix(&evidence_output, "Attached evidence ");

    anvics(dir.path(), &["evidence", "list", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains(&evidence))
        .stdout(predicate::str::contains("active"));
    anvics(dir.path(), &["evidence", "show", &evidence])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: active"))
        .stdout(predicate::str::contains("summary: Verification passed"));
    anvics(
        dir.path(),
        &[
            "evidence",
            "supersede",
            &evidence,
            "--reason",
            "Obsolete broad verification",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("status: superseded"))
    .stdout(predicate::str::contains(
        "superseded_reason: Obsolete broad verification",
    ));
    anvics(dir.path(), &["evidence", "list", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("No evidence"));
    anvics(
        dir.path(),
        &[
            "evidence",
            "list",
            "--thread",
            &thread,
            "--include-superseded",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains(&evidence))
    .stdout(predicate::str::contains("superseded"));

    let socket_dir = tempdir().unwrap();
    let socket = socket_dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    daemon_anvics(dir.path(), &socket, &["evidence", "show", &evidence])
        .assert()
        .success()
        .stdout(predicate::str::contains("status: superseded"));
    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn command_run_records_artifacts_and_review_evidence() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Command Run",
            "--task",
            "Change and verify app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");

    let command = anvics(
        dir.path(),
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "verify app",
            "--summary",
            "Verified app.txt contents",
            "--",
            "sh",
            "-c",
            "printf 'verified\\n' > app.txt && cat app.txt",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Ran command"))
    .stdout(predicate::str::contains("exit_code: 0"))
    .stdout(predicate::str::contains("projection: materialized_dir"))
    .stdout(predicate::str::contains("command_executor: in_process"))
    .stdout(predicate::str::contains(
        "projection_capabilities: readable=true writable=true file_effects=true",
    ))
    .stdout(predicate::str::contains("policy: mutating"))
    .stdout(predicate::str::contains("projection_setup_ms: "))
    .stdout(predicate::str::contains("command_ms: "))
    .stdout(predicate::str::contains("reconcile_ms: "))
    .stdout(predicate::str::contains("cleanup_ms: "))
    .stdout(predicate::str::contains("projection_files: 1"))
    .stdout(predicate::str::contains("projection_bytes: 5"))
    .stdout(predicate::str::contains("file_effects:"))
    .stdout(predicate::str::contains("- Modified: app.txt"))
    .stdout(predicate::str::contains("stdout: "))
    .get_output()
    .stdout
    .clone();
    let command_event = value_after_prefix(&command, "Ran command ");
    let stdout_path = value_after_prefix(&command, "stdout: ");
    assert_eq!(fs::read_to_string(stdout_path).unwrap(), "verified\n");

    #[cfg(not(feature = "vfs-fuse"))]
    {
        let auto_command = anvics(
            dir.path(),
            &[
                "command",
                "run",
                "--workspace",
                &workspace,
                "--label",
                "auto read",
                "--summary",
                "Read through auto projection",
                "--projection",
                "auto",
                "--",
                "cat",
                "app.txt",
            ],
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("projection: materialized_dir"))
        .stdout(predicate::str::contains("projection_fallback_reason:"))
        .get_output()
        .stdout
        .clone();
        let auto_command_event = value_after_prefix(&auto_command, "Ran command ");
        anvics(dir.path(), &["command", "show", &auto_command_event])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"projection_fallback_reason\""));
    }

    anvics(dir.path(), &["command", "show", &command_event])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"command_label\": \"verify app\"",
        ))
        .stdout(predicate::str::contains(
            "\"projection_kind\": \"materialized_dir\"",
        ))
        .stdout(predicate::str::contains("\"projection_capabilities\""))
        .stdout(predicate::str::contains("\"runtime_metrics\""))
        .stdout(predicate::str::contains(
            "\"command_executor\": \"in_process\"",
        ))
        .stdout(predicate::str::contains(
            "\"command_policy_class\": \"mutating\"",
        ))
        .stdout(predicate::str::contains("\"path\": \"app.txt\""));
    anvics(
        dir.path(),
        &["workspace", "snapshot", &workspace, "--message", "verified"],
    )
    .assert()
    .success();
    let review_output = anvics(dir.path(), &["review", "create", "--thread", &thread])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let review = value_after_prefix(&review_output, "Created review ");
    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("anvics-run:"))
    .stdout(predicate::str::contains("stdout:"))
    .stdout(predicate::str::contains("policy: mutating"))
    .stdout(predicate::str::contains("executor: in_process"))
    .stdout(predicate::str::contains("runtime: setup="))
    .stdout(predicate::str::contains("file effects: modified `app.txt`"))
    .stdout(predicate::str::contains("## File Effects"))
    .stdout(predicate::str::contains(
        "`app.txt` (source) - proposed ChangeUnit",
    ))
    .stdout(predicate::str::contains("## Change Units"))
    .stdout(predicate::str::contains("`app.txt` (source)"))
    .stdout(predicate::str::contains("Verified app.txt contents"));
}

#[test]
fn command_run_can_execute_through_worker_process() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Worker",
            "--task",
            "Run verification through anvics-worker",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let worker = assert_cmd::cargo::cargo_bin("anvics-worker");

    let output = Command::cargo_bin("anvics")
        .unwrap()
        .env("ANVICS_COMMAND_EXECUTOR", "worker")
        .env("ANVICS_WORKER_BIN", worker)
        .args([
            "--repo",
            dir.path().to_str().unwrap(),
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "worker verify",
            "--summary",
            "Worker verified and changed app.txt",
            "--",
            "sh",
            "-c",
            "printf 'worker\\n' > app.txt && cat app.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("command_executor: worker"))
        .stdout(predicate::str::contains("- Modified: app.txt"))
        .get_output()
        .stdout
        .clone();
    let command_event = value_after_prefix(&output, "Ran command ");
    let stdout_path = value_after_prefix(&output, "stdout: ");
    assert_eq!(fs::read_to_string(stdout_path).unwrap(), "worker\n");

    anvics(dir.path(), &["command", "show", &command_event])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command_executor\": \"worker\""));
    anvics(
        dir.path(),
        &["workspace", "snapshot", &workspace, "--message", "worker"],
    )
    .assert()
    .success();
    let review_output = anvics(dir.path(), &["review", "create", "--thread", &thread])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let review = value_after_prefix(&review_output, "Created review ");
    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("executor: worker"));
}

#[test]
fn command_worker_check_reports_worker_health() {
    let dir = tempdir().unwrap();
    let worker = assert_cmd::cargo::cargo_bin("anvics-worker");

    Command::cargo_bin("anvics")
        .unwrap()
        .args([
            "--repo",
            dir.path().to_str().unwrap(),
            "command",
            "worker-check",
            "--worker-bin",
            worker.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("worker: "))
        .stdout(predicate::str::contains("exit_code: 0"))
        .stdout(predicate::str::contains("timed_out: false"))
        .stdout(predicate::str::contains("duration_ms: "))
        .stdout(predicate::str::contains("stdout: worker-ok"));
}

#[test]
fn command_policy_blocks_risky_commands_and_records_override() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Policy Gate",
            "--task",
            "Verify command policy behavior",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");

    for (program, expected) in [
        ("curl", "Networked"),
        ("docker", "HostEscapeRisk"),
        ("vim", "Interactive"),
    ] {
        anvics(
            dir.path(),
            &[
                "command",
                "run",
                "--workspace",
                &workspace,
                "--label",
                "blocked",
                "--summary",
                "Blocked command should not run",
                "--",
                program,
                "--version",
            ],
        )
        .assert()
        .failure()
        .stderr(predicate::str::contains("command policy blocked"))
        .stderr(predicate::str::contains(expected));
    }
    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("evidence_count: 0"));
    anvics(dir.path(), &["events", "list", "--since", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("CommandStarted").not())
        .stdout(predicate::str::contains("EvidenceAttached").not());

    anvics(
        dir.path(),
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "git version",
            "--summary",
            "Operator approved checking the Git binary version",
            "--allow-command-risk",
            "--command-risk-reason",
            "Operator approved local binary version check",
            "--",
            "git",
            "--version",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: networked"))
    .stdout(predicate::str::contains(
        "command_policy_override: Operator approved local binary version check",
    ));

    anvics(
        dir.path(),
        &["command", "classify", "--", "curl", "https://example.com"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: networked"))
    .stdout(predicate::str::contains("blocked: true"))
    .stdout(predicate::str::contains(
        "override: --allow-command-risk --command-risk-reason <reason>",
    ));

    let risky_command_file = dir.path().join("risky-command.sh");
    fs::write(&risky_command_file, "curl https://example.com\n").unwrap();
    anvics(
        dir.path(),
        &[
            "command",
            "classify",
            "--command-file",
            risky_command_file.to_str().unwrap(),
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: networked"))
    .stdout(predicate::str::contains("blocked: true"));
    anvics(
        dir.path(),
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "risky file",
            "--summary",
            "Risky command file should be blocked",
            "--command-file",
            risky_command_file.to_str().unwrap(),
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("command policy blocked"))
    .stderr(predicate::str::contains("Networked"));

    let command = anvics(
        dir.path(),
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "git version second",
            "--summary",
            "Operator approved checking the Git binary version again",
            "--allow-command-risk",
            "--command-risk-reason",
            "Operator approved local binary version check",
            "--",
            "git",
            "--version",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let command_event = value_after_prefix(&command, "Ran command ");
    anvics(dir.path(), &["command", "show", &command_event])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"command_policy_class\": \"networked\"",
        ))
        .stdout(predicate::str::contains(
            "\"command_policy_override_reason\": \"Operator approved local binary version check\"",
        ));
    let allowed_command_file = dir.path().join("git-version-command.sh");
    fs::write(&allowed_command_file, "git --version\n").unwrap();
    anvics(
        dir.path(),
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "git version file",
            "--summary",
            "Operator approved checking Git version from a command file",
            "--command-file",
            allowed_command_file.to_str().unwrap(),
            "--allow-command-risk",
            "--command-risk-reason",
            "Operator approved command-file version check",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: networked"))
    .stdout(predicate::str::contains(
        "command_policy_override: Operator approved command-file version check",
    ));

    anvics(
        dir.path(),
        &["workspace", "snapshot", &workspace, "--message", "policy"],
    )
    .assert()
    .success();
    let review_output = anvics(dir.path(), &["review", "create", "--thread", &thread])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let review = value_after_prefix(&review_output, "Created review ");
    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: networked"))
    .stdout(predicate::str::contains(
        "command policy override: Operator approved local binary version check",
    ))
    .stdout(predicate::str::contains(
        "command policy override: Operator approved command-file version check",
    ));
}

#[test]
fn agent_accept_with_command_run_exports_patch_and_failed_run_does_not_publish() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Command Accept",
            "--task",
            "Change app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    fs::write(format!("{workspace_path}/app.txt"), "accepted\n").unwrap();
    let patch_path = dir.path().join("accepted-command.patch");

    let accept = anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "verify accepted",
            "--run-summary",
            "Anvics verified app.txt before accepting",
            "--output",
            patch_path.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "grep accepted app.txt",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Accepted agent workspace"))
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&accept, "review: ");
    assert!(patch_path.exists());

    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("anvics-run:"))
    .stdout(predicate::str::contains("verify accepted"));

    let clean = tempdir().unwrap();
    fs::write(clean.path().join("app.txt"), "base\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(clean.path())
        .output()
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

    let failed_prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Failed Command Accept",
            "--task",
            "Try a failing verification",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let failed_thread = value_after_prefix(&failed_prepare, "thread: ");
    let failed_workspace = value_after_prefix(&failed_prepare, "workspace: ");
    anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &failed_workspace,
            "--run-label",
            "verify failure",
            "--run-summary",
            "Verification failed as expected",
            "--",
            "false",
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("failed with exit code"));
    anvics(dir.path(), &["agent", "status", "--thread", &failed_thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("evidence_count: 1"))
        .stdout(predicate::str::contains("publication_status: unpublished"));

    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("publication_status: published"));
}

#[test]
fn agent_accept_blocks_risky_command_before_publication() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Risky Accept",
            "--task",
            "Try a risky verification command",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    fs::write(format!("{workspace_path}/app.txt"), "changed\n").unwrap();

    anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "network verify",
            "--run-summary",
            "Network verification should be blocked",
            "--",
            "curl",
            "https://example.com",
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("command policy blocked"))
    .stderr(predicate::str::contains("Networked"));
    let command_file = dir.path().join("network-verify.sh");
    fs::write(&command_file, "curl https://example.com\n").unwrap();
    anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "network file verify",
            "--run-summary",
            "Network command file should be blocked",
            "--command-file",
            command_file.to_str().unwrap(),
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("command policy blocked"))
    .stderr(predicate::str::contains("Networked"));
    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("evidence_count: 0"))
        .stdout(predicate::str::contains("reviews: none"))
        .stdout(predicate::str::contains("publication_status: unpublished"));
}

#[cfg(feature = "vfs-fuse")]
#[test]
fn agent_accept_with_fuse_projection_exports_patch() {
    if !run_fuse_tests() {
        eprintln!("skipping FUSE projection test; set ANVICS_RUN_FUSE_TESTS=1");
        return;
    }

    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    fs::write(dir.path().join("rename_me.txt"), "rename me\n").unwrap();
    fs::create_dir_all(dir.path().join("old_dir")).unwrap();
    fs::write(dir.path().join("old_dir/child.txt"), "child\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "FUSE Accept",
            "--task",
            "Change app.txt through the runtime projection",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let patch_path = dir.path().join("fuse-accept.patch");

    let accept = anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "fuse verify",
            "--run-summary",
            "Verified through FUSE projection",
            "--projection",
            "fuse-mount",
            "--output",
            patch_path.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            ": > app.txt && printf 'accepted through fuse\\n' >> app.txt && printf 'new\\n' > added.tmp && mv added.tmp added.txt && mv rename_me.txt renamed.txt && mv old_dir new_dir && grep 'accepted through fuse' app.txt",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Accepted agent workspace"))
    .stdout(predicate::str::contains("patch: "))
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&accept, "review: ");
    assert!(patch_path.exists());

    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("fuse verify"))
    .stdout(predicate::str::contains("anvics-run:"))
    .stdout(predicate::str::contains("projection: fuse_mount"))
    .stdout(predicate::str::contains("added `added.txt`"))
    .stdout(predicate::str::contains("added `new_dir/child.txt`"))
    .stdout(predicate::str::contains("added `renamed.txt`"))
    .stdout(predicate::str::contains("deleted `old_dir/child.txt`"))
    .stdout(predicate::str::contains("deleted `rename_me.txt`"))
    .stdout(predicate::str::contains("modified `app.txt`"));

    let clean = tempdir().unwrap();
    fs::write(clean.path().join("app.txt"), "base\n").unwrap();
    fs::write(clean.path().join("rename_me.txt"), "rename me\n").unwrap();
    fs::create_dir_all(clean.path().join("old_dir")).unwrap();
    fs::write(clean.path().join("old_dir/child.txt"), "child\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(clean.path())
        .output()
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
        fs::read_to_string(clean.path().join("app.txt")).unwrap(),
        "accepted through fuse\n"
    );
    assert_eq!(
        fs::read_to_string(clean.path().join("added.txt")).unwrap(),
        "new\n"
    );
    assert_eq!(
        fs::read_to_string(clean.path().join("renamed.txt")).unwrap(),
        "rename me\n"
    );
    assert_eq!(
        fs::read_to_string(clean.path().join("new_dir/child.txt")).unwrap(),
        "child\n"
    );
    assert!(!clean.path().join("rename_me.txt").exists());
    assert!(!clean.path().join("old_dir").exists());
}

#[test]
fn agent_enter_and_coordination_status_report_related_work() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let first = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Agent A",
            "--task",
            "Change app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let second = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Agent B",
            "--task",
            "Also change app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let first_workspace = value_after_prefix(&first, "workspace: ");
    let first_path = value_after_prefix(&first, "workspace_path: ");
    let second_workspace = value_after_prefix(&second, "workspace: ");

    anvics(
        dir.path(),
        &[
            "agent",
            "enter",
            "--workspace",
            &first_workspace,
            "--name",
            "codex-a",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Entered agent session"))
    .stdout(predicate::str::contains("unknown changes possible"));
    anvics(
        dir.path(),
        &[
            "agent",
            "enter",
            "--workspace",
            &second_workspace,
            "--name",
            "codex-b",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("unknown changes possible"));

    fs::write(format!("{first_path}/app.txt"), "agent a\n").unwrap();
    anvics(
        dir.path(),
        &[
            "workspace",
            "snapshot",
            &first_workspace,
            "--message",
            "agent a",
        ],
    )
    .assert()
    .success();
    anvics(
        dir.path(),
        &["coordination", "status", "--workspace", &second_workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("known_changed_paths: app.txt"))
    .stdout(predicate::str::contains("codex-a"));

    let sessions = anvics(
        dir.path(),
        &["agent", "sessions", "--workspace", &first_workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("codex-a"))
    .get_output()
    .stdout
    .clone();
    let session = String::from_utf8(sessions)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_owned();
    anvics(dir.path(), &["agent", "leave", "--session", &session])
        .assert()
        .success()
        .stdout(predicate::str::contains("Left agent session"));
}

#[test]
fn secret_risk_scan_blocks_publish_until_override() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Secret Risk",
            "--task",
            "Add a config fixture",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    fs::write(
        format!("{workspace_path}/config.env"),
        format!("OPENAI_API_KEY={secret}\n"),
    )
    .unwrap();
    let finish = anvics(
        dir.path(),
        &[
            "agent",
            "finish",
            "--workspace",
            &workspace,
            "--command",
            "manual verification",
            "--exit-code",
            "0",
            "--summary",
            "Config fixture was added",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&finish, "review: ");

    let scan = anvics(dir.path(), &["risk", "scan", "--review", &review])
        .assert()
        .success()
        .stdout(predicate::str::contains("Risk scan"))
        .stdout(predicate::str::contains("openai_token"))
        .stdout(predicate::str::contains("config.env"))
        .stdout(predicate::str::contains(secret).not())
        .get_output()
        .stdout
        .clone();
    let finding = value_after_prefix(&scan, "finding: ");
    anvics(dir.path(), &["risk", "list", "--review", &review])
        .assert()
        .success()
        .stdout(predicate::str::contains(&finding))
        .stdout(predicate::str::contains(secret).not());
    anvics(dir.path(), &["risk", "show", &finding])
        .assert()
        .success()
        .stdout(predicate::str::contains("detector: "))
        .stdout(predicate::str::contains("config.env"))
        .stdout(predicate::str::contains(secret).not());

    anvics(
        dir.path(),
        &[
            "publish", "create", "--thread", &thread, "--review", &review,
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("publication blocked"));
    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("publication_status: unpublished"));

    anvics(
        dir.path(),
        &[
            "publish",
            "create",
            "--thread",
            &thread,
            "--review",
            &review,
            "--allow-secret-risk",
            "--override-reason",
            "fixture secret is intentional",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Created publication"));
    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Risk Notes"))
    .stdout(predicate::str::contains("openai_token"))
    .stdout(predicate::str::contains("fixture secret is intentional"))
    .stdout(predicate::str::contains(secret).not());
}

#[test]
fn agent_accept_run_blocks_when_command_stdout_contains_secret() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();
    let prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Leaky Command",
            "--task",
            "Edit app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    fs::write(format!("{workspace_path}/app.txt"), "accepted\n").unwrap();

    anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "leaky verify",
            "--run-summary",
            "Command emitted fixture secret",
            "--",
            "sh",
            "-c",
            &format!("printf 'OPENAI_API_KEY={secret}\\n'"),
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("publication blocked"))
    .stderr(predicate::str::contains("Recovery hint"))
    .stderr(predicate::str::contains("agent status --workspace"))
    .stderr(predicate::str::contains("risk list --review"))
    .stderr(predicate::str::contains("evidence supersede "))
    .stderr(predicate::str::contains("--reason \"<audited reason>\""))
    .stderr(predicate::str::contains("rerun agent accept"))
    .stderr(predicate::str::contains("--allow-secret-risk"));
    let status = anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("evidence_count: 1"))
        .stdout(predicate::str::contains("publication_status: unpublished"))
        .get_output()
        .stdout
        .clone();
    let review = value_after_prefix(&status, "review: ");
    let risk_list = anvics(dir.path(), &["risk", "list", "--review", &review])
        .assert()
        .success()
        .stdout(predicate::str::contains("evidence: "))
        .get_output()
        .stdout
        .clone();
    let evidence = value_after_prefix(&risk_list, "evidence: ");
    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("CommandStdout"))
    .stdout(predicate::str::contains("openai_token"))
    .stdout(predicate::str::contains(secret).not());
    anvics(
        dir.path(),
        &[
            "evidence",
            "supersede",
            &evidence,
            "--reason",
            "Obsolete leaky verification",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("status: superseded"));
}

#[test]
fn daemon_backed_cli_flow_uses_socket_api() {
    let dir = tempdir().unwrap();
    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);

    daemon_anvics(dir.path(), &socket, &["repo", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("not initialized"));

    daemon_anvics(dir.path(), &socket, &["repo", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Anvics repository"));
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    daemon_anvics(
        dir.path(),
        &socket,
        &["snapshot", "create", "--message", "base"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Created snapshot"));
    daemon_anvics(dir.path(), &socket, &["snapshot", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("base"));
    daemon_anvics(dir.path(), &socket, &["repo", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized"));
    anvics(
        dir.path(),
        &["daemon", "ping", "--socket", socket.to_str().unwrap()],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("daemon: ok"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn daemon_backed_secret_risk_blocks_and_override_publishes() {
    let dir = tempdir().unwrap();
    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";

    daemon_anvics(dir.path(), &socket, &["repo", "init"])
        .assert()
        .success();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &["snapshot", "create", "--message", "base"],
    )
    .assert()
    .success();
    let prepare = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "prepare",
            "--title",
            "Daemon Secret Risk",
            "--task",
            "Add a fixture config",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    fs::write(
        format!("{workspace_path}/config.env"),
        format!("OPENAI_API_KEY={secret}\n"),
    )
    .unwrap();
    let finish = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "finish",
            "--workspace",
            &workspace,
            "--command",
            "manual verification",
            "--exit-code",
            "0",
            "--summary",
            "Config fixture was added",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&finish, "review: ");

    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "publish", "create", "--thread", &thread, "--review", &review,
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("daemon error"))
    .stderr(predicate::str::contains("publication blocked"));
    daemon_anvics(dir.path(), &socket, &["risk", "list", "--review", &review])
        .assert()
        .success()
        .stdout(predicate::str::contains("openai_token"))
        .stdout(predicate::str::contains(secret).not());
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "publish",
            "create",
            "--thread",
            &thread,
            "--review",
            &review,
            "--allow-secret-risk",
            "--override-reason",
            "fixture secret is intentional",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Created publication"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn daemon_backed_command_policy_matches_direct_cli() {
    let dir = tempdir().unwrap();
    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);

    daemon_anvics(dir.path(), &socket, &["repo", "init"])
        .assert()
        .success();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &["snapshot", "create", "--message", "base"],
    )
    .assert()
    .success();
    let prepare = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "prepare",
            "--title",
            "Daemon Policy",
            "--task",
            "Verify daemon command policy behavior",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");

    daemon_anvics(
        dir.path(),
        &socket,
        &["command", "classify", "--", "docker", "ps"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: host_escape_risk"))
    .stdout(predicate::str::contains("blocked: true"));

    let daemon_command_file = dir.path().join("daemon-risky-command.sh");
    fs::write(&daemon_command_file, "docker ps\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "command",
            "classify",
            "--command-file",
            daemon_command_file.to_str().unwrap(),
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: host_escape_risk"))
    .stdout(predicate::str::contains("blocked: true"));

    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "blocked command file",
            "--summary",
            "Host escape command file should be blocked",
            "--command-file",
            daemon_command_file.to_str().unwrap(),
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("daemon error"))
    .stderr(predicate::str::contains("command policy blocked"))
    .stderr(predicate::str::contains("HostEscapeRisk"));

    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "blocked docker",
            "--summary",
            "Host escape command should be blocked",
            "--",
            "docker",
            "ps",
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("daemon error"))
    .stderr(predicate::str::contains("command policy blocked"))
    .stderr(predicate::str::contains("HostEscapeRisk"));
    daemon_anvics(
        dir.path(),
        &socket,
        &["agent", "status", "--thread", &thread],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("evidence_count: 0"));

    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "command",
            "run",
            "--workspace",
            &workspace,
            "--label",
            "git version",
            "--summary",
            "Operator approved checking Git version through daemon",
            "--allow-command-risk",
            "--command-risk-reason",
            "Operator approved daemon version check",
            "--",
            "git",
            "--version",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: networked"))
    .stdout(predicate::str::contains(
        "command_policy_override: Operator approved daemon version check",
    ));

    fs::write(format!("{workspace_path}/app.txt"), "daemon accepted\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "network accept",
            "--run-summary",
            "Network verification should be blocked",
            "--",
            "curl",
            "https://example.com",
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("daemon error"))
    .stderr(predicate::str::contains("command policy blocked"))
    .stderr(predicate::str::contains("Networked"));
    let accept_command_file = dir.path().join("daemon-accept-risky-command.sh");
    fs::write(&accept_command_file, "curl https://example.com\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "network accept file",
            "--run-summary",
            "Network command file should be blocked",
            "--command-file",
            accept_command_file.to_str().unwrap(),
        ],
    )
    .assert()
    .failure()
    .stderr(predicate::str::contains("daemon error"))
    .stderr(predicate::str::contains("command policy blocked"))
    .stderr(predicate::str::contains("Networked"));
    daemon_anvics(
        dir.path(),
        &socket,
        &["agent", "status", "--thread", &thread],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("publication_status: unpublished"));

    let patch_path = dir.path().join("daemon-policy.patch");
    let accept = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "git version accept",
            "--run-summary",
            "Operator approved risky verification while accepting",
            "--allow-command-risk",
            "--command-risk-reason",
            "Operator approved daemon accept version check",
            "--output",
            patch_path.to_str().unwrap(),
            "--",
            "git",
            "--version",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Accepted agent workspace"))
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&accept, "review: ");
    daemon_anvics(
        dir.path(),
        &socket,
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("policy: networked"))
    .stdout(predicate::str::contains(
        "command policy override: Operator approved daemon accept version check",
    ));
    assert!(patch_path.exists());

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn daemon_backed_full_agent_flow_exports_patch_and_events() {
    let dir = tempdir().unwrap();
    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);

    daemon_anvics(dir.path(), &socket, &["repo", "init"])
        .assert()
        .success();
    fs::write(dir.path().join("modified.txt"), "before\n").unwrap();
    fs::write(dir.path().join("deleted.txt"), "delete me\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &["snapshot", "create", "--message", "base"],
    )
    .assert()
    .success();
    let prepare = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "prepare",
            "--title",
            "Daemon Agent",
            "--task",
            "Modify, add, and delete files",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Prepared agent task"))
    .get_output()
    .stdout
    .clone();
    let thread = value_after_prefix(&prepare, "thread: ");
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let workspace_path = value_after_prefix(&prepare, "workspace_path: ");
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "enter",
            "--workspace",
            &workspace,
            "--name",
            "codex-daemon",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Entered agent session"));

    fs::write(format!("{workspace_path}/modified.txt"), "after\n").unwrap();
    fs::remove_file(format!("{workspace_path}/deleted.txt")).unwrap();
    fs::write(format!("{workspace_path}/added.txt"), "new\n").unwrap();
    daemon_anvics(dir.path(), &socket, &["workspace", "diff", &workspace])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added: added.txt"))
        .stdout(predicate::str::contains("Deleted: deleted.txt"))
        .stdout(predicate::str::contains("Modified: modified.txt"))
        .stdout(predicate::str::contains("(source)"));
    daemon_anvics(
        dir.path(),
        &socket,
        &["workspace", "diff", &workspace, "--format", "patch"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains(
        "diff --git a/added.txt b/added.txt",
    ))
    .stdout(predicate::str::contains(
        "diff --git a/modified.txt b/modified.txt",
    ))
    .stdout(predicate::str::contains(
        "diff --git a/deleted.txt b/deleted.txt",
    ));
    let patch_path = dir.path().join("daemon.patch");
    let accept = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "daemon verify",
            "--run-summary",
            "Daemon-backed agent result accepted",
            "--output",
            patch_path.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "cat modified.txt && cat added.txt",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Accepted agent workspace"))
    .stdout(predicate::str::contains("patch: "))
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&accept, "review: ");

    daemon_anvics(
        dir.path(),
        &socket,
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("daemon verify"))
    .stdout(predicate::str::contains("anvics-run:"))
    .stdout(predicate::str::contains("runtime: setup="))
    .stdout(predicate::str::contains("Modified: `modified.txt`"));
    daemon_anvics(
        dir.path(),
        &socket,
        &["agent", "status", "--thread", &thread],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("publication_status: published"));
    daemon_anvics(
        dir.path(),
        &socket,
        &["agent", "sessions", "--workspace", &workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Finished"))
    .stdout(predicate::str::contains("codex-daemon"));
    daemon_anvics(dir.path(), &socket, &["events", "list", "--since", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("RepositoryInitialized"))
        .stdout(predicate::str::contains("CommandStarted"))
        .stdout(predicate::str::contains("CommandFinished"))
        .stdout(predicate::str::contains("LegacyPatchExported"));

    let clean = tempdir().unwrap();
    fs::write(clean.path().join("modified.txt"), "before\n").unwrap();
    fs::write(clean.path().join("deleted.txt"), "delete me\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(clean.path())
        .output()
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

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[cfg(feature = "vfs-fuse")]
#[test]
fn daemon_backed_agent_accept_with_fuse_projection_exports_patch() {
    if !run_fuse_tests() {
        eprintln!("skipping daemon FUSE projection test; set ANVICS_RUN_FUSE_TESTS=1");
        return;
    }

    let dir = tempdir().unwrap();
    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);

    daemon_anvics(dir.path(), &socket, &["repo", "init"])
        .assert()
        .success();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &["snapshot", "create", "--message", "base"],
    )
    .assert()
    .success();
    let prepare = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "prepare",
            "--title",
            "Daemon FUSE Accept",
            "--task",
            "Change app.txt through the daemon runtime projection",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let workspace = value_after_prefix(&prepare, "workspace: ");
    let patch_path = dir.path().join("daemon-fuse-accept.patch");

    let accept = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--run-label",
            "daemon fuse verify",
            "--run-summary",
            "Daemon verified through FUSE projection",
            "--projection",
            "fuse-mount",
            "--output",
            patch_path.to_str().unwrap(),
            "--",
            "sh",
            "-c",
            "printf 'daemon accepted through fuse\\n' > app.txt && grep 'daemon accepted through fuse' app.txt",
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Accepted agent workspace"))
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&accept, "review: ");
    assert!(patch_path.exists());

    daemon_anvics(
        dir.path(),
        &socket,
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("daemon fuse verify"))
    .stdout(predicate::str::contains("projection: fuse_mount"))
    .stdout(predicate::str::contains("file effects: modified `app.txt`"));

    let clean = tempdir().unwrap();
    fs::write(clean.path().join("app.txt"), "base\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(clean.path())
        .output()
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

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn daemon_backed_two_agent_overlap_and_error_output() {
    let dir = tempdir().unwrap();
    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);

    daemon_anvics(dir.path(), &socket, &["repo", "init"])
        .assert()
        .success();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &["snapshot", "create", "--message", "base"],
    )
    .assert()
    .success();
    daemon_anvics(dir.path(), &socket, &["thread", "show", "missing-thread"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("daemon error"))
        .stderr(predicate::str::contains("thread does not exist"));

    let first = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "prepare",
            "--title",
            "Agent A",
            "--task",
            "Change app.txt for Agent A",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let second = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "prepare",
            "--title",
            "Agent B",
            "--task",
            "Change app.txt for Agent B",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let first_workspace = value_after_prefix(&first, "workspace: ");
    let first_path = value_after_prefix(&first, "workspace_path: ");
    let second_workspace = value_after_prefix(&second, "workspace: ");
    let second_path = value_after_prefix(&second, "workspace_path: ");

    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "enter",
            "--workspace",
            &first_workspace,
            "--name",
            "codex-a",
        ],
    )
    .assert()
    .success();
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "enter",
            "--workspace",
            &second_workspace,
            "--name",
            "codex-b",
        ],
    )
    .assert()
    .success();
    fs::write(format!("{first_path}/app.txt"), "agent a\n").unwrap();
    fs::write(format!("{second_path}/app.txt"), "agent b\n").unwrap();
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "workspace",
            "snapshot",
            &first_workspace,
            "--message",
            "Agent A checkpoint",
        ],
    )
    .assert()
    .success();
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "workspace",
            "snapshot",
            &second_workspace,
            "--message",
            "Agent B checkpoint",
        ],
    )
    .assert()
    .success();
    daemon_anvics(
        dir.path(),
        &socket,
        &["coordination", "status", "--workspace", &first_workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("overlap_paths: app.txt"))
    .stdout(predicate::str::contains("Potential path overlap"));
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "finish",
            "--workspace",
            &second_workspace,
            "--command",
            "agent-b",
            "--exit-code",
            "0",
            "--summary",
            "Agent B changed app.txt",
        ],
    )
    .assert()
    .success();
    let first_accept = daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "accept",
            "--workspace",
            &first_workspace,
            "--command",
            "agent-a",
            "--exit-code",
            "0",
            "--summary",
            "Agent A changed app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&first_accept, "review: ");

    daemon_anvics(
        dir.path(),
        &socket,
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("also changed: app.txt"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

#[test]
fn two_prepared_agents_report_overlap_notes() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();

    let first_prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Agent A",
            "--task",
            "Change app.txt for Agent A",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let second_prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Agent B",
            "--task",
            "Change app.txt for Agent B",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();

    let first_workspace = value_after_prefix(&first_prepare, "workspace: ");
    let first_path = value_after_prefix(&first_prepare, "workspace_path: ");
    let second_workspace = value_after_prefix(&second_prepare, "workspace: ");
    let second_path = value_after_prefix(&second_prepare, "workspace_path: ");

    anvics(
        dir.path(),
        &[
            "agent",
            "enter",
            "--workspace",
            &first_workspace,
            "--name",
            "codex-a",
        ],
    )
    .assert()
    .success();
    anvics(
        dir.path(),
        &[
            "agent",
            "enter",
            "--workspace",
            &second_workspace,
            "--name",
            "codex-b",
        ],
    )
    .assert()
    .success();
    fs::write(format!("{first_path}/app.txt"), "agent a\n").unwrap();
    fs::write(format!("{second_path}/app.txt"), "agent b\n").unwrap();
    anvics(
        dir.path(),
        &[
            "workspace",
            "snapshot",
            &first_workspace,
            "--message",
            "Agent A checkpoint",
        ],
    )
    .assert()
    .success();
    anvics(
        dir.path(),
        &[
            "workspace",
            "snapshot",
            &second_workspace,
            "--message",
            "Agent B checkpoint",
        ],
    )
    .assert()
    .success();
    anvics(
        dir.path(),
        &["coordination", "status", "--workspace", &first_workspace],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("overlap_paths: app.txt"))
    .stdout(predicate::str::contains("Potential path overlap"));

    anvics(
        dir.path(),
        &[
            "agent",
            "finish",
            "--workspace",
            &second_workspace,
            "--command",
            "agent-b",
            "--exit-code",
            "0",
            "--summary",
            "Agent B changed app.txt",
        ],
    )
    .assert()
    .success();
    let first_accept = anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &first_workspace,
            "--command",
            "agent-a",
            "--exit-code",
            "0",
            "--summary",
            "Agent A changed app.txt",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let review = value_after_prefix(&first_accept, "review: ");

    anvics(
        dir.path(),
        &["review", "show", &review, "--format", "markdown"],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("also changed: app.txt"));
}

#[test]
fn agent_resolve_prepares_resolver_packet_and_daemon_matches() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("app.txt"), "base\n").unwrap();

    anvics(dir.path(), &["repo", "init"]).assert().success();
    anvics(dir.path(), &["snapshot", "create", "--message", "base"])
        .assert()
        .success();

    let first_prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Candidate A",
            "--task",
            "Write candidate A",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let first_workspace = value_after_prefix(&first_prepare, "workspace: ");
    let first_workspace_path = value_after_prefix(&first_prepare, "workspace_path: ");
    fs::write(
        std::path::Path::new(&first_workspace_path).join("app.txt"),
        "candidate a\n",
    )
    .unwrap();
    let first_finish = anvics(
        dir.path(),
        &[
            "agent",
            "finish",
            "--workspace",
            &first_workspace,
            "--command",
            "true",
            "--exit-code",
            "0",
            "--summary",
            "candidate A verified",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let first_review = value_after_prefix(&first_finish, "review: ");

    let second_prepare = anvics(
        dir.path(),
        &[
            "agent",
            "prepare",
            "--title",
            "Candidate B",
            "--task",
            "Write candidate B",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let second_workspace = value_after_prefix(&second_prepare, "workspace: ");
    let second_workspace_path = value_after_prefix(&second_prepare, "workspace_path: ");
    fs::write(
        std::path::Path::new(&second_workspace_path).join("app.txt"),
        "candidate b\n",
    )
    .unwrap();
    let second_finish = anvics(
        dir.path(),
        &[
            "agent",
            "finish",
            "--workspace",
            &second_workspace,
            "--command",
            "true",
            "--exit-code",
            "0",
            "--summary",
            "candidate B verified",
        ],
    )
    .assert()
    .success()
    .get_output()
    .stdout
    .clone();
    let second_review = value_after_prefix(&second_finish, "review: ");

    let command_prefix = "cargo run -q -p anvics-cli --bin anvics --";
    let resolve = anvics(
        dir.path(),
        &[
            "agent",
            "resolve",
            "--review",
            &first_review,
            "--review",
            &second_review,
            "--title",
            "Resolve candidates",
            "--task",
            "Keep useful behavior from both candidates.",
            "--agent-command",
            command_prefix,
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Prepared agent task"))
    .stdout(predicate::str::contains("agent_command: cargo run"))
    .stdout(predicate::str::contains("launch_prompt: cargo run"))
    .get_output()
    .stdout
    .clone();
    let packet = value_after_prefix(&resolve, "packet: ");
    let packet_text = fs::read_to_string(packet).unwrap();
    assert!(packet_text.contains(&first_review));
    assert!(packet_text.contains(&second_review));
    assert!(packet_text.contains("Keep useful behavior from both candidates."));
    assert!(packet_text.contains("candidate A verified"));
    assert!(packet_text.contains(command_prefix));

    let socket = dir.path().join("anvics.sock");
    let mut daemon = start_daemon(&socket);
    daemon_anvics(
        dir.path(),
        &socket,
        &[
            "agent",
            "resolve",
            "--review",
            &first_review,
            "--review",
            &second_review,
            "--agent-command",
            command_prefix,
        ],
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("Prepared agent task"))
    .stdout(predicate::str::contains("agent_command: cargo run"));
    daemon.kill().unwrap();
    daemon.wait().unwrap();
}

fn anvics(repo: &std::path::Path, args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("anvics").unwrap();
    command.args(["--repo", repo.to_str().unwrap()]).args(args);
    command
}

fn daemon_anvics(repo: &std::path::Path, socket: &std::path::Path, args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("anvics").unwrap();
    command
        .env("ANVICS_DAEMON_SOCKET", socket)
        .args(["--repo", repo.to_str().unwrap(), "--use-daemon"])
        .args(args);
    command
}

fn start_daemon(socket: &std::path::Path) -> std::process::Child {
    let daemon = StdCommand::new(assert_cmd::cargo::cargo_bin("anvicsd"))
        .args(["--socket", socket.to_str().unwrap()])
        .spawn()
        .unwrap();
    wait_for_socket(socket);
    daemon
}

fn wait_for_socket(socket: &std::path::Path) {
    let started = Instant::now();
    while !socket.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "timed out waiting for daemon socket"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(feature = "vfs-fuse")]
fn run_fuse_tests() -> bool {
    std::env::var("ANVICS_RUN_FUSE_TESTS").ok().as_deref() == Some("1")
}

fn created_id(output: &[u8], prefix: &str) -> String {
    value_after_prefix(output, prefix)
}

fn value_after_prefix(output: &[u8], prefix: &str) -> String {
    String::from_utf8(output.to_vec())
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .unwrap()
        .to_owned()
}
