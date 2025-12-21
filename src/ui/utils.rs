use crate::backend::SUGGEST_PREFIX;

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
    /// Whether this is a user cancellation (may not need to be shown)
    pub is_cancelled: bool,
}

impl ErrorDisplay {
    /// Parse an anyhow::Error into a displayable format
    pub fn from_anyhow(error: &anyhow::Error) -> Self {
        let full_message = error.to_string();

        // Try to extract suggestion
        let (clean_message, _suggestion) = if let Some((msg, cmd)) = parse_suggestion(&full_message)
        {
            (msg, Some(cmd))
        } else {
            (full_message.clone(), None)
        };

        // Detect if this is a cancellation
        let is_cancelled = clean_message.to_lowercase().contains("cancelled")
            || clean_message.to_lowercase().contains("canceled")
            || clean_message
                .to_lowercase()
                .contains("authorization was cancelled");

        // Create a short title
        let title = Self::create_title(&clean_message);

        ErrorDisplay {
            title,
            is_cancelled,
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
}
