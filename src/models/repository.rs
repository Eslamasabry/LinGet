use crate::models::PackageSource;
use serde::{Deserialize, Serialize};

/// Represents a package repository/remote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Repository name/identifier
    pub name: String,
    /// Repository URL (if applicable)
    pub url: Option<String>,
    /// Whether this repository is enabled
    pub enabled: bool,
    /// Package source this repository belongs to
    pub source: PackageSource,
    /// Description/title of the repository
    pub description: Option<String>,
}

impl Repository {
    pub fn new(
        name: impl Into<String>,
        source: PackageSource,
        enabled: bool,
        url: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            url,
            enabled,
            source,
            description: None,
        }
    }
}
