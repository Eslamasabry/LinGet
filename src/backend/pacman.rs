use super::PackageBackend;
use super::{run_pkexec, Suggest};
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct PacmanBackend;

impl PacmanBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PacmanBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for PacmanBackend {
    fn is_available() -> bool {
        which::which("pacman").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // pacman -Qi
        // Parsing -Qi is verbose. Let's use -Q with custom formatting if possible or just parse lines.
        // `pacman -Q` gives "name version".
        // `expac` is better but extra dependency.
        // Let's iterate `pacman -Q` and maybe fetch details on demand or just basic info.
        // For basic list, let's use `pacman -Qi` and parse blocks.

        let output = Command::new("pacman")
            .arg("-Qi")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list installed pacman packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        let mut current_pkg = Package {
            name: String::new(),
            version: String::new(),
            available_version: None,
            description: String::new(),
            source: PackageSource::Pacman,
            status: PackageStatus::Installed,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            enrichment: None,
        };

        let mut has_data = false;

        for line in stdout.lines() {
            if line.trim().is_empty() {
                if has_data && !current_pkg.name.is_empty() {
                    packages.push(current_pkg.clone());
                    has_data = false;
                    current_pkg = Package {
                        name: String::new(),
                        version: String::new(),
                        available_version: None,
                        description: String::new(),
                        source: PackageSource::Pacman,
                        status: PackageStatus::Installed,
                        size: None,
                        homepage: None,
                        license: None,
                        maintainer: None,
                        dependencies: Vec::new(),
                        install_date: None,
                        enrichment: None,
                    };
                }
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().to_string();

                match key {
                    "Name" => {
                        current_pkg.name = value;
                        has_data = true;
                    }
                    "Version" => current_pkg.version = value,
                    "Description" => current_pkg.description = value,
                    "URL" => current_pkg.homepage = Some(value),
                    "Licenses" => current_pkg.license = Some(value),
                    "Packager" => current_pkg.maintainer = Some(value),
                    "Installed Size" => {
                        // Value e.g. "123.45 KiB" - simple parsing difficult, skip for now or store as is if field was string
                    }
                    _ => {}
                }
            }
        }

        // Push last one
        if has_data && !current_pkg.name.is_empty() {
            packages.push(current_pkg);
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // pacman -Qu lists upgradable packages
        // Requires DB sync first (pacman -Sy), but usually done by system or we can do `checkupdates` script if available
        // Let's assume `checkupdates` or `pacman -Qu`. `checkupdates` is safer as it uses temp DB.

        let cmd = if which::which("checkupdates").is_ok() {
            "checkupdates"
        } else {
            "pacman"
        };

        let mut command = Command::new(cmd);
        if cmd == "pacman" {
            command.arg("-Qu");
        }

        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check pacman updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // Format: name old_ver -> new_ver
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let name = parts[0].to_string();
                let new_version = parts[3].to_string();

                packages.push(Package {
                    name,
                    version: parts[1].to_string(),
                    available_version: Some(new_version),
                    description: String::new(),
                    source: PackageSource::Pacman,
                    status: PackageStatus::UpdateAvailable,
                    size: None,
                    homepage: None,
                    license: None,
                    maintainer: None,
                    dependencies: Vec::new(),
                    install_date: None,
                    enrichment: None,
                });
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        run_pkexec(
            "pacman",
            &["-S", "--noconfirm", "--", name],
            &format!("Failed to install pacman package {}", name),
            Suggest {
                command: format!("sudo pacman -S --noconfirm -- {}", name),
            },
        )
        .await
    }

    async fn remove(&self, name: &str) -> Result<()> {
        run_pkexec(
            "pacman",
            &["-Rs", "--noconfirm", "--", name],
            &format!("Failed to remove pacman package {}", name),
            Suggest {
                command: format!("sudo pacman -Rs --noconfirm -- {}", name),
            },
        )
        .await
    }

    async fn update(&self, name: &str) -> Result<()> {
        run_pkexec(
            "pacman",
            &["-S", "--noconfirm", "--", name],
            &format!("Failed to update pacman package {}", name),
            Suggest {
                command: format!("sudo pacman -S --noconfirm -- {}", name),
            },
        )
        .await
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // pacman -Ss query
        let output = Command::new("pacman")
            .args(["-Ss", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search pacman packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        let lines: Vec<&str> = stdout.lines().collect();
        // Output format:
        // repo/name version [installed]
        //     Description

        let mut i = 0;
        while i < lines.len() {
            let line1 = lines[i];
            if let Some((repo_name, version_status)) = line1.split_once(' ') {
                if let Some((_, name)) = repo_name.split_once('/') {
                    let version = version_status
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let description = if i + 1 < lines.len() {
                        lines[i + 1].trim().to_string()
                    } else {
                        String::new()
                    };

                    packages.push(Package {
                        name: name.to_string(),
                        version,
                        available_version: None,
                        description,
                        source: PackageSource::Pacman,
                        status: PackageStatus::NotInstalled,
                        size: None,
                        homepage: None,
                        license: None,
                        maintainer: None,
                        dependencies: Vec::new(),
                        install_date: None,
                        enrichment: None,
                    });
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }

        Ok(packages)
    }
}
