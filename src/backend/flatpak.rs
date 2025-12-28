use super::PackageBackend;
use crate::models::{
    FlatpakMetadata, FlatpakPermission, FlatpakRuntime, InstallationType, Package, PackageSource,
    PackageStatus, PermissionCategory, Repository,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Parse human-readable size strings like "1.2 GB", "500 MB", "100 kB"
fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let mut num_end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + c.len_utf8();
        } else if !c.is_whitespace() {
            break;
        }
    }
    let num: f64 = s[..num_end].trim().parse().ok()?;
    let unit = s[num_end..].trim().to_lowercase();
    let multiplier: u64 = match unit.as_str() {
        "b" | "bytes" => 1,
        "kb" | "kib" => 1024,
        "mb" | "mib" => 1024 * 1024,
        "gb" | "gib" => 1024 * 1024 * 1024,
        "tb" | "tib" => 1024 * 1024 * 1024 * 1024,
        _ => return None,
    };
    Some((num * multiplier as f64) as u64)
}

pub struct FlatpakBackend;

impl FlatpakBackend {
    pub fn new() -> Self {
        Self
    }

    /// Get detailed metadata for a Flatpak application including sandbox permissions
    pub async fn get_metadata(&self, app_id: &str) -> Result<FlatpakMetadata> {
        // Get basic info using flatpak info
        let info_output = Command::new("flatpak")
            .args(["info", "--show-metadata", app_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get flatpak metadata")?;

        let metadata_str = String::from_utf8_lossy(&info_output.stdout);

        // Parse the metadata
        let mut metadata = Self::parse_metadata(&metadata_str, app_id);

        // Get additional info
        let info_output = Command::new("flatpak")
            .args(["info", app_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get flatpak info")?;

        let info_str = String::from_utf8_lossy(&info_output.stdout);
        Self::parse_info(&info_str, &mut metadata);

        Ok(metadata)
    }

    /// Parse the metadata from flatpak info --show-metadata output
    fn parse_metadata(content: &str, app_id: &str) -> FlatpakMetadata {
        let mut metadata = FlatpakMetadata {
            app_id: app_id.to_string(),
            ..Default::default()
        };

        let mut current_section = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Section headers
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Key-value pairs
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match current_section.as_str() {
                    "Application" | "Runtime" => match key {
                        "runtime" => {
                            if let Some(rt) = Self::parse_runtime_ref(value) {
                                metadata.runtime = Some(rt);
                            }
                        }
                        "sdk" => metadata.sdk = Some(value.to_string()),
                        _ => {}
                    },
                    "Context" => {
                        Self::parse_context_permissions(key, value, &mut metadata.permissions);
                    }
                    "Session Bus Policy" => {
                        Self::parse_dbus_permissions(
                            key,
                            value,
                            PermissionCategory::SessionBus,
                            &mut metadata.permissions,
                        );
                    }
                    "System Bus Policy" => {
                        Self::parse_dbus_permissions(
                            key,
                            value,
                            PermissionCategory::SystemBus,
                            &mut metadata.permissions,
                        );
                    }
                    "Environment" => {
                        metadata.permissions.push(FlatpakPermission::from_raw(
                            PermissionCategory::Environment,
                            &format!("{}={}", key, value),
                        ));
                    }
                    "Extension" => {
                        // Track extensions
                        if key.starts_with("Extension ") {
                            let ext_name = key.strip_prefix("Extension ").unwrap_or(key);
                            metadata.extensions.push(ext_name.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        metadata
    }

    /// Parse runtime reference string
    fn parse_runtime_ref(runtime_ref: &str) -> Option<FlatpakRuntime> {
        // Format: org.gnome.Platform/x86_64/45 or org.gnome.Platform//45
        let parts: Vec<&str> = runtime_ref.split('/').collect();
        if parts.len() >= 3 {
            Some(FlatpakRuntime {
                id: parts[0].to_string(),
                version: parts.get(2).unwrap_or(&"").to_string(),
                branch: parts.get(3).unwrap_or(&"stable").to_string(),
            })
        } else if parts.len() == 2 {
            Some(FlatpakRuntime {
                id: parts[0].to_string(),
                version: parts[1].to_string(),
                branch: "stable".to_string(),
            })
        } else {
            None
        }
    }

    /// Parse Context section permissions
    fn parse_context_permissions(key: &str, value: &str, permissions: &mut Vec<FlatpakPermission>) {
        let category = match key {
            "filesystems" => PermissionCategory::Filesystem,
            "sockets" => PermissionCategory::Socket,
            "devices" => PermissionCategory::Device,
            "shared" => PermissionCategory::Share,
            "features" => PermissionCategory::Other,
            "persistent" => PermissionCategory::Filesystem,
            _ => return,
        };

        // Values are semicolon-separated
        for item in value.split(';') {
            let item = item.trim();
            if !item.is_empty() {
                permissions.push(FlatpakPermission::from_raw(category, item));
            }
        }
    }

    /// Parse D-Bus permissions
    fn parse_dbus_permissions(
        bus_name: &str,
        access: &str,
        category: PermissionCategory,
        permissions: &mut Vec<FlatpakPermission>,
    ) {
        // Access can be: talk, own, see, none
        let perm_str = match access.trim() {
            "none" => format!("!{}", bus_name),
            "talk" => format!("{} (talk)", bus_name),
            "own" => format!("{} (own)", bus_name),
            "see" => format!("{} (see)", bus_name),
            _ => format!("{} ({})", bus_name, access),
        };
        permissions.push(FlatpakPermission::from_raw(category, &perm_str));
    }

    /// Parse additional info from flatpak info output
    fn parse_info(content: &str, metadata: &mut FlatpakMetadata) {
        for line in content.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "Ref" | "ID" => {
                        if metadata.app_id.is_empty() {
                            metadata.app_id = value.to_string();
                        }
                    }
                    "Origin" => metadata.remote = Some(value.to_string()),
                    "Commit" => metadata.commit = Some(value.to_string()),
                    "Installation" => {
                        metadata.installation = if value.to_lowercase().contains("system") {
                            InstallationType::System
                        } else {
                            InstallationType::User
                        };
                    }
                    "Arch" => metadata.arch = Some(value.to_string()),
                    "Branch" => metadata.branch = Some(value.to_string()),
                    "End-of-life" | "EOL" => {
                        metadata.is_eol = true;
                        if !value.is_empty() && value != "yes" && value != "true" {
                            metadata.eol_reason = Some(value.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// List all runtimes installed on the system
    pub async fn list_runtimes(&self) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args([
                "list",
                "--runtime",
                "--columns=application,version,name,size,origin",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list flatpak runtimes")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let app_id = parts[0].to_string();
                let version = parts[1].to_string();
                let name = parts[2].to_string();
                let size = parts.get(3).and_then(|s| parse_human_size(s));

                packages.push(Package {
                    name: app_id,
                    version,
                    available_version: None,
                    description: format!("Runtime: {}", name),
                    source: PackageSource::Flatpak,
                    status: PackageStatus::Installed,
                    size,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    /// Get permissions override for an application
    pub async fn get_overrides(&self, app_id: &str) -> Result<Vec<FlatpakPermission>> {
        let output = Command::new("flatpak")
            .args(["override", "--show", app_id])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get flatpak overrides")?;

        let override_str = String::from_utf8_lossy(&output.stdout);
        let metadata = Self::parse_metadata(&override_str, app_id);
        Ok(metadata.permissions)
    }

    /// Reset all overrides for an application
    pub async fn reset_overrides(&self, app_id: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["override", "--user", "--reset", app_id])
            .status()
            .await
            .context("Failed to reset flatpak overrides")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to reset overrides for {}", app_id)
        }
    }
}

impl Default for FlatpakBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for FlatpakBackend {
    fn is_available() -> bool {
        which::which("flatpak").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // Include size column (returns bytes)
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,name,size"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list flatpak apps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let app_id = parts[0].to_string();
                let version = parts[1].to_string();
                let name = parts[2].to_string();
                // Parse size (flatpak returns human-readable like "1.2 GB")
                let size = parts.get(3).and_then(|s| parse_human_size(s));

                packages.push(Package {
                    name: app_id,
                    version,
                    available_version: None,
                    description: name,
                    source: PackageSource::Flatpak,
                    status: PackageStatus::Installed,
                    size,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args([
                "remote-ls",
                "--updates",
                "--app",
                "--columns=application,version,name",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check flatpak updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let app_id = parts[0].to_string();
                let new_version = parts[1].to_string();
                let name = parts[2].to_string();

                let mut pkg = Package {
                    name: app_id,
                    version: String::new(),
                    available_version: Some(new_version),
                    description: name,
                    source: PackageSource::Flatpak,
                    status: PackageStatus::UpdateAvailable,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                    enrichment: None,
                };
                pkg.update_category = Some(pkg.detect_update_category());
                packages.push(pkg);
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["install", "-y", name])
            .status()
            .await
            .context("Failed to install flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install flatpak {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["uninstall", "-y", name])
            .status()
            .await
            .context("Failed to remove flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove flatpak {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["update", "-y", name])
            .status()
            .await
            .context("Failed to update flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update flatpak {}", name)
        }
    }

    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        // flatpak remotes lists all configured remotes
        let output = Command::new("flatpak")
            .args(["remotes", "--columns=name,url,options"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list flatpak remotes")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut repos = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                let name = parts[0].to_string();
                let url = parts
                    .get(1)
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                let options = parts.get(2).unwrap_or(&"");
                // Check if disabled by looking at options
                let enabled = !options.contains("disabled");

                repos.push(Repository::new(name, PackageSource::Flatpak, enabled, url));
            }
        }

        Ok(repos)
    }

    async fn add_repository(&self, url: &str, name: Option<&str>) -> Result<()> {
        // flatpak remote-add <name> <url>
        let repo_name = name.unwrap_or("custom");
        let status = Command::new("flatpak")
            .args(["remote-add", "--if-not-exists", repo_name, url])
            .status()
            .await
            .context("Failed to add flatpak remote")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to add flatpak remote {}", url)
        }
    }

    async fn remove_repository(&self, name: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["remote-delete", "--force", name])
            .status()
            .await
            .context("Failed to remove flatpak remote")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove flatpak remote {}", name)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args([
                "search",
                query,
                "--columns=application,version,name,description",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search flatpak")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(1).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: parts.get(2).unwrap_or(&"").to_string(),
                    source: PackageSource::Flatpak,
                    status: PackageStatus::NotInstalled,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        let remote_origin = self.get_app_origin(name).await?;
        let app_ref = self.get_app_ref(name).await?;
        let commit_hashes = self.get_commit_history(&remote_origin, &app_ref).await?;

        let skip_current_version = 1;
        let previous_versions: Vec<String> = commit_hashes
            .into_iter()
            .skip(skip_current_version)
            .take(10)
            .collect();

        Ok(previous_versions)
    }

    async fn downgrade_to(&self, name: &str, commit_hash: &str) -> Result<()> {
        let status = Command::new("flatpak")
            .args(["update", "-y", &format!("--commit={}", commit_hash), name])
            .status()
            .await
            .context("Failed to downgrade flatpak")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to downgrade {} to commit {}", name, commit_hash)
        }
    }

    async fn get_cache_size(&self) -> Result<u64> {
        let output = Command::new("flatpak")
            .args(["list", "--unused", "--columns=size"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get unused flatpak size")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut total: u64 = 0;

        for line in stdout.lines() {
            if let Some(size) = parse_human_size(line.trim()) {
                total += size;
            }
        }

        Ok(total)
    }

    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        let output = Command::new("flatpak")
            .args([
                "list",
                "--unused",
                "--columns=application,version,name,size",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list unused flatpaks")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if !parts.is_empty() && !parts[0].is_empty() {
                let size = parts.get(3).and_then(|s| parse_human_size(s));
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts.get(1).unwrap_or(&"").to_string(),
                    available_version: None,
                    description: format!("{} (unused runtime)", parts.get(2).unwrap_or(&"Unused")),
                    source: PackageSource::Flatpak,
                    status: PackageStatus::Installed,
                    size,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    update_category: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    async fn cleanup_cache(&self) -> Result<u64> {
        let before = self.get_cache_size().await.unwrap_or(0);

        let status = Command::new("flatpak")
            .args(["uninstall", "-y", "--unused"])
            .status()
            .await
            .context("Failed to remove unused flatpaks")?;

        if !status.success() {
            anyhow::bail!("Failed to clean up unused Flatpak runtimes");
        }

        Ok(before)
    }

    async fn get_reverse_dependencies(&self, name: &str) -> Result<Vec<String>> {
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,runtime"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list flatpak apps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut deps = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let app_id = parts[0].trim();
                let runtime = parts[1].trim();
                if runtime.contains(name) && app_id != name {
                    deps.push(app_id.to_string());
                }
            }
        }

        Ok(deps)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Flatpak
    }

    async fn get_package_commands(&self, name: &str) -> Result<Vec<(String, std::path::PathBuf)>> {
        let mut commands = Vec::new();

        if let Ok(flatpak_path) = which::which("flatpak") {
            commands.push((format!("flatpak run {}", name), flatpak_path));
        }

        let export_dirs = [
            std::path::PathBuf::from("/var/lib/flatpak/exports/bin"),
            dirs::home_dir()
                .unwrap_or_default()
                .join(".local/share/flatpak/exports/bin"),
        ];

        for dir in &export_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(cmd_name) = path.file_name().and_then(|n| n.to_str()) {
                        if (cmd_name.contains(name) || cmd_name == name)
                            && !commands.iter().any(|(n, _)| n == cmd_name)
                        {
                            commands.push((cmd_name.to_string(), path));
                        }
                    }
                }
            }
        }

        Ok(commands)
    }
}

impl FlatpakBackend {
    async fn get_app_origin(&self, name: &str) -> Result<String> {
        let output = Command::new("flatpak")
            .args(["info", "--show-origin", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get flatpak app info")?;

        if !output.status.success() {
            anyhow::bail!("App {} not found", name);
        }

        let origin = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if origin.is_empty() {
            anyhow::bail!("Could not determine remote for {}", name);
        }

        Ok(origin)
    }

    async fn get_app_ref(&self, name: &str) -> Result<String> {
        let output = Command::new("flatpak")
            .args(["info", "--show-ref", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get flatpak ref")?;

        let app_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if app_ref.is_empty() {
            anyhow::bail!("Could not determine ref for {}", name);
        }

        Ok(app_ref)
    }

    async fn get_commit_history(&self, remote: &str, app_ref: &str) -> Result<Vec<String>> {
        let output = Command::new("flatpak")
            .args(["remote-info", "--log", remote, app_ref])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get flatpak commit log")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();

        for line in stdout.lines() {
            if let Some(commit) = line.trim().strip_prefix("Commit:") {
                let full_hash = commit.trim();
                if !full_hash.is_empty() {
                    let short_hash = &full_hash[..full_hash.len().min(12)];
                    commits.push(short_hash.to_string());
                }
            }
        }

        Ok(commits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PermissionCategory, PrivacyLevel, SandboxRating};

    #[test]
    fn test_parse_human_size() {
        assert_eq!(parse_human_size("1.0 GB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_human_size("500 MB"), Some(500 * 1024 * 1024));
        assert_eq!(parse_human_size("100 kB"), Some(100 * 1024));
        assert_eq!(parse_human_size("1024 bytes"), Some(1024));
        assert_eq!(parse_human_size("invalid"), None);
        assert_eq!(parse_human_size(""), None);
    }

    #[test]
    fn test_parse_runtime_ref() {
        let runtime = FlatpakBackend::parse_runtime_ref("org.gnome.Platform/x86_64/45");
        assert!(runtime.is_some());
        let rt = runtime.unwrap();
        assert_eq!(rt.id, "org.gnome.Platform");
        assert_eq!(rt.version, "45");
    }

    #[test]
    fn test_parse_metadata_basic() {
        let metadata_content = r#"
[Application]
runtime=org.gnome.Platform/x86_64/45
sdk=org.gnome.Sdk/x86_64/45

[Context]
shared=network;ipc;
sockets=x11;wayland;pulseaudio;
devices=dri;
filesystems=host;xdg-download;
"#;
        let metadata = FlatpakBackend::parse_metadata(metadata_content, "com.example.App");

        assert_eq!(metadata.app_id, "com.example.App");
        assert!(metadata.runtime.is_some());
        assert_eq!(metadata.runtime.as_ref().unwrap().id, "org.gnome.Platform");
        assert!(!metadata.permissions.is_empty());

        // Check that we have network permission
        assert!(metadata.has_network_access());

        // Check that we have filesystem access
        assert!(metadata.has_full_filesystem_access());
    }

    #[test]
    fn test_parse_metadata_with_dbus() {
        let metadata_content = r#"
[Application]
runtime=org.gnome.Platform/x86_64/45

[Context]
shared=network;

[Session Bus Policy]
org.freedesktop.Notifications=talk
org.gtk.vfs.*=talk
org.freedesktop.secrets=talk
"#;
        let metadata = FlatpakBackend::parse_metadata(metadata_content, "com.example.App");

        // Should have D-Bus permissions
        let dbus_perms: Vec<_> = metadata
            .permissions
            .iter()
            .filter(|p| p.category == PermissionCategory::SessionBus)
            .collect();

        assert!(!dbus_perms.is_empty());
    }

    #[test]
    fn test_permission_from_raw() {
        // Test filesystem permission
        let perm = FlatpakPermission::from_raw(PermissionCategory::Filesystem, "host");
        assert_eq!(perm.value, "host");
        assert!(!perm.negated);
        assert_eq!(perm.privacy_level, PrivacyLevel::High);

        // Test negated permission
        let neg_perm = FlatpakPermission::from_raw(PermissionCategory::Filesystem, "!host");
        assert!(neg_perm.negated);

        // Test socket permission
        let socket_perm = FlatpakPermission::from_raw(PermissionCategory::Socket, "x11");
        assert_eq!(socket_perm.category, PermissionCategory::Socket);
        assert_eq!(socket_perm.privacy_level, PrivacyLevel::Medium);
    }

    #[test]
    fn test_sandbox_summary_ratings() {
        // Test strong sandbox (no high-risk permissions)
        let metadata = FlatpakMetadata {
            app_id: "com.example.Sandboxed".to_string(),
            permissions: vec![
                FlatpakPermission::from_raw(PermissionCategory::Socket, "wayland"),
                FlatpakPermission::from_raw(PermissionCategory::Device, "dri"),
            ],
            ..Default::default()
        };
        let summary = metadata.sandbox_summary();
        assert_eq!(summary.rating, SandboxRating::Strong);

        // Test weak sandbox (full filesystem + network)
        let weak_metadata = FlatpakMetadata {
            app_id: "com.example.Weak".to_string(),
            permissions: vec![
                FlatpakPermission::from_raw(PermissionCategory::Filesystem, "host"),
                FlatpakPermission::from_raw(PermissionCategory::Share, "network"),
            ],
            ..Default::default()
        };
        let weak_summary = weak_metadata.sandbox_summary();
        assert_eq!(weak_summary.rating, SandboxRating::Weak);
    }

    #[test]
    fn test_permissions_by_category() {
        let metadata = FlatpakMetadata {
            app_id: "com.example.App".to_string(),
            permissions: vec![
                FlatpakPermission::from_raw(PermissionCategory::Filesystem, "home"),
                FlatpakPermission::from_raw(PermissionCategory::Filesystem, "xdg-download"),
                FlatpakPermission::from_raw(PermissionCategory::Socket, "x11"),
                FlatpakPermission::from_raw(PermissionCategory::Device, "dri"),
            ],
            ..Default::default()
        };

        let grouped = metadata.permissions_by_category();

        // Should have 3 categories
        assert_eq!(grouped.len(), 3);

        // Filesystem should have 2 permissions
        let fs_group = grouped
            .iter()
            .find(|(cat, _)| *cat == PermissionCategory::Filesystem);
        assert!(fs_group.is_some());
        assert_eq!(fs_group.unwrap().1.len(), 2);
    }

    #[test]
    fn test_flatpak_runtime_display() {
        let runtime = FlatpakRuntime {
            id: "org.gnome.Platform".to_string(),
            version: "45".to_string(),
            branch: "stable".to_string(),
        };

        assert_eq!(format!("{}", runtime), "org.gnome.Platform/45/stable");
    }

    #[test]
    fn test_installation_type_display() {
        assert_eq!(format!("{}", InstallationType::User), "User");
        assert_eq!(format!("{}", InstallationType::System), "System");
    }

    #[test]
    fn test_privacy_level_ordering() {
        assert!(PrivacyLevel::Low < PrivacyLevel::Medium);
        assert!(PrivacyLevel::Medium < PrivacyLevel::High);
    }

    #[test]
    fn test_sandbox_rating_ordering() {
        assert!(SandboxRating::Strong < SandboxRating::Good);
        assert!(SandboxRating::Good < SandboxRating::Moderate);
        assert!(SandboxRating::Moderate < SandboxRating::Weak);
    }

    #[test]
    fn test_max_privacy_level() {
        let metadata = FlatpakMetadata {
            app_id: "com.example.App".to_string(),
            permissions: vec![
                FlatpakPermission::from_raw(PermissionCategory::Socket, "wayland"), // Low
                FlatpakPermission::from_raw(PermissionCategory::Socket, "x11"),     // Medium
            ],
            ..Default::default()
        };

        assert_eq!(metadata.max_privacy_level(), PrivacyLevel::Medium);
    }

    #[test]
    fn test_parse_info() {
        let info_content = r#"
        Ref: com.example.App/x86_64/stable
        Origin: flathub
        Commit: abc123def456
        Installation: user
        Arch: x86_64
        Branch: stable
        "#;

        let mut metadata = FlatpakMetadata::default();
        FlatpakBackend::parse_info(info_content, &mut metadata);

        assert_eq!(metadata.remote, Some("flathub".to_string()));
        assert_eq!(metadata.commit, Some("abc123def456".to_string()));
        assert_eq!(metadata.installation, InstallationType::User);
        assert_eq!(metadata.arch, Some("x86_64".to_string()));
        assert_eq!(metadata.branch, Some("stable".to_string()));
    }

    #[test]
    fn test_permission_category_metadata() {
        // Test icon names
        assert_eq!(
            PermissionCategory::Filesystem.icon_name(),
            "folder-symbolic"
        );
        assert_eq!(
            PermissionCategory::Socket.icon_name(),
            "network-wired-symbolic"
        );

        // Test descriptions
        assert_eq!(
            PermissionCategory::Filesystem.description(),
            "Filesystem Access"
        );
        assert_eq!(PermissionCategory::Socket.description(), "Socket Access");
    }

    #[test]
    fn test_flatpak_backend_is_available() {
        // This will depend on whether flatpak is installed on the test system
        // At minimum, it shouldn't panic
        let _ = FlatpakBackend::is_available();
    }
}
