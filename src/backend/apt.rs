use super::streaming::StreamLine;
use super::PackageBackend;
use super::{run_pkexec, run_pkexec_with_logs, Suggest};
use crate::models::{Package, PackageSource, PackageStatus, Repository};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;

pub struct AptBackend;

impl AptBackend {
    pub fn new() -> Self {
        Self
    }

    async fn run_dpkg_query(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("dpkg-query")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute dpkg-query command")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Parse APT sources list files to extract repository information.
    /// Parses both /etc/apt/sources.list and /etc/apt/sources.list.d/*.list files.
    fn parse_sources_list(content: &str, filename: &str) -> Vec<Repository> {
        let mut repos = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle deb822 format (.sources files) - simplified parsing
            if line.starts_with("Types:") || line.starts_with("URIs:") {
                // For deb822 format, we'll extract basic info
                continue;
            }

            // Parse traditional one-line format: deb [options] uri suite [component1] [component2] ...
            // or: deb-src [options] uri suite [component1] [component2] ...
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let repo_type = parts[0];
            if repo_type != "deb" && repo_type != "deb-src" {
                continue;
            }

            // Check if there are options in brackets
            let (url_idx, options) = if parts.len() > 1 && parts[1].starts_with('[') {
                // Find the closing bracket
                let mut end_idx = 1;
                for (i, part) in parts.iter().enumerate().skip(1) {
                    if part.ends_with(']') {
                        end_idx = i;
                        break;
                    }
                }
                (end_idx + 1, Some(parts[1..=end_idx].join(" ")))
            } else {
                (1, None)
            };

            if parts.len() <= url_idx {
                continue;
            }

            let url = parts[url_idx].to_string();
            let suite = parts.get(url_idx + 1).map(|s| s.to_string());
            let components: Vec<String> = parts
                .get(url_idx + 2..)
                .map(|c| c.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();

            // Create a descriptive name
            let name = if let Some(ref s) = suite {
                if components.is_empty() {
                    format!("{} ({} {})", filename, repo_type, s)
                } else {
                    format!(
                        "{} ({} {} {})",
                        filename,
                        repo_type,
                        s,
                        components.join(" ")
                    )
                }
            } else {
                format!("{} ({})", filename, repo_type)
            };

            // Check if the line is commented out (already filtered) or has disabled option
            let enabled = options
                .as_ref()
                .map(|o| !o.contains("enabled=no"))
                .unwrap_or(true);

            let description = if let Some(ref s) = suite {
                Some(format!("{} {} {}", repo_type, s, components.join(" ")))
            } else {
                Some(repo_type.to_string())
            };

            let mut repo = Repository::new(name, PackageSource::Apt, enabled, Some(url));
            repo.description = description;
            repos.push(repo);
        }

        repos
    }

    /// Read and parse all APT sources files
    async fn read_sources_files(&self) -> Result<Vec<Repository>> {
        let mut all_repos = Vec::new();

        // Read main sources.list
        let sources_list = Path::new("/etc/apt/sources.list");
        if sources_list.exists() {
            if let Ok(content) = tokio::fs::read_to_string(sources_list).await {
                all_repos.extend(Self::parse_sources_list(&content, "sources.list"));
            }
        }

        // Read sources.list.d/*.list files
        let sources_dir = Path::new("/etc/apt/sources.list.d");
        if sources_dir.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(sources_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("list") {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let filename = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown");
                            all_repos.extend(Self::parse_sources_list(&content, filename));
                        }
                    }
                }
            }
        }

        Ok(all_repos)
    }

    /// Get detailed package information using apt-cache show
    #[allow(dead_code)]
    async fn get_package_details(&self, name: &str) -> Result<PackageDetails> {
        let output = Command::new("apt-cache")
            .args(["show", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get package details")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut details = PackageDetails::default();

        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                match key {
                    "Homepage" => details.homepage = Some(value.to_string()),
                    "Maintainer" => details.maintainer = Some(value.to_string()),
                    "Depends" => {
                        details.dependencies = value
                            .split(',')
                            .map(|s| {
                                // Remove version constraints like " (>= 1.0)"
                                s.split('(').next().unwrap_or(s).trim().to_string()
                            })
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "Section" => details.section = Some(value.to_string()),
                    "Priority" => details.priority = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Ok(details)
    }

    #[allow(dead_code)]
    pub async fn refresh_cache(&self) -> Result<()> {
        run_pkexec(
            "apt",
            &["update"],
            "Failed to refresh package cache",
            Suggest {
                command: "sudo apt update".to_string(),
            },
        )
        .await
    }
}

/// Helper struct to hold package details from apt-cache show
#[derive(Default)]
struct PackageDetails {
    homepage: Option<String>,
    maintainer: Option<String>,
    dependencies: Vec<String>,
    #[allow(dead_code)]
    section: Option<String>,
    #[allow(dead_code)]
    priority: Option<String>,
}

impl Default for AptBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for AptBackend {
    fn is_available() -> bool {
        which::which("apt").is_ok() && which::which("dpkg-query").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // Include Installed-Size (in KB) in the query
        let output = self
            .run_dpkg_query(&[
                "-W",
                "--showformat=${Package}\\t${Version}\\t${Installed-Size}\\t${Description}\\n",
            ])
            .await?;

        let mut packages = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1].to_string();
                // Size is in KB, convert to bytes
                let size = parts
                    .get(2)
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|kb| kb * 1024);
                let description = parts.get(3).unwrap_or(&"").to_string();

                // Take only the first line of description
                let description = description.lines().next().unwrap_or("").to_string();

                packages.push(Package {
                    name,
                    version,
                    available_version: None,
                    description,
                    source: PackageSource::Apt,
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
        // First, simulate an update to get the list
        let output = Command::new("apt")
            .args(["list", "--upgradable"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check for updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines().skip(1) {
            // Skip "Listing..." header
            // Format: package/source version arch [upgradable from: old_version]
            if let Some(name) = line.split('/').next() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let new_version = parts.get(1).unwrap_or(&"").to_string();
                    let old_version = line
                        .split("from: ")
                        .nth(1)
                        .map(|s| s.trim_end_matches(']').to_string())
                        .unwrap_or_default();

                    let mut pkg = Package {
                        name: name.to_string(),
                        version: old_version,
                        available_version: Some(new_version),
                        description: String::new(),
                        source: PackageSource::Apt,
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
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        run_pkexec(
            "apt",
            &["install", "-y", "--", name],
            &format!("Failed to install package {}", name),
            Suggest {
                command: format!("sudo apt install -y -- {}", name),
            },
        )
        .await
    }

    async fn install_streaming(
        &self,
        name: &str,
        log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        match log_sender {
            Some(sender) => {
                run_pkexec_with_logs(
                    "apt",
                    &["install", "-y", "--", name],
                    &format!("Failed to install package {}", name),
                    Suggest {
                        command: format!("sudo apt install -y -- {}", name),
                    },
                    sender,
                )
                .await
            }
            None => self.install(name).await,
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        run_pkexec(
            "apt",
            &["remove", "-y", "--", name],
            &format!("Failed to remove package {}", name),
            Suggest {
                command: format!("sudo apt remove -y -- {}", name),
            },
        )
        .await
    }

    async fn remove_streaming(
        &self,
        name: &str,
        log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        match log_sender {
            Some(sender) => {
                run_pkexec_with_logs(
                    "apt",
                    &["remove", "-y", "--", name],
                    &format!("Failed to remove package {}", name),
                    Suggest {
                        command: format!("sudo apt remove -y -- {}", name),
                    },
                    sender,
                )
                .await
            }
            None => self.remove(name).await,
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        run_pkexec(
            "apt",
            &["install", "--only-upgrade", "-y", "--", name],
            &format!("Failed to update package {}", name),
            Suggest {
                command: format!("sudo apt install --only-upgrade -y -- {}", name),
            },
        )
        .await
    }

    async fn update_streaming(
        &self,
        name: &str,
        log_sender: Option<mpsc::Sender<StreamLine>>,
    ) -> Result<()> {
        match log_sender {
            Some(sender) => {
                run_pkexec_with_logs(
                    "apt",
                    &["install", "--only-upgrade", "-y", "--", name],
                    &format!("Failed to update package {}", name),
                    Suggest {
                        command: format!("sudo apt install --only-upgrade -y -- {}", name),
                    },
                    sender,
                )
                .await
            }
            None => self.update(name).await,
        }
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        // `apt-cache madison <pkg>` output:
        //  pkg | 1.2.3-1 | http://... focal/main amd64 Packages
        let output = Command::new("apt-cache")
            .args(["madison", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list package versions")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions = Vec::new();
        for line in stdout.lines() {
            let cols: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if cols.len() >= 2 && !cols[1].is_empty() {
                versions.push(cols[1].to_string());
            }
        }
        versions.sort();
        versions.dedup();
        versions.reverse(); // newest first
        Ok(versions)
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let target = format!("{}={}", name, version);
        run_pkexec(
            "apt",
            &["install", "-y", "--allow-downgrades", "--", &target],
            &format!("Failed to downgrade package {}", name),
            Suggest {
                command: format!("sudo apt install -y --allow-downgrades -- {}", target),
            },
        )
        .await
    }

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        // apt-get changelog fetches the Debian changelog for the package
        let output = Command::new("apt-get")
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
            // Convert Debian changelog format to markdown-ish format
            let changelog = stdout
                .lines()
                .take(500) // Limit to reasonable size
                .collect::<Vec<_>>()
                .join("\n");
            Ok(Some(format!("```\n{}\n```", changelog)))
        } else {
            Ok(None)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("apt-cache")
            .args(["search", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines().take(50) {
            // Limit results
            let parts: Vec<&str> = line.splitn(2, " - ").collect();
            if parts.len() == 2 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: String::new(),
                    available_version: None,
                    description: parts[1].to_string(),
                    source: PackageSource::Apt,
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

    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        self.read_sources_files().await
    }

    async fn add_repository(&self, url: &str, name: Option<&str>) -> Result<()> {
        // Use add-apt-repository if available, otherwise create a .list file
        if which::which("add-apt-repository").is_ok() {
            // add-apt-repository can handle PPA and regular URLs
            run_pkexec(
                "add-apt-repository",
                &["-y", url],
                &format!("Failed to add repository {}", url),
                Suggest {
                    command: format!("sudo add-apt-repository -y {}", url),
                },
            )
            .await?;

            // Refresh the package cache after adding repository
            self.refresh_cache().await
        } else {
            // Fallback: Create a .list file in /etc/apt/sources.list.d/
            let repo_name = name.unwrap_or("custom");
            let safe_name: String = repo_name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let filename = format!("{}.list", safe_name);
            let filepath = format!("/etc/apt/sources.list.d/{}", filename);

            // Prepare the content - assume it's a deb line if not already formatted
            let content = if url.starts_with("deb ") || url.starts_with("deb-src ") {
                format!("{}\n", url)
            } else {
                // Assume it's a URL and create a basic deb line
                format!("deb {} stable main\n", url)
            };

            // Write the file using tee with pkexec
            let mut child = std::process::Command::new("pkexec")
                .args(["tee", &filepath])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn pkexec")?;

            // Write content to stdin
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(content.as_bytes())
                    .context("Failed to write to stdin")?;
            }

            // Wait for the process to complete
            let status = child.wait().context("Failed to wait for pkexec")?;

            if !status.success() {
                anyhow::bail!(
                    "Failed to add repository. Try manually: echo '{}' | sudo tee {}",
                    content.trim(),
                    filepath
                );
            }

            // Refresh the package cache after adding repository
            self.refresh_cache().await
        }
    }

    async fn remove_repository(&self, name: &str) -> Result<()> {
        // Try to use add-apt-repository --remove if available
        if which::which("add-apt-repository").is_ok() && name.starts_with("ppa:") {
            run_pkexec(
                "add-apt-repository",
                &["--remove", "-y", name],
                &format!("Failed to remove repository {}", name),
                Suggest {
                    command: format!("sudo add-apt-repository --remove -y {}", name),
                },
            )
            .await
        } else {
            // Try to find and remove the .list file
            // The name format is typically "filename (deb suite components)"
            // Extract the filename from the repository name
            let filename = if let Some(paren_start) = name.find('(') {
                name[..paren_start].trim()
            } else {
                name
            };

            // Check if it's in sources.list.d
            let list_path = format!("/etc/apt/sources.list.d/{}", filename);
            if Path::new(&list_path).exists() {
                run_pkexec(
                    "rm",
                    &["-f", &list_path],
                    &format!("Failed to remove repository file {}", list_path),
                    Suggest {
                        command: format!("sudo rm -f {}", list_path),
                    },
                )
                .await
            } else {
                // The repository might be in the main sources.list file
                // We can't easily remove individual entries from it
                anyhow::bail!(
                    "Cannot automatically remove this repository. Please manually edit /etc/apt/sources.list or the appropriate file in /etc/apt/sources.list.d/"
                )
            }
        }
    }

    async fn get_cache_size(&self) -> Result<u64> {
        let cache_dir = Path::new("/var/cache/apt/archives");
        if !cache_dir.exists() {
            return Ok(0);
        }

        let output = Command::new("du")
            .args(["-sb", "/var/cache/apt/archives"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get cache size")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let size = stdout
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Ok(size)
    }

    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        let output = Command::new("apt")
            .args(["autoremove", "--dry-run"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check orphaned packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        let mut in_remove_section = false;

        for line in stdout.lines() {
            if line.contains("The following packages will be REMOVED:") {
                in_remove_section = true;
                continue;
            }
            if in_remove_section {
                if line.trim().is_empty() || line.starts_with("0 ") {
                    break;
                }
                for pkg_name in line.split_whitespace() {
                    let pkg_name = pkg_name.trim();
                    if !pkg_name.is_empty() && !pkg_name.starts_with('(') {
                        packages.push(Package {
                            name: pkg_name.to_string(),
                            version: String::new(),
                            available_version: None,
                            description: "Orphaned package (no longer needed)".to_string(),
                            source: PackageSource::Apt,
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
            "apt",
            &["clean"],
            "Failed to clean APT cache",
            Suggest {
                command: "sudo apt clean".to_string(),
            },
        )
        .await?;

        let after = self.get_cache_size().await.unwrap_or(0);
        Ok(before.saturating_sub(after))
    }

    async fn get_reverse_dependencies(&self, name: &str) -> Result<Vec<String>> {
        let output = Command::new("apt-cache")
            .args(["rdepends", "--installed", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get reverse dependencies")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut deps = Vec::new();

        for line in stdout.lines().skip(2) {
            let dep = line.trim().trim_start_matches('|').trim();
            if !dep.is_empty() && dep != name {
                deps.push(dep.to_string());
            }
        }

        Ok(deps)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Apt
    }

    async fn get_package_commands(&self, name: &str) -> Result<Vec<(String, std::path::PathBuf)>> {
        let output = Command::new("dpkg")
            .args(["-L", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list package files")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commands = Vec::new();

        for line in stdout.lines() {
            let path = std::path::Path::new(line.trim());
            if path.starts_with("/usr/bin")
                || path.starts_with("/usr/sbin")
                || path.starts_with("/bin")
                || path.starts_with("/sbin")
            {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !name.is_empty() {
                        commands.push((name.to_string(), path.to_path_buf()));
                    }
                }
            }
        }

        Ok(commands)
    }

    async fn check_lock_status(&self) -> super::LockStatus {
        use std::path::PathBuf;

        let lock_paths = [
            "/var/lib/dpkg/lock-frontend",
            "/var/lib/dpkg/lock",
            "/var/lib/apt/lists/lock",
            "/var/cache/apt/archives/lock",
        ];

        let mut status = super::LockStatus::default();

        for lock_path in &lock_paths {
            let path = Path::new(lock_path);
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

                        if let Some(pid) = stdout.split_whitespace().next() {
                            if let Ok(comm_output) = Command::new("ps")
                                .args(["-p", pid, "-o", "comm="])
                                .stdout(Stdio::piped())
                                .output()
                                .await
                            {
                                let comm = String::from_utf8_lossy(&comm_output.stdout);
                                if !comm.trim().is_empty() {
                                    status.lock_holder = Some(comm.trim().to_string());
                                }
                            }
                        }
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
    fn test_parse_sources_list_basic() {
        let content = r#"
# This is a comment
deb http://archive.ubuntu.com/ubuntu jammy main restricted

deb http://archive.ubuntu.com/ubuntu jammy-updates main restricted
deb-src http://archive.ubuntu.com/ubuntu jammy main restricted
"#;

        let repos = AptBackend::parse_sources_list(content, "sources.list");
        assert_eq!(repos.len(), 3);

        assert!(repos[0]
            .url
            .as_ref()
            .unwrap()
            .contains("archive.ubuntu.com"));
        assert!(repos[0].enabled);
        assert_eq!(repos[0].source, PackageSource::Apt);
    }

    #[test]
    fn test_parse_sources_list_with_options() {
        let content = r#"
deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu jammy stable
deb [arch=amd64] http://example.com/repo stable main contrib
"#;

        let repos = AptBackend::parse_sources_list(content, "docker.list");
        assert_eq!(repos.len(), 2);

        assert!(repos[0]
            .url
            .as_ref()
            .unwrap()
            .contains("download.docker.com"));
        assert!(repos[1].url.as_ref().unwrap().contains("example.com"));
    }

    #[test]
    fn test_parse_sources_list_empty_and_comments() {
        let content = r#"
# Comment line
   # Another comment with leading whitespace


# deb http://disabled.example.com/repo stable main
"#;

        let repos = AptBackend::parse_sources_list(content, "test.list");
        assert_eq!(repos.len(), 0);
    }

    #[test]
    fn test_parse_sources_list_disabled_option() {
        let content = r#"
deb [enabled=no] http://disabled.example.com/repo stable main
deb http://enabled.example.com/repo stable main
"#;

        let repos = AptBackend::parse_sources_list(content, "test.list");
        assert_eq!(repos.len(), 2);

        // First repo should be disabled
        assert!(!repos[0].enabled);
        // Second repo should be enabled
        assert!(repos[1].enabled);
    }

    #[test]
    fn test_is_available() {
        // This test verifies the availability check runs without panic
        // The result depends on whether apt/dpkg-query is installed
        let _available = AptBackend::is_available();
    }
}
