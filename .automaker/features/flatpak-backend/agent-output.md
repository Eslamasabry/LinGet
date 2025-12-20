
🔧 Tool: Task
Input: {
  "subagent_type": "Explore",
  "prompt": "Explore this codebase to understand its structure and architecture. I need to implement support for sandboxed Flatpak applications across all Linux distributions. \n\nPlease find:\n1. The overall project structure and what kind of application this is\n2. Any existing backend implementations (look for patterns like \"backend\", \"provider\", \"handler\" etc.)\n3. Any existing Linux distribution support code\n4. Any sandbox-related code that might already exist\n5. Test file patterns and locations\n6. Configuration files and how features are typically added\n\nBe very thorough - check src/, lib/, packages/, and any other common source directories."
}

🔧 Tool: Task
Input: {
  "subagent_type": "Explore",
  "description": "Explore codebase structure",
  "prompt": "Explore this codebase to understand its structure and architecture. I need to implement support for sandboxed Flatpak applications across all Linux distributions. \n\nPlease find:\n1. The overall project structure and what kind of application this is\n2. Any existing backend implementations (look for patterns like \"backend\", \"provider\", \"handler\" etc.)\n3. Any existing Linux distribution support code\n4. Any sandbox-related code that might already exist\n5. Test file patterns and locations\n6. Configuration files and how features are typically added\n\nBe very thorough - check src/, lib/, packages/, and any other common source directories."
}

🔧 Tool: Bash
Input: {
  "command": "find /home/eslam/Storage/Code/LinGet -type f -name \"*.toml\" -o -name \"*.json\" -o -name \"*.yaml\" -o -name \"*.yml\" | head -20",
  "description": "Find configuration files in the root and subdirectories"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet"
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/ | head -30",
  "description": "List root directory contents"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/Cargo.toml"
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/src/",
  "description": "List source directory structure"
}

🔧 Tool: Bash
Input: {
  "command": "find /home/eslam/Storage/Code/LinGet/src -type f -name \"*.rs\" | sort",
  "description": "Find all Rust source files"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/mod.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/traits.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/flatpak.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/apt.rs",
  "limit": 100
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/dnf.rs",
  "limit": 100
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/package.rs",
  "limit": 150
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/.automaker/features/",
  "description": "List feature directory contents"
}

🔧 Tool: Bash
Input: {
  "command": "find /home/eslam/Storage/Code/LinGet -type f -name \"*test*\" -o -name \"*spec*\" | head -20",
  "description": "Find test files"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/apt-backend/feature.json"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/flatpak-backend/feature.json"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/dnf-backend/feature.json"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/cargo-backend/feature.json"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/package.rs",
  "offset": 150
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/providers.rs",
  "limit": 150
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/src/models/",
  "description": "List models directory"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/snap.rs",
  "limit": 150
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs",
  "limit": 100
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/src/cli/commands/",
  "description": "List CLI commands directory"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/pacman.rs",
  "limit": 80
}

🔧 Tool: Bash
Input: {
  "command": "grep -r \"sandbox\\|bubblewrap\\|firejail\\|seccomp\\|apparmor\" /home/eslam/Storage/Code/LinGet/src --include=\"*.rs\" | head -20",
  "description": "Search for sandbox-related code"
}

🔧 Tool: Bash
Input: {
  "command": "find /home/eslam/Storage/Code/LinGet -type d -name \"tests\" -o -type d -name \"test\" 2>/dev/null",
  "description": "Find test directories"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/README.md"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/PLAN.md",
  "limit": 200
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/main.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/app.rs"
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/src/ui/",
  "description": "List UI module structure"
}

🔧 Tool: Bash
Input: {
  "command": "grep -r \"fedora\\|debian\\|ubuntu\\|arch\\|opensuse\\|distro\\|lsb\\|os_release\" /home/eslam/Storage/Code/LinGet/src --include=\"*.rs\" | head -15",
  "description": "Search for distribution detection code"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/config.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/providers.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/providers.rs",
  "offset": 150,
  "limit": 100
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/CLAUDE.md"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/CONTRIBUTING.md"
}

🔧 Tool: Bash
Input: {
  "command": "head -100 /home/eslam/Storage/Code/LinGet/resources/style.css",
  "description": "Show CSS styling for sources"
}

🔧 Tool: Bash
Input: {
  "command": "wc -l /home/eslam/Storage/Code/LinGet/src/ui/window.rs /home/eslam/Storage/Code/LinGet/src/backend/*.rs",
  "description": "Get line counts of major files"
}

Now I have a comprehensive understanding of the codebase. Let me read the existing Flatpak backend to understand what's already implemented and what needs to be enhanced:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/flatpak.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/traits.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/mod.rs"
}

Now let me read the Package model to understand the data structures and see if there's already sandbox-related metadata:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/package.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/mod.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/providers.rs"
}

Now let me check the feature specification file and look at how the UI displays package details to understand the existing patterns:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/flatpak-backend/spec.json"
}

🔧 Tool: Glob
Input: {
  "pattern": "**/*package_details*",
  "path": "/home/eslam/Storage/Code/LinGet/src"
}

🔧 Tool: Glob
Input: {
  "pattern": "**/*details*",
  "path": "/home/eslam/Storage/Code/LinGet/src"
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/.automaker/features/flatpak-backend/ 2>/dev/null || echo \"Directory not found\"",
  "description": "Check flatpak feature directory"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/flatpak-backend/feature.json"
}

🔧 Tool: Glob
Input: {
  "pattern": "**/ui/**/*.rs",
  "path": "/home/eslam/Storage/Code/LinGet/src"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/mod.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs"
}

Now I have a comprehensive understanding of the codebase. Let me create a todo list and implement the Flatpak sandbox support feature:

🔧 Tool: TodoWrite
Input: {
  "todos": [
    {
      "content": "Add FlatpakMetadata struct to models for sandbox permissions",
      "status": "in_progress",
      "activeForm": "Adding FlatpakMetadata struct to models"
    },
    {
      "content": "Enhance Flatpak backend with sandbox permissions detection",
      "status": "pending",
      "activeForm": "Enhancing Flatpak backend with sandbox detection"
    },
    {
      "content": "Add method to get app permissions and runtime info",
      "status": "pending",
      "activeForm": "Adding method for app permissions"
    },
    {
      "content": "Update Package model to include Flatpak-specific metadata",
      "status": "pending",
      "activeForm": "Updating Package model"
    },
    {
      "content": "Add CLI command to show Flatpak permissions",
      "status": "pending",
      "activeForm": "Adding CLI command for permissions"
    },
    {
      "content": "Update UI to display sandbox information",
      "status": "pending",
      "activeForm": "Updating UI for sandbox display"
    },
    {
      "content": "Add tests for Flatpak sandbox functionality",
      "status": "pending",
      "activeForm": "Adding tests for Flatpak sandbox"
    }
  ]
}

Let me first create the Flatpak-specific metadata structures. I'll add a new file for Flatpak metadata:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/enrichment.rs"
}

Now I understand the enrichment pattern. Let me create a new file for Flatpak-specific metadata structures and add sandbox support to the Flatpak backend. Let me first create the Flatpak metadata module:

🔧 Tool: Write
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/flatpak_metadata.rs",
  "content": "use serde::{Deserialize, Serialize};\nuse std::fmt;\n\n/// Represents the runtime environment for a Flatpak application\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub struct FlatpakRuntime {\n    /// Runtime identifier (e.g., \"org.gnome.Platform\")\n    pub id: String,\n    /// Runtime version (e.g., \"45\")\n    pub version: String,\n    /// Runtime branch (e.g., \"stable\")\n    pub branch: String,\n}\n\nimpl fmt::Display for FlatpakRuntime {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        write!(f, \"{}/{}/{}\", self.id, self.version, self.branch)\n    }\n}\n\n/// Represents a Flatpak permission category\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub enum PermissionCategory {\n    /// Filesystem access permissions\n    Filesystem,\n    /// Socket access (network, session bus, etc.)\n    Socket,\n    /// Device access (dri, all, etc.)\n    Device,\n    /// Shared resources (network, ipc)\n    Share,\n    /// Environment variables\n    Environment,\n    /// D-Bus session bus access\n    SessionBus,\n    /// D-Bus system bus access\n    SystemBus,\n    /// Other/unknown permissions\n    Other,\n}\n\nimpl PermissionCategory {\n    /// Returns an icon name for the permission category\n    pub fn icon_name(&self) -> &'static str {\n        match self {\n            PermissionCategory::Filesystem => \"folder-symbolic\",\n            PermissionCategory::Socket => \"network-wired-symbolic\",\n            PermissionCategory::Device => \"computer-symbolic\",\n            PermissionCategory::Share => \"network-workgroup-symbolic\",\n            PermissionCategory::Environment => \"preferences-other-symbolic\",\n            PermissionCategory::SessionBus => \"preferences-system-symbolic\",\n            PermissionCategory::SystemBus => \"system-run-symbolic\",\n            PermissionCategory::Other => \"dialog-information-symbolic\",\n        }\n    }\n\n    /// Returns a human-readable description of the category\n    pub fn description(&self) -> &'static str {\n        match self {\n            PermissionCategory::Filesystem => \"Filesystem Access\",\n            PermissionCategory::Socket => \"Socket Access\",\n            PermissionCategory::Device => \"Device Access\",\n            PermissionCategory::Share => \"Shared Resources\",\n            PermissionCategory::Environment => \"Environment Variables\",\n            PermissionCategory::SessionBus => \"Session Bus Access\",\n            PermissionCategory::SystemBus => \"System Bus Access\",\n            PermissionCategory::Other => \"Other Permissions\",\n        }\n    }\n}\n\nimpl fmt::Display for PermissionCategory {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        write!(f, \"{}\", self.description())\n    }\n}\n\n/// Represents a single Flatpak permission\n#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]\npub struct FlatpakPermission {\n    /// Permission category\n    pub category: PermissionCategory,\n    /// Permission value (e.g., \"host\", \"x11\", \"pulseaudio\")\n    pub value: String,\n    /// Whether this is a negated permission (starts with !)\n    pub negated: bool,\n    /// Human-readable description of what this permission allows\n    pub description: String,\n    /// Privacy impact level (higher = more sensitive)\n    pub privacy_level: PrivacyLevel,\n}\n\nimpl FlatpakPermission {\n    /// Create a new permission from a raw permission string\n    pub fn from_raw(category: PermissionCategory, raw: &str) -> Self {\n        let negated = raw.starts_with('!');\n        let value = if negated {\n            raw.trim_start_matches('!')\n        } else {\n            raw\n        }\n        .to_string();\n\n        let (description, privacy_level) = Self::describe_permission(category, &value, negated);\n\n        Self {\n            category,\n            value,\n            negated,\n            description,\n            privacy_level,\n        }\n    }\n\n    fn describe_permission(\n        category: PermissionCategory,\n        value: &str,\n        negated: bool,\n    ) -> (String, PrivacyLevel) {\n        let prefix = if negated { \"Denied: \" } else { \"\" };\n\n        match category {\n            PermissionCategory::Filesystem => {\n                let (desc, level) = match value {\n                    \"host\" => (\n                        \"Full access to all files\",\n                        PrivacyLevel::High,\n                    ),\n                    \"host-os\" => (\n                        \"Access to host operating system files\",\n                        PrivacyLevel::High,\n                    ),\n                    \"host-etc\" => (\n                        \"Access to /etc directory\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"home\" => (\n                        \"Full access to home directory\",\n                        PrivacyLevel::High,\n                    ),\n                    \"xdg-desktop\" => (\n                        \"Access to Desktop folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-documents\" => (\n                        \"Access to Documents folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-download\" => (\n                        \"Access to Downloads folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-music\" => (\n                        \"Access to Music folder\",\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-pictures\" => (\n                        \"Access to Pictures folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-videos\" => (\n                        \"Access to Videos folder\",\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-config\" => (\n                        \"Access to configuration files\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-cache\" => (\n                        \"Access to cache directory\",\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-data\" => (\n                        \"Access to application data\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-run\" => (\n                        \"Access to runtime directory\",\n                        PrivacyLevel::Low,\n                    ),\n                    _ if value.starts_with('/') => (\n                        format!(\"Access to {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                    _ if value.starts_with('~') => (\n                        format!(\"Access to {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                    _ => (\n                        format!(\"Filesystem: {}\", value).leak(),\n                        PrivacyLevel::Low,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Socket => {\n                let (desc, level) = match value {\n                    \"x11\" => (\"X11 window system access\", PrivacyLevel::Medium),\n                    \"wayland\" => (\"Wayland display access\", PrivacyLevel::Low),\n                    \"fallback-x11\" => (\"Fallback X11 access\", PrivacyLevel::Medium),\n                    \"pulseaudio\" => (\"Audio playback and recording\", PrivacyLevel::Medium),\n                    \"session-bus\" => (\"D-Bus session bus access\", PrivacyLevel::Medium),\n                    \"system-bus\" => (\"D-Bus system bus access\", PrivacyLevel::High),\n                    \"ssh-auth\" => (\"SSH authentication agent\", PrivacyLevel::High),\n                    \"pcsc\" => (\"Smart card access\", PrivacyLevel::High),\n                    \"cups\" => (\"Printing access\", PrivacyLevel::Low),\n                    \"gpg-agent\" => (\"GPG agent access\", PrivacyLevel::High),\n                    _ => (\n                        format!(\"Socket: {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Device => {\n                let (desc, level) = match value {\n                    \"dri\" => (\"GPU/graphics acceleration\", PrivacyLevel::Low),\n                    \"kvm\" => (\"Kernel virtualization access\", PrivacyLevel::High),\n                    \"shm\" => (\"Shared memory access\", PrivacyLevel::Low),\n                    \"all\" => (\"All device access\", PrivacyLevel::High),\n                    _ => (\n                        format!(\"Device: {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Share => {\n                let (desc, level) = match value {\n                    \"network\" => (\"Network access\", PrivacyLevel::Medium),\n                    \"ipc\" => (\"Inter-process communication\", PrivacyLevel::Low),\n                    _ => (\n                        format!(\"Share: {}\", value).leak(),\n                        PrivacyLevel::Low,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Environment => (\n                format!(\"{}Environment variable: {}\", prefix, value),\n                PrivacyLevel::Low,\n            ),\n            PermissionCategory::SessionBus => {\n                let level = if value.contains(\"org.freedesktop.secrets\")\n                    || value.contains(\"org.gnome.keyring\")\n                {\n                    PrivacyLevel::High\n                } else if value.contains(\"org.freedesktop.Notifications\") {\n                    PrivacyLevel::Low\n                } else {\n                    PrivacyLevel::Medium\n                };\n                (format!(\"{}D-Bus: {}\", prefix, value), level)\n            }\n            PermissionCategory::SystemBus => (\n                format!(\"{}System D-Bus: {}\", prefix, value),\n                PrivacyLevel::High,\n            ),\n            PermissionCategory::Other => (format!(\"{}{}\", prefix, value), PrivacyLevel::Low),\n        }\n    }\n}\n\n/// Privacy impact level of a permission\n#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\npub enum PrivacyLevel {\n    /// Low privacy impact\n    Low,\n    /// Medium privacy impact\n    Medium,\n    /// High privacy impact (sensitive data access)\n    High,\n}\n\nimpl PrivacyLevel {\n    /// Returns a CSS class for styling\n    pub fn css_class(&self) -> &'static str {\n        match self {\n            PrivacyLevel::Low => \"privacy-low\",\n            PrivacyLevel::Medium => \"privacy-medium\",\n            PrivacyLevel::High => \"privacy-high\",\n        }\n    }\n\n    /// Returns an icon name for the privacy level\n    pub fn icon_name(&self) -> &'static str {\n        match self {\n            PrivacyLevel::Low => \"security-low-symbolic\",\n            PrivacyLevel::Medium => \"security-medium-symbolic\",\n            PrivacyLevel::High => \"security-high-symbolic\",\n        }\n    }\n}\n\nimpl fmt::Display for PrivacyLevel {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        match self {\n            PrivacyLevel::Low => write!(f, \"Low\"),\n            PrivacyLevel::Medium => write!(f, \"Medium\"),\n            PrivacyLevel::High => write!(f, \"High\"),\n        }\n    }\n}\n\n/// Complete metadata for a Flatpak application including sandbox information\n#[derive(Debug, Clone, Default, Serialize, Deserialize)]\npub struct FlatpakMetadata {\n    /// Application ID\n    pub app_id: String,\n    /// Runtime information\n    pub runtime: Option<FlatpakRuntime>,\n    /// SDK used for development (if available)\n    pub sdk: Option<String>,\n    /// List of permissions\n    pub permissions: Vec<FlatpakPermission>,\n    /// Remote/repository the app was installed from\n    pub remote: Option<String>,\n    /// Installation type (user or system)\n    pub installation: InstallationType,\n    /// Commit hash\n    pub commit: Option<String>,\n    /// Whether this is an EOL (end of life) application\n    pub is_eol: bool,\n    /// EOL reason if applicable\n    pub eol_reason: Option<String>,\n    /// Application architecture\n    pub arch: Option<String>,\n    /// Branch (e.g., \"stable\", \"beta\")\n    pub branch: Option<String>,\n    /// List of extensions used by this app\n    pub extensions: Vec<String>,\n}\n\n/// Type of Flatpak installation\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]\npub enum InstallationType {\n    /// User-level installation\n    #[default]\n    User,\n    /// System-wide installation\n    System,\n}\n\nimpl fmt::Display for InstallationType {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        match self {\n            InstallationType::User => write!(f, \"User\"),\n            InstallationType::System => write!(f, \"System\"),\n        }\n    }\n}\n\nimpl FlatpakMetadata {\n    /// Returns the highest privacy level among all permissions\n    pub fn max_privacy_level(&self) -> PrivacyLevel {\n        self.permissions\n            .iter()\n            .map(|p| p.privacy_level)\n            .max()\n            .unwrap_or(PrivacyLevel::Low)\n    }\n\n    /// Returns permissions grouped by category\n    pub fn permissions_by_category(&self) -> Vec<(PermissionCategory, Vec<&FlatpakPermission>)> {\n        use std::collections::BTreeMap;\n\n        let mut grouped: BTreeMap<PermissionCategory, Vec<&FlatpakPermission>> = BTreeMap::new();\n\n        for perm in &self.permissions {\n            grouped.entry(perm.category).or_default().push(perm);\n        }\n\n        grouped.into_iter().collect()\n    }\n\n    /// Returns true if the app has network access\n    pub fn has_network_access(&self) -> bool {\n        self.permissions.iter().any(|p| {\n            p.category == PermissionCategory::Share\n                && p.value == \"network\"\n                && !p.negated\n        })\n    }\n\n    /// Returns true if the app has full filesystem access\n    pub fn has_full_filesystem_access(&self) -> bool {\n        self.permissions.iter().any(|p| {\n            p.category == PermissionCategory::Filesystem\n                && (p.value == \"host\" || p.value == \"home\")\n                && !p.negated\n        })\n    }\n\n    /// Returns a summary of the sandbox security\n    pub fn sandbox_summary(&self) -> SandboxSummary {\n        let max_level = self.max_privacy_level();\n        let has_network = self.has_network_access();\n        let has_full_fs = self.has_full_filesystem_access();\n\n        let (rating, description) = if has_full_fs && has_network {\n            (\n                SandboxRating::Weak,\n                \"This app has full filesystem and network access\",\n            )\n        } else if has_full_fs {\n            (\n                SandboxRating::Moderate,\n                \"This app has full filesystem access but no network\",\n            )\n        } else if has_network && max_level == PrivacyLevel::High {\n            (\n                SandboxRating::Moderate,\n                \"This app has network access and some sensitive permissions\",\n            )\n        } else if max_level == PrivacyLevel::High {\n            (\n                SandboxRating::Moderate,\n                \"This app has some sensitive permissions\",\n            )\n        } else if has_network {\n            (SandboxRating::Good, \"This app has network access only\")\n        } else {\n            (SandboxRating::Strong, \"This app is well sandboxed\")\n        };\n\n        SandboxSummary {\n            rating,\n            description: description.to_string(),\n            total_permissions: self.permissions.len(),\n            high_risk_count: self\n                .permissions\n                .iter()\n                .filter(|p| p.privacy_level == PrivacyLevel::High && !p.negated)\n                .count(),\n        }\n    }\n}\n\n/// Overall sandbox security rating\n#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\npub enum SandboxRating {\n    /// Strong sandbox, minimal permissions\n    Strong,\n    /// Good sandbox with reasonable permissions\n    Good,\n    /// Moderate sandbox with some concerning permissions\n    Moderate,\n    /// Weak sandbox, extensive permissions\n    Weak,\n}\n\nimpl SandboxRating {\n    /// Returns a CSS class for styling\n    pub fn css_class(&self) -> &'static str {\n        match self {\n            SandboxRating::Strong => \"sandbox-strong\",\n            SandboxRating::Good => \"sandbox-good\",\n            SandboxRating::Moderate => \"sandbox-moderate\",\n            SandboxRating::Weak => \"sandbox-weak\",\n        }\n    }\n\n    /// Returns an icon name for the rating\n    pub fn icon_name(&self) -> &'static str {\n        match self {\n            SandboxRating::Strong => \"emblem-ok-symbolic\",\n            SandboxRating::Good => \"emblem-default-symbolic\",\n            SandboxRating::Moderate => \"dialog-warning-symbolic\",\n            SandboxRating::Weak => \"dialog-error-symbolic\",\n        }\n    }\n}\n\nimpl fmt::Display for SandboxRating {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        match self {\n            SandboxRating::Strong => write!(f, \"Strong\"),\n            SandboxRating::Good => write!(f, \"Good\"),\n            SandboxRating::Moderate => write!(f, \"Moderate\"),\n            SandboxRating::Weak => write!(f, \"Weak\"),\n        }\n    }\n}\n\n/// Summary of sandbox security for display\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct SandboxSummary {\n    /// Overall security rating\n    pub rating: SandboxRating,\n    /// Human-readable description\n    pub description: String,\n    /// Total number of permissions\n    pub total_permissions: usize,\n    /// Number of high-risk permissions\n    pub high_risk_count: usize,\n}\n"
}

Now let me update the models/mod.rs to include the new module:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/mod.rs",
  "old_string": "mod backup;\nmod cache;\nmod config;\nmod enrichment;\nmod history;\nmod icons;\nmod package;\nmod repository;\n\npub use backup::*;\npub use cache::*;\npub use config::*;\npub use enrichment::*;\npub use history::*;\npub use icons::*;\npub use package::*;\npub use repository::*;",
  "new_string": "mod backup;\nmod cache;\nmod config;\nmod enrichment;\nmod flatpak_metadata;\nmod history;\nmod icons;\nmod package;\nmod repository;\n\npub use backup::*;\npub use cache::*;\npub use config::*;\npub use enrichment::*;\npub use flatpak_metadata::*;\npub use history::*;\npub use icons::*;\npub use package::*;\npub use repository::*;"
}

🔧 Tool: TodoWrite
Input: {
  "todos": [
    {
      "content": "Add FlatpakMetadata struct to models for sandbox permissions",
      "status": "completed",
      "activeForm": "Adding FlatpakMetadata struct to models"
    },
    {
      "content": "Enhance Flatpak backend with sandbox permissions detection",
      "status": "in_progress",
      "activeForm": "Enhancing Flatpak backend with sandbox detection"
    },
    {
      "content": "Add method to get app permissions and runtime info",
      "status": "pending",
      "activeForm": "Adding method for app permissions"
    },
    {
      "content": "Update Package model to include Flatpak-specific metadata",
      "status": "pending",
      "activeForm": "Updating Package model"
    },
    {
      "content": "Add CLI command to show Flatpak permissions",
      "status": "pending",
      "activeForm": "Adding CLI command for permissions"
    },
    {
      "content": "Update UI to display sandbox information",
      "status": "pending",
      "activeForm": "Updating UI for sandbox display"
    },
    {
      "content": "Add tests for Flatpak sandbox functionality",
      "status": "pending",
      "activeForm": "Adding tests for Flatpak sandbox"
    }
  ]
}

Now let me enhance the Flatpak backend to add sandbox permissions detection:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/flatpak.rs",
  "old_string": "use super::PackageBackend;\nuse crate::models::{Package, PackageSource, PackageStatus, Repository};\nuse anyhow::{Context, Result};\nuse async_trait::async_trait;\nuse std::process::Stdio;\nuse tokio::process::Command;",
  "new_string": "use super::PackageBackend;\nuse crate::models::{\n    FlatpakMetadata, FlatpakPermission, FlatpakRuntime, InstallationType, Package, PackageSource,\n    PackageStatus, PermissionCategory, Repository,\n};\nuse anyhow::{Context, Result};\nuse async_trait::async_trait;\nuse std::process::Stdio;\nuse tokio::process::Command;"
}

Now let me add the sandbox metadata functions to the FlatpakBackend:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/flatpak.rs",
  "old_string": "pub struct FlatpakBackend;\n\nimpl FlatpakBackend {\n    pub fn new() -> Self {\n        Self\n    }\n}\n\nimpl Default for FlatpakBackend {\n    fn default() -> Self {\n        Self::new()\n    }\n}",
  "new_string": "pub struct FlatpakBackend;\n\nimpl FlatpakBackend {\n    pub fn new() -> Self {\n        Self\n    }\n\n    /// Get detailed metadata for a Flatpak application including sandbox permissions\n    pub async fn get_metadata(&self, app_id: &str) -> Result<FlatpakMetadata> {\n        // Get basic info using flatpak info\n        let info_output = Command::new(\"flatpak\")\n            .args([\"info\", \"--show-metadata\", app_id])\n            .stdout(Stdio::piped())\n            .stderr(Stdio::piped())\n            .output()\n            .await\n            .context(\"Failed to get flatpak metadata\")?;\n\n        let metadata_str = String::from_utf8_lossy(&info_output.stdout);\n\n        // Parse the metadata\n        let mut metadata = Self::parse_metadata(&metadata_str, app_id);\n\n        // Get additional info\n        let info_output = Command::new(\"flatpak\")\n            .args([\"info\", app_id])\n            .stdout(Stdio::piped())\n            .stderr(Stdio::piped())\n            .output()\n            .await\n            .context(\"Failed to get flatpak info\")?;\n\n        let info_str = String::from_utf8_lossy(&info_output.stdout);\n        Self::parse_info(&info_str, &mut metadata);\n\n        Ok(metadata)\n    }\n\n    /// Get metadata for a specific installation type (user or system)\n    pub async fn get_metadata_for_installation(\n        &self,\n        app_id: &str,\n        installation: InstallationType,\n    ) -> Result<FlatpakMetadata> {\n        let install_arg = match installation {\n            InstallationType::User => \"--user\",\n            InstallationType::System => \"--system\",\n        };\n\n        // Get basic info using flatpak info\n        let info_output = Command::new(\"flatpak\")\n            .args([\"info\", install_arg, \"--show-metadata\", app_id])\n            .stdout(Stdio::piped())\n            .stderr(Stdio::piped())\n            .output()\n            .await\n            .context(\"Failed to get flatpak metadata\")?;\n\n        let metadata_str = String::from_utf8_lossy(&info_output.stdout);\n        let mut metadata = Self::parse_metadata(&metadata_str, app_id);\n        metadata.installation = installation;\n\n        // Get additional info\n        let info_output = Command::new(\"flatpak\")\n            .args([\"info\", install_arg, app_id])\n            .stdout(Stdio::piped())\n            .stderr(Stdio::piped())\n            .output()\n            .await\n            .context(\"Failed to get flatpak info\")?;\n\n        let info_str = String::from_utf8_lossy(&info_output.stdout);\n        Self::parse_info(&info_str, &mut metadata);\n\n        Ok(metadata)\n    }\n\n    /// Parse the metadata from flatpak info --show-metadata output\n    fn parse_metadata(content: &str, app_id: &str) -> FlatpakMetadata {\n        let mut metadata = FlatpakMetadata {\n            app_id: app_id.to_string(),\n            ..Default::default()\n        };\n\n        let mut current_section = String::new();\n\n        for line in content.lines() {\n            let line = line.trim();\n\n            // Section headers\n            if line.starts_with('[') && line.ends_with(']') {\n                current_section = line[1..line.len() - 1].to_string();\n                continue;\n            }\n\n            // Key-value pairs\n            if let Some((key, value)) = line.split_once('=') {\n                let key = key.trim();\n                let value = value.trim();\n\n                match current_section.as_str() {\n                    \"Application\" | \"Runtime\" => match key {\n                        \"runtime\" => {\n                            if let Some(rt) = Self::parse_runtime_ref(value) {\n                                metadata.runtime = Some(rt);\n                            }\n                        }\n                        \"sdk\" => metadata.sdk = Some(value.to_string()),\n                        _ => {}\n                    },\n                    \"Context\" => {\n                        Self::parse_context_permissions(key, value, &mut metadata.permissions);\n                    }\n                    \"Session Bus Policy\" => {\n                        Self::parse_dbus_permissions(\n                            key,\n                            value,\n                            PermissionCategory::SessionBus,\n                            &mut metadata.permissions,\n                        );\n                    }\n                    \"System Bus Policy\" => {\n                        Self::parse_dbus_permissions(\n                            key,\n                            value,\n                            PermissionCategory::SystemBus,\n                            &mut metadata.permissions,\n                        );\n                    }\n                    \"Environment\" => {\n                        metadata.permissions.push(FlatpakPermission::from_raw(\n                            PermissionCategory::Environment,\n                            &format!(\"{}={}\", key, value),\n                        ));\n                    }\n                    \"Extension\" => {\n                        // Track extensions\n                        if key.starts_with(\"Extension \") {\n                            let ext_name = key.strip_prefix(\"Extension \").unwrap_or(key);\n                            metadata.extensions.push(ext_name.to_string());\n                        }\n                    }\n                    _ => {}\n                }\n            }\n        }\n\n        metadata\n    }\n\n    /// Parse runtime reference string\n    fn parse_runtime_ref(runtime_ref: &str) -> Option<FlatpakRuntime> {\n        // Format: org.gnome.Platform/x86_64/45 or org.gnome.Platform//45\n        let parts: Vec<&str> = runtime_ref.split('/').collect();\n        if parts.len() >= 3 {\n            Some(FlatpakRuntime {\n                id: parts[0].to_string(),\n                version: parts.get(2).unwrap_or(&\"\").to_string(),\n                branch: parts.get(3).unwrap_or(&\"stable\").to_string(),\n            })\n        } else if parts.len() == 2 {\n            Some(FlatpakRuntime {\n                id: parts[0].to_string(),\n                version: parts[1].to_string(),\n                branch: \"stable\".to_string(),\n            })\n        } else {\n            None\n        }\n    }\n\n    /// Parse Context section permissions\n    fn parse_context_permissions(key: &str, value: &str, permissions: &mut Vec<FlatpakPermission>) {\n        let category = match key {\n            \"filesystems\" => PermissionCategory::Filesystem,\n            \"sockets\" => PermissionCategory::Socket,\n            \"devices\" => PermissionCategory::Device,\n            \"shared\" => PermissionCategory::Share,\n            \"features\" => PermissionCategory::Other,\n            \"persistent\" => PermissionCategory::Filesystem,\n            _ => return,\n        };\n\n        // Values are semicolon-separated\n        for item in value.split(';') {\n            let item = item.trim();\n            if !item.is_empty() {\n                permissions.push(FlatpakPermission::from_raw(category, item));\n            }\n        }\n    }\n\n    /// Parse D-Bus permissions\n    fn parse_dbus_permissions(\n        bus_name: &str,\n        access: &str,\n        category: PermissionCategory,\n        permissions: &mut Vec<FlatpakPermission>,\n    ) {\n        // Access can be: talk, own, see, none\n        let perm_str = match access.trim() {\n            \"none\" => format!(\"!{}\", bus_name),\n            \"talk\" => format!(\"{} (talk)\", bus_name),\n            \"own\" => format!(\"{} (own)\", bus_name),\n            \"see\" => format!(\"{} (see)\", bus_name),\n            _ => format!(\"{} ({})\", bus_name, access),\n        };\n        permissions.push(FlatpakPermission::from_raw(category, &perm_str));\n    }\n\n    /// Parse additional info from flatpak info output\n    fn parse_info(content: &str, metadata: &mut FlatpakMetadata) {\n        for line in content.lines() {\n            let line = line.trim();\n            if let Some((key, value)) = line.split_once(':') {\n                let key = key.trim();\n                let value = value.trim();\n\n                match key {\n                    \"Ref\" | \"ID\" => {\n                        if metadata.app_id.is_empty() {\n                            metadata.app_id = value.to_string();\n                        }\n                    }\n                    \"Origin\" => metadata.remote = Some(value.to_string()),\n                    \"Commit\" => metadata.commit = Some(value.to_string()),\n                    \"Installation\" => {\n                        metadata.installation = if value.to_lowercase().contains(\"system\") {\n                            InstallationType::System\n                        } else {\n                            InstallationType::User\n                        };\n                    }\n                    \"Arch\" => metadata.arch = Some(value.to_string()),\n                    \"Branch\" => metadata.branch = Some(value.to_string()),\n                    \"End-of-life\" | \"EOL\" => {\n                        metadata.is_eol = true;\n                        if !value.is_empty() && value != \"yes\" && value != \"true\" {\n                            metadata.eol_reason = Some(value.to_string());\n                        }\n                    }\n                    _ => {}\n                }\n            }\n        }\n    }\n\n    /// List all runtimes installed on the system\n    pub async fn list_runtimes(&self) -> Result<Vec<Package>> {\n        let output = Command::new(\"flatpak\")\n            .args([\n                \"list\",\n                \"--runtime\",\n                \"--columns=application,version,name,size,origin\",\n            ])\n            .stdout(Stdio::piped())\n            .stderr(Stdio::piped())\n            .output()\n            .await\n            .context(\"Failed to list flatpak runtimes\")?;\n\n        let stdout = String::from_utf8_lossy(&output.stdout);\n        let mut packages = Vec::new();\n\n        for line in stdout.lines() {\n            let parts: Vec<&str> = line.split('\\t').collect();\n            if parts.len() >= 3 {\n                let app_id = parts[0].to_string();\n                let version = parts[1].to_string();\n                let name = parts[2].to_string();\n                let size = parts.get(3).and_then(|s| parse_human_size(s));\n\n                packages.push(Package {\n                    name: app_id,\n                    version,\n                    available_version: None,\n                    description: format!(\"Runtime: {}\", name),\n                    source: PackageSource::Flatpak,\n                    status: PackageStatus::Installed,\n                    size,\n                    homepage: None,\n                    license: None,\n                    maintainer: None,\n                    dependencies: Vec::new(),\n                    install_date: None,\n                    enrichment: None,\n                });\n            }\n        }\n\n        Ok(packages)\n    }\n\n    /// Get permissions override for an application\n    pub async fn get_overrides(&self, app_id: &str) -> Result<Vec<FlatpakPermission>> {\n        let output = Command::new(\"flatpak\")\n            .args([\"override\", \"--show\", app_id])\n            .stdout(Stdio::piped())\n            .stderr(Stdio::piped())\n            .output()\n            .await\n            .context(\"Failed to get flatpak overrides\")?;\n\n        let override_str = String::from_utf8_lossy(&output.stdout);\n        let metadata = Self::parse_metadata(&override_str, app_id);\n        Ok(metadata.permissions)\n    }\n\n    /// Add a permission override for an application\n    pub async fn add_override(&self, app_id: &str, permission: &str) -> Result<()> {\n        let status = Command::new(\"flatpak\")\n            .args([\"override\", \"--user\", permission, app_id])\n            .status()\n            .await\n            .context(\"Failed to add flatpak override\")?;\n\n        if status.success() {\n            Ok(())\n        } else {\n            anyhow::bail!(\"Failed to add override {} for {}\", permission, app_id)\n        }\n    }\n\n    /// Remove a permission override for an application\n    pub async fn remove_override(&self, app_id: &str, permission: &str) -> Result<()> {\n        // To remove, we need to use the negated version\n        let neg_perm = if permission.starts_with(\"--\") {\n            permission.replacen(\"--\", \"--no\", 1)\n        } else {\n            format!(\"--no{}\", permission.trim_start_matches('-'))\n        };\n\n        let status = Command::new(\"flatpak\")\n            .args([\"override\", \"--user\", &neg_perm, app_id])\n            .status()\n            .await\n            .context(\"Failed to remove flatpak override\")?;\n\n        if status.success() {\n            Ok(())\n        } else {\n            anyhow::bail!(\"Failed to remove override {} for {}\", permission, app_id)\n        }\n    }\n\n    /// Reset all overrides for an application\n    pub async fn reset_overrides(&self, app_id: &str) -> Result<()> {\n        let status = Command::new(\"flatpak\")\n            .args([\"override\", \"--user\", \"--reset\", app_id])\n            .status()\n            .await\n            .context(\"Failed to reset flatpak overrides\")?;\n\n        if status.success() {\n            Ok(())\n        } else {\n            anyhow::bail!(\"Failed to reset overrides for {}\", app_id)\n        }\n    }\n\n    /// Check if an application is sandboxed (has limited permissions)\n    pub async fn is_well_sandboxed(&self, app_id: &str) -> Result<bool> {\n        let metadata = self.get_metadata(app_id).await?;\n        let summary = metadata.sandbox_summary();\n        Ok(matches!(\n            summary.rating,\n            crate::models::SandboxRating::Strong | crate::models::SandboxRating::Good\n        ))\n    }\n}\n\nimpl Default for FlatpakBackend {\n    fn default() -> Self {\n        Self::new()\n    }\n}"
}

Now let me update the todo list and add the PackageManager method for getting Flatpak metadata:

🔧 Tool: TodoWrite
Input: {
  "todos": [
    {
      "content": "Add FlatpakMetadata struct to models for sandbox permissions",
      "status": "completed",
      "activeForm": "Adding FlatpakMetadata struct to models"
    },
    {
      "content": "Enhance Flatpak backend with sandbox permissions detection",
      "status": "completed",
      "activeForm": "Enhancing Flatpak backend with sandbox detection"
    },
    {
      "content": "Add method to get app permissions and runtime info",
      "status": "completed",
      "activeForm": "Adding method for app permissions"
    },
    {
      "content": "Update Package model to include Flatpak-specific metadata",
      "status": "in_progress",
      "activeForm": "Updating Package model"
    },
    {
      "content": "Add CLI command to show Flatpak permissions",
      "status": "pending",
      "activeForm": "Adding CLI command for permissions"
    },
    {
      "content": "Update UI to display sandbox information",
      "status": "pending",
      "activeForm": "Updating UI for sandbox display"
    },
    {
      "content": "Add tests for Flatpak sandbox functionality",
      "status": "pending",
      "activeForm": "Adding tests for Flatpak sandbox"
    }
  ]
}

Now let me add a method to the PackageManager to get Flatpak metadata:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/mod.rs",
  "old_string": "use crate::models::{Package, PackageSource, Repository};\nuse anyhow::Result;\nuse std::collections::{HashMap, HashSet};",
  "new_string": "use crate::models::{FlatpakMetadata, FlatpakPermission, Package, PackageSource, Repository};\nuse anyhow::Result;\nuse std::collections::{HashMap, HashSet};"
}

Now let me add the Flatpak-specific methods to PackageManager:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/mod.rs",
  "offset": 350
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/mod.rs",
  "old_string": "        all_results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));\n        Ok(all_results)\n    }\n}\n\nimpl Default for PackageManager {\n    fn default() -> Self {\n        Self::new()\n    }\n}",
  "new_string": "        all_results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));\n        Ok(all_results)\n    }\n\n    // =========================================================================\n    // Flatpak-specific methods for sandbox management\n    // =========================================================================\n\n    /// Get detailed Flatpak metadata including sandbox permissions for an application\n    pub async fn get_flatpak_metadata(&self, app_id: &str) -> Result<FlatpakMetadata> {\n        if !self.backends.contains_key(&PackageSource::Flatpak) {\n            anyhow::bail!(\"Flatpak backend is not available\");\n        }\n\n        let backend = FlatpakBackend::new();\n        backend.get_metadata(app_id).await\n    }\n\n    /// Get the permission overrides for a Flatpak application\n    pub async fn get_flatpak_overrides(&self, app_id: &str) -> Result<Vec<FlatpakPermission>> {\n        if !self.backends.contains_key(&PackageSource::Flatpak) {\n            anyhow::bail!(\"Flatpak backend is not available\");\n        }\n\n        let backend = FlatpakBackend::new();\n        backend.get_overrides(app_id).await\n    }\n\n    /// Add a permission override for a Flatpak application\n    pub async fn add_flatpak_override(&self, app_id: &str, permission: &str) -> Result<()> {\n        if !self.backends.contains_key(&PackageSource::Flatpak) {\n            anyhow::bail!(\"Flatpak backend is not available\");\n        }\n\n        let backend = FlatpakBackend::new();\n        backend.add_override(app_id, permission).await\n    }\n\n    /// Remove a permission override for a Flatpak application\n    pub async fn remove_flatpak_override(&self, app_id: &str, permission: &str) -> Result<()> {\n        if !self.backends.contains_key(&PackageSource::Flatpak) {\n            anyhow::bail!(\"Flatpak backend is not available\");\n        }\n\n        let backend = FlatpakBackend::new();\n        backend.remove_override(app_id, permission).await\n    }\n\n    /// Reset all overrides for a Flatpak application\n    pub async fn reset_flatpak_overrides(&self, app_id: &str) -> Result<()> {\n        if !self.backends.contains_key(&PackageSource::Flatpak) {\n            anyhow::bail!(\"Flatpak backend is not available\");\n        }\n\n        let backend = FlatpakBackend::new();\n        backend.reset_overrides(app_id).await\n    }\n\n    /// List all Flatpak runtimes installed on the system\n    pub async fn list_flatpak_runtimes(&self) -> Result<Vec<Package>> {\n        if !self.backends.contains_key(&PackageSource::Flatpak) {\n            anyhow::bail!(\"Flatpak backend is not available\");\n        }\n\n        let backend = FlatpakBackend::new();\n        backend.list_runtimes().await\n    }\n\n    /// Check if a Flatpak application is well sandboxed\n    pub async fn is_flatpak_well_sandboxed(&self, app_id: &str) -> Result<bool> {\n        if !self.backends.contains_key(&PackageSource::Flatpak) {\n            anyhow::bail!(\"Flatpak backend is not available\");\n        }\n\n        let backend = FlatpakBackend::new();\n        backend.is_well_sandboxed(app_id).await\n    }\n}\n\nimpl Default for PackageManager {\n    fn default() -> Self {\n        Self::new()\n    }\n}"
}

Now let me update the todo list and add a CLI command for showing Flatpak permissions. First, let me check the existing CLI structure:

🔧 Tool: TodoWrite
Input: {
  "todos": [
    {
      "content": "Add FlatpakMetadata struct to models for sandbox permissions",
      "status": "completed",
      "activeForm": "Adding FlatpakMetadata struct to models"
    },
    {
      "content": "Enhance Flatpak backend with sandbox permissions detection",
      "status": "completed",
      "activeForm": "Enhancing Flatpak backend with sandbox detection"
    },
    {
      "content": "Add method to get app permissions and runtime info",
      "status": "completed",
      "activeForm": "Adding method for app permissions"
    },
    {
      "content": "Update Package model to include Flatpak-specific metadata",
      "status": "completed",
      "activeForm": "Updating Package model"
    },
    {
      "content": "Add CLI command to show Flatpak permissions",
      "status": "in_progress",
      "activeForm": "Adding CLI command for permissions"
    },
    {
      "content": "Update UI to display sandbox information",
      "status": "pending",
      "activeForm": "Updating UI for sandbox display"
    },
    {
      "content": "Add tests for Flatpak sandbox functionality",
      "status": "pending",
      "activeForm": "Adding tests for Flatpak sandbox"
    }
  ]
}

🔧 Tool: Glob
Input: {
  "pattern": "**/cli/**/*.rs",
  "path": "/home/eslam/Storage/Code/LinGet/src"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/mod.rs"
}

Now let me look at how the `info` command works as a reference, then create a new `permissions` command:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/info.rs"
}

Now let me create the `permissions` command for Flatpak:

🔧 Tool: Write
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/permissions.rs",
  "content": "use crate::backend::PackageManager;\nuse crate::cli::OutputWriter;\nuse crate::models::{PermissionCategory, PrivacyLevel, SandboxRating};\nuse anyhow::{bail, Result};\nuse console::style;\nuse indicatif::{ProgressBar, ProgressStyle};\nuse std::sync::Arc;\nuse tokio::sync::Mutex;\n\n/// Subcommand actions for permissions\npub enum PermissionsAction {\n    /// Show permissions for an app\n    Show,\n    /// Show only override permissions\n    Overrides,\n    /// Reset all overrides for an app\n    Reset,\n}\n\npub async fn run(\n    pm: Arc<Mutex<PackageManager>>,\n    app_id: &str,\n    action: PermissionsAction,\n    writer: &OutputWriter,\n) -> Result<()> {\n    let spinner = if !writer.is_quiet() && !writer.is_json() {\n        let pb = ProgressBar::new_spinner();\n        pb.set_style(\n            ProgressStyle::default_spinner()\n                .template(\"{spinner:.cyan} {msg}\")\n                .unwrap(),\n        );\n        pb.set_message(format!(\"Fetching permissions for {}...\", app_id));\n        pb.enable_steady_tick(std::time::Duration::from_millis(80));\n        Some(pb)\n    } else {\n        None\n    };\n\n    let manager = pm.lock().await;\n\n    match action {\n        PermissionsAction::Show => {\n            let metadata = manager.get_flatpak_metadata(app_id).await?;\n\n            if let Some(pb) = spinner {\n                pb.finish_and_clear();\n            }\n\n            if writer.is_json() {\n                let json = serde_json::to_string_pretty(&metadata)?;\n                println!(\"{}\", json);\n                return Ok(());\n            }\n\n            // Print header\n            println!(\n                \"\\n{} {}\",\n                style(\"Flatpak Permissions:\").bold().cyan(),\n                style(&metadata.app_id).bold()\n            );\n            println!(\"{}\", style(\"─\".repeat(60)).dim());\n\n            // Print sandbox summary\n            let summary = metadata.sandbox_summary();\n            let rating_icon = match summary.rating {\n                SandboxRating::Strong => style(\"●\").green(),\n                SandboxRating::Good => style(\"●\").cyan(),\n                SandboxRating::Moderate => style(\"●\").yellow(),\n                SandboxRating::Weak => style(\"●\").red(),\n            };\n            println!(\n                \"\\n{} {} {}\",\n                rating_icon,\n                style(\"Sandbox Rating:\").bold(),\n                style(format!(\"{}\", summary.rating)).bold()\n            );\n            println!(\"  {}\", style(&summary.description).dim());\n            println!(\n                \"  {} total permissions, {} high-risk\",\n                summary.total_permissions, summary.high_risk_count\n            );\n\n            // Print runtime info\n            if let Some(ref runtime) = metadata.runtime {\n                println!(\n                    \"\\n{} {}\",\n                    style(\"Runtime:\").bold(),\n                    style(runtime.to_string()).dim()\n                );\n            }\n\n            // Print installation info\n            println!(\n                \"{} {}\",\n                style(\"Installation:\").bold(),\n                style(format!(\"{}\", metadata.installation)).dim()\n            );\n\n            if let Some(ref remote) = metadata.remote {\n                println!(\n                    \"{} {}\",\n                    style(\"Remote:\").bold(),\n                    style(remote).dim()\n                );\n            }\n\n            if metadata.is_eol {\n                println!(\n                    \"\\n{} {}\",\n                    style(\"WARNING:\").bold().red(),\n                    style(\"This application is end-of-life\").red()\n                );\n                if let Some(ref reason) = metadata.eol_reason {\n                    println!(\"  {}\", style(reason).dim());\n                }\n            }\n\n            // Print permissions by category\n            let grouped = metadata.permissions_by_category();\n            if grouped.is_empty() {\n                println!(\n                    \"\\n{}\",\n                    style(\"No special permissions required\").green()\n                );\n            } else {\n                println!(\"\\n{}\", style(\"Permissions:\").bold());\n\n                for (category, permissions) in grouped {\n                    println!(\n                        \"\\n  {} {}\",\n                        style(\"▸\").cyan(),\n                        style(category.description()).bold()\n                    );\n\n                    for perm in permissions {\n                        let level_indicator = match perm.privacy_level {\n                            PrivacyLevel::Low => style(\"○\").dim(),\n                            PrivacyLevel::Medium => style(\"◐\").yellow(),\n                            PrivacyLevel::High => style(\"●\").red(),\n                        };\n\n                        let value_style = if perm.negated {\n                            style(&perm.value).strikethrough().dim()\n                        } else {\n                            style(&perm.value).white()\n                        };\n\n                        println!(\"    {} {} - {}\", level_indicator, value_style, style(&perm.description).dim());\n                    }\n                }\n            }\n\n            // Print legend\n            println!(\"\\n{}\", style(\"─\".repeat(60)).dim());\n            println!(\n                \"{} {} Low  {} Medium  {} High risk\",\n                style(\"Legend:\").dim(),\n                style(\"○\").dim(),\n                style(\"◐\").yellow(),\n                style(\"●\").red()\n            );\n        }\n\n        PermissionsAction::Overrides => {\n            let overrides = manager.get_flatpak_overrides(app_id).await?;\n\n            if let Some(pb) = spinner {\n                pb.finish_and_clear();\n            }\n\n            if writer.is_json() {\n                let json = serde_json::to_string_pretty(&overrides)?;\n                println!(\"{}\", json);\n                return Ok(());\n            }\n\n            println!(\n                \"\\n{} {}\",\n                style(\"Permission Overrides:\").bold().cyan(),\n                style(app_id).bold()\n            );\n            println!(\"{}\", style(\"─\".repeat(60)).dim());\n\n            if overrides.is_empty() {\n                println!(\"\\n{}\", style(\"No permission overrides set\").dim());\n            } else {\n                for perm in &overrides {\n                    let prefix = if perm.negated {\n                        style(\"- DENIED\").red()\n                    } else {\n                        style(\"+ ALLOWED\").green()\n                    };\n                    println!(\n                        \"  {} {} ({})\",\n                        prefix,\n                        style(&perm.value).bold(),\n                        perm.category.description()\n                    );\n                }\n            }\n\n            println!(\"\\n{}\", style(\"Tip: Use 'flatpak override' to modify permissions\").dim());\n        }\n\n        PermissionsAction::Reset => {\n            manager.reset_flatpak_overrides(app_id).await?;\n\n            if let Some(pb) = spinner {\n                pb.finish_and_clear();\n            }\n\n            if !writer.is_quiet() {\n                writer.success(&format!(\"Reset all permission overrides for {}\", app_id));\n            }\n        }\n    }\n\n    Ok(())\n}\n\n/// Show a quick sandbox summary for a Flatpak app\npub async fn show_sandbox_summary(\n    pm: Arc<Mutex<PackageManager>>,\n    app_id: &str,\n    writer: &OutputWriter,\n) -> Result<()> {\n    let manager = pm.lock().await;\n    let metadata = manager.get_flatpak_metadata(app_id).await?;\n    let summary = metadata.sandbox_summary();\n\n    if writer.is_json() {\n        let json = serde_json::to_string_pretty(&summary)?;\n        println!(\"{}\", json);\n        return Ok(());\n    }\n\n    let rating_color = match summary.rating {\n        SandboxRating::Strong => style(format!(\"{}\", summary.rating)).green(),\n        SandboxRating::Good => style(format!(\"{}\", summary.rating)).cyan(),\n        SandboxRating::Moderate => style(format!(\"{}\", summary.rating)).yellow(),\n        SandboxRating::Weak => style(format!(\"{}\", summary.rating)).red(),\n    };\n\n    println!(\n        \"{}: {} - {}\",\n        style(\"Sandbox\").bold(),\n        rating_color,\n        style(&summary.description).dim()\n    );\n\n    Ok(())\n}\n\n/// List all Flatpak runtimes\npub async fn list_runtimes(\n    pm: Arc<Mutex<PackageManager>>,\n    writer: &OutputWriter,\n) -> Result<()> {\n    let spinner = if !writer.is_quiet() && !writer.is_json() {\n        let pb = ProgressBar::new_spinner();\n        pb.set_style(\n            ProgressStyle::default_spinner()\n                .template(\"{spinner:.cyan} {msg}\")\n                .unwrap(),\n        );\n        pb.set_message(\"Listing Flatpak runtimes...\");\n        pb.enable_steady_tick(std::time::Duration::from_millis(80));\n        Some(pb)\n    } else {\n        None\n    };\n\n    let manager = pm.lock().await;\n    let runtimes = manager.list_flatpak_runtimes().await?;\n\n    if let Some(pb) = spinner {\n        pb.finish_and_clear();\n    }\n\n    if writer.is_json() {\n        let json = serde_json::to_string_pretty(&runtimes)?;\n        println!(\"{}\", json);\n        return Ok(());\n    }\n\n    println!(\n        \"\\n{} ({} runtimes)\",\n        style(\"Flatpak Runtimes\").bold().cyan(),\n        runtimes.len()\n    );\n    println!(\"{}\", style(\"─\".repeat(60)).dim());\n\n    for runtime in &runtimes {\n        println!(\n            \"  {} {} {}\",\n            style(\"•\").dim(),\n            style(&runtime.name).bold(),\n            style(format!(\"v{}\", runtime.version)).dim()\n        );\n        if !runtime.description.is_empty() {\n            println!(\"    {}\", style(&runtime.description).dim());\n        }\n    }\n\n    Ok(())\n}\n"
}

Now let me update the CLI commands module to include the permissions command:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/mod.rs",
  "old_string": "pub mod check;\npub mod completions;\npub mod info;\npub mod install;\npub mod list;\npub mod providers;\npub mod remove;\npub mod search;\npub mod sources;\npub mod update;",
  "new_string": "pub mod check;\npub mod completions;\npub mod info;\npub mod install;\npub mod list;\npub mod permissions;\npub mod providers;\npub mod remove;\npub mod search;\npub mod sources;\npub mod update;"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/mod.rs"
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/mod.rs",
  "old_string": "pub mod check;\npub mod completions;\npub mod info;\npub mod install;\npub mod list;\npub mod providers;\npub mod remove;\npub mod search;\npub mod sources;\npub mod update;",
  "new_string": "pub mod check;\npub mod completions;\npub mod info;\npub mod install;\npub mod list;\npub mod permissions;\npub mod providers;\npub mod remove;\npub mod search;\npub mod sources;\npub mod update;"
}

Now let me update the main CLI file to add the permissions command:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs",
  "old_string": "    /// Generate shell completions\n    Completions {\n        /// Shell to generate completions for\n        shell: clap_complete::Shell,\n    },\n\n    /// Launch interactive TUI mode\n    Tui,\n\n    /// Launch graphical user interface (default when no command given)\n    Gui,\n}",
  "new_string": "    /// Generate shell completions\n    Completions {\n        /// Shell to generate completions for\n        shell: clap_complete::Shell,\n    },\n\n    /// Show Flatpak sandbox permissions for an application\n    Permissions {\n        /// Flatpak application ID\n        app_id: String,\n\n        /// Show only overrides\n        #[arg(short, long)]\n        overrides: bool,\n\n        /// Reset all overrides for this app\n        #[arg(long)]\n        reset: bool,\n    },\n\n    /// List Flatpak runtimes installed on the system\n    Runtimes,\n\n    /// Launch interactive TUI mode\n    Tui,\n\n    /// Launch graphical user interface (default when no command given)\n    Gui,\n}"
}

Now let me add the handlers for these new commands in the run function:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs",
  "old_string": "        Commands::Completions { shell } => {\n            commands::completions::run(shell);\n            Ok(())\n        }\n        Commands::Tui => tui::run().await,\n        Commands::Gui => {\n            // This is handled in main.rs - should not reach here\n            unreachable!(\"GUI command should be handled in main.rs\")\n        }\n    }\n}",
  "new_string": "        Commands::Completions { shell } => {\n            commands::completions::run(shell);\n            Ok(())\n        }\n        Commands::Permissions {\n            app_id,\n            overrides,\n            reset,\n        } => {\n            let action = if reset {\n                commands::permissions::PermissionsAction::Reset\n            } else if overrides {\n                commands::permissions::PermissionsAction::Overrides\n            } else {\n                commands::permissions::PermissionsAction::Show\n            };\n            commands::permissions::run(pm, &app_id, action, &writer).await\n        }\n        Commands::Runtimes => commands::permissions::list_runtimes(pm, &writer).await,\n        Commands::Tui => tui::run().await,\n        Commands::Gui => {\n            // This is handled in main.rs - should not reach here\n            unreachable!(\"GUI command should be handled in main.rs\")\n        }\n    }\n}"
}

Now let me update the todo list and add UI support for displaying sandbox information:

🔧 Tool: TodoWrite
Input: {
  "todos": [
    {
      "content": "Add FlatpakMetadata struct to models for sandbox permissions",
      "status": "completed",
      "activeForm": "Adding FlatpakMetadata struct to models"
    },
    {
      "content": "Enhance Flatpak backend with sandbox permissions detection",
      "status": "completed",
      "activeForm": "Enhancing Flatpak backend with sandbox detection"
    },
    {
      "content": "Add method to get app permissions and runtime info",
      "status": "completed",
      "activeForm": "Adding method for app permissions"
    },
    {
      "content": "Update Package model to include Flatpak-specific metadata",
      "status": "completed",
      "activeForm": "Updating Package model"
    },
    {
      "content": "Add CLI command to show Flatpak permissions",
      "status": "completed",
      "activeForm": "Adding CLI command for permissions"
    },
    {
      "content": "Update UI to display sandbox information",
      "status": "in_progress",
      "activeForm": "Updating UI for sandbox display"
    },
    {
      "content": "Add tests for Flatpak sandbox functionality",
      "status": "pending",
      "activeForm": "Adding tests for Flatpak sandbox"
    }
  ]
}

Now let me add a UI component for displaying Flatpak sandbox permissions. First, let me check the structure of the package details enrichment section:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/enrichment.rs"
}

Now let me create a new file for the Flatpak sandbox UI component:

🔧 Tool: Write
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/sandbox.rs",
  "content": "use crate::backend::PackageManager;\nuse crate::models::{\n    FlatpakMetadata, PermissionCategory, PrivacyLevel, SandboxRating, SandboxSummary,\n};\nuse gtk4::prelude::*;\nuse gtk4::{self as gtk, glib};\nuse libadwaita as adw;\nuse libadwaita::prelude::*;\nuse std::sync::Arc;\nuse tokio::sync::Mutex;\n\n/// Build the sandbox permissions section for Flatpak packages\npub fn build_sandbox_section(\n    pm: Arc<Mutex<PackageManager>>,\n    app_id: String,\n) -> gtk::Box {\n    let section = gtk::Box::builder()\n        .orientation(gtk::Orientation::Vertical)\n        .spacing(12)\n        .margin_top(12)\n        .build();\n    section.add_css_class(\"sandbox-section\");\n\n    // Loading state\n    let loading_box = gtk::Box::builder()\n        .orientation(gtk::Orientation::Horizontal)\n        .spacing(8)\n        .halign(gtk::Align::Center)\n        .margin_top(8)\n        .margin_bottom(8)\n        .build();\n    let spinner = gtk::Spinner::builder().spinning(true).build();\n    let loading_label = gtk::Label::new(Some(\"Loading sandbox info...\"));\n    loading_label.add_css_class(\"dim-label\");\n    loading_label.add_css_class(\"caption\");\n    loading_box.append(&spinner);\n    loading_box.append(&loading_label);\n    section.append(&loading_box);\n\n    // Fetch metadata asynchronously\n    let section_clone = section.clone();\n    glib::spawn_future_local(async move {\n        let manager = pm.lock().await;\n        let metadata_result = manager.get_flatpak_metadata(&app_id).await;\n        drop(manager);\n\n        // Clear loading state\n        while let Some(child) = section_clone.first_child() {\n            section_clone.remove(&child);\n        }\n\n        match metadata_result {\n            Ok(metadata) => {\n                build_sandbox_content(&section_clone, &metadata);\n            }\n            Err(e) => {\n                let error_label = gtk::Label::builder()\n                    .label(format!(\"Could not load sandbox info: {}\", e))\n                    .wrap(true)\n                    .xalign(0.0)\n                    .build();\n                error_label.add_css_class(\"dim-label\");\n                section_clone.append(&error_label);\n            }\n        }\n    });\n\n    section\n}\n\nfn build_sandbox_content(section: &gtk::Box, metadata: &FlatpakMetadata) {\n    let summary = metadata.sandbox_summary();\n\n    // Sandbox rating header\n    let rating_box = build_rating_box(&summary);\n    section.append(&rating_box);\n\n    // Runtime info\n    if let Some(ref runtime) = metadata.runtime {\n        let runtime_row = adw::ActionRow::builder()\n            .title(\"Runtime\")\n            .subtitle(&runtime.to_string())\n            .build();\n        runtime_row.add_css_class(\"property\");\n\n        let runtime_icon = gtk::Image::builder()\n            .icon_name(\"application-x-executable-symbolic\")\n            .build();\n        runtime_icon.add_css_class(\"dim-label\");\n        runtime_row.add_prefix(&runtime_icon);\n\n        section.append(&runtime_row);\n    }\n\n    // Installation info\n    let install_row = adw::ActionRow::builder()\n        .title(\"Installation\")\n        .subtitle(&format!(\"{}\", metadata.installation))\n        .build();\n    install_row.add_css_class(\"property\");\n\n    let install_icon = gtk::Image::builder()\n        .icon_name(if metadata.installation == crate::models::InstallationType::System {\n            \"computer-symbolic\"\n        } else {\n            \"user-home-symbolic\"\n        })\n        .build();\n    install_icon.add_css_class(\"dim-label\");\n    install_row.add_prefix(&install_icon);\n    section.append(&install_row);\n\n    // EOL warning if applicable\n    if metadata.is_eol {\n        let warning_box = gtk::Box::builder()\n            .orientation(gtk::Orientation::Horizontal)\n            .spacing(8)\n            .margin_top(8)\n            .build();\n        warning_box.add_css_class(\"warning-bar\");\n\n        let warning_icon = gtk::Image::builder()\n            .icon_name(\"dialog-warning-symbolic\")\n            .build();\n        warning_icon.add_css_class(\"error\");\n\n        let warning_label = gtk::Label::builder()\n            .label(if let Some(ref reason) = metadata.eol_reason {\n                format!(\"End of Life: {}\", reason)\n            } else {\n                \"This application has reached end of life\".to_string()\n            })\n            .wrap(true)\n            .xalign(0.0)\n            .build();\n        warning_label.add_css_class(\"error\");\n\n        warning_box.append(&warning_icon);\n        warning_box.append(&warning_label);\n        section.append(&warning_box);\n    }\n\n    // Permissions expander\n    if !metadata.permissions.is_empty() {\n        let permissions_expander = build_permissions_expander(metadata);\n        section.append(&permissions_expander);\n    } else {\n        let no_perms_label = gtk::Label::builder()\n            .label(\"No special permissions required\")\n            .xalign(0.0)\n            .margin_top(8)\n            .build();\n        no_perms_label.add_css_class(\"success\");\n        no_perms_label.add_css_class(\"caption\");\n        section.append(&no_perms_label);\n    }\n}\n\nfn build_rating_box(summary: &SandboxSummary) -> gtk::Box {\n    let rating_box = gtk::Box::builder()\n        .orientation(gtk::Orientation::Horizontal)\n        .spacing(12)\n        .build();\n    rating_box.add_css_class(\"sandbox-rating-box\");\n\n    // Rating icon\n    let (icon_name, css_class) = match summary.rating {\n        SandboxRating::Strong => (\"emblem-ok-symbolic\", \"success\"),\n        SandboxRating::Good => (\"emblem-default-symbolic\", \"accent\"),\n        SandboxRating::Moderate => (\"dialog-warning-symbolic\", \"warning\"),\n        SandboxRating::Weak => (\"dialog-error-symbolic\", \"error\"),\n    };\n\n    let rating_icon = gtk::Image::builder()\n        .icon_name(icon_name)\n        .pixel_size(32)\n        .build();\n    rating_icon.add_css_class(css_class);\n\n    // Rating text\n    let text_box = gtk::Box::builder()\n        .orientation(gtk::Orientation::Vertical)\n        .spacing(2)\n        .valign(gtk::Align::Center)\n        .build();\n\n    let rating_label = gtk::Label::builder()\n        .label(format!(\"Sandbox: {}\", summary.rating))\n        .xalign(0.0)\n        .build();\n    rating_label.add_css_class(\"heading\");\n    rating_label.add_css_class(css_class);\n\n    let desc_label = gtk::Label::builder()\n        .label(&summary.description)\n        .xalign(0.0)\n        .wrap(true)\n        .build();\n    desc_label.add_css_class(\"caption\");\n    desc_label.add_css_class(\"dim-label\");\n\n    text_box.append(&rating_label);\n    text_box.append(&desc_label);\n\n    rating_box.append(&rating_icon);\n    rating_box.append(&text_box);\n\n    rating_box\n}\n\nfn build_permissions_expander(metadata: &FlatpakMetadata) -> gtk::Expander {\n    let expander = gtk::Expander::builder()\n        .label(format!(\n            \"Permissions ({} total, {} high-risk)\",\n            metadata.permissions.len(),\n            metadata\n                .permissions\n                .iter()\n                .filter(|p| p.privacy_level == PrivacyLevel::High && !p.negated)\n                .count()\n        ))\n        .expanded(false)\n        .margin_top(8)\n        .build();\n    expander.add_css_class(\"card\");\n\n    let content = gtk::Box::builder()\n        .orientation(gtk::Orientation::Vertical)\n        .spacing(8)\n        .margin_start(12)\n        .margin_end(12)\n        .margin_top(8)\n        .margin_bottom(12)\n        .build();\n\n    // Group permissions by category\n    let grouped = metadata.permissions_by_category();\n\n    for (category, permissions) in grouped {\n        let category_box = gtk::Box::builder()\n            .orientation(gtk::Orientation::Vertical)\n            .spacing(4)\n            .margin_top(4)\n            .build();\n\n        // Category header\n        let header = gtk::Box::builder()\n            .orientation(gtk::Orientation::Horizontal)\n            .spacing(8)\n            .build();\n\n        let cat_icon = gtk::Image::builder()\n            .icon_name(category.icon_name())\n            .build();\n        cat_icon.add_css_class(\"dim-label\");\n\n        let cat_label = gtk::Label::builder()\n            .label(category.description())\n            .xalign(0.0)\n            .build();\n        cat_label.add_css_class(\"caption\");\n        cat_label.add_css_class(\"heading\");\n\n        header.append(&cat_icon);\n        header.append(&cat_label);\n        category_box.append(&header);\n\n        // Permission items\n        for perm in permissions {\n            let perm_row = gtk::Box::builder()\n                .orientation(gtk::Orientation::Horizontal)\n                .spacing(8)\n                .margin_start(24)\n                .build();\n\n            // Privacy level indicator\n            let (level_icon, level_class) = match perm.privacy_level {\n                PrivacyLevel::Low => (\"●\", \"dim-label\"),\n                PrivacyLevel::Medium => (\"●\", \"warning\"),\n                PrivacyLevel::High => (\"●\", \"error\"),\n            };\n\n            let level_label = gtk::Label::new(Some(level_icon));\n            level_label.add_css_class(level_class);\n\n            // Permission value\n            let value_label = gtk::Label::builder()\n                .label(if perm.negated {\n                    format!(\"✗ {}\", perm.value)\n                } else {\n                    perm.value.clone()\n                })\n                .xalign(0.0)\n                .build();\n\n            if perm.negated {\n                value_label.add_css_class(\"dim-label\");\n            }\n\n            // Description tooltip\n            let desc_label = gtk::Label::builder()\n                .label(&perm.description)\n                .xalign(0.0)\n                .hexpand(true)\n                .ellipsize(gtk::pango::EllipsizeMode::End)\n                .build();\n            desc_label.add_css_class(\"dim-label\");\n            desc_label.add_css_class(\"caption\");\n\n            perm_row.append(&level_label);\n            perm_row.append(&value_label);\n            perm_row.append(&desc_label);\n            perm_row.set_tooltip_text(Some(&perm.description));\n\n            category_box.append(&perm_row);\n        }\n\n        content.append(&category_box);\n    }\n\n    // Legend\n    let legend = gtk::Box::builder()\n        .orientation(gtk::Orientation::Horizontal)\n        .spacing(16)\n        .margin_top(12)\n        .halign(gtk::Align::Center)\n        .build();\n\n    let low_box = create_legend_item(\"●\", \"dim-label\", \"Low\");\n    let med_box = create_legend_item(\"●\", \"warning\", \"Medium\");\n    let high_box = create_legend_item(\"●\", \"error\", \"High\");\n\n    legend.append(&low_box);\n    legend.append(&med_box);\n    legend.append(&high_box);\n    content.append(&legend);\n\n    expander.set_child(Some(&content));\n    expander\n}\n\nfn create_legend_item(icon: &str, class: &str, label: &str) -> gtk::Box {\n    let item = gtk::Box::builder()\n        .orientation(gtk::Orientation::Horizontal)\n        .spacing(4)\n        .build();\n\n    let icon_label = gtk::Label::new(Some(icon));\n    icon_label.add_css_class(class);\n\n    let text_label = gtk::Label::new(Some(label));\n    text_label.add_css_class(\"caption\");\n    text_label.add_css_class(\"dim-label\");\n\n    item.append(&icon_label);\n    item.append(&text_label);\n    item\n}\n\n/// Build a compact sandbox badge for the package row\npub fn build_sandbox_badge(rating: SandboxRating) -> gtk::Box {\n    let badge = gtk::Box::builder()\n        .orientation(gtk::Orientation::Horizontal)\n        .spacing(4)\n        .build();\n    badge.add_css_class(\"sandbox-badge\");\n\n    let (icon_name, css_class) = match rating {\n        SandboxRating::Strong => (\"security-high-symbolic\", \"success\"),\n        SandboxRating::Good => (\"security-medium-symbolic\", \"accent\"),\n        SandboxRating::Moderate => (\"security-medium-symbolic\", \"warning\"),\n        SandboxRating::Weak => (\"security-low-symbolic\", \"error\"),\n    };\n\n    let icon = gtk::Image::builder()\n        .icon_name(icon_name)\n        .pixel_size(12)\n        .build();\n    icon.add_css_class(css_class);\n\n    badge.append(&icon);\n    badge.set_tooltip_text(Some(&format!(\"Sandbox: {}\", rating)));\n\n    badge\n}\n"
}

Now let me update the package_details module to include the sandbox module:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/mod.rs",
  "old_string": "pub mod enrichment;\npub mod panel;\n\npub use panel::PackageDetailsPanel;",
  "new_string": "pub mod enrichment;\npub mod panel;\npub mod sandbox;\n\npub use panel::PackageDetailsPanel;\npub use sandbox::{build_sandbox_badge, build_sandbox_section};"
}

Now let me integrate the sandbox section into the package details panel for Flatpak packages. First, let me read the panel.rs to see where to add the sandbox section:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "offset": 200,
  "limit": 150
}

I need to add the sandbox section after the enrichment box and before the details group, but only for Flatpak packages. Let me find the right location and add the integration:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "old_string": "use crate::backend::PackageManager;\nuse crate::models::{fetch_enrichment, Config, Package, PackageSource, PackageStatus};\nuse crate::ui::package_details::enrichment;\nuse crate::ui::{CommandCenter, CommandEventKind, PackageOp, RetrySpec};",
  "new_string": "use crate::backend::PackageManager;\nuse crate::models::{fetch_enrichment, Config, Package, PackageSource, PackageStatus};\nuse crate::ui::package_details::{enrichment, sandbox};\nuse crate::ui::{CommandCenter, CommandEventKind, PackageOp, RetrySpec};"
}

Now let me add the sandbox section after the details group for Flatpak packages:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "old_string": "        ignore_row.add_suffix(&ignore_switch);\n        details_group.add(&ignore_row);\n\n        content.append(&details_group);\n\n        let changelog_expander = gtk::Expander::builder()\n            .label(\"Release History\")\n            .expanded(false)\n            .margin_top(8)\n            .build();\n        changelog_expander.add_css_class(\"card\");",
  "new_string": "        ignore_row.add_suffix(&ignore_switch);\n        details_group.add(&ignore_row);\n\n        content.append(&details_group);\n\n        // Add sandbox permissions section for Flatpak packages\n        if package.source == PackageSource::Flatpak {\n            let sandbox_section = sandbox::build_sandbox_section(pm.clone(), package.name.clone());\n            content.append(&sandbox_section);\n        }\n\n        let changelog_expander = gtk::Expander::builder()\n            .label(\"Release History\")\n            .expanded(false)\n            .margin_top(8)\n            .build();\n        changelog_expander.add_css_class(\"card\");"
}

Now let me add CSS styles for the sandbox-related UI components. First, let me read the existing styles:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/resources/style.css",
  "offset": 450
}

Now let me add sandbox-related CSS styles:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/resources/style.css",
  "old_string": "/* Improved dim-label contrast */\n.dim-label {\n    opacity: 0.65;\n}",
  "new_string": "/* Improved dim-label contrast */\n.dim-label {\n    opacity: 0.65;\n}\n\n/* ==========================================================================\n   FLATPAK SANDBOX PERMISSIONS\n   Styles for displaying sandbox security information\n   ========================================================================== */\n\n/* Sandbox section container */\n.sandbox-section {\n    margin-top: 16px;\n    padding: 12px;\n    background-color: alpha(@view_fg_color, 0.03);\n    border-radius: 12px;\n    border: 1px solid alpha(@borders, 0.3);\n}\n\n/* Sandbox rating box */\n.sandbox-rating-box {\n    padding: 8px 0;\n}\n\n/* Sandbox badge (compact inline) */\n.sandbox-badge {\n    padding: 2px 6px;\n    background-color: alpha(@view_fg_color, 0.05);\n    border-radius: 4px;\n}\n\n/* Privacy level colors */\n.privacy-low {\n    color: alpha(@view_fg_color, 0.5);\n}\n\n.privacy-medium {\n    color: @yellow_4;\n}\n\n.privacy-high {\n    color: @red_4;\n}\n\n/* Sandbox rating classes */\n.sandbox-strong {\n    color: @green_4;\n}\n\n.sandbox-good {\n    color: @blue_4;\n}\n\n.sandbox-moderate {\n    color: @yellow_4;\n}\n\n.sandbox-weak {\n    color: @red_4;\n}\n\n/* Warning bar for EOL apps */\n.warning-bar {\n    padding: 8px 12px;\n    background-color: alpha(@yellow_4, 0.15);\n    border-radius: 8px;\n    border: 1px solid alpha(@yellow_4, 0.3);\n}"
}

Now let me update the todo list and add the tests:

🔧 Tool: TodoWrite
Input: {
  "todos": [
    {
      "content": "Add FlatpakMetadata struct to models for sandbox permissions",
      "status": "completed",
      "activeForm": "Adding FlatpakMetadata struct to models"
    },
    {
      "content": "Enhance Flatpak backend with sandbox permissions detection",
      "status": "completed",
      "activeForm": "Enhancing Flatpak backend with sandbox detection"
    },
    {
      "content": "Add method to get app permissions and runtime info",
      "status": "completed",
      "activeForm": "Adding method for app permissions"
    },
    {
      "content": "Update Package model to include Flatpak-specific metadata",
      "status": "completed",
      "activeForm": "Updating Package model"
    },
    {
      "content": "Add CLI command to show Flatpak permissions",
      "status": "completed",
      "activeForm": "Adding CLI command for permissions"
    },
    {
      "content": "Update UI to display sandbox information",
      "status": "completed",
      "activeForm": "Updating UI for sandbox display"
    },
    {
      "content": "Add tests for Flatpak sandbox functionality",
      "status": "in_progress",
      "activeForm": "Adding tests for Flatpak sandbox"
    }
  ]
}

Now let me add tests for the Flatpak sandbox functionality. First, let me check the existing providers.rs tests as a pattern:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/providers.rs",
  "offset": 285
}

Now let me add tests to the flatpak.rs file and flatpak_metadata.rs:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/flatpak.rs",
  "offset": 480
}

Now let me add tests at the end of the flatpak.rs file:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/backend/flatpak.rs",
  "old_string": "        Ok(packages)\n    }\n}",
  "new_string": "        Ok(packages)\n    }\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    use crate::models::{PermissionCategory, PrivacyLevel, SandboxRating};\n\n    #[test]\n    fn test_parse_human_size() {\n        assert_eq!(parse_human_size(\"1.0 GB\"), Some(1024 * 1024 * 1024));\n        assert_eq!(parse_human_size(\"500 MB\"), Some(500 * 1024 * 1024));\n        assert_eq!(parse_human_size(\"100 kB\"), Some(100 * 1024));\n        assert_eq!(parse_human_size(\"1024 bytes\"), Some(1024));\n        assert_eq!(parse_human_size(\"invalid\"), None);\n        assert_eq!(parse_human_size(\"\"), None);\n    }\n\n    #[test]\n    fn test_parse_runtime_ref() {\n        let runtime = FlatpakBackend::parse_runtime_ref(\"org.gnome.Platform/x86_64/45\");\n        assert!(runtime.is_some());\n        let rt = runtime.unwrap();\n        assert_eq!(rt.id, \"org.gnome.Platform\");\n        assert_eq!(rt.version, \"45\");\n    }\n\n    #[test]\n    fn test_parse_metadata_basic() {\n        let metadata_content = r#\"\n[Application]\nruntime=org.gnome.Platform/x86_64/45\nsdk=org.gnome.Sdk/x86_64/45\n\n[Context]\nshared=network;ipc;\nsockets=x11;wayland;pulseaudio;\ndevices=dri;\nfilesystems=host;xdg-download;\n\"#;\n        let metadata = FlatpakBackend::parse_metadata(metadata_content, \"com.example.App\");\n\n        assert_eq!(metadata.app_id, \"com.example.App\");\n        assert!(metadata.runtime.is_some());\n        assert_eq!(metadata.runtime.as_ref().unwrap().id, \"org.gnome.Platform\");\n        assert!(!metadata.permissions.is_empty());\n\n        // Check that we have network permission\n        assert!(metadata.has_network_access());\n\n        // Check that we have filesystem access\n        assert!(metadata.has_full_filesystem_access());\n    }\n\n    #[test]\n    fn test_parse_metadata_with_dbus() {\n        let metadata_content = r#\"\n[Application]\nruntime=org.gnome.Platform/x86_64/45\n\n[Context]\nshared=network;\n\n[Session Bus Policy]\norg.freedesktop.Notifications=talk\norg.gtk.vfs.*=talk\norg.freedesktop.secrets=talk\n\"#;\n        let metadata = FlatpakBackend::parse_metadata(metadata_content, \"com.example.App\");\n\n        // Should have D-Bus permissions\n        let dbus_perms: Vec<_> = metadata\n            .permissions\n            .iter()\n            .filter(|p| p.category == PermissionCategory::SessionBus)\n            .collect();\n\n        assert!(!dbus_perms.is_empty());\n    }\n\n    #[test]\n    fn test_permission_from_raw() {\n        // Test filesystem permission\n        let perm = FlatpakPermission::from_raw(PermissionCategory::Filesystem, \"host\");\n        assert_eq!(perm.value, \"host\");\n        assert!(!perm.negated);\n        assert_eq!(perm.privacy_level, PrivacyLevel::High);\n\n        // Test negated permission\n        let neg_perm = FlatpakPermission::from_raw(PermissionCategory::Filesystem, \"!host\");\n        assert!(neg_perm.negated);\n\n        // Test socket permission\n        let socket_perm = FlatpakPermission::from_raw(PermissionCategory::Socket, \"x11\");\n        assert_eq!(socket_perm.category, PermissionCategory::Socket);\n        assert_eq!(socket_perm.privacy_level, PrivacyLevel::Medium);\n    }\n\n    #[test]\n    fn test_sandbox_summary_ratings() {\n        // Test strong sandbox (no high-risk permissions)\n        let metadata = FlatpakMetadata {\n            app_id: \"com.example.Sandboxed\".to_string(),\n            permissions: vec![\n                FlatpakPermission::from_raw(PermissionCategory::Socket, \"wayland\"),\n                FlatpakPermission::from_raw(PermissionCategory::Device, \"dri\"),\n            ],\n            ..Default::default()\n        };\n        let summary = metadata.sandbox_summary();\n        assert_eq!(summary.rating, SandboxRating::Strong);\n\n        // Test weak sandbox (full filesystem + network)\n        let weak_metadata = FlatpakMetadata {\n            app_id: \"com.example.Weak\".to_string(),\n            permissions: vec![\n                FlatpakPermission::from_raw(PermissionCategory::Filesystem, \"host\"),\n                FlatpakPermission::from_raw(PermissionCategory::Share, \"network\"),\n            ],\n            ..Default::default()\n        };\n        let weak_summary = weak_metadata.sandbox_summary();\n        assert_eq!(weak_summary.rating, SandboxRating::Weak);\n    }\n\n    #[test]\n    fn test_permissions_by_category() {\n        let metadata = FlatpakMetadata {\n            app_id: \"com.example.App\".to_string(),\n            permissions: vec![\n                FlatpakPermission::from_raw(PermissionCategory::Filesystem, \"home\"),\n                FlatpakPermission::from_raw(PermissionCategory::Filesystem, \"xdg-download\"),\n                FlatpakPermission::from_raw(PermissionCategory::Socket, \"x11\"),\n                FlatpakPermission::from_raw(PermissionCategory::Device, \"dri\"),\n            ],\n            ..Default::default()\n        };\n\n        let grouped = metadata.permissions_by_category();\n\n        // Should have 3 categories\n        assert_eq!(grouped.len(), 3);\n\n        // Filesystem should have 2 permissions\n        let fs_group = grouped.iter().find(|(cat, _)| *cat == PermissionCategory::Filesystem);\n        assert!(fs_group.is_some());\n        assert_eq!(fs_group.unwrap().1.len(), 2);\n    }\n\n    #[test]\n    fn test_flatpak_runtime_display() {\n        let runtime = FlatpakRuntime {\n            id: \"org.gnome.Platform\".to_string(),\n            version: \"45\".to_string(),\n            branch: \"stable\".to_string(),\n        };\n\n        assert_eq!(format!(\"{}\", runtime), \"org.gnome.Platform/45/stable\");\n    }\n\n    #[test]\n    fn test_installation_type_display() {\n        assert_eq!(format!(\"{}\", InstallationType::User), \"User\");\n        assert_eq!(format!(\"{}\", InstallationType::System), \"System\");\n    }\n\n    #[test]\n    fn test_privacy_level_ordering() {\n        assert!(PrivacyLevel::Low < PrivacyLevel::Medium);\n        assert!(PrivacyLevel::Medium < PrivacyLevel::High);\n    }\n\n    #[test]\n    fn test_sandbox_rating_ordering() {\n        assert!(SandboxRating::Strong < SandboxRating::Good);\n        assert!(SandboxRating::Good < SandboxRating::Moderate);\n        assert!(SandboxRating::Moderate < SandboxRating::Weak);\n    }\n\n    #[test]\n    fn test_max_privacy_level() {\n        let metadata = FlatpakMetadata {\n            app_id: \"com.example.App\".to_string(),\n            permissions: vec![\n                FlatpakPermission::from_raw(PermissionCategory::Socket, \"wayland\"),  // Low\n                FlatpakPermission::from_raw(PermissionCategory::Socket, \"x11\"),       // Medium\n            ],\n            ..Default::default()\n        };\n\n        assert_eq!(metadata.max_privacy_level(), PrivacyLevel::Medium);\n    }\n\n    #[test]\n    fn test_parse_info() {\n        let info_content = r#\"\n        Ref: com.example.App/x86_64/stable\n        Origin: flathub\n        Commit: abc123def456\n        Installation: user\n        Arch: x86_64\n        Branch: stable\n        \"#;\n\n        let mut metadata = FlatpakMetadata::default();\n        FlatpakBackend::parse_info(info_content, &mut metadata);\n\n        assert_eq!(metadata.remote, Some(\"flathub\".to_string()));\n        assert_eq!(metadata.commit, Some(\"abc123def456\".to_string()));\n        assert_eq!(metadata.installation, InstallationType::User);\n        assert_eq!(metadata.arch, Some(\"x86_64\".to_string()));\n        assert_eq!(metadata.branch, Some(\"stable\".to_string()));\n    }\n\n    #[test]\n    fn test_permission_category_metadata() {\n        // Test icon names\n        assert_eq!(PermissionCategory::Filesystem.icon_name(), \"folder-symbolic\");\n        assert_eq!(PermissionCategory::Socket.icon_name(), \"network-wired-symbolic\");\n\n        // Test descriptions\n        assert_eq!(PermissionCategory::Filesystem.description(), \"Filesystem Access\");\n        assert_eq!(PermissionCategory::Socket.description(), \"Socket Access\");\n    }\n\n    #[test]\n    fn test_flatpak_backend_is_available() {\n        // This will depend on whether flatpak is installed on the test system\n        // At minimum, it shouldn't panic\n        let _ = FlatpakBackend::is_available();\n    }\n}"
}

I notice some files were modified by linters. Let me verify the CLI and UI changes are still in place, and fix any issues. Let me also check and reapply the permissions module if it was removed:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/mod.rs"
}

🔧 Tool: Bash
Input: {
  "command": "ls -la /home/eslam/Storage/Code/LinGet/src/cli/commands/permissions.rs 2>/dev/null || echo \"File not found\"",
  "description": "Check if permissions.rs exists"
}

The linter reverted the CLI changes. Let me re-add the permissions module and the CLI commands:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/mod.rs",
  "old_string": "pub mod check;\npub mod completions;\npub mod info;\npub mod install;\npub mod list;\npub mod providers;\npub mod remove;\npub mod search;\npub mod sources;\npub mod update;",
  "new_string": "pub mod check;\npub mod completions;\npub mod info;\npub mod install;\npub mod list;\npub mod permissions;\npub mod providers;\npub mod remove;\npub mod search;\npub mod sources;\npub mod update;"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs",
  "offset": 128,
  "limit": 40
}

The linter reverted the CLI changes too. Let me re-add the permissions command and the runtimes command:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs",
  "old_string": "    /// Generate shell completions\n    Completions {\n        /// Shell to generate completions for\n        shell: clap_complete::Shell,\n    },\n\n    /// Launch interactive TUI mode\n    Tui,\n\n    /// Launch graphical user interface (default when no command given)\n    Gui,\n}",
  "new_string": "    /// Generate shell completions\n    Completions {\n        /// Shell to generate completions for\n        shell: clap_complete::Shell,\n    },\n\n    /// Show Flatpak sandbox permissions for an application\n    Permissions {\n        /// Flatpak application ID\n        app_id: String,\n\n        /// Show only overrides\n        #[arg(short, long)]\n        overrides: bool,\n\n        /// Reset all overrides for this app\n        #[arg(long)]\n        reset: bool,\n    },\n\n    /// List Flatpak runtimes installed on the system\n    Runtimes,\n\n    /// Launch interactive TUI mode\n    Tui,\n\n    /// Launch graphical user interface (default when no command given)\n    Gui,\n}"
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs",
  "offset": 280
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/mod.rs",
  "old_string": "        Commands::Completions { shell } => {\n            commands::completions::run(shell);\n            Ok(())\n        }\n        Commands::Tui => tui::run().await,\n        Commands::Gui => {\n            // This is handled in main.rs - should not reach here\n            unreachable!(\"GUI command should be handled in main.rs\")\n        }\n    }\n}",
  "new_string": "        Commands::Completions { shell } => {\n            commands::completions::run(shell);\n            Ok(())\n        }\n        Commands::Permissions {\n            app_id,\n            overrides,\n            reset,\n        } => {\n            let action = if reset {\n                commands::permissions::PermissionsAction::Reset\n            } else if overrides {\n                commands::permissions::PermissionsAction::Overrides\n            } else {\n                commands::permissions::PermissionsAction::Show\n            };\n            commands::permissions::run(pm, &app_id, action, &writer).await\n        }\n        Commands::Runtimes => commands::permissions::list_runtimes(pm, &writer).await,\n        Commands::Tui => tui::run().await,\n        Commands::Gui => {\n            // This is handled in main.rs - should not reach here\n            unreachable!(\"GUI command should be handled in main.rs\")\n        }\n    }\n}"
}

Now let me check and re-add the UI changes that were reverted:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/mod.rs"
}

I see the package_details/mod.rs was also reverted. Let me re-add the sandbox module export:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/mod.rs",
  "old_string": "pub mod enrichment;\npub mod panel;\n\npub use panel::PackageDetailsPanel;",
  "new_string": "pub mod enrichment;\npub mod panel;\npub mod sandbox;\n\npub use panel::PackageDetailsPanel;\npub use sandbox::{build_sandbox_badge, build_sandbox_section};"
}

Now let me check if the CSS styles were also reverted:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/resources/style.css",
  "offset": 780,
  "limit": 80
}

The CSS styles were also reverted. Let me re-add them:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/resources/style.css",
  "old_string": "/* Improved dim-label contrast */\n.dim-label {\n    opacity: 0.65;\n}\n\n/* Numeric/version text */",
  "new_string": "/* Improved dim-label contrast */\n.dim-label {\n    opacity: 0.65;\n}\n\n/* ==========================================================================\n   FLATPAK SANDBOX PERMISSIONS\n   Styles for displaying sandbox security information\n   ========================================================================== */\n\n/* Sandbox section container */\n.sandbox-section {\n    margin-top: 16px;\n    padding: 12px;\n    background-color: alpha(@view_fg_color, 0.03);\n    border-radius: 12px;\n    border: 1px solid alpha(@borders, 0.3);\n}\n\n/* Sandbox rating box */\n.sandbox-rating-box {\n    padding: 8px 0;\n}\n\n/* Sandbox badge (compact inline) */\n.sandbox-badge {\n    padding: 2px 6px;\n    background-color: alpha(@view_fg_color, 0.05);\n    border-radius: 4px;\n}\n\n/* Privacy level colors */\n.privacy-low {\n    color: alpha(@view_fg_color, 0.5);\n}\n\n.privacy-medium {\n    color: @yellow_4;\n}\n\n.privacy-high {\n    color: @red_4;\n}\n\n/* Sandbox rating classes */\n.sandbox-strong {\n    color: @green_4;\n}\n\n.sandbox-good {\n    color: @blue_4;\n}\n\n.sandbox-moderate {\n    color: @yellow_4;\n}\n\n.sandbox-weak {\n    color: @red_4;\n}\n\n/* Warning bar for EOL apps */\n.warning-bar {\n    padding: 8px 12px;\n    background-color: alpha(@yellow_4, 0.15);\n    border-radius: 8px;\n    border: 1px solid alpha(@yellow_4, 0.3);\n}\n\n/* Numeric/version text */"
}

Now let me check if the panel.rs sandbox integration was also reverted:

🔧 Tool: Grep
Input: {
  "pattern": "sandbox::build_sandbox_section",
  "path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "output_mode": "content"
}

The panel.rs integration was also reverted. Let me re-add it:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "limit": 10
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "old_string": "use crate::backend::PackageManager;\nuse crate::models::{fetch_enrichment, Config, Package, PackageSource, PackageStatus};\nuse crate::ui::package_details::enrichment;\nuse crate::ui::{CommandCenter, CommandEventKind, PackageOp, RetrySpec};",
  "new_string": "use crate::backend::PackageManager;\nuse crate::models::{fetch_enrichment, Config, Package, PackageSource, PackageStatus};\nuse crate::ui::package_details::{enrichment, sandbox};\nuse crate::ui::{CommandCenter, CommandEventKind, PackageOp, RetrySpec};"
}

Now let me find the right place in panel.rs to add the sandbox section for Flatpak packages:

🔧 Tool: Grep
Input: {
  "pattern": "content.append\\(&details_group\\)",
  "path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "output_mode": "content",
  "-n": true,
  "-A": 5
}

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "offset": 314,
  "limit": 15
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/panel.rs",
  "old_string": "        ignore_row.add_suffix(&ignore_switch);\n        details_group.add(&ignore_row);\n\n        content.append(&details_group);\n\n        let changelog_expander = gtk::Expander::builder()\n            .label(\"Release History\")\n            .expanded(false)\n            .margin_top(8)\n            .build();\n        changelog_expander.add_css_class(\"card\");",
  "new_string": "        ignore_row.add_suffix(&ignore_switch);\n        details_group.add(&ignore_row);\n\n        content.append(&details_group);\n\n        // Add sandbox permissions section for Flatpak packages\n        if package.source == PackageSource::Flatpak {\n            let sandbox_section = sandbox::build_sandbox_section(pm.clone(), package.name.clone());\n            content.append(&sandbox_section);\n        }\n\n        let changelog_expander = gtk::Expander::builder()\n            .label(\"Release History\")\n            .expanded(false)\n            .margin_top(8)\n            .build();\n        changelog_expander.add_css_class(\"card\");"
}

Now let me verify everything compiles by running a check:

🔧 Tool: Bash
Input: {
  "command": "cargo check 2>&1 | head -100",
  "description": "Check if project compiles",
  "timeout": 180000
}

There are some compilation errors. Let me fix them:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/flatpak_metadata.rs",
  "offset": 115,
  "limit": 100
}

The issue is using `.leak()` on a `format!()` which returns a mutable reference. Let me fix this by using `Box::leak` properly or just cloning the string:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/flatpak_metadata.rs",
  "old_string": "    fn describe_permission(\n        category: PermissionCategory,\n        value: &str,\n        negated: bool,\n    ) -> (String, PrivacyLevel) {\n        let prefix = if negated { \"Denied: \" } else { \"\" };\n\n        match category {\n            PermissionCategory::Filesystem => {\n                let (desc, level) = match value {\n                    \"host\" => (\n                        \"Full access to all files\",\n                        PrivacyLevel::High,\n                    ),\n                    \"host-os\" => (\n                        \"Access to host operating system files\",\n                        PrivacyLevel::High,\n                    ),\n                    \"host-etc\" => (\n                        \"Access to /etc directory\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"home\" => (\n                        \"Full access to home directory\",\n                        PrivacyLevel::High,\n                    ),\n                    \"xdg-desktop\" => (\n                        \"Access to Desktop folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-documents\" => (\n                        \"Access to Documents folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-download\" => (\n                        \"Access to Downloads folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-music\" => (\n                        \"Access to Music folder\",\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-pictures\" => (\n                        \"Access to Pictures folder\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-videos\" => (\n                        \"Access to Videos folder\",\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-config\" => (\n                        \"Access to configuration files\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-cache\" => (\n                        \"Access to cache directory\",\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-data\" => (\n                        \"Access to application data\",\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-run\" => (\n                        \"Access to runtime directory\",\n                        PrivacyLevel::Low,\n                    ),\n                    _ if value.starts_with('/') => (\n                        format!(\"Access to {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                    _ if value.starts_with('~') => (\n                        format!(\"Access to {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                    _ => (\n                        format!(\"Filesystem: {}\", value).leak(),\n                        PrivacyLevel::Low,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Socket => {\n                let (desc, level) = match value {\n                    \"x11\" => (\"X11 window system access\", PrivacyLevel::Medium),\n                    \"wayland\" => (\"Wayland display access\", PrivacyLevel::Low),\n                    \"fallback-x11\" => (\"Fallback X11 access\", PrivacyLevel::Medium),\n                    \"pulseaudio\" => (\"Audio playback and recording\", PrivacyLevel::Medium),\n                    \"session-bus\" => (\"D-Bus session bus access\", PrivacyLevel::Medium),\n                    \"system-bus\" => (\"D-Bus system bus access\", PrivacyLevel::High),\n                    \"ssh-auth\" => (\"SSH authentication agent\", PrivacyLevel::High),\n                    \"pcsc\" => (\"Smart card access\", PrivacyLevel::High),\n                    \"cups\" => (\"Printing access\", PrivacyLevel::Low),\n                    \"gpg-agent\" => (\"GPG agent access\", PrivacyLevel::High),\n                    _ => (\n                        format!(\"Socket: {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }",
  "new_string": "    fn describe_permission(\n        category: PermissionCategory,\n        value: &str,\n        negated: bool,\n    ) -> (String, PrivacyLevel) {\n        let prefix = if negated { \"Denied: \" } else { \"\" };\n\n        match category {\n            PermissionCategory::Filesystem => {\n                let (desc, level): (String, PrivacyLevel) = match value {\n                    \"host\" => (\n                        \"Full access to all files\".to_string(),\n                        PrivacyLevel::High,\n                    ),\n                    \"host-os\" => (\n                        \"Access to host operating system files\".to_string(),\n                        PrivacyLevel::High,\n                    ),\n                    \"host-etc\" => (\n                        \"Access to /etc directory\".to_string(),\n                        PrivacyLevel::Medium,\n                    ),\n                    \"home\" => (\n                        \"Full access to home directory\".to_string(),\n                        PrivacyLevel::High,\n                    ),\n                    \"xdg-desktop\" => (\n                        \"Access to Desktop folder\".to_string(),\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-documents\" => (\n                        \"Access to Documents folder\".to_string(),\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-download\" => (\n                        \"Access to Downloads folder\".to_string(),\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-music\" => (\n                        \"Access to Music folder\".to_string(),\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-pictures\" => (\n                        \"Access to Pictures folder\".to_string(),\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-videos\" => (\n                        \"Access to Videos folder\".to_string(),\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-config\" => (\n                        \"Access to configuration files\".to_string(),\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-cache\" => (\n                        \"Access to cache directory\".to_string(),\n                        PrivacyLevel::Low,\n                    ),\n                    \"xdg-data\" => (\n                        \"Access to application data\".to_string(),\n                        PrivacyLevel::Medium,\n                    ),\n                    \"xdg-run\" => (\n                        \"Access to runtime directory\".to_string(),\n                        PrivacyLevel::Low,\n                    ),\n                    _ if value.starts_with('/') => (\n                        format!(\"Access to {}\", value),\n                        PrivacyLevel::Medium,\n                    ),\n                    _ if value.starts_with('~') => (\n                        format!(\"Access to {}\", value),\n                        PrivacyLevel::Medium,\n                    ),\n                    _ => (\n                        format!(\"Filesystem: {}\", value),\n                        PrivacyLevel::Low,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Socket => {\n                let (desc, level): (String, PrivacyLevel) = match value {\n                    \"x11\" => (\"X11 window system access\".to_string(), PrivacyLevel::Medium),\n                    \"wayland\" => (\"Wayland display access\".to_string(), PrivacyLevel::Low),\n                    \"fallback-x11\" => (\"Fallback X11 access\".to_string(), PrivacyLevel::Medium),\n                    \"pulseaudio\" => (\"Audio playback and recording\".to_string(), PrivacyLevel::Medium),\n                    \"session-bus\" => (\"D-Bus session bus access\".to_string(), PrivacyLevel::Medium),\n                    \"system-bus\" => (\"D-Bus system bus access\".to_string(), PrivacyLevel::High),\n                    \"ssh-auth\" => (\"SSH authentication agent\".to_string(), PrivacyLevel::High),\n                    \"pcsc\" => (\"Smart card access\".to_string(), PrivacyLevel::High),\n                    \"cups\" => (\"Printing access\".to_string(), PrivacyLevel::Low),\n                    \"gpg-agent\" => (\"GPG agent access\".to_string(), PrivacyLevel::High),\n                    _ => (\n                        format!(\"Socket: {}\", value),\n                        PrivacyLevel::Medium,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }"
}

Now let me fix the remaining match arms for Device and Share categories:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/flatpak_metadata.rs",
  "offset": 214,
  "limit": 50
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/flatpak_metadata.rs",
  "old_string": "            PermissionCategory::Device => {\n                let (desc, level) = match value {\n                    \"dri\" => (\"GPU/graphics acceleration\", PrivacyLevel::Low),\n                    \"kvm\" => (\"Kernel virtualization access\", PrivacyLevel::High),\n                    \"shm\" => (\"Shared memory access\", PrivacyLevel::Low),\n                    \"all\" => (\"All device access\", PrivacyLevel::High),\n                    _ => (\n                        format!(\"Device: {}\", value).leak(),\n                        PrivacyLevel::Medium,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Share => {\n                let (desc, level) = match value {\n                    \"network\" => (\"Network access\", PrivacyLevel::Medium),\n                    \"ipc\" => (\"Inter-process communication\", PrivacyLevel::Low),\n                    _ => (\n                        format!(\"Share: {}\", value).leak(),\n                        PrivacyLevel::Low,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }",
  "new_string": "            PermissionCategory::Device => {\n                let (desc, level): (String, PrivacyLevel) = match value {\n                    \"dri\" => (\"GPU/graphics acceleration\".to_string(), PrivacyLevel::Low),\n                    \"kvm\" => (\"Kernel virtualization access\".to_string(), PrivacyLevel::High),\n                    \"shm\" => (\"Shared memory access\".to_string(), PrivacyLevel::Low),\n                    \"all\" => (\"All device access\".to_string(), PrivacyLevel::High),\n                    _ => (\n                        format!(\"Device: {}\", value),\n                        PrivacyLevel::Medium,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }\n            PermissionCategory::Share => {\n                let (desc, level): (String, PrivacyLevel) = match value {\n                    \"network\" => (\"Network access\".to_string(), PrivacyLevel::Medium),\n                    \"ipc\" => (\"Inter-process communication\".to_string(), PrivacyLevel::Low),\n                    _ => (\n                        format!(\"Share: {}\", value),\n                        PrivacyLevel::Low,\n                    ),\n                };\n                (format!(\"{}{}\", prefix, desc), level)\n            }"
}

Now let me check the compile again:

🔧 Tool: Bash
Input: {
  "command": "cargo check 2>&1 | head -80",
  "description": "Check if project compiles",
  "timeout": 180000
}

Let me add `Ord` and `PartialOrd` to `PermissionCategory`:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/models/flatpak_metadata.rs",
  "old_string": "/// Represents a Flatpak permission category\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub enum PermissionCategory {",
  "new_string": "/// Represents a Flatpak permission category\n#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]\npub enum PermissionCategory {"
}

🔧 Tool: Bash
Input: {
  "command": "cargo check 2>&1 | head -80",
  "description": "Check if project compiles",
  "timeout": 180000
}

The project compiles now with just warnings. Let me clean up the unused imports:

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/cli/commands/permissions.rs",
  "old_string": "use crate::backend::PackageManager;\nuse crate::cli::OutputWriter;\nuse crate::models::{PermissionCategory, PrivacyLevel, SandboxRating};\nuse anyhow::{bail, Result};",
  "new_string": "use crate::backend::PackageManager;\nuse crate::cli::OutputWriter;\nuse crate::models::{PrivacyLevel, SandboxRating};\nuse anyhow::Result;"
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/sandbox.rs",
  "old_string": "use crate::backend::PackageManager;\nuse crate::models::{\n    FlatpakMetadata, PermissionCategory, PrivacyLevel, SandboxRating, SandboxSummary,\n};",
  "new_string": "use crate::backend::PackageManager;\nuse crate::models::{FlatpakMetadata, PrivacyLevel, SandboxRating, SandboxSummary};"
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/src/ui/package_details/mod.rs",
  "old_string": "pub mod enrichment;\npub mod panel;\npub mod sandbox;\n\npub use panel::PackageDetailsPanel;\npub use sandbox::{build_sandbox_badge, build_sandbox_section};",
  "new_string": "pub mod enrichment;\npub mod panel;\npub mod sandbox;\n\npub use panel::PackageDetailsPanel;"
}

Now let me run the tests:

🔧 Tool: Bash
Input: {
  "command": "cargo test flatpak 2>&1 | head -80",
  "description": "Run Flatpak tests",
  "timeout": 180000
}

Let me see more of the test output:

🔧 Tool: Bash
Input: {
  "command": "cargo test flatpak 2>&1 | tail -60",
  "description": "Run Flatpak tests tail output",
  "timeout": 180000
}

All 15 tests pass. Now let me update the todo list to mark everything as complete:

🔧 Tool: TodoWrite
Input: {
  "todos": [
    {
      "content": "Add FlatpakMetadata struct to models for sandbox permissions",
      "status": "completed",
      "activeForm": "Adding FlatpakMetadata struct to models"
    },
    {
      "content": "Enhance Flatpak backend with sandbox permissions detection",
      "status": "completed",
      "activeForm": "Enhancing Flatpak backend with sandbox detection"
    },
    {
      "content": "Add method to get app permissions and runtime info",
      "status": "completed",
      "activeForm": "Adding method for app permissions"
    },
    {
      "content": "Update Package model to include Flatpak-specific metadata",
      "status": "completed",
      "activeForm": "Updating Package model"
    },
    {
      "content": "Add CLI command to show Flatpak permissions",
      "status": "completed",
      "activeForm": "Adding CLI command for permissions"
    },
    {
      "content": "Update UI to display sandbox information",
      "status": "completed",
      "activeForm": "Updating UI for sandbox display"
    },
    {
      "content": "Add tests for Flatpak sandbox functionality",
      "status": "completed",
      "activeForm": "Adding tests for Flatpak sandbox"
    }
  ]
}

Now let me update the feature status file for the Flatpak backend:

🔧 Tool: Read
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/flatpak-backend/feature.json"
}

🔧 Tool: Edit
Input: {
  "file_path": "/home/eslam/Storage/Code/LinGet/.automaker/features/flatpak-backend/feature.json",
  "old_string": "{\n  \"id\": \"flatpak-backend\",\n  \"category\": \"Backend\",\n  \"title\": \"Flatpak Backend Implementation\",\n  \"description\": \"Implement support for sandboxed Flatpak applications across all Linux distributions.\",\n  \"status\": \"in_progress\",\n  \"priority\": 1,\n  \"complexity\": \"moderate\",\n  \"dependencies\": [\n    \"core-package-detection\"\n  ],\n  \"createdAt\": \"2025-12-19T20:29:52.536Z\",\n  \"updatedAt\": \"2025-12-20T05:11:13.624Z\",\n  \"branchName\": \"main\",\n  \"startedAt\": \"2025-12-20T05:11:12.460Z\"\n}",
  "new_string": "{\n  \"id\": \"flatpak-backend\",\n  \"category\": \"Backend\",\n  \"title\": \"Flatpak Backend Implementation\",\n  \"description\": \"Implement support for sandboxed Flatpak applications across all Linux distributions.\",\n  \"status\": \"completed\",\n  \"priority\": 1,\n  \"complexity\": \"moderate\",\n  \"dependencies\": [\n    \"core-package-detection\"\n  ],\n  \"createdAt\": \"2025-12-19T20:29:52.536Z\",\n  \"updatedAt\": \"2025-12-20T09:30:00.000Z\",\n  \"branchName\": \"main\",\n  \"startedAt\": \"2025-12-20T05:11:12.460Z\",\n  \"completedAt\": \"2025-12-20T09:30:00.000Z\"\n}"
}

<summary>
## Summary: Flatpak Backend Implementation - Sandboxed Application Support

### Changes Implemented
- **FlatpakMetadata Model**: Added comprehensive data structures for Flatpak application sandbox information including:
  - `FlatpakMetadata` - Main metadata structure containing app ID, runtime, permissions, installation type, and EOL status
  - `FlatpakPermission` - Individual permission with category, value, description, and privacy level
  - `PermissionCategory` - Enum for filesystem, socket, device, share, environment, D-Bus permissions
  - `PrivacyLevel` - Low/Medium/High privacy impact levels
  - `SandboxRating` - Strong/Good/Moderate/Weak sandbox security rating
  - `SandboxSummary` - Summary of overall sandbox security

- **Enhanced Flatpak Backend**: Extended `FlatpakBackend` with new methods:
  - `get_metadata(app_id)` - Get full sandbox metadata for an app
  - `get_metadata_for_installation(app_id, installation_type)` - Get metadata for user/system installation
  - `parse_metadata()` - Parse flatpak metadata files
  - `parse_info()` - Parse flatpak info output
  - `list_runtimes()` - List all installed Flatpak runtimes
  - `get_overrides(app_id)` - Get permission overrides
  - `add_override()` / `remove_override()` / `reset_overrides()` - Manage permission overrides
  - `is_well_sandboxed(app_id)` - Check if app is well sandboxed

- **PackageManager Integration**: Added Flatpak-specific methods to the main `PackageManager`:
  - `get_flatpak_metadata()`, `get_flatpak_overrides()`
  - `add_flatpak_override()`, `remove_flatpak_override()`, `reset_flatpak_overrides()`
  - `list_flatpak_runtimes()`, `is_flatpak_well_sandboxed()`

- **CLI Commands**: Added new CLI commands:
  - `linget permissions <app_id>` - Show sandbox permissions for a Flatpak app
  - `linget permissions <app_id> --overrides` - Show only permission overrides
  - `linget permissions <app_id> --reset` - Reset all permission overrides
  - `linget runtimes` - List all installed Flatpak runtimes

- **UI Integration**: Added sandbox section in package details panel for Flatpak apps:
  - Sandbox rating display (Strong/Good/Moderate/Weak)
  - Runtime information display
  - Installation type (User/System)
  - EOL warning display
  - Expandable permissions list grouped by category
  - Privacy level indicators for each permission

- **CSS Styling**: Added styles for sandbox UI components:
  - `.sandbox-section`, `.sandbox-rating-box`, `.sandbox-badge`
  - Privacy level colors (`.privacy-low`, `.privacy-medium`, `.privacy-high`)
  - Sandbox rating colors (`.sandbox-strong`, `.sandbox-good`, `.sandbox-moderate`, `.sandbox-weak`)
  - Warning bar for EOL apps

- **Tests**: Added 15 comprehensive unit tests covering:
  - Permission parsing and categorization
  - Metadata parsing from flatpak output
  - Sandbox rating calculations
  - Privacy level ordering
  - Runtime display formatting

### Files Modified
- `src/models/flatpak_metadata.rs` (NEW - ~500 lines)
- `src/models/mod.rs`
- `src/backend/flatpak.rs` (extended with sandbox methods + tests)
- `src/backend/mod.rs` (added Flatpak-specific PackageManager methods)
- `src/cli/commands/permissions.rs` (NEW - CLI command)
- `src/cli/commands/mod.rs`
- `src/cli/mod.rs` (added permissions and runtimes commands)
- `src/ui/package_details/sandbox.rs` (NEW - UI component)
- `src/ui/package_details/mod.rs`
- `src/ui/package_details/panel.rs`
- `resources/style.css` (sandbox styling)
- `.automaker/features/flatpak-backend/feature.json`

### Notes for Developer
- The implementation works across all Linux distributions that have Flatpak installed
- Permission descriptions include privacy impact levels to help users understand security implications
- The sandbox rating system provides at-a-glance security assessment
- EOL (End of Life) applications are flagged with warnings
- All 15 unit tests pass
- The feature integrates seamlessly with the existing GUI, CLI, and backend architecture
</summary>