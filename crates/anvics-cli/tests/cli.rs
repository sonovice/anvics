use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::process::Command as StdCommand;
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
    assert!(packet_text.contains("only editable area"));
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
    fs::write(format!("{workspace_path}/app.txt"), "accepted\n").unwrap();
    let artifact = dir.path().join("accept-artifact.txt");
    fs::write(&artifact, "compact accept artifact\n").unwrap();
    let patch_path = dir.path().join("custom.patch");

    let accept_output = anvics(
        dir.path(),
        &[
            "agent",
            "accept",
            "--workspace",
            &workspace,
            "--command",
            "cat app.txt",
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
    anvics(dir.path(), &["agent", "status", "--thread", &thread])
        .assert()
        .success()
        .stdout(predicate::str::contains("publication_status: published"))
        .stdout(predicate::str::contains(&publication));

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

    fs::write(format!("{first_path}/app.txt"), "agent a\n").unwrap();
    fs::write(format!("{second_path}/app.txt"), "agent b\n").unwrap();

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

fn anvics(repo: &std::path::Path, args: &[&str]) -> Command {
    let mut command = Command::cargo_bin("anvics").unwrap();
    command.args(["--repo", repo.to_str().unwrap()]).args(args);
    command
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
