//! Bounded, argument-vector-only subprocesses. No shell and no implicit retry.
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

const OUTPUT_CAP: u64 = 512 * 1024;

async fn capped(reader: impl AsyncRead + Unpin) -> anyhow::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(OUTPUT_CAP + 1).read_to_end(&mut bytes).await?;
    if bytes.len() as u64 > OUTPUT_CAP {
        bail!("subprocess output limit exceeded");
    }
    Ok(bytes)
}

pub async fn run(
    cwd: &Path,
    program: &str,
    args: &[&str],
    stdin: Option<&str>,
    index: Option<&Path>,
) -> anyhow::Result<String> {
    let mut command = Command::new(program);
    command
        .current_dir(cwd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GH_PROMPT_DISABLED", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let mut child = command.spawn().context("cannot start host subprocess")?;
    let mut input = child.stdin.take().context("missing stdin")?;
    let out = child.stdout.take().context("missing stdout")?;
    let err = child.stderr.take().context("missing stderr")?;
    let operation = async {
        let write = async {
            if let Some(text) = stdin {
                input.write_all(text.as_bytes()).await?;
            }
            drop(input);
            Ok::<_, anyhow::Error>(())
        };
        let (_, out, err, status) = tokio::try_join!(write, capped(out), capped(err), async {
            Ok::<_, anyhow::Error>(child.wait().await?)
        })?;
        if !status.success() {
            // Do not expose credentials embedded in Git transport diagnostics.
            bail!("{program} operation failed (exit {:?})", status.code());
        }
        let _ = err;
        Ok(String::from_utf8(out).context("non-UTF8 subprocess output")?)
    };
    match tokio::time::timeout(Duration::from_secs(60), operation).await {
        Ok(result) => result,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            bail!("{program} operation timed out; mutation result may be uncertain")
        }
    }
}

pub async fn git(cwd: &Path, args: &[&str]) -> anyhow::Result<String> {
    run(cwd, "git", args, None, None).await
}

pub fn required(value: &str, name: &str) -> anyhow::Result<()> {
    if value.trim().is_empty() || value.starts_with('-') || value.contains(['\0', '\n', '\r']) {
        bail!("invalid {name}");
    }
    Ok(())
}
