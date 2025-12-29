#![allow(dead_code)]

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::debug;

/// Strips ANSI escape codes (colors, cursor movements) from terminal output.
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    // CSI: ESC [ params letter
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch.is_ascii_alphabetic() {
                            break;
                        }
                    }
                    continue;
                } else if next == ']' {
                    // OSC: ESC ] ... BEL/ST
                    chars.next();
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        if ch == '\x07' || ch == '\\' {
                            break;
                        }
                    }
                    continue;
                }
            }
            continue;
        }
        result.push(c);
    }

    result
}

#[derive(Debug, Clone)]
pub enum StreamLine {
    Stdout(String),
    Stderr(String),
}

#[derive(Debug)]
pub struct StreamResult {
    pub exit_code: Option<i32>,
    pub success: bool,
}

pub async fn run_streaming(
    program: &str,
    args: &[&str],
    line_sender: mpsc::Sender<StreamLine>,
) -> Result<StreamResult> {
    debug!(
        command = %program,
        args = ?args,
        "Starting streaming command"
    );

    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn command: {}", program))?;

    let stdout = child.stdout.take().context("Failed to capture stdout")?;
    let stderr = child.stderr.take().context("Failed to capture stderr")?;

    let stdout_sender = line_sender.clone();
    let stderr_sender = line_sender;

    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let cleaned = strip_ansi(&line);
            if !cleaned.trim().is_empty()
                && stdout_sender
                    .send(StreamLine::Stdout(cleaned))
                    .await
                    .is_err()
            {
                break;
            }
        }
    });

    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let cleaned = strip_ansi(&line);
            if !cleaned.trim().is_empty()
                && stderr_sender
                    .send(StreamLine::Stderr(cleaned))
                    .await
                    .is_err()
            {
                break;
            }
        }
    });

    let status = child.wait().await.context("Failed to wait for command")?;

    let _ = stdout_task.await;
    let _ = stderr_task.await;

    let exit_code = status.code();
    let success = status.success();

    debug!(
        command = %program,
        exit_code = ?exit_code,
        success = success,
        "Streaming command completed"
    );

    Ok(StreamResult { exit_code, success })
}

pub async fn run_pkexec_streaming(
    program: &str,
    args: &[&str],
    line_sender: mpsc::Sender<StreamLine>,
) -> Result<StreamResult> {
    debug!(
        command = %program,
        args = ?args,
        "Starting streaming pkexec command"
    );

    let mut full_args = vec![program];
    full_args.extend(args);

    run_streaming("pkexec", &full_args, line_sender).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_colors() {
        assert_eq!(strip_ansi("\x1b[32mgreen\x1b[0m"), "green");
        assert_eq!(strip_ansi("\x1b[1;31mred bold\x1b[0m"), "red bold");
        assert_eq!(strip_ansi("no colors"), "no colors");
    }

    #[test]
    fn test_strip_ansi_cursor() {
        assert_eq!(strip_ansi("\x1b[2Kcleared line"), "cleared line");
        assert_eq!(strip_ansi("\x1b[Aup one line"), "up one line");
    }

    #[test]
    fn test_strip_ansi_mixed() {
        let input = "\x1b[32m==>\x1b[0m \x1b[1mInstalling\x1b[0m package...";
        assert_eq!(strip_ansi(input), "==> Installing package...");
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi(""), "");
        assert_eq!(strip_ansi("\x1b[0m"), "");
    }

    #[tokio::test]
    async fn test_run_streaming_simple() {
        let (tx, mut rx) = mpsc::channel(100);

        let result = run_streaming("echo", &["hello", "world"], tx)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));

        let mut lines = Vec::new();
        while let Ok(line) = rx.try_recv() {
            lines.push(line);
        }

        assert!(!lines.is_empty());
        if let StreamLine::Stdout(line) = &lines[0] {
            assert_eq!(line, "hello world");
        }
    }

    #[tokio::test]
    async fn test_run_streaming_stderr() {
        let (tx, mut rx) = mpsc::channel(100);

        let result = run_streaming("sh", &["-c", "echo error >&2"], tx)
            .await
            .unwrap();

        assert!(result.success);

        let mut lines = Vec::new();
        while let Ok(line) = rx.try_recv() {
            lines.push(line);
        }

        let has_stderr = lines.iter().any(|l| matches!(l, StreamLine::Stderr(_)));
        assert!(has_stderr);
    }
}
