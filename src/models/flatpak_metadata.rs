use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the runtime environment for a Flatpak application
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakRuntime {
    /// Runtime identifier (e.g., "org.gnome.Platform")
    pub id: String,
    /// Runtime version (e.g., "45")
    pub version: String,
    /// Runtime branch (e.g., "stable")
    pub branch: String,
}

impl fmt::Display for FlatpakRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.id, self.version, self.branch)
    }
}

/// Represents a Flatpak permission category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PermissionCategory {
    /// Filesystem access permissions
    Filesystem,
    /// Socket access (network, session bus, etc.)
    Socket,
    /// Device access (dri, all, etc.)
    Device,
    /// Shared resources (network, ipc)
    Share,
    /// Environment variables
    Environment,
    /// D-Bus session bus access
    SessionBus,
    /// D-Bus system bus access
    SystemBus,
    /// Other/unknown permissions
    Other,
}

impl PermissionCategory {
    /// Returns an icon name for the permission category
    pub fn icon_name(&self) -> &'static str {
        match self {
            PermissionCategory::Filesystem => "folder-symbolic",
            PermissionCategory::Socket => "network-wired-symbolic",
            PermissionCategory::Device => "computer-symbolic",
            PermissionCategory::Share => "network-workgroup-symbolic",
            PermissionCategory::Environment => "preferences-other-symbolic",
            PermissionCategory::SessionBus => "preferences-system-symbolic",
            PermissionCategory::SystemBus => "system-run-symbolic",
            PermissionCategory::Other => "dialog-information-symbolic",
        }
    }

    /// Returns a human-readable description of the category
    pub fn description(&self) -> &'static str {
        match self {
            PermissionCategory::Filesystem => "Filesystem Access",
            PermissionCategory::Socket => "Socket Access",
            PermissionCategory::Device => "Device Access",
            PermissionCategory::Share => "Shared Resources",
            PermissionCategory::Environment => "Environment Variables",
            PermissionCategory::SessionBus => "Session Bus Access",
            PermissionCategory::SystemBus => "System Bus Access",
            PermissionCategory::Other => "Other Permissions",
        }
    }
}

impl fmt::Display for PermissionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Represents a single Flatpak permission
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakPermission {
    /// Permission category
    pub category: PermissionCategory,
    /// Permission value (e.g., "host", "x11", "pulseaudio")
    pub value: String,
    /// Whether this is a negated permission (starts with !)
    pub negated: bool,
    /// Human-readable description of what this permission allows
    pub description: String,
    /// Privacy impact level (higher = more sensitive)
    pub privacy_level: PrivacyLevel,
}

impl FlatpakPermission {
    /// Create a new permission from a raw permission string
    pub fn from_raw(category: PermissionCategory, raw: &str) -> Self {
        let negated = raw.starts_with('!');
        let value = if negated {
            raw.trim_start_matches('!')
        } else {
            raw
        }
        .to_string();

        let (description, privacy_level) = Self::describe_permission(category, &value, negated);

        Self {
            category,
            value,
            negated,
            description,
            privacy_level,
        }
    }

    fn describe_permission(
        category: PermissionCategory,
        value: &str,
        negated: bool,
    ) -> (String, PrivacyLevel) {
        let prefix = if negated { "Denied: " } else { "" };

        match category {
            PermissionCategory::Filesystem => {
                let (desc, level): (String, PrivacyLevel) = match value {
                    "host" => (
                        "Full access to all files".to_string(),
                        PrivacyLevel::High,
                    ),
                    "host-os" => (
                        "Access to host operating system files".to_string(),
                        PrivacyLevel::High,
                    ),
                    "host-etc" => (
                        "Access to /etc directory".to_string(),
                        PrivacyLevel::Medium,
                    ),
                    "home" => (
                        "Full access to home directory".to_string(),
                        PrivacyLevel::High,
                    ),
                    "xdg-desktop" => (
                        "Access to Desktop folder".to_string(),
                        PrivacyLevel::Medium,
                    ),
                    "xdg-documents" => (
                        "Access to Documents folder".to_string(),
                        PrivacyLevel::Medium,
                    ),
                    "xdg-download" => (
                        "Access to Downloads folder".to_string(),
                        PrivacyLevel::Medium,
                    ),
                    "xdg-music" => (
                        "Access to Music folder".to_string(),
                        PrivacyLevel::Low,
                    ),
                    "xdg-pictures" => (
                        "Access to Pictures folder".to_string(),
                        PrivacyLevel::Medium,
                    ),
                    "xdg-videos" => (
                        "Access to Videos folder".to_string(),
                        PrivacyLevel::Low,
                    ),
                    "xdg-config" => (
                        "Access to configuration files".to_string(),
                        PrivacyLevel::Medium,
                    ),
                    "xdg-cache" => (
                        "Access to cache directory".to_string(),
                        PrivacyLevel::Low,
                    ),
                    "xdg-data" => (
                        "Access to application data".to_string(),
                        PrivacyLevel::Medium,
                    ),
                    "xdg-run" => (
                        "Access to runtime directory".to_string(),
                        PrivacyLevel::Low,
                    ),
                    _ if value.starts_with('/') => (
                        format!("Access to {}", value),
                        PrivacyLevel::Medium,
                    ),
                    _ if value.starts_with('~') => (
                        format!("Access to {}", value),
                        PrivacyLevel::Medium,
                    ),
                    _ => (
                        format!("Filesystem: {}", value),
                        PrivacyLevel::Low,
                    ),
                };
                (format!("{}{}", prefix, desc), level)
            }
            PermissionCategory::Socket => {
                let (desc, level): (String, PrivacyLevel) = match value {
                    "x11" => ("X11 window system access".to_string(), PrivacyLevel::Medium),
                    "wayland" => ("Wayland display access".to_string(), PrivacyLevel::Low),
                    "fallback-x11" => ("Fallback X11 access".to_string(), PrivacyLevel::Medium),
                    "pulseaudio" => ("Audio playback and recording".to_string(), PrivacyLevel::Medium),
                    "session-bus" => ("D-Bus session bus access".to_string(), PrivacyLevel::Medium),
                    "system-bus" => ("D-Bus system bus access".to_string(), PrivacyLevel::High),
                    "ssh-auth" => ("SSH authentication agent".to_string(), PrivacyLevel::High),
                    "pcsc" => ("Smart card access".to_string(), PrivacyLevel::High),
                    "cups" => ("Printing access".to_string(), PrivacyLevel::Low),
                    "gpg-agent" => ("GPG agent access".to_string(), PrivacyLevel::High),
                    _ => (
                        format!("Socket: {}", value),
                        PrivacyLevel::Medium,
                    ),
                };
                (format!("{}{}", prefix, desc), level)
            }
            PermissionCategory::Device => {
                let (desc, level): (String, PrivacyLevel) = match value {
                    "dri" => ("GPU/graphics acceleration".to_string(), PrivacyLevel::Low),
                    "kvm" => ("Kernel virtualization access".to_string(), PrivacyLevel::High),
                    "shm" => ("Shared memory access".to_string(), PrivacyLevel::Low),
                    "all" => ("All device access".to_string(), PrivacyLevel::High),
                    _ => (
                        format!("Device: {}", value),
                        PrivacyLevel::Medium,
                    ),
                };
                (format!("{}{}", prefix, desc), level)
            }
            PermissionCategory::Share => {
                let (desc, level): (String, PrivacyLevel) = match value {
                    "network" => ("Network access".to_string(), PrivacyLevel::Medium),
                    "ipc" => ("Inter-process communication".to_string(), PrivacyLevel::Low),
                    _ => (
                        format!("Share: {}", value),
                        PrivacyLevel::Low,
                    ),
                };
                (format!("{}{}", prefix, desc), level)
            }
            PermissionCategory::Environment => (
                format!("{}Environment variable: {}", prefix, value),
                PrivacyLevel::Low,
            ),
            PermissionCategory::SessionBus => {
                let level = if value.contains("org.freedesktop.secrets")
                    || value.contains("org.gnome.keyring")
                {
                    PrivacyLevel::High
                } else if value.contains("org.freedesktop.Notifications") {
                    PrivacyLevel::Low
                } else {
                    PrivacyLevel::Medium
                };
                (format!("{}D-Bus: {}", prefix, value), level)
            }
            PermissionCategory::SystemBus => (
                format!("{}System D-Bus: {}", prefix, value),
                PrivacyLevel::High,
            ),
            PermissionCategory::Other => (format!("{}{}", prefix, value), PrivacyLevel::Low),
        }
    }
}

/// Privacy impact level of a permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// Low privacy impact
    Low,
    /// Medium privacy impact
    Medium,
    /// High privacy impact (sensitive data access)
    High,
}

impl PrivacyLevel {
    /// Returns a CSS class for styling
    pub fn css_class(&self) -> &'static str {
        match self {
            PrivacyLevel::Low => "privacy-low",
            PrivacyLevel::Medium => "privacy-medium",
            PrivacyLevel::High => "privacy-high",
        }
    }

    /// Returns an icon name for the privacy level
    pub fn icon_name(&self) -> &'static str {
        match self {
            PrivacyLevel::Low => "security-low-symbolic",
            PrivacyLevel::Medium => "security-medium-symbolic",
            PrivacyLevel::High => "security-high-symbolic",
        }
    }
}

impl fmt::Display for PrivacyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrivacyLevel::Low => write!(f, "Low"),
            PrivacyLevel::Medium => write!(f, "Medium"),
            PrivacyLevel::High => write!(f, "High"),
        }
    }
}

/// Complete metadata for a Flatpak application including sandbox information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlatpakMetadata {
    /// Application ID
    pub app_id: String,
    /// Runtime information
    pub runtime: Option<FlatpakRuntime>,
    /// SDK used for development (if available)
    pub sdk: Option<String>,
    /// List of permissions
    pub permissions: Vec<FlatpakPermission>,
    /// Remote/repository the app was installed from
    pub remote: Option<String>,
    /// Installation type (user or system)
    pub installation: InstallationType,
    /// Commit hash
    pub commit: Option<String>,
    /// Whether this is an EOL (end of life) application
    pub is_eol: bool,
    /// EOL reason if applicable
    pub eol_reason: Option<String>,
    /// Application architecture
    pub arch: Option<String>,
    /// Branch (e.g., "stable", "beta")
    pub branch: Option<String>,
    /// List of extensions used by this app
    pub extensions: Vec<String>,
}

/// Type of Flatpak installation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum InstallationType {
    /// User-level installation
    #[default]
    User,
    /// System-wide installation
    System,
}

impl fmt::Display for InstallationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallationType::User => write!(f, "User"),
            InstallationType::System => write!(f, "System"),
        }
    }
}

impl FlatpakMetadata {
    /// Returns the highest privacy level among all permissions
    pub fn max_privacy_level(&self) -> PrivacyLevel {
        self.permissions
            .iter()
            .map(|p| p.privacy_level)
            .max()
            .unwrap_or(PrivacyLevel::Low)
    }

    /// Returns permissions grouped by category
    pub fn permissions_by_category(&self) -> Vec<(PermissionCategory, Vec<&FlatpakPermission>)> {
        use std::collections::BTreeMap;

        let mut grouped: BTreeMap<PermissionCategory, Vec<&FlatpakPermission>> = BTreeMap::new();

        for perm in &self.permissions {
            grouped.entry(perm.category).or_default().push(perm);
        }

        grouped.into_iter().collect()
    }

    /// Returns true if the app has network access
    pub fn has_network_access(&self) -> bool {
        self.permissions.iter().any(|p| {
            p.category == PermissionCategory::Share
                && p.value == "network"
                && !p.negated
        })
    }

    /// Returns true if the app has full filesystem access
    pub fn has_full_filesystem_access(&self) -> bool {
        self.permissions.iter().any(|p| {
            p.category == PermissionCategory::Filesystem
                && (p.value == "host" || p.value == "home")
                && !p.negated
        })
    }

    /// Returns a summary of the sandbox security
    pub fn sandbox_summary(&self) -> SandboxSummary {
        let max_level = self.max_privacy_level();
        let has_network = self.has_network_access();
        let has_full_fs = self.has_full_filesystem_access();

        let (rating, description) = if has_full_fs && has_network {
            (
                SandboxRating::Weak,
                "This app has full filesystem and network access",
            )
        } else if has_full_fs {
            (
                SandboxRating::Moderate,
                "This app has full filesystem access but no network",
            )
        } else if has_network && max_level == PrivacyLevel::High {
            (
                SandboxRating::Moderate,
                "This app has network access and some sensitive permissions",
            )
        } else if max_level == PrivacyLevel::High {
            (
                SandboxRating::Moderate,
                "This app has some sensitive permissions",
            )
        } else if has_network {
            (SandboxRating::Good, "This app has network access only")
        } else {
            (SandboxRating::Strong, "This app is well sandboxed")
        };

        SandboxSummary {
            rating,
            description: description.to_string(),
            total_permissions: self.permissions.len(),
            high_risk_count: self
                .permissions
                .iter()
                .filter(|p| p.privacy_level == PrivacyLevel::High && !p.negated)
                .count(),
        }
    }
}

/// Overall sandbox security rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SandboxRating {
    /// Strong sandbox, minimal permissions
    Strong,
    /// Good sandbox with reasonable permissions
    Good,
    /// Moderate sandbox with some concerning permissions
    Moderate,
    /// Weak sandbox, extensive permissions
    Weak,
}

impl SandboxRating {
    /// Returns a CSS class for styling
    pub fn css_class(&self) -> &'static str {
        match self {
            SandboxRating::Strong => "sandbox-strong",
            SandboxRating::Good => "sandbox-good",
            SandboxRating::Moderate => "sandbox-moderate",
            SandboxRating::Weak => "sandbox-weak",
        }
    }

    /// Returns an icon name for the rating
    pub fn icon_name(&self) -> &'static str {
        match self {
            SandboxRating::Strong => "emblem-ok-symbolic",
            SandboxRating::Good => "emblem-default-symbolic",
            SandboxRating::Moderate => "dialog-warning-symbolic",
            SandboxRating::Weak => "dialog-error-symbolic",
        }
    }
}

impl fmt::Display for SandboxRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxRating::Strong => write!(f, "Strong"),
            SandboxRating::Good => write!(f, "Good"),
            SandboxRating::Moderate => write!(f, "Moderate"),
            SandboxRating::Weak => write!(f, "Weak"),
        }
    }
}

/// Summary of sandbox security for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSummary {
    /// Overall security rating
    pub rating: SandboxRating,
    /// Human-readable description
    pub description: String,
    /// Total number of permissions
    pub total_permissions: usize,
    /// Number of high-risk permissions
    pub high_risk_count: usize,
}
