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
            });
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // No reliable "outdated" command without external tooling; keep empty.
        Ok(Vec::new())
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
            });

            if packages.len() >= 50 {
                break;
            }
        }

        Ok(packages)
    }
}
