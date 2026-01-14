use crate::backends::PackageBackend;
use crate::pkexec::{run_pkexec, run_pkexec_with_logs, Suggest};
use crate::streaming::StreamLine;
use crate::traits::LockStatus;
use anyhow::{Context, Result};
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus, Repository};
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::sync::mpsc;
use which::which;

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

    fn parse_sources_list(content: &str, filename: &str) -> Vec<Repository> {
        let mut repos = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with("Types:") || line.starts_with("URIs:") {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }

            let repo_type = parts[0];
            if repo_type != "deb" && repo_type != "deb-src" {
                continue;
            }

            let (url_idx, options) = if parts.len() > 1 && parts[1].starts_with('[') {
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

    async fn read_sources_files(&self) -> Result<Vec<Repository>> {
        let mut all_repos = Vec::new();

        let sources_list = Path::new("/etc/apt/sources.list");
        if sources_list.exists() {
            if let Ok(content) = tokio::fs::read_to_string(sources_list).await {
                all_repos.extend(Self::parse_sources_list(&content, "sources.list"));
            }
        }

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
}

impl Default for AptBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for AptBackend {
    fn is_available() -> bool {
        which("apt").is_ok() && which("dpkg-query").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
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
                let size = parts
                    .get(2)
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|kb| kb * 1024);
                let description = parts.get(3).unwrap_or(&"").to_string();
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
                });
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
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
                    };
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
                });
            }
        }

        Ok(packages)
    }

    async fn list_repositories(&self) -> Result<Vec<Repository>> {
        self.read_sources_files().await
    }

    async fn add_repository(&self, url: &str, name: Option<&str>) -> Result<()> {
        if which("add-apt-repository").is_ok() {
            run_pkexec(
                "add-apt-repository",
                &["-y", url],
                &format!("Failed to add repository {}", url),
                Suggest {
                    command: format!("sudo add-apt-repository -y {}", url),
                },
            )
            .await
        } else {
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

            let content = if url.starts_with("deb ") || url.starts_with("deb-src ") {
                format!("{}\n", url)
            } else {
                format!("deb {} stable main\n", url)
            };

            let mut child = std::process::Command::new("pkexec")
                .args(["tee", &filepath])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to spawn pkexec")?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(content.as_bytes())
                    .context("Failed to write to stdin")?;
            }

            let status = child.wait().context("Failed to wait for pkexec")?;

            if !status.success() {
                anyhow::bail!(
                    "Failed to add repository. Try manually: echo '{}' | sudo tee {}",
                    content.trim(),
                    filepath
                );
            }

            Ok(())
        }
    }

    async fn remove_repository(&self, name: &str) -> Result<()> {
        if which("add-apt-repository").is_ok() && name.starts_with("ppa:") {
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
            let filename = if let Some(paren_start) = name.find('(') {
                name[..paren_start].trim()
            } else {
                name
            };

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

    async fn check_lock_status(&self) -> LockStatus {
        let lock_paths = [
            "/var/lib/dpkg/lock-frontend",
            "/var/lib/dpkg/lock",
            "/var/lib/apt/lists/lock",
            "/var/cache/apt/archives/lock",
        ];

        let mut status = LockStatus::default();

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
                        status.lock_files.push(Path::new(lock_path).to_path_buf());

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
