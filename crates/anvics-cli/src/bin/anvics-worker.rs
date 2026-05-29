use anvics_core::{CommandWorkerRequest, CommandWorkerResponse};
use anyhow::{Context, Result};
use std::{
    io::{self, Read},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    let mut input = Vec::new();
    io::stdin()
        .read_to_end(&mut input)
        .context("failed to read worker request")?;
    let request: CommandWorkerRequest =
        serde_json::from_slice(&input).context("failed to parse worker request")?;
    let response = run_command(request).context("failed to run worker command")?;
    serde_json::to_writer(io::stdout(), &response).context("failed to write worker response")?;
    Ok(())
}

fn run_command(request: CommandWorkerRequest) -> Result<CommandWorkerResponse> {
    anyhow::ensure!(!request.argv.is_empty(), "worker argv must not be empty");
    let timeout = Duration::from_secs(request.timeout_seconds);
    let cwd = PathBuf::from(request.cwd);
    let mut child = Command::new(&request.argv[0])
        .args(&request.argv[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn worker command")?;
    let started = Instant::now();
    let mut timed_out = false;

    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            child.kill()?;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }

    let output = child.wait_with_output()?;
    let exit_code = if timed_out {
        -1
    } else {
        output.status.code().unwrap_or(-1)
    };

    Ok(CommandWorkerResponse {
        exit_code,
        timed_out,
        duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        stdout: output.stdout,
        stderr: output.stderr,
    })
}
