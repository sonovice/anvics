use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
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
