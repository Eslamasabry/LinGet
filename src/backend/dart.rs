use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct DartBackend {
    cmd: String,
}

impl DartBackend {
    pub fn new() -> Self {
        let cmd = if which::which("dart").is_ok() {
            "dart".to_string()
        } else {
            "flutter".to_string()
        };
        Self { cmd }
    }

    /// Simple semver comparison - returns true if new_ver > old_ver
    fn is_newer_version(new_ver: &str, old_ver: &str) -> bool {
        let parse_version = |s: &str| -> Vec<u64> {
            s.split(['.', '-', '+'])
                .filter_map(|p| p.parse::<u64>().ok())
                .collect()
        };

        let new_parts = parse_version(new_ver);
        let old_parts = parse_version(old_ver);

        for i in 0..new_parts.len().max(old_parts.len()) {
            let new_part = new_parts.get(i).copied().unwrap_or(0);
            let old_part = old_parts.get(i).copied().unwrap_or(0);
            if new_part > old_part {
                return true;
            } else if new_part < old_part {
                return false;
            }
        }
        false
    }

    async fn run_pub_global(&self, args: &[&str], context_msg: &str) -> Result<String> {
        let output = Command::new(&self.cmd)
            .args(["pub", "global"])
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| context_msg.to_string())?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{}: {}", context_msg, stderr.trim())
        }
    }
}

impl Default for DartBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for DartBackend {
    fn is_available() -> bool {
        which::which("dart").is_ok() || which::which("flutter").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // Output varies by version; parse the simple "name version" lines.
        let stdout = self
            .run_pub_global(&["list"], "Failed to list dart pub global packages")
            .await?;

        let mut packages = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Skip headers like "Activated packages:" if present.
            if line.ends_with(':') || line.starts_with("Activated") {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let name = parts[0].to_string();
            let version = parts.get(1).copied().unwrap_or("").to_string();

            packages.push(Package {
                name,
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Dart,
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

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let installed = self.list_installed().await.unwrap_or_default();
        if installed.is_empty() {
            return Ok(Vec::new());
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let mut packages_with_updates = Vec::new();

        // Check each package against pub.dev API (limit concurrent requests)
        for pkg in installed {
            let url = format!("https://pub.dev/api/packages/{}", pkg.name);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(latest_version) = json
                            .get("latest")
                            .and_then(|l| l.get("version"))
                            .and_then(|v| v.as_str())
                        {
                            // Compare versions - if different, there's an update
                            if !pkg.version.is_empty()
                                && latest_version != pkg.version
                                && Self::is_newer_version(latest_version, &pkg.version)
                            {
                                packages_with_updates.push(Package {
                                    name: pkg.name,
                                    version: pkg.version,
                                    available_version: Some(latest_version.to_string()),
                                    description: String::new(),
                                    source: PackageSource::Dart,
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
                    }
                }
                _ => {
                    // Skip packages that fail to fetch - might be private or renamed
                    tracing::debug!("Failed to check updates for dart package: {}", pkg.name);
                }
            }
        }

        Ok(packages_with_updates)
    }

    async fn install(&self, name: &str) -> Result<()> {
        // Activating again upgrades to the newest version.
        let _ = self
            .run_pub_global(
                &["activate", name],
                "Failed to activate dart pub global package",
            )
            .await?;
        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let _ = self
            .run_pub_global(
                &["deactivate", name],
                "Failed to deactivate dart pub global package",
            )
            .await?;
        Ok(())
    }

    async fn update(&self, name: &str) -> Result<()> {
        // Best-effort upgrade.
        let _ = self
            .run_pub_global(
                &["activate", name],
                "Failed to update dart pub global package",
            )
            .await?;
        Ok(())
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        if version.trim().is_empty() {
            anyhow::bail!("Version is required");
        }
        let _ = self
            .run_pub_global(
                &["activate", name, version],
                "Failed to activate a specific dart pub global package version",
            )
            .await?;
        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // `dart pub search <query>` exists on recent SDKs. If missing, return empty.
        let output = Command::new(&self.cmd)
            .args(["pub", "search", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search pub packages")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("Showing") || line.starts_with("Package") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let name = parts[0].to_string();
            let description = if parts.len() > 1 {
                parts[1..].join(" ")
            } else {
                String::new()
            };

            packages.push(Package {
                name,
                version: String::new(),
                available_version: None,
                description,
                source: PackageSource::Dart,
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

            if packages.len() >= 50 {
                break;
            }
        }

        Ok(packages)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Dart
    }

    async fn get_package_commands(&self, name: &str) -> Result<Vec<(String, std::path::PathBuf)>> {
        let mut commands = Vec::new();
        let pub_cache_bin = dirs::home_dir().unwrap_or_default().join(".pub-cache/bin");

        if pub_cache_bin.exists() {
            if let Ok(entries) = std::fs::read_dir(&pub_cache_bin) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(cmd_name) = path.file_name().and_then(|n| n.to_str()) {
                        if cmd_name == name || cmd_name.starts_with(&format!("{}_", name)) {
                            commands.push((cmd_name.to_string(), path));
                        }
                    }
                }
            }
        }

        Ok(commands)
    }
}
