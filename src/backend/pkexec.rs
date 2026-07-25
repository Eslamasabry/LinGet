use super::streaming::{run_streaming, StreamLine};
use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use std::io::{self, IsTerminal};
use std::os::fd::{FromRawFd, RawFd};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

/// Registration is a D-Bus round trip; it should be immediate.
const AGENT_REGISTRATION_TIMEOUT: Duration = Duration::from_secs(5);

pub const SUGGEST_PREFIX: &str = "LINGET_SUGGEST:";

#[derive(Debug, Clone)]
pub struct Suggest {
    pub command: String,
}

/// A request from a background operation to borrow the terminal.
///
/// A TUI owns the alternate screen, raw mode and stdin, so a password prompt
/// printed underneath it is both invisible and unanswerable. The UI hands the
/// terminal back for the duration of the prompt and takes it again afterwards.
pub struct TerminalHandover {
    /// Signalled by the UI once the terminal is usable.
    pub granted: tokio::sync::oneshot::Sender<()>,
    /// Awaited by the UI; resolves when the prompt is done.
    pub finished: tokio::sync::oneshot::Receiver<()>,
}

static TERMINAL_HANDOVER: OnceCell<mpsc::Sender<TerminalHandover>> = OnceCell::new();

/// Let privileged operations borrow the terminal from the UI that owns it.
pub fn install_terminal_handover(sender: mpsc::Sender<TerminalHandover>) {
    let _ = TERMINAL_HANDOVER.set(sender);
}

/// Returned by [`borrow_terminal`]; dropping or sending on it returns the
/// terminal to the UI.
type TerminalReturn = tokio::sync::oneshot::Sender<()>;

async fn borrow_terminal() -> Option<TerminalReturn> {
    let sender = TERMINAL_HANDOVER.get()?;
    let (granted, granted_rx) = tokio::sync::oneshot::channel();
    let (finished_tx, finished) = tokio::sync::oneshot::channel();

    sender
        .send(TerminalHandover { granted, finished })
        .await
        .ok()?;
    // Do not prompt until the UI has actually stepped aside.
    granted_rx.await.ok()?;
    Some(finished_tx)
}

/// A polkit agent that prompts on this terminal.
///
/// polkit sends an authentication request to whichever agent is registered for
/// the requesting subject. On a desktop that is the shell's graphical dialog,
/// which draws on the seat's screen — useless over SSH, in a detached tmux, or
/// anywhere without a display, where it leaves the operation waiting forever on
/// an answer that cannot arrive.
///
/// `pkttyagent` registers a text agent for this process, taking precedence over
/// the desktop one (no `--fallback`), so the password prompt lands here.
struct TtyAgent {
    child: tokio::process::Child,
    /// Returned to the UI when the prompt is finished with.
    terminal: Option<TerminalReturn>,
}

impl TtyAgent {
    /// Register, and wait until polkit has actually accepted the registration.
    ///
    /// The wait matters: if pkexec asked for authorisation first, polkit would
    /// route to the desktop agent and the prompt would vanish again. pkttyagent
    /// closes `--notify-fd` once registered, which is the sanctioned way to
    /// observe that without racing.
    async fn register() -> Option<Self> {
        if !io::stdin().is_terminal() {
            return None;
        }

        // Take the terminal from the UI first: pkttyagent draws the prompt and
        // reads the password itself, so it needs the real screen and stdin.
        let terminal = borrow_terminal().await;

        let (mut ready, notify_fd) = notify_pipe()?;

        let child = Command::new("pkttyagent")
            .arg("--process")
            .arg(std::process::id().to_string())
            .arg("--notify-fd")
            .arg(notify_fd.to_string())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| {
                debug!(%error, "pkttyagent unavailable; leaving polkit to pick an agent");
            })
            .ok()?;

        // Our copy of the write end must go, or the read below never sees EOF.
        unsafe { libc::close(notify_fd) };

        let registered = tokio::time::timeout(AGENT_REGISTRATION_TIMEOUT, async {
            let mut buf = [0u8; 1];
            let _ = ready.read(&mut buf).await;
        })
        .await;

        let mut agent = TtyAgent { child, terminal };
        if registered.is_err() {
            warn!("pkttyagent did not register in time; the prompt may not appear here");
            agent.stop().await;
            return None;
        }

        debug!("Registered terminal polkit agent for this process");
        Some(agent)
    }

    fn holds_terminal(&self) -> bool {
        self.terminal.is_some()
    }

    /// Give the terminal back while leaving the agent registered.
    ///
    /// Once the privileged command produces output it is past authentication,
    /// and there is nothing left to prompt for. Holding the terminal beyond
    /// that point would leave a TUI suspended in front of a blank screen for
    /// the whole of a long install.
    fn release_terminal(&mut self) {
        if let Some(terminal) = self.terminal.take() {
            let _ = terminal.send(());
        }
    }

    async fn stop(&mut self) {
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
        self.release_terminal();
    }
}

/// A pipe whose write end survives exec, for `pkttyagent --notify-fd`.
///
/// Rust marks new descriptors close-on-exec, which would make the child close
/// it immediately and look like instant registration.
fn notify_pipe() -> Option<(tokio::net::unix::pipe::Receiver, RawFd)> {
    let mut fds = [0 as RawFd; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return None;
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);

    let flags = unsafe { libc::fcntl(write_fd, libc::F_GETFD) };
    if flags < 0 || unsafe { libc::fcntl(write_fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) } < 0 {
        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
        return None;
    }

    // Safety: read_fd is freshly created above and not owned elsewhere.
    let receiver = unsafe { std::fs::File::from_raw_fd(read_fd) };
    match tokio::net::unix::pipe::Receiver::from_file(receiver) {
        Ok(receiver) => Some((receiver, write_fd)),
        Err(_) => {
            unsafe { libc::close(write_fd) };
            None
        }
    }
}

/// Whether this session could display a graphical polkit prompt.
///
/// polkit routes authentication to whichever agent is registered for the user,
/// which on a desktop is the shell's graphical dialog. That dialog appears on
/// the seat's screen — not in the terminal running LinGet. Over SSH, in a
/// detached tmux, or on any session without a display, the prompt is therefore
/// invisible to the person who triggered it, and the operation waits forever
/// for an answer that cannot arrive.
pub fn graphical_prompt_available() -> bool {
    ["DISPLAY", "WAYLAND_DISPLAY"]
        .iter()
        .any(|key| std::env::var_os(key).is_some_and(|value| !value.is_empty()))
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

    // Held for the whole call: polkit must still find the terminal agent when
    // pkexec asks, and the prompt has to stay answerable until it is answered.
    let mut tty_agent = TtyAgent::register().await;

    let output = Command::new("pkexec")
        .arg(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await;

    if let Some(agent) = tty_agent.as_mut() {
        agent.stop().await;
    }

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
            msg.push_str(
                "\n\nAuthorization was denied. Please try again with the correct password.",
            );
        }
        AuthErrorKind::NoAgent => {
            msg.push_str(
                "\n\nNo authentication agent is available. Make sure a polkit agent is running.",
            );
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

pub async fn run_pkexec_with_logs(
    program: &str,
    args: &[&str],
    context_msg: &str,
    suggest: Suggest,
    log_sender: mpsc::Sender<StreamLine>,
) -> Result<()> {
    let full_command = format!("pkexec {} {}", program, args.join(" "));
    debug!(
        command = %full_command,
        operation = %context_msg,
        "Executing privileged command"
    );

    let mut full_args: Vec<&str> = Vec::with_capacity(args.len() + 1);
    full_args.push(program);
    full_args.extend_from_slice(args);

    let (internal_tx, mut internal_rx) = mpsc::channel::<StreamLine>(200);
    let stderr_acc = Arc::new(Mutex::new(String::new()));
    let stderr_acc_clone = stderr_acc.clone();

    // Output means pkexec is past authentication, so the terminal can go back
    // to the UI even though the command itself is still running.
    let (first_output_tx, first_output_rx) = tokio::sync::oneshot::channel();
    let mut first_output_tx = Some(first_output_tx);

    let forward_task = tokio::spawn(async move {
        while let Some(line) = internal_rx.recv().await {
            if let Some(signal) = first_output_tx.take() {
                let _ = signal.send(());
            }
            if let StreamLine::Stderr(ref s) = line {
                let mut guard = stderr_acc_clone.lock().await;
                if !guard.is_empty() {
                    guard.push('\n');
                }
                guard.push_str(s);
            }

            let _ = log_sender.send(line).await;
        }
    });

    // Held for the whole call: polkit must still find the terminal agent when
    // pkexec asks, and the prompt has to stay answerable until it is answered.
    let mut tty_agent = TtyAgent::register().await;

    let streamed = {
        let stream = run_streaming("pkexec", &full_args, internal_tx);
        tokio::pin!(stream);
        tokio::pin!(first_output_rx);
        loop {
            tokio::select! {
                result = &mut stream => break result,
                _ = &mut first_output_rx, if tty_agent.as_ref().is_some_and(|a| a.holds_terminal()) => {
                    if let Some(agent) = tty_agent.as_mut() {
                        agent.release_terminal();
                    }
                }
            }
        }
    };

    if let Some(agent) = tty_agent.as_mut() {
        agent.stop().await;
    }

    let output = match streamed {
        Ok(o) => o,
        Err(e) => {
            let _ = forward_task.await;

            if let Some(io_err) = e.root_cause().downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::NotFound {
                    error!(
                        error = %io_err,
                        "pkexec not found - polkit may not be installed"
                    );
                    anyhow::bail!(
                        "{}. pkexec is not installed. Install polkit to enable privilege escalation.\n\n{} {}\n",
                        context_msg,
                        SUGGEST_PREFIX,
                        suggest.command
                    );
                }
            }

            return Err(e).with_context(|| context_msg.to_string());
        }
    };

    let _ = forward_task.await;

    if output.success {
        info!(
            command = %program,
            operation = %context_msg,
            "Privileged command completed successfully"
        );
        return Ok(());
    }

    let stderr = stderr_acc.lock().await.trim().to_string();
    let exit_code = output.exit_code;
    let auth_error = detect_auth_error(&stderr, exit_code);

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

    let mut msg = context_msg.to_string();

    match auth_error {
        AuthErrorKind::Cancelled => {
            msg.push_str("\n\nAuthorization was cancelled.");
        }
        AuthErrorKind::Denied => {
            msg.push_str(
                "\n\nAuthorization was denied. Please try again with the correct password.",
            );
        }
        AuthErrorKind::NoAgent => {
            msg.push_str(
                "\n\nNo authentication agent is available. Make sure a polkit agent is running.",
            );
        }
        AuthErrorKind::Unknown => {
            if !stderr.is_empty() {
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
        assert_eq!(detect_auth_error("", Some(126)), AuthErrorKind::Cancelled);
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

#[cfg(test)]
mod tty_agent_tests {
    use super::*;

    /// Exercises the real privileged path with a command that changes nothing,
    /// to confirm the polkit prompt lands on this terminal rather than on a
    /// desktop session. Needs a tty and a human (or a fed password), so it is
    /// ignored by default:
    ///
    /// ```text
    /// script -qec "cargo test prompts_on_this_terminal -- --ignored --nocapture" /dev/null
    /// ```
    #[tokio::test]
    #[ignore = "prompts for a password; run under a tty"]
    async fn prompts_on_this_terminal() {
        let result = run_pkexec(
            "/bin/true",
            &[],
            "verify terminal authentication",
            Suggest {
                command: "sudo /bin/true".to_string(),
            },
        )
        .await;
        println!("run_pkexec returned: {result:?}");
    }
}
