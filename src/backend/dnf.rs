use super::PackageBackend;
use super::{run_pkexec, Suggest};
use crate::models::{Package, PackageSource, PackageStatus, Repository};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct DnfBackend;

impl DnfBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DnfBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for DnfBackend {
    fn is_available() -> bool {
        which::which("dnf").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // dnf repoquery --installed with size information
        // %{SIZE} returns the installed size in bytes
        let output = Command::new("dnf")
            .args([
                "repoquery",
                "--installed",
                "--queryformat",
                "%{NAME}|%{VERSION}|%{SIZE}|%{SUMMARY}",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list installed dnf packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                // Parse size (DNF returns size in bytes as a number)
                let size = parts[2].trim().parse::<u64>().ok();

                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    available_version: None,
                    description: parts[3].to_string(),
                    source: PackageSource::Dnf,
                    status: PackageStatus::Installed,
                    size,
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

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // dnf check-update
        // Output format is roughly:
        // package.arch  version  repo
        // Returns exit code 100 if updates available
        let output = Command::new("dnf")
            .arg("check-update")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check dnf updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Skip potential header lines or system messages until we see package list
        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }
            // Heuristic: lines with 3 columns are likely packages
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                // Ignore "Security:" or "Obsoleting" lines
                if parts[0].ends_with(':') {
                    continue;
                }

                // dnf output often includes architecture in name (e.g. package.x86_64)
                let name_arch = parts[0];
                let name = name_arch.split('.').next().unwrap_or(name_arch).to_string();
                let version = parts[1].to_string();

                packages.push(Package {
                    name,
                    version: String::new(), // We don't know current version easily here
                    available_version: Some(version),
                    description: String::new(),
                    source: PackageSource::Dnf,
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
            "dnf",
            &["install", "-y", "--", name],
            &format!("Failed to install dnf package {}", name),
            Suggest {
                command: format!("sudo dnf install -y -- {}", name),
            },
        )
        .await
    }

    async fn remove(&self, name: &str) -> Result<()> {
        run_pkexec(
            "dnf",
            &["remove", "-y", "--", name],
            &format!("Failed to remove dnf package {}", name),
            Suggest {
                command: format!("sudo dnf remove -y -- {}", name),
            },
        )
        .await
    }

    async fn update(&self, name: &str) -> Result<()> {
        run_pkexec(
            "dnf",
            &["update", "-y", "--", name],
            &format!("Failed to update dnf package {}", name),
            Suggest {
                command: format!("sudo dnf update -y -- {}", name),
            },
        )
        .await
    }

    async fn downgrade(&self, name: &str) -> Result<()> {
        run_pkexec(
            "dnf",
            &["downgrade", "-y", "--", name],
            &format!("Failed to downgrade dnf package {}", name),
            Suggest {
                command: format!("sudo dnf downgrade -y -- {}", name),
            },
        )
        .await
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // dnf search query
        let output = Command::new("dnf")
            .args(["search", "-q", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search dnf packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        // Output format:
        // name.arch : summary
        for line in stdout.lines().take(50) {
            // Limit results like APT
            if let Some((name_part, summary)) = line.split_once(" : ") {
                let name = name_part
                    .split('.')
                    .next()
                    .unwrap_or(name_part)
                    .trim()
                    .to_string();

                packages.push(Package {
                    name,
                    version: String::new(),
                    available_version: None,
                    description: summary.trim().to_string(),
                    source: PackageSource::Dnf,
                    status: PackageStatus::NotInstalled,
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

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        // Use `dnf repoquery` to list all available versions of a package
        // This shows all versions available in the repositories
        let output = Command::new("dnf")
            .args([
                "repoquery",
                "--showduplicates",
                "--queryformat",
                "%{VERSION}-%{RELEASE}",
                name,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list package versions")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions: Vec<String> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Remove duplicates and sort
        versions.sort();
        versions.dedup();
        versions.reverse(); // newest first
        Ok(versions)
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        // DNF supports installing a specific version with package-version syntax
        let target = format!("{}-{}", name, version);
        run_pkexec(
            "dnf",
            &["downgrade", "-y", "--", &target],
            &format!(
                "Failed to downgrade package {} to version {}",
                name, version
            ),
            Suggest {
                command: format!("sudo dnf downgrade -y -- {}", target),
            },
        )
        .await
    }

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        // DNF can show changelogs using `dnf changelog <package>`
        // This requires the yum-plugin-changelog or dnf-plugins-core package
        let output = Command::new("dnf")
            .args(["changelog", "--", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get changelog")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                return Ok(None);
            }
            // Format as markdown code block like APT does
            let changelog = stdout
                .lines()
                .take(500) // Limit to reasonable size
                .collect::<Vec<_>>()
                .join("\n");
            Ok(Some(format!("```\n{}\n```", changelog)))
        } else {
            // Changelog command may not be available on all systems
            Ok(None)
        }
    }

    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        // dnf repolist -v shows all repos with their details
        // Use simpler format: dnf repolist --all shows enabled and disabled repos
        let output = Command::new("dnf")
            .args(["repolist", "--all", "-v"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list dnf repositories")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut repos = Vec::new();

        // Parse verbose output which contains repo blocks like:
        // Repo-id      : fedora
        // Repo-name    : Fedora 39 - x86_64
        // Repo-status  : enabled
        // Repo-baseurl : https://...
        let mut current_id: Option<String> = None;
        let mut current_name: Option<String> = None;
        let mut current_enabled = true;
        let mut current_url: Option<String> = None;

        for line in stdout.lines() {
            let line = line.trim();

            if line.starts_with("Repo-id") {
                // Save previous repo if exists
                if let Some(id) = current_id.take() {
                    repos.push(Repository {
                        name: id,
                        url: current_url.take(),
                        enabled: current_enabled,
                        source: PackageSource::Dnf,
                        description: current_name.take(),
                    });
                }
                // Start new repo
                if let Some(value) = line.split(':').nth(1) {
                    current_id = Some(value.trim().to_string());
                }
                current_enabled = true;
            } else if line.starts_with("Repo-name") {
                if let Some(value) = line.split(':').nth(1) {
                    current_name = Some(value.trim().to_string());
                }
            } else if line.starts_with("Repo-status") {
                if let Some(value) = line.split(':').nth(1) {
                    current_enabled = value.trim().to_lowercase() == "enabled";
                }
            } else if line.starts_with("Repo-baseurl") {
                // URL may contain colons, so use splitn to get everything after the first colon
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() > 1 {
                    let url = parts[1].trim();
                    // Take first URL if multiple are listed (comma-separated)
                    current_url = Some(url.split(',').next().unwrap_or(url).trim().to_string());
                }
            }
        }

        // Don't forget the last repo
        if let Some(id) = current_id {
            repos.push(Repository {
                name: id,
                url: current_url,
                enabled: current_enabled,
                source: PackageSource::Dnf,
                description: current_name,
            });
        }

        Ok(repos)
    }

    async fn add_repository(&self, url: &str, name: Option<&str>) -> Result<()> {
        // dnf config-manager --add-repo <url>
        // Note: dnf-plugins-core must be installed for config-manager
        // If a name is provided, we could use it for the repo file name
        let repo_name = name.unwrap_or("custom");

        // First try to add the repo using config-manager
        run_pkexec(
            "dnf",
            &["config-manager", "--add-repo", url],
            &format!("Failed to add repository {}", url),
            Suggest {
                command: format!("sudo dnf config-manager --add-repo {}", url),
            },
        )
        .await?;

        // If a custom name was provided and is different from the URL-derived name,
        // the user may need to rename the repo file manually
        if name.is_some() {
            tracing::info!(
                "Repository added from URL. Custom name '{}' may require manual repo file configuration.",
                repo_name
            );
        }

        Ok(())
    }

    async fn remove_repository(&self, name: &str) -> Result<()> {
        // Disable the repository using config-manager
        // This is safer than deleting the repo file
        run_pkexec(
            "dnf",
            &["config-manager", "--set-disabled", name],
            &format!("Failed to disable repository {}", name),
            Suggest {
                command: format!("sudo dnf config-manager --set-disabled {}", name),
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dnf_backend_is_available() {
        // Just verify the method exists and returns a bool
        // The actual availability depends on the system
        let _ = DnfBackend::is_available();
    }

    #[test]
    fn test_dnf_backend_creation() {
        let backend = DnfBackend::new();
        let _default = DnfBackend::default();
        // Just verify the backend can be created
        drop(backend);
    }
}
