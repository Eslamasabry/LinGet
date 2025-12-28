use super::PackageBackend;
use crate::backend::SUGGEST_PREFIX;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

pub struct PipBackend;

impl PipBackend {
    pub fn new() -> Self {
        Self
    }

    fn get_pip_command() -> &'static str {
        // Try pip3 first, then pip
        if which::which("pip3").is_ok() {
            "pip3"
        } else {
            "pip"
        }
    }
}

impl Default for PipBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct PipPackageInfo {
    name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct PipOutdatedInfo {
    name: String,
    version: String,
    latest_version: String,
}

#[async_trait]
impl PackageBackend for PipBackend {
    fn is_available() -> bool {
        which::which("pip3").is_ok() || which::which("pip").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["list", "--format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list pip packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<Vec<PipPackageInfo>>(&stdout) {
            for pkg in parsed {
                packages.push(Package {
                    name: pkg.name,
                    version: pkg.version,
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Pip,
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
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["list", "--outdated", "--format=json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check pip updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<Vec<PipOutdatedInfo>>(&stdout) {
            for pkg in parsed {
                packages.push(Package {
                    name: pkg.name,
                    version: pkg.version,
                    available_version: Some(pkg.latest_version),
                    description: String::new(),
                    source: PackageSource::Pip,
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
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["install", "--user", name])
            .status()
            .await
            .context("Failed to install pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install pip package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["uninstall", "-y", name])
            .status()
            .await
            .context("Failed to remove pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove pip package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let status = Command::new(pip)
            .args(["install", "--user", "--upgrade", name])
            .status()
            .await
            .context("Failed to update pip package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update pip package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let pip = Self::get_pip_command();
        let spec = format!("{}=={}", name, version);
        let output = Command::new(pip)
            .args(["install", "--user", &spec])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to install a specific pip package version")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let lowered = stderr.to_lowercase();
        if lowered.contains("can not perform a '--user' install")
            || lowered.contains("user site-packages are not visible")
        {
            anyhow::bail!(
                "pip rejected a --user install (likely a virtualenv).\n\n{} {} install {}\n",
                SUGGEST_PREFIX,
                pip,
                spec
            );
        }

        anyhow::bail!("Failed to install {}: {}", spec, stderr);
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["index", "versions", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let Ok(output) = output else {
            return Ok(Vec::new());
        };
        if !output.status.success() {
            return Ok(Vec::new());
        }

        // Output example:
        //   <name> (<installed>) - Available versions: x, y, z
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut versions: Vec<String> = Vec::new();
        for line in stdout.lines() {
            let Some((_, tail)) = line.split_once("Available versions:") else {
                continue;
            };
            for v in tail.split(',') {
                let v = v.trim();
                if !v.is_empty() {
                    versions.push(v.to_string());
                }
            }
        }

        Ok(versions)
    }

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        // Fetch release info from PyPI JSON API
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let url = format!("https://pypi.org/pypi/{}/json", name);
        let resp = client.get(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(json) = r.json::<serde_json::Value>().await {
                    if let Some(releases) = json.get("releases").and_then(|v| v.as_object()) {
                        let mut changelog = String::new();
                        changelog.push_str(&format!("# {} Release History\n\n", name));

                        // Get versions sorted by date (most recent first)
                        let mut version_dates: Vec<(&str, &str)> = releases
                            .iter()
                            .filter_map(|(version, files)| {
                                files.as_array().and_then(|arr| {
                                    arr.first().and_then(|f| {
                                        f.get("upload_time")
                                            .and_then(|t| t.as_str())
                                            .map(|date| (version.as_str(), date))
                                    })
                                })
                            })
                            .collect();

                        version_dates.sort_by(|a, b| b.1.cmp(a.1));

                        for (i, (version, date)) in version_dates.iter().take(20).enumerate() {
                            let date_part = date.split('T').next().unwrap_or(date);
                            if i == 0 {
                                changelog.push_str(&format!("## v{} (Latest)\n", version));
                            } else {
                                changelog.push_str(&format!("## v{}\n", version));
                            }
                            changelog.push_str(&format!("*Released: {}*\n\n", date_part));
                        }

                        if version_dates.len() > 20 {
                            changelog.push_str(&format!(
                                "\n*...and {} more releases*\n",
                                version_dates.len() - 20
                            ));
                        }

                        return Ok(Some(changelog));
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let url = format!("https://pypi.org/pypi/{}/json", query);
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(info) = json.get("info") {
                        let name = info
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(query)
                            .to_string();
                        let version = info
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let description = info
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let homepage = info
                            .get("home_page")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .or_else(|| info.get("project_url").and_then(|v| v.as_str()))
                            .map(|s| s.to_string());
                        let license = info
                            .get("license")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());
                        let maintainer = info
                            .get("author")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string());

                        packages.push(Package {
                            name,
                            version,
                            available_version: None,
                            description,
                            source: PackageSource::Pip,
                            status: PackageStatus::NotInstalled,
                            size: None,
                            homepage,
                            license,
                            maintainer,
                            dependencies: Vec::new(),
                            install_date: None,
                            update_category: None,
                            enrichment: None,
                        });
                    }
                }
            }
        }

        if packages.is_empty() {
            let query_lower = query.to_lowercase();
            if query_lower != query {
                let url = format!("https://pypi.org/pypi/{}/json", query_lower);
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            if let Some(info) = json.get("info") {
                                let name = info
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&query_lower)
                                    .to_string();
                                let version = info
                                    .get("version")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let description = info
                                    .get("summary")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                packages.push(Package {
                                    name,
                                    version,
                                    available_version: None,
                                    description,
                                    source: PackageSource::Pip,
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
                    }
                }
            }
        }

        Ok(packages)
    }

    async fn get_cache_size(&self) -> Result<u64> {
        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["cache", "info"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get pip cache info")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut total_size: u64 = 0;

        for line in stdout.lines() {
            if line.contains("Size:") {
                if let Some(size_str) = line.split(':').nth(1) {
                    let size_str = size_str.trim();
                    total_size = Self::parse_pip_size(size_str);
                }
            }
        }

        Ok(total_size)
    }

    async fn get_orphaned_packages(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn cleanup_cache(&self) -> Result<u64> {
        let before = self.get_cache_size().await.unwrap_or(0);

        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["cache", "purge"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to purge pip cache")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to purge pip cache: {}", stderr);
        }

        Ok(before)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Pip
    }

    async fn get_package_commands(&self, name: &str) -> Result<Vec<(String, std::path::PathBuf)>> {
        let output = Command::new("python3")
            .args([
                "-c",
                "import sys\nimport importlib.metadata\ntry:\n    dist = importlib.metadata.distribution(sys.argv[1])\n    try:\n        eps = dist.entry_points.select(group='console_scripts')\n    except Exception:\n        eps = [e for e in dist.entry_points if getattr(e, 'group', None) == 'console_scripts']\n    for e in eps:\n        print(e.name)\nexcept Exception:\n    pass\n",
                name,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to query python entry points")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commands = Vec::new();
        for cmd in stdout.lines().map(str::trim).filter(|s| !s.is_empty()) {
            if let Ok(path) = which::which(cmd) {
                commands.push((cmd.to_string(), path));
            }
        }

        if !commands.is_empty() {
            return Ok(commands);
        }

        let pip = Self::get_pip_command();
        let output = Command::new(pip)
            .args(["show", "--files", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to get package files")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut location: Option<std::path::PathBuf> = None;
        let mut in_files_section = false;

        for line in stdout.lines() {
            if let Some(rest) = line.strip_prefix("Location:") {
                let loc = rest.trim();
                if !loc.is_empty() {
                    location = Some(std::path::PathBuf::from(loc));
                }
            }

            if line.starts_with("Files:") {
                in_files_section = true;
                continue;
            }

            if in_files_section && line.starts_with("  ") {
                let file = line.trim();
                if let Some(cmd_name) = file.rsplit('/').next() {
                    if cmd_name.is_empty() {
                        continue;
                    }

                    if file.contains("/bin/") || file.starts_with("../../../bin/") {
                        let path = location
                            .as_ref()
                            .map(|loc| loc.join(file))
                            .unwrap_or_else(|| std::path::PathBuf::from(cmd_name));
                        commands.push((cmd_name.to_string(), path));
                    }
                }
            } else if in_files_section && !line.starts_with("  ") && !line.is_empty() {
                break;
            }
        }

        Ok(commands)
    }
}

impl PipBackend {
    fn parse_pip_size(s: &str) -> u64 {
        let s = s.trim();
        let mut num_end = 0;
        for (i, c) in s.char_indices() {
            if c.is_ascii_digit() || c == '.' {
                num_end = i + c.len_utf8();
            } else if !c.is_whitespace() {
                break;
            }
        }
        let num: f64 = s[..num_end].trim().parse().unwrap_or(0.0);
        let unit = s[num_end..].trim().to_lowercase();
        let multiplier: u64 = match unit.as_str() {
            "b" | "bytes" => 1,
            "kb" | "kib" => 1024,
            "mb" | "mib" => 1024 * 1024,
            "gb" | "gib" => 1024 * 1024 * 1024,
            _ => 1,
        };
        (num * multiplier as f64) as u64
    }
}
