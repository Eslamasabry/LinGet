//! Error types for LinGet
//!
//! This module provides structured error types with user-friendly messages
//! and actionable suggestions for common error scenarios.

use crate::models::PackageSource;
use std::fmt;
use thiserror::Error;

/// Main error type for LinGet operations
#[derive(Error, Debug)]
pub enum LinGetError {
    /// Package not found in any source
    #[error("Package '{name}' not found")]
    PackageNotFound {
        name: String,
        source_filter: Option<PackageSource>,
        suggestions: Vec<String>,
    },

    /// Package source is not available on this system
    #[error("Package source is not available on this system")]
    SourceNotAvailable {
        pkg_source: PackageSource,
        install_hint: Option<String>,
    },

    /// Package source is disabled by user
    #[error("Package source is disabled")]
    SourceDisabled { pkg_source: PackageSource },

    /// Backend command failed
    #[error("{operation} failed for '{package}'")]
    BackendError {
        operation: BackendOperation,
        package: String,
        pkg_source: PackageSource,
        details: String,
        suggestion: Option<String>,
    },

    /// Authentication/authorization failed
    #[error("Authorization required")]
    AuthorizationFailed {
        operation: String,
        suggestion: String,
    },

    /// Network-related error
    #[error("Network error: {message}")]
    NetworkError {
        message: String,
        is_timeout: bool,
        suggestion: Option<String>,
    },

    /// Permission denied (file system or other)
    #[error("Permission denied: {path}")]
    PermissionDenied {
        path: String,
        suggestion: String,
    },

    /// Package already installed
    #[error("Package '{name}' is already installed")]
    AlreadyInstalled {
        name: String,
        pkg_source: PackageSource,
        version: String,
    },

    /// Package not installed (for remove/update operations)
    #[error("Package '{name}' is not installed")]
    NotInstalled {
        name: String,
        pkg_source: Option<PackageSource>,
    },

    /// Invalid package name
    #[error("Invalid package name: {reason}")]
    InvalidPackageName { name: String, reason: String },

    /// Version not available
    #[error("Version '{version}' not available for '{package}'")]
    VersionNotAvailable {
        package: String,
        version: String,
        available_versions: Vec<String>,
    },

    /// Dependency conflict
    #[error("Dependency conflict for '{package}'")]
    DependencyConflict {
        package: String,
        conflicts: Vec<String>,
        suggestion: Option<String>,
    },

    /// Disk space insufficient
    #[error("Insufficient disk space")]
    InsufficientDiskSpace {
        required: Option<u64>,
        available: Option<u64>,
    },

    /// Package is currently in use (e.g., running snap)
    #[error("Package '{name}' is currently in use")]
    PackageInUse { name: String, suggestion: String },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigError { message: String, path: Option<String> },

    /// Cache error
    #[error("Cache error: {message}")]
    CacheError { message: String, suggestion: String },

    /// Command execution failed
    #[error("Command failed: {command}")]
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    /// Operation cancelled by user
    #[error("Operation cancelled")]
    Cancelled,

    /// Generic/unknown error with context
    #[error("{context}: {message}")]
    Other { context: String, message: String },
}

/// Types of backend operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendOperation {
    Install,
    Remove,
    Update,
    Downgrade,
    Search,
    List,
    CheckUpdates,
    AddRepository,
    RemoveRepository,
    RefreshCache,
}

impl fmt::Display for BackendOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendOperation::Install => write!(f, "Installation"),
            BackendOperation::Remove => write!(f, "Removal"),
            BackendOperation::Update => write!(f, "Update"),
            BackendOperation::Downgrade => write!(f, "Downgrade"),
            BackendOperation::Search => write!(f, "Search"),
            BackendOperation::List => write!(f, "List"),
            BackendOperation::CheckUpdates => write!(f, "Update check"),
            BackendOperation::AddRepository => write!(f, "Add repository"),
            BackendOperation::RemoveRepository => write!(f, "Remove repository"),
            BackendOperation::RefreshCache => write!(f, "Cache refresh"),
        }
    }
}

impl LinGetError {
    /// Create a PackageNotFound error with optional suggestions
    pub fn package_not_found(name: impl Into<String>, source_filter: Option<PackageSource>) -> Self {
        LinGetError::PackageNotFound {
            name: name.into(),
            source_filter,
            suggestions: Vec::new(),
        }
    }

    /// Create a PackageNotFound error with suggestions
    pub fn package_not_found_with_suggestions(
        name: impl Into<String>,
        source_filter: Option<PackageSource>,
        suggestions: Vec<String>,
    ) -> Self {
        LinGetError::PackageNotFound {
            name: name.into(),
            source_filter,
            suggestions,
        }
    }

    /// Create a SourceNotAvailable error
    pub fn source_not_available(pkg_source: PackageSource) -> Self {
        LinGetError::SourceNotAvailable {
            install_hint: pkg_source.install_hint().map(|s| s.to_string()),
            pkg_source,
        }
    }

    /// Create a BackendError from an anyhow error
    pub fn backend_error(
        operation: BackendOperation,
        package: impl Into<String>,
        pkg_source: PackageSource,
        error: &anyhow::Error,
    ) -> Self {
        let details = error.to_string();
        let suggestion = Self::extract_suggestion(&details);

        LinGetError::BackendError {
            operation,
            package: package.into(),
            pkg_source,
            details: Self::clean_error_message(&details),
            suggestion,
        }
    }

    /// Create an authorization failed error
    pub fn auth_failed(operation: impl Into<String>, command: impl Into<String>) -> Self {
        LinGetError::AuthorizationFailed {
            operation: operation.into(),
            suggestion: format!("Try running: {}", command.into()),
        }
    }

    /// Create a network error
    pub fn network_error(message: impl Into<String>, is_timeout: bool) -> Self {
        let msg = message.into();
        let suggestion = if is_timeout {
            Some("Check your internet connection and try again".to_string())
        } else if msg.contains("SSL") || msg.contains("TLS") || msg.contains("certificate") {
            Some("There may be a certificate issue. Check your system time and certificates".to_string())
        } else {
            Some("Check your internet connection and firewall settings".to_string())
        };

        LinGetError::NetworkError {
            message: msg,
            is_timeout,
            suggestion,
        }
    }

    /// Create a package in use error
    pub fn package_in_use(name: impl Into<String>) -> Self {
        LinGetError::PackageInUse {
            name: name.into(),
            suggestion: "Close all running instances of the application and try again".to_string(),
        }
    }

    /// Get a user-friendly message suitable for display in UI
    pub fn user_message(&self) -> String {
        match self {
            LinGetError::PackageNotFound {
                name,
                source_filter,
                suggestions,
            } => {
                let mut msg = if let Some(src) = source_filter {
                    format!("Package '{}' was not found in {}", name, src)
                } else {
                    format!("Package '{}' was not found in any available source", name)
                };

                if !suggestions.is_empty() {
                    msg.push_str("\n\nDid you mean:");
                    for s in suggestions.iter().take(5) {
                        msg.push_str(&format!("\n  - {}", s));
                    }
                }
                msg
            }

            LinGetError::SourceNotAvailable {
                pkg_source,
                install_hint,
            } => {
                let mut msg = format!("{} is not available on this system", pkg_source);
                if let Some(hint) = install_hint {
                    msg.push_str(&format!("\n\nTo use {}: {}", pkg_source, hint));
                }
                msg
            }

            LinGetError::SourceDisabled { pkg_source } => {
                format!(
                    "{} is currently disabled.\n\nEnable it in Settings or run: linget sources enable {}",
                    pkg_source,
                    format!("{:?}", pkg_source).to_lowercase()
                )
            }

            LinGetError::BackendError {
                operation,
                package,
                pkg_source,
                details,
                suggestion,
            } => {
                let mut msg = format!("{} failed for '{}' ({})", operation, package, pkg_source);
                if !details.is_empty() {
                    msg.push_str(&format!("\n\nDetails: {}", details));
                }
                if let Some(sug) = suggestion {
                    msg.push_str(&format!("\n\nSuggestion: {}", sug));
                }
                msg
            }

            LinGetError::AuthorizationFailed {
                operation,
                suggestion,
            } => {
                format!(
                    "{} requires elevated privileges.\n\nAuthorization was cancelled or denied.\n\n{}",
                    operation, suggestion
                )
            }

            LinGetError::NetworkError {
                message,
                suggestion,
                ..
            } => {
                let mut msg = format!("Network error: {}", message);
                if let Some(sug) = suggestion {
                    msg.push_str(&format!("\n\n{}", sug));
                }
                msg
            }

            LinGetError::PermissionDenied { path, suggestion } => {
                format!(
                    "Permission denied accessing: {}\n\n{}",
                    path, suggestion
                )
            }

            LinGetError::AlreadyInstalled {
                name,
                pkg_source,
                version,
            } => {
                format!(
                    "Package '{}' is already installed from {} (version {})",
                    name, pkg_source, version
                )
            }

            LinGetError::NotInstalled { name, pkg_source } => {
                if let Some(src) = pkg_source {
                    format!("Package '{}' is not installed from {}", name, src)
                } else {
                    format!("Package '{}' is not installed", name)
                }
            }

            LinGetError::InvalidPackageName { name, reason } => {
                format!("Invalid package name '{}': {}", name, reason)
            }

            LinGetError::VersionNotAvailable {
                package,
                version,
                available_versions,
            } => {
                let mut msg = format!("Version '{}' is not available for '{}'", version, package);
                if !available_versions.is_empty() {
                    msg.push_str("\n\nAvailable versions:");
                    for v in available_versions.iter().take(10) {
                        msg.push_str(&format!("\n  - {}", v));
                    }
                    if available_versions.len() > 10 {
                        msg.push_str(&format!("\n  ... and {} more", available_versions.len() - 10));
                    }
                }
                msg
            }

            LinGetError::DependencyConflict {
                package,
                conflicts,
                suggestion,
            } => {
                let mut msg = format!("Cannot install '{}' due to dependency conflicts:", package);
                for c in conflicts.iter().take(5) {
                    msg.push_str(&format!("\n  - {}", c));
                }
                if let Some(sug) = suggestion {
                    msg.push_str(&format!("\n\n{}", sug));
                }
                msg
            }

            LinGetError::InsufficientDiskSpace { required, available } => {
                let mut msg = "Insufficient disk space for this operation".to_string();
                if let (Some(req), Some(avail)) = (required, available) {
                    let req_str = humansize::format_size(*req, humansize::BINARY);
                    let avail_str = humansize::format_size(*avail, humansize::BINARY);
                    msg.push_str(&format!(
                        "\n\nRequired: {}\nAvailable: {}",
                        req_str, avail_str
                    ));
                }
                msg.push_str("\n\nFree up some disk space and try again");
                msg
            }

            LinGetError::PackageInUse { name, suggestion } => {
                format!(
                    "Package '{}' is currently in use.\n\n{}",
                    name, suggestion
                )
            }

            LinGetError::ConfigError { message, path } => {
                if let Some(p) = path {
                    format!("Configuration error in '{}': {}", p, message)
                } else {
                    format!("Configuration error: {}", message)
                }
            }

            LinGetError::CacheError { message, suggestion } => {
                format!("Cache error: {}\n\n{}", message, suggestion)
            }

            LinGetError::CommandFailed {
                command,
                exit_code,
                stderr,
            } => {
                let mut msg = format!("Command '{}' failed", command);
                if let Some(code) = exit_code {
                    msg.push_str(&format!(" with exit code {}", code));
                }
                if !stderr.is_empty() {
                    msg.push_str(&format!("\n\n{}", stderr));
                }
                msg
            }

            LinGetError::Cancelled => "Operation was cancelled".to_string(),

            LinGetError::Other { context, message } => {
                format!("{}: {}", context, message)
            }
        }
    }

    /// Get a short summary suitable for toasts/notifications
    pub fn short_message(&self) -> String {
        match self {
            LinGetError::PackageNotFound { name, .. } => format!("Package '{}' not found", name),
            LinGetError::SourceNotAvailable { pkg_source, .. } => format!("{} not available", pkg_source),
            LinGetError::SourceDisabled { pkg_source } => format!("{} is disabled", pkg_source),
            LinGetError::BackendError {
                operation, package, ..
            } => format!("{} failed for '{}'", operation, package),
            LinGetError::AuthorizationFailed { .. } => "Authorization failed".to_string(),
            LinGetError::NetworkError { .. } => "Network error".to_string(),
            LinGetError::PermissionDenied { .. } => "Permission denied".to_string(),
            LinGetError::AlreadyInstalled { name, .. } => format!("'{}' already installed", name),
            LinGetError::NotInstalled { name, .. } => format!("'{}' not installed", name),
            LinGetError::InvalidPackageName { name, .. } => format!("Invalid name: '{}'", name),
            LinGetError::VersionNotAvailable { version, .. } => format!("Version '{}' not available", version),
            LinGetError::DependencyConflict { package, .. } => format!("Conflict for '{}'", package),
            LinGetError::InsufficientDiskSpace { .. } => "Insufficient disk space".to_string(),
            LinGetError::PackageInUse { name, .. } => format!("'{}' is in use", name),
            LinGetError::ConfigError { .. } => "Configuration error".to_string(),
            LinGetError::CacheError { .. } => "Cache error".to_string(),
            LinGetError::CommandFailed { .. } => "Command failed".to_string(),
            LinGetError::Cancelled => "Cancelled".to_string(),
            LinGetError::Other { context, .. } => context.clone(),
        }
    }

    /// Check if this is a user-cancellation error
    pub fn is_cancelled(&self) -> bool {
        matches!(self, LinGetError::Cancelled | LinGetError::AuthorizationFailed { .. })
    }

    /// Check if this error should be logged at error level (vs warning)
    pub fn is_error_level(&self) -> bool {
        !matches!(
            self,
            LinGetError::Cancelled
                | LinGetError::AlreadyInstalled { .. }
                | LinGetError::NotInstalled { .. }
        )
    }

    /// Extract suggestion command from error message (for LINGET_SUGGEST: prefix)
    fn extract_suggestion(message: &str) -> Option<String> {
        let marker = crate::backend::SUGGEST_PREFIX;
        if let Some(idx) = message.find(marker) {
            let cmd = message[idx + marker.len()..].trim();
            if !cmd.is_empty() {
                return Some(cmd.to_string());
            }
        }
        None
    }

    /// Clean error message by removing internal markers
    fn clean_error_message(message: &str) -> String {
        let marker = crate::backend::SUGGEST_PREFIX;
        if let Some(idx) = message.find(marker) {
            message[..idx].trim().to_string()
        } else {
            message.to_string()
        }
    }
}

/// Convert from anyhow::Error to LinGetError for better context
impl From<anyhow::Error> for LinGetError {
    fn from(err: anyhow::Error) -> Self {
        let msg = err.to_string();

        // Try to detect common error patterns
        if msg.contains("not found") || msg.contains("No such file") {
            if msg.contains("command") || msg.contains("pkexec") {
                return LinGetError::CommandFailed {
                    command: "unknown".to_string(),
                    exit_code: None,
                    stderr: msg,
                };
            }
        }

        if msg.contains("permission denied") || msg.contains("Permission denied") {
            return LinGetError::PermissionDenied {
                path: "unknown".to_string(),
                suggestion: "Try running with elevated privileges".to_string(),
            };
        }

        if msg.contains("authorization") || msg.contains("Authentication") {
            return LinGetError::AuthorizationFailed {
                operation: "Operation".to_string(),
                suggestion: LinGetError::extract_suggestion(&msg)
                    .unwrap_or_else(|| "Try again with proper authorization".to_string()),
            };
        }

        if msg.contains("network") || msg.contains("connection") || msg.contains("timeout") {
            let is_timeout = msg.contains("timeout") || msg.contains("timed out");
            return LinGetError::network_error(msg, is_timeout);
        }

        LinGetError::Other {
            context: "Error".to_string(),
            message: msg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_not_found_message() {
        let err = LinGetError::package_not_found("foobar", Some(PackageSource::Apt));
        assert!(err.user_message().contains("foobar"));
        assert!(err.user_message().contains("APT"));
    }

    #[test]
    fn test_source_not_available_message() {
        let err = LinGetError::source_not_available(PackageSource::Flatpak);
        assert!(err.user_message().contains("Flatpak"));
        assert!(err.user_message().contains("flatpak"));
    }

    #[test]
    fn test_backend_operation_display() {
        assert_eq!(format!("{}", BackendOperation::Install), "Installation");
        assert_eq!(format!("{}", BackendOperation::Remove), "Removal");
    }

    #[test]
    fn test_short_message() {
        let err = LinGetError::PackageInUse {
            name: "test-app".to_string(),
            suggestion: "Close it".to_string(),
        };
        assert!(err.short_message().contains("test-app"));
        assert!(err.short_message().contains("in use"));
    }

    #[test]
    fn test_is_cancelled() {
        assert!(LinGetError::Cancelled.is_cancelled());
        assert!(!LinGetError::package_not_found("foo", None).is_cancelled());
    }
}
