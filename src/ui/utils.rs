use crate::backend::SUGGEST_PREFIX;
use crate::models::LinGetError;

/// Parse a suggestion command from an error message
/// Returns (cleaned message, suggestion command) if a suggestion was found
pub fn parse_suggestion(message: &str) -> Option<(String, String)> {
    let idx = message.find(SUGGEST_PREFIX)?;
    let command = message[idx + SUGGEST_PREFIX.len()..].trim();
    if command.is_empty() {
        return None;
    }
    Some((message[..idx].trim().to_string(), command.to_string()))
}

/// Parsed error information for UI display
#[derive(Debug, Clone)]
pub struct ErrorDisplay {
    /// Short title for the error (suitable for toasts)
    pub title: String,
    /// Detailed message (suitable for dialogs/command center)
    pub details: String,
    /// Optional suggestion command
    pub suggestion: Option<String>,
    /// Whether this is a user cancellation (may not need to be shown)
    pub is_cancelled: bool,
}

impl ErrorDisplay {
    /// Parse an anyhow::Error into a displayable format
    pub fn from_anyhow(error: &anyhow::Error) -> Self {
        let full_message = error.to_string();

        // Try to extract suggestion
        let (clean_message, suggestion) = if let Some((msg, cmd)) = parse_suggestion(&full_message) {
            (msg, Some(cmd))
        } else {
            (full_message.clone(), None)
        };

        // Detect if this is a cancellation
        let is_cancelled = clean_message.to_lowercase().contains("cancelled")
            || clean_message.to_lowercase().contains("canceled")
            || clean_message.to_lowercase().contains("authorization was cancelled");

        // Create a short title
        let title = Self::create_title(&clean_message);

        ErrorDisplay {
            title,
            details: clean_message,
            suggestion,
            is_cancelled,
        }
    }

    /// Parse a LinGetError into a displayable format
    pub fn from_linget_error(error: &LinGetError) -> Self {
        ErrorDisplay {
            title: error.short_message(),
            details: error.user_message(),
            suggestion: None, // Could be extracted from error if needed
            is_cancelled: error.is_cancelled(),
        }
    }

    /// Create a short title from a full error message
    fn create_title(message: &str) -> String {
        // Take the first line or first 50 characters
        let first_line = message.lines().next().unwrap_or(message);
        if first_line.len() > 60 {
            format!("{}...", &first_line[..57])
        } else {
            first_line.to_string()
        }
    }

    /// Format for toast notification (short)
    pub fn toast_message(&self) -> String {
        if self.is_cancelled {
            "Operation cancelled".to_string()
        } else {
            self.title.clone()
        }
    }

    /// Format for command center (detailed)
    pub fn command_center_message(&self) -> String {
        let mut msg = self.details.clone();
        if let Some(ref sug) = self.suggestion {
            msg.push_str(&format!("\n\nTry running:\n{}", sug));
        }
        msg
    }
}

/// Classify the type of error for appropriate UI treatment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// User cancellation - may not need to show an error
    Cancelled,
    /// Minor issue - show a brief toast
    Warning,
    /// Significant error - show in command center
    Error,
    /// Critical error - show dialog
    Critical,
}

impl ErrorSeverity {
    /// Determine severity from an error message
    pub fn from_message(message: &str) -> Self {
        let lower = message.to_lowercase();

        // User cancellations
        if lower.contains("cancelled")
            || lower.contains("canceled")
            || lower.contains("authorization was cancelled")
        {
            return ErrorSeverity::Cancelled;
        }

        // Critical errors
        if lower.contains("disk space")
            || lower.contains("corruption")
            || lower.contains("database locked")
        {
            return ErrorSeverity::Critical;
        }

        // Warnings (things that may be expected)
        if lower.contains("already installed")
            || lower.contains("not installed")
            || lower.contains("not found")
        {
            return ErrorSeverity::Warning;
        }

        // Default to error
        ErrorSeverity::Error
    }

    /// Determine severity from a LinGetError
    pub fn from_linget_error(error: &LinGetError) -> Self {
        if error.is_cancelled() {
            return ErrorSeverity::Cancelled;
        }

        match error {
            LinGetError::AlreadyInstalled { .. } | LinGetError::NotInstalled { .. } => {
                ErrorSeverity::Warning
            }
            LinGetError::InsufficientDiskSpace { .. }
            | LinGetError::PermissionDenied { .. }
            | LinGetError::ConfigError { .. } => ErrorSeverity::Critical,
            _ => ErrorSeverity::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_suggestion() {
        let msg = "Failed to install package\n\nLINGET_SUGGEST: sudo apt install foo";
        let result = parse_suggestion(msg);
        assert!(result.is_some());
        let (clean, cmd) = result.unwrap();
        assert!(clean.contains("Failed to install"));
        assert!(cmd.contains("sudo apt"));
    }

    #[test]
    fn test_error_display_from_anyhow() {
        let error = anyhow::anyhow!("Test error message");
        let display = ErrorDisplay::from_anyhow(&error);
        assert!(!display.is_cancelled);
        assert_eq!(display.title, "Test error message");
    }

    #[test]
    fn test_error_severity_cancelled() {
        let severity = ErrorSeverity::from_message("Operation was cancelled by user");
        assert_eq!(severity, ErrorSeverity::Cancelled);
    }

    #[test]
    fn test_error_severity_warning() {
        let severity = ErrorSeverity::from_message("Package 'foo' is already installed");
        assert_eq!(severity, ErrorSeverity::Warning);
    }
}
