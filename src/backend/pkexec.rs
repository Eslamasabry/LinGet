use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info, warn};

pub const SUGGEST_PREFIX: &str = "LINGET_SUGGEST:";

#[derive(Debug, Clone)]
pub struct Suggest {
    pub command: String,
}

/// Detects the type of privilege escalation error from stderr
fn detect_auth_error(stderr: &str, exit_code: Option<i32>) -> AuthErrorKind {
    let lowered = stderr.to_lowercase();

    // User explicitly cancelled the dialog
    if lowered.contains("dismissed")
        || lowered.contains("cancelled")
        || lowered.contains("canceled")
        || exit_code == Some(126)
    {
        return AuthErrorKind::Cancelled;
    }

    // Authentication failed (wrong password, timeout, etc.)
    if lowered.contains("authentication")
        || lowered.contains("authorization")
        || lowered.contains("not authorized")
        || lowered.contains("password")
        || exit_code == Some(127)
    {
        return AuthErrorKind::Denied;
    }

    // Polkit agent not available
    if lowered.contains("no agent") || lowered.contains("polkit") {
        return AuthErrorKind::NoAgent;
    }

    AuthErrorKind::Unknown
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthErrorKind {
    Cancelled,
    Denied,
    NoAgent,
    Unknown,
}

/// Run a command with pkexec for privilege escalation
///
/// # Arguments
/// * `program` - The program to run (e.g., "apt")
/// * `args` - Arguments to pass to the program
/// * `context_msg` - Human-readable description of the operation for error messages
/// * `suggest` - Alternative command suggestion if pkexec fails
pub async fn run_pkexec(
    program: &str,
    args: &[&str],
    context_msg: &str,
    suggest: Suggest,
) -> Result<()> {
    let full_command = format!("pkexec {} {}", program, args.join(" "));
    debug!(
        command = %full_command,
        operation = %context_msg,
        "Executing privileged command"
    );

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
                error!(
                    error = %e,
                    "pkexec not found - polkit may not be installed"
                );
                anyhow::bail!(
                    "{}. pkexec is not installed. Install polkit to enable privilege escalation.\n\n{} {}\n",
                    context_msg,
                    SUGGEST_PREFIX,
                    suggest.command
                );
            }
            error!(
                error = %e,
                command = %full_command,
                "Failed to execute pkexec"
            );
            return Err(e).with_context(|| context_msg.to_string());
        }
    };

    if output.status.success() {
        info!(
            command = %program,
            operation = %context_msg,
            "Privileged command completed successfully"
        );
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let exit_code = output.status.code();
    let auth_error = detect_auth_error(&stderr, exit_code);

    // Log the error with appropriate level
    match auth_error {
        AuthErrorKind::Cancelled => {
            info!(
                command = %program,
                operation = %context_msg,
                "User cancelled authorization dialog"
            );
        }
        AuthErrorKind::Denied => {
            warn!(
                command = %program,
                operation = %context_msg,
                exit_code = ?exit_code,
                "Authorization denied"
            );
        }
        AuthErrorKind::NoAgent => {
            error!(
                command = %program,
                operation = %context_msg,
                "No polkit agent available - cannot prompt for authorization"
            );
        }
        AuthErrorKind::Unknown => {
            error!(
                command = %program,
                operation = %context_msg,
                exit_code = ?exit_code,
                stderr = %stderr,
                "Privileged command failed"
            );
        }
    }

    // Build user-friendly error message
    let mut msg = context_msg.to_string();

    match auth_error {
        AuthErrorKind::Cancelled => {
            msg.push_str("\n\nAuthorization was cancelled.");
        }
        AuthErrorKind::Denied => {
            msg.push_str("\n\nAuthorization was denied. Please try again with the correct password.");
        }
        AuthErrorKind::NoAgent => {
            msg.push_str("\n\nNo authentication agent is available. Make sure a polkit agent is running.");
        }
        AuthErrorKind::Unknown => {
            if !stderr.is_empty() {
                // Truncate very long stderr messages
                let stderr_display = if stderr.len() > 500 {
                    format!("{}...", &stderr[..500])
                } else {
                    stderr.clone()
                };
                msg.push_str(&format!(": {}", stderr_display));
            } else if let Some(code) = exit_code {
                msg.push_str(&format!(" (exit code {})", code));
            }
        }
    }

    anyhow::bail!("{}\n\n{} {}\n", msg, SUGGEST_PREFIX, suggest.command);
}

/// Run a command without privilege escalation, with proper error handling
pub async fn run_command(
    program: &str,
    args: &[&str],
    context_msg: &str,
) -> Result<String> {
    let full_command = format!("{} {}", program, args.join(" "));
    debug!(
        command = %full_command,
        operation = %context_msg,
        "Executing command"
    );

    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to execute {}", program))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        debug!(
            command = %program,
            operation = %context_msg,
            stdout_len = stdout.len(),
            "Command completed successfully"
        );
        Ok(stdout)
    } else {
        let exit_code = output.status.code();
        warn!(
            command = %full_command,
            operation = %context_msg,
            exit_code = ?exit_code,
            stderr = %stderr,
            "Command failed"
        );

        let mut msg = context_msg.to_string();
        if !stderr.is_empty() {
            msg.push_str(&format!(": {}", stderr.trim()));
        } else if let Some(code) = exit_code {
            msg.push_str(&format!(" (exit code {})", code));
        }
        anyhow::bail!("{}", msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_auth_error_cancelled() {
        assert_eq!(
            detect_auth_error("user dismissed the dialog", None),
            AuthErrorKind::Cancelled
        );
        assert_eq!(
            detect_auth_error("operation cancelled", None),
            AuthErrorKind::Cancelled
        );
        assert_eq!(
            detect_auth_error("", Some(126)),
            AuthErrorKind::Cancelled
        );
    }

    #[test]
    fn test_detect_auth_error_denied() {
        assert_eq!(
            detect_auth_error("authentication failed", None),
            AuthErrorKind::Denied
        );
        assert_eq!(
            detect_auth_error("Not authorized", None),
            AuthErrorKind::Denied
        );
    }

    #[test]
    fn test_detect_auth_error_no_agent() {
        assert_eq!(
            detect_auth_error("No agent available", None),
            AuthErrorKind::NoAgent
        );
    }

    #[test]
    fn test_detect_auth_error_unknown() {
        assert_eq!(
            detect_auth_error("some random error", Some(1)),
            AuthErrorKind::Unknown
        );
    }
}
