use super::PackageBackend;
use super::SUGGEST_PREFIX;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::process::Stdio;
use tokio::process::Command;

pub struct SnapBackend;

impl SnapBackend {
    pub fn new() -> Self {
        Self
    }

    fn parse_pids(stderr: &str) -> Vec<String> {
        let mut pids = Vec::new();
        for token in stderr.split(|c: char| !c.is_ascii_digit()) {
            if !token.is_empty() {
                pids.push(token.to_string());
            }
        }
        pids.sort();
        pids.dedup();
        pids
    }

    fn snap_supports_terminate_flag() -> bool {
        static SUPPORTED: OnceCell<bool> = OnceCell::new();
        *SUPPORTED.get_or_init(|| {
            let checks = [
                (["help", "refresh"].as_slice(), "--terminate"),
                (["refresh", "--help"].as_slice(), "--terminate"),
            ];

            for (args, needle) in checks {
                if let Ok(out) = std::process::Command::new("snap").args(args).output() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let text = format!("{}\n{}", stdout, stderr);
                    if text.contains(needle) {
                        return true;
                    }
                }
            }
            false
        })
    }

    fn format_snap_running_error(name: &str, stderr: &str) -> String {
        // Example:
        // error: cannot refresh "code-insiders": snap "code-insiders" has running apps (code-insiders), pids:
        //        15704
        let pids = Self::parse_pids(stderr);

        if pids.is_empty() {
            format!(
                "Can't update snap \"{}\" because it is running. Close it and retry.",
                name
            )
        } else {
            let shown: String = pids.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
            let suffix = if pids.len() > 5 { ", …" } else { "" };
            format!(
                "Can't update snap \"{}\" because it is running (pids: {}{}). Close it and retry.",
                name, shown, suffix
            )
        }
    }

    fn running_apps_suggest_command(name: &str, stderr: &str) -> String {
        if Self::snap_supports_terminate_flag() {
            format!("sudo snap refresh {} --terminate", name)
        } else {
            let pids = Self::parse_pids(stderr);
            if pids.is_empty() {
                format!("sudo snap refresh {}", name)
            } else {
                format!("kill {} && sudo snap refresh {}", pids.join(" "), name)
            }
        }
    }

    async fn run_pkexec_snap(&self, args: &[&str], context_msg: &str) -> Result<(bool, String)> {
        let output = Command::new("pkexec")
            .args(["snap"])
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| context_msg.to_string())?;

        Ok((
            output.status.success(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

impl Default for SnapBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for SnapBackend {
    fn is_available() -> bool {
        which::which("snap").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("snap")
            .args(["list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list snap packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip header line
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();

                // Skip base snaps and snapd itself (they're system components)
                if name == "bare"
                    || name == "snapd"
                    || name.starts_with("core")
                    || name.starts_with("gnome-")
                    || name.starts_with("gtk-")
                    || name.starts_with("mesa-")
                {
                    continue;
                }

                // Skip fetching descriptions individually - too slow
                // Descriptions can be fetched on demand when viewing details
                let description = String::new();

                packages.push(Package {
                    name,
                    version,
                    available_version: None,
                    description,
                    source: PackageSource::Snap,
                    status: PackageStatus::Installed,
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

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let output = Command::new("snap")
            .args(["refresh", "--list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check snap updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip header line if present
        for line in stdout.lines() {
            // Skip header and empty lines
            if line.starts_with("Name") || line.is_empty() || line.contains("All snaps up to date")
            {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let new_version = parts[1].to_string();

                packages.push(Package {
                    name,
                    version: String::new(),
                    available_version: Some(new_version),
                    description: String::new(),
                    source: PackageSource::Snap,
                    status: PackageStatus::UpdateAvailable,
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

    async fn install(&self, name: &str) -> Result<()> {
        let (ok, stderr) = self
            .run_pkexec_snap(&["install", name], "Failed to install snap")
            .await?;

        if ok {
            Ok(())
        } else {
            anyhow::bail!(
                "Failed to install snap \"{}\"{}",
                name,
                if stderr.trim().is_empty() {
                    String::new()
                } else {
                    format!(": {}", stderr.trim())
                }
            )
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let (ok, stderr) = self
            .run_pkexec_snap(&["remove", name], "Failed to remove snap")
            .await?;

        if ok {
            Ok(())
        } else {
            anyhow::bail!(
                "Failed to remove snap \"{}\"{}",
                name,
                if stderr.trim().is_empty() {
                    String::new()
                } else {
                    format!(": {}", stderr.trim())
                }
            )
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let (ok, stderr) = self
            .run_pkexec_snap(&["refresh", name], "Failed to update snap")
            .await?;

        if ok {
            Ok(())
        } else {
            let stderr_trim = stderr.trim().to_string();
            if stderr_trim.contains("has running apps") {
                anyhow::bail!(
                    "{}\n\n{} {}\n",
                    Self::format_snap_running_error(name, &stderr_trim),
                    SUGGEST_PREFIX,
                    format_args!("{}", Self::running_apps_suggest_command(name, &stderr_trim))
                )
            }

            anyhow::bail!(
                "Failed to update snap \"{}\"{}",
                name,
                if stderr_trim.is_empty() {
                    String::new()
                } else {
                    format!(": {}", stderr_trim)
                }
            )
        }
    }

    async fn downgrade(&self, name: &str) -> Result<()> {
        let (ok, stderr) = self
            .run_pkexec_snap(&["revert", name], "Failed to downgrade snap")
            .await?;

        if ok {
            Ok(())
        } else {
            let stderr_trim = stderr.trim().to_string();
            anyhow::bail!(
                "Failed to downgrade snap \"{}\"{}\n\n{} {}\n",
                name,
                if stderr_trim.is_empty() {
                    String::new()
                } else {
                    format!(": {}", stderr_trim)
                },
                SUGGEST_PREFIX,
                format_args!("sudo snap revert {}", name)
            )
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("snap")
            .args(["find", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search snaps")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip header line
        for line in stdout.lines().skip(1).take(50) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                // Description is the rest after publisher and notes
                let description = if parts.len() > 4 {
                    parts[4..].join(" ")
                } else {
                    String::new()
                };

                packages.push(Package {
                    name,
                    version,
                    available_version: None,
                    description,
                    source: PackageSource::Snap,
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

    async fn get_cache_size(&self) -> Result<u64> {
        let disabled_revisions = self.list_disabled_revisions().await?;
        let mut total_size: u64 = 0;

        for (name, revision) in &disabled_revisions {
            if let Ok(size) = self.get_revision_size(name, revision).await {
                total_size += size;
            }
        }

        Ok(total_size)
    }

    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        let disabled_revisions = self.list_disabled_revisions().await?;
        let mut packages = Vec::new();

        for (name, revision) in disabled_revisions {
            let size = self.get_revision_size(&name, &revision).await.ok();

            packages.push(Package {
                name: name.clone(),
                version: String::new(),
                available_version: None,
                description: format!("Old revision {} (disabled)", revision),
                source: PackageSource::Snap,
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

        Ok(packages)
    }

    async fn cleanup_cache(&self) -> Result<u64> {
        let before = self.get_cache_size().await.unwrap_or(0);
        let disabled_revisions = self.list_disabled_revisions().await?;

        if disabled_revisions.is_empty() {
            return Ok(0);
        }

        let mut removed_any = false;

        for (name, revision) in &disabled_revisions {
            let (ok, _stderr) = self
                .run_pkexec_snap(
                    &["remove", name, "--revision", revision],
                    &format!("Failed to remove snap revision {} {}", name, revision),
                )
                .await?;

            if ok {
                removed_any = true;
            }
        }

        if !removed_any {
            return Ok(0);
        }

        Ok(before)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Snap
    }

    async fn get_package_commands(&self, name: &str) -> Result<Vec<(String, std::path::PathBuf)>> {
        let mut commands = Vec::new();
        let snap_bin = std::path::PathBuf::from("/snap/bin");

        if snap_bin.exists() {
            if let Ok(entries) = std::fs::read_dir(&snap_bin) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(cmd_name) = path.file_name().and_then(|n| n.to_str()) {
                        if cmd_name == name || cmd_name.starts_with(&format!("{}.", name)) {
                            commands.push((cmd_name.to_string(), path));
                        }
                    }
                }
            }
        }

        Ok(commands)
    }
}

impl SnapBackend {
    async fn list_disabled_revisions(&self) -> Result<Vec<(String, String)>> {
        let output = Command::new("snap")
            .args(["list", "--all"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list snap revisions")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut revisions = Vec::new();

        for line in stdout.lines().skip(1) {
            if line.contains("disabled") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    revisions.push((parts[0].to_string(), parts[2].to_string()));
                }
            }
        }

        Ok(revisions)
    }

    async fn get_revision_size(&self, name: &str, revision: &str) -> Result<u64> {
        let snap_path = format!("/var/lib/snapd/snaps/{}_{}.snap", name, revision);
        let path = std::path::Path::new(&snap_path);

        if path.exists() {
            let metadata = tokio::fs::metadata(path)
                .await
                .context("Failed to get snap file metadata")?;
            Ok(metadata.len())
        } else {
            Ok(0)
        }
    }
}
