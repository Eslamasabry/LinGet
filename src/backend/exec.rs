//! Shared handling for package-manager invocations.
//!
//! Backends shell out to tools that are inconsistent about exit codes: several
//! report a non-zero status for entirely benign reasons (unmet optional
//! dependencies, extraneous packages, "no matches"), while still writing a
//! perfectly good listing to stdout. Treating every non-zero exit as fatal
//! would make those backends unusable.
//!
//! The distinction that actually matters to a user is narrower: a command that
//! failed *and* produced nothing we could parse has not told us the machine has
//! no packages — it has told us nothing at all. Reporting that as an empty list
//! is a silent lie, and it is indistinguishable from a clean system.

use anyhow::{bail, Result};
use std::process::Output;

/// Decide whether a listing invocation should be reported as a failure.
///
/// `parsed` is the number of entries successfully read out of the output. When
/// the command produced usable rows, its exit status is ignored deliberately.
pub fn ensure_listing_succeeded(tool: &str, output: &Output, parsed: usize) -> Result<()> {
    if parsed > 0 || output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no error output");

    match output.status.code() {
        Some(code) => bail!("`{tool}` failed with exit code {code}: {detail}"),
        None => bail!("`{tool}` was terminated by a signal: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    fn output(code: i32, stderr: &str) -> Output {
        Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn success_is_always_accepted() {
        assert!(ensure_listing_succeeded("apt", &output(0, ""), 0).is_ok());
        assert!(ensure_listing_succeeded("apt", &output(0, ""), 12).is_ok());
    }

    #[test]
    fn parsed_rows_outweigh_a_nonzero_exit() {
        // `npm list -g` exits non-zero over unmet peer deps but still prints the
        // tree; the same is true of several other tools.
        assert!(ensure_listing_succeeded("npm", &output(1, "unmet peer dep"), 25).is_ok());
    }

    #[test]
    fn a_failure_with_nothing_parsed_is_reported() {
        let error =
            ensure_listing_succeeded("dpkg-query", &output(2, "dpkg-query: no packages"), 0)
                .expect_err("a failed command with no output must not look like an empty system");
        let message = error.to_string();
        assert!(message.contains("dpkg-query"), "{message}");
        assert!(message.contains("exit code 2"), "{message}");
        assert!(message.contains("no packages"), "{message}");
    }

    #[test]
    fn missing_stderr_still_produces_a_usable_message() {
        let error =
            ensure_listing_succeeded("flatpak", &output(1, ""), 0).expect_err("should fail");
        assert!(error.to_string().contains("no error output"));
    }
}
