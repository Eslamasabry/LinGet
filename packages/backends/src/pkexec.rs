use crate::streaming::{StreamLine, StreamType};
use anyhow::{Context, Result};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct Suggest {
    pub command: String,
}

pub async fn run_pkexec(
    command: &str,
    args: &[&str],
    error_msg: &str,
    suggest: Suggest,
) -> Result<()> {
    let mut all_args: Vec<&str> = vec![command];
    all_args.extend_from_slice(args);

    let output = Command::new("pkexec")
        .args(&all_args)
        .output()
        .await
        .with_context(|| error_msg.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "{}\n{}\n\nTry: {}",
            error_msg,
            stderr.trim(),
            suggest.command
        );
    }

    Ok(())
}

pub async fn run_pkexec_with_logs(
    _command: &str,
    _args: &[&str],
    _error_msg: &str,
    _suggest: Suggest,
    _log_sender: mpsc::Sender<StreamLine>,
) -> Result<()> {
    Ok(())
}
