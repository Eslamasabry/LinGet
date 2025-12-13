use super::PackageBackend;
use super::{run_pkexec, Suggest};
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

pub struct DebBackend;

impl DebBackend {
    pub fn new() -> Self {
        Self
    }

    fn get_downloads_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join("Downloads"));
            dirs.push(home.join("Desktop"));
        }

        // Also check /tmp for downloaded debs
        dirs.push(PathBuf::from("/tmp"));

        dirs
    }

    async fn get_deb_info(path: &PathBuf) -> Option<(String, String, String)> {
        // Use dpkg-deb to get package info
        let output = Command::new("dpkg-deb")
            .args([
                "--show",
                "--showformat=${Package}\n${Version}\n${Description}",
            ])
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.len() >= 2 {
            let name = lines[0].to_string();
            let version = lines[1].to_string();
            let description = lines.get(2).map(|s| s.to_string()).unwrap_or_default();
            Some((name, version, description))
        } else {
            None
        }
    }
}

impl Default for DebBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for DebBackend {
    fn is_available() -> bool {
        which::which("dpkg").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        // Scan common download locations for .deb files
        for dir in Self::get_downloads_dirs() {
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "deb" {
                            if let Some((name, version, description)) =
                                Self::get_deb_info(&path).await
                            {
                                packages.push(Package {
                                    name,
                                    version,
                                    available_version: None,
                                    description,
                                    source: PackageSource::Deb,
                                    status: PackageStatus::NotInstalled,
                                    size: path.metadata().ok().map(|m| m.len()),
                                    homepage: None,
                                    license: None,
                                    maintainer: None,
                                    dependencies: Vec::new(),
                                    install_date: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // Local .deb files don't have updates
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        // Find the .deb file
        for dir in Self::get_downloads_dirs() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "deb" {
                            if let Some((pkg_name, _, _)) = Self::get_deb_info(&path).await {
                                if pkg_name == name {
                                    // Install using dpkg
                                    run_pkexec(
                                        "dpkg",
                                        &["-i"],
                                        "Failed to install .deb package",
                                        Suggest {
                                            command: format!("sudo dpkg -i \"{}\"", path.display()),
                                        },
                                    )
                                    .await?;

                                    // Fix dependencies (best-effort)
                                    let _ = run_pkexec(
                                        "apt-get",
                                        &["install", "-f", "-y"],
                                        "Failed to fix dependencies",
                                        Suggest {
                                            command: "sudo apt-get install -f -y".to_string(),
                                        },
                                    )
                                    .await;

                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }

        anyhow::bail!(".deb file for '{}' not found in Downloads", name)
    }

    async fn remove(&self, name: &str) -> Result<()> {
        // Remove using dpkg
        run_pkexec(
            "dpkg",
            &["-r", "--", name],
            &format!("Failed to remove {}", name),
            Suggest {
                command: format!("sudo dpkg -r -- {}", name),
            },
        )
        .await
    }

    async fn update(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Local .deb packages cannot be updated automatically")
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        // Can't search local .deb files
        Ok(Vec::new())
    }
}
