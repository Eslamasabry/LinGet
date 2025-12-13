use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

pub const SUGGEST_PREFIX: &str = "LINGET_SUGGEST:";

#[derive(Debug, Clone)]
pub struct Suggest {
    pub command: String,
}

pub async fn run_pkexec(
    program: &str,
    args: &[&str],
    context_msg: &str,
    suggest: Suggest,
) -> Result<()> {
    let output = Command::new("pkexec")
        .arg(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await;

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::bail!(
                    "{}. pkexec is not installed.\n\n{} {}\n",
                    context_msg,
                    SUGGEST_PREFIX,
                    suggest.command
                );
            }
            return Err(e).with_context(|| context_msg.to_string());
        }
    };

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let mut msg = context_msg.to_string();
    if !stderr.is_empty() {
        msg.push_str(&format!(": {}", stderr));
    } else if let Some(code) = output.status.code() {
        msg.push_str(&format!(" (exit code {})", code));
    }

    // Most common cases:
    // - user canceled auth dialog
    // - polkit denied
    // pkexec doesn't standardize exit codes across distros, so match on stderr too.
    let lowered = stderr.to_lowercase();
    if lowered.contains("authentication")
        || lowered.contains("authorization")
        || lowered.contains("not authorized")
    {
        msg.push_str("\n\nAuthorization was canceled or denied.");
    }

    anyhow::bail!("{}\n\n{} {}\n", msg, SUGGEST_PREFIX, suggest.command);
}
