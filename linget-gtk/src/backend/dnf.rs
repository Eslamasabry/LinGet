use super::PackageBackend;
use super::{run_pkexec, Suggest};
use crate::models::{Package, PackageSource, PackageStatus, Repository};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

fn extract_package_name_from_nevra(nevra: &str) -> String {
    let parts: Vec<&str> = nevra.rsplitn(3, '-').collect();
    if parts.len() >= 3 {
        parts[2].to_string()
    } else {
        nevra.split('-').next().unwrap_or(nevra).to_string()
    }
}

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
                    update_category: None,
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
                    update_category: None,
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
                    update_category: None,
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

    async fn get_cache_size(&self) -> Result<u64> {
        let cache_path = std::path::Path::new("/var/cache/dnf");
        if !cache_path.exists() {
            return Ok(0);
        }

        let output = Command::new("du")
            .args(["-sb", "/var/cache/dnf"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get dnf cache size")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let size = stdout
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(size)
    }

    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        let output = Command::new("dnf")
            .args(["autoremove", "--assumeno"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list dnf orphaned packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        let mut in_remove_section = false;

        for line in stdout.lines() {
            if line.contains("Removing:") || line.contains("Dependencies resolved") {
                in_remove_section = true;
                continue;
            }

            if in_remove_section && line.trim().is_empty() {
                continue;
            }

            if in_remove_section
                && (line.starts_with("Transaction Summary") || line.starts_with("Is this ok"))
            {
                break;
            }

            if in_remove_section {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && !parts[0].starts_with('=') {
                    let name = parts[0].to_string();
                    if name.chars().next().is_some_and(|c| c.is_alphabetic()) {
                        packages.push(Package {
                            name: name.clone(),
                            version: String::new(),
                            available_version: None,
                            description: format!("Unused dependency: {}", name),
                            source: PackageSource::Dnf,
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
            }
        }

        Ok(packages)
    }

    async fn cleanup_cache(&self) -> Result<u64> {
        let before = self.get_cache_size().await.unwrap_or(0);

        run_pkexec(
            "dnf",
            &["clean", "all"],
            "Failed to clean dnf cache",
            Suggest {
                command: "sudo dnf clean all".to_string(),
            },
        )
        .await?;

        let after = self.get_cache_size().await.unwrap_or(0);
        Ok(before.saturating_sub(after))
    }

    async fn get_reverse_dependencies(&self, name: &str) -> Result<Vec<String>> {
        let output = Command::new("dnf")
            .args(["repoquery", "--installed", "--whatrequires", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get reverse dependencies")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut deps = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let dep_name = extract_package_name_from_nevra(line);
            if !dep_name.is_empty() && dep_name != name && !deps.contains(&dep_name) {
                deps.push(dep_name);
            }
        }

        Ok(deps)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Dnf
    }

    async fn check_lock_status(&self) -> super::LockStatus {
        use std::path::PathBuf;

        let lock_paths = [
            "/var/run/dnf.pid",
            "/var/lib/rpm/.rpm.lock",
            "/var/cache/dnf/metadata_lock.pid",
        ];

        let mut status = super::LockStatus::default();

        if std::path::Path::new("/var/run/dnf.pid").exists() {
            if let Ok(pid_content) = std::fs::read_to_string("/var/run/dnf.pid") {
                let pid = pid_content.trim();
                let proc_path = format!("/proc/{}", pid);
                if std::path::Path::new(&proc_path).exists() {
                    status.is_locked = true;
                    status.lock_files.push(PathBuf::from("/var/run/dnf.pid"));

                    if let Ok(output) = Command::new("ps")
                        .args(["-p", pid, "-o", "comm="])
                        .stdout(Stdio::piped())
                        .output()
                        .await
                    {
                        let comm = String::from_utf8_lossy(&output.stdout);
                        if !comm.trim().is_empty() {
                            status.lock_holder = Some(comm.trim().to_string());
                        }
                    }
                }
            }
        }

        for lock_path in &lock_paths[1..] {
            let path = std::path::Path::new(lock_path);
            if path.exists() {
                if let Ok(output) = Command::new("fuser")
                    .arg(lock_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !stdout.trim().is_empty() {
                        status.is_locked = true;
                        status.lock_files.push(PathBuf::from(lock_path));
                    }
                }
            }
        }

        status
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
