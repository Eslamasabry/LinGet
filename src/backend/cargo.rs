use super::PackageBackend;
use crate::models::{Package, PackageEnrichment, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Cargo backend for managing Rust crates installed via `cargo install`
pub struct CargoBackend {
    client: reqwest::Client,
}

impl CargoBackend {
    pub fn new() -> Self {
        // Create an HTTP client with proper User-Agent (required by crates.io)
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("linget (https://github.com/linget/linget)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Simple semver comparison - returns true if new_ver > old_ver
    fn is_newer_version(new_ver: &str, old_ver: &str) -> bool {
        let parse_version = |s: &str| -> Vec<u64> {
            // Split on '.', '-', '+' to handle pre-release versions
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

    /// Fetch crate metadata from crates.io API
    async fn fetch_crate_info(&self, name: &str) -> Option<CrateInfo> {
        let url = format!("https://crates.io/api/v1/crates/{}", name);
        let resp = self.client.get(&url).send().await.ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let json: serde_json::Value = resp.json().await.ok()?;
        let crate_data = json.get("crate")?;

        Some(CrateInfo {
            max_version: crate_data
                .get("max_version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: crate_data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            homepage: crate_data
                .get("homepage")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            repository: crate_data
                .get("repository")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            documentation: crate_data
                .get("documentation")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            downloads: crate_data.get("downloads").and_then(|v| v.as_u64()),
            categories: json
                .get("categories")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| c.get("category").and_then(|v| v.as_str()))
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default(),
            keywords: json
                .get("keywords")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|k| k.get("keyword").and_then(|v| v.as_str()))
                        .map(|s| s.to_string())
                        .collect()
                })
                .unwrap_or_default(),
            updated_at: crate_data
                .get("updated_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    /// Fetch available versions for a crate
    async fn fetch_crate_versions(&self, name: &str) -> Result<Vec<CrateVersion>> {
        let url = format!("https://crates.io/api/v1/crates/{}/versions", name);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let json: serde_json::Value = resp.json().await?;
        let versions = json
            .get("versions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let num = v.get("num").and_then(|n| n.as_str())?;
                        let yanked = v.get("yanked").and_then(|y| y.as_bool()).unwrap_or(false);
                        let created_at = v
                            .get("created_at")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string());
                        Some(CrateVersion {
                            num: num.to_string(),
                            yanked,
                            created_at,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(versions)
    }

    /// Create package enrichment from crate info
    fn create_enrichment(info: &CrateInfo) -> PackageEnrichment {
        PackageEnrichment {
            icon_url: None, // Cargo doesn't have icons
            screenshots: Vec::new(),
            categories: info.categories.clone(),
            developer: None,
            rating: None,
            downloads: info.downloads,
            summary: if info.description.is_empty() {
                None
            } else {
                Some(info.description.clone())
            },
            repository: info.repository.clone(),
            keywords: info.keywords.clone(),
            last_updated: info.updated_at.clone(),
        }
    }
}

impl Default for CargoBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Crate information from crates.io API
#[derive(Debug, Clone, Deserialize)]
struct CrateInfo {
    max_version: String,
    description: String,
    homepage: Option<String>,
    repository: Option<String>,
    documentation: Option<String>,
    downloads: Option<u64>,
    categories: Vec<String>,
    keywords: Vec<String>,
    updated_at: Option<String>,
}

/// Version information for a crate
#[derive(Debug, Clone)]
struct CrateVersion {
    num: String,
    yanked: bool,
    created_at: Option<String>,
}

#[async_trait]
impl PackageBackend for CargoBackend {
    fn is_available() -> bool {
        which::which("cargo").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("cargo")
            .args(["install", "--list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run 'cargo install --list'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to list installed cargo crates: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // Format: "package_name v1.2.3:" followed by binary names on subsequent lines
            if !line.ends_with(':') {
                continue;
            }

            let header = line.trim_end_matches(':').trim();
            let Some((name, version_part)) = header.split_once(' ') else {
                continue;
            };

            let version = version_part.trim_start_matches('v').to_string();
            if name.is_empty() {
                continue;
            }

            packages.push(Package {
                name: name.to_string(),
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Cargo,
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

        // Optionally enrich packages with metadata from crates.io
        // We do this in parallel for better performance
        let enrichment_futures: Vec<_> = packages
            .iter()
            .map(|pkg| self.fetch_crate_info(&pkg.name))
            .collect();

        let enrichments = futures::future::join_all(enrichment_futures).await;

        for (pkg, info_opt) in packages.iter_mut().zip(enrichments.into_iter()) {
            if let Some(info) = info_opt {
                pkg.description = info.description.clone();
                pkg.homepage = info.homepage.clone().or(info.repository.clone());
                pkg.enrichment = Some(Self::create_enrichment(&info));
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let installed = self.list_installed().await.unwrap_or_default();
        if installed.is_empty() {
            return Ok(Vec::new());
        }

        let mut packages_with_updates = Vec::new();

        // Check each package against crates.io API
        for pkg in installed {
            if let Some(info) = self.fetch_crate_info(&pkg.name).await {
                // Compare versions - if different and newer, there's an update
                if !pkg.version.is_empty()
                    && info.max_version != pkg.version
                    && Self::is_newer_version(&info.max_version, &pkg.version)
                {
                    // Create enrichment before consuming the info struct
                    let enrichment = Self::create_enrichment(&info);
                    let homepage = info.homepage.or(info.repository);

                    let mut pkg = Package {
                        name: pkg.name,
                        version: pkg.version,
                        available_version: Some(info.max_version),
                        description: info.description,
                        source: PackageSource::Cargo,
                        status: PackageStatus::UpdateAvailable,
                        size: None,
                        homepage,
                        license: None,
                        maintainer: None,
                        dependencies: Vec::new(),
                        install_date: None,
                        update_category: None,
                        enrichment: Some(enrichment),
                    };
                    pkg.update_category = Some(pkg.detect_update_category());
                    packages_with_updates.push(pkg);
                }
            } else {
                // Skip packages that fail to fetch - might be yanked or renamed
                tracing::debug!("Failed to check updates for cargo crate: {}", pkg.name);
            }
        }

        Ok(packages_with_updates)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = Command::new("cargo")
            .args(["install", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run cargo install")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let lowered = stderr.to_lowercase();

        // Provide helpful error messages
        if lowered.contains("could not find") || lowered.contains("no crate") {
            anyhow::bail!(
                "Crate '{}' not found on crates.io. Check the name and try again.",
                name
            );
        }

        if lowered.contains("failed to compile") {
            anyhow::bail!(
                "Failed to compile crate '{}'. This may require additional system dependencies.\n\n{}",
                name,
                stderr.lines().take(10).collect::<Vec<_>>().join("\n")
            );
        }

        if lowered.contains("rustup") || lowered.contains("toolchain") {
            anyhow::bail!(
                "Rust toolchain issue detected. Try running 'rustup update' first.\n\n{}",
                stderr.trim()
            );
        }

        anyhow::bail!(
            "Failed to install cargo crate '{}': {}",
            name,
            stderr.trim()
        )
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = Command::new("cargo")
            .args(["uninstall", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run cargo uninstall")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let lowered = stderr.to_lowercase();

        if lowered.contains("not installed") || lowered.contains("is not installed") {
            anyhow::bail!("Crate '{}' is not installed.", name);
        }

        anyhow::bail!(
            "Failed to uninstall cargo crate '{}': {}",
            name,
            stderr.trim()
        )
    }

    async fn update(&self, name: &str) -> Result<()> {
        // Reinstall with --force to pull the latest version
        let output = Command::new("cargo")
            .args(["install", name, "--force"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run cargo install --force")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let lowered = stderr.to_lowercase();

        if lowered.contains("failed to compile") {
            anyhow::bail!(
                "Failed to compile crate '{}' during update. This may require additional system dependencies.\n\n{}",
                name,
                stderr.lines().take(10).collect::<Vec<_>>().join("\n")
            );
        }

        anyhow::bail!("Failed to update cargo crate '{}': {}", name, stderr.trim())
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let output = Command::new("cargo")
            .args(["install", name, "--version", version, "--force"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run cargo install with specific version")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let lowered = stderr.to_lowercase();

        if lowered.contains("could not find") || lowered.contains("no matching") {
            // Check if the version exists
            if let Ok(versions) = self.fetch_crate_versions(name).await {
                let available: Vec<_> = versions
                    .iter()
                    .filter(|v| !v.yanked)
                    .take(5)
                    .map(|v| v.num.as_str())
                    .collect();

                if !available.is_empty() {
                    anyhow::bail!(
                        "Version '{}' not found for '{}'. Available versions: {}",
                        version,
                        name,
                        available.join(", ")
                    );
                }
            }
            anyhow::bail!("Version '{}' not found for crate '{}'", version, name);
        }

        anyhow::bail!("Failed to install {} v{}: {}", name, version, stderr.trim())
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        let versions = self.fetch_crate_versions(name).await?;

        // Filter out yanked versions and return version numbers
        let available: Vec<String> = versions
            .into_iter()
            .filter(|v| !v.yanked)
            .map(|v| v.num)
            .collect();

        Ok(available)
    }

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        let versions = self.fetch_crate_versions(name).await?;

        if versions.is_empty() {
            return Ok(None);
        }

        let mut changelog = String::new();
        changelog.push_str(&format!("# {} Version History\n\n", name));

        // Fetch crate info for description
        if let Some(info) = self.fetch_crate_info(name).await {
            if !info.description.is_empty() {
                changelog.push_str(&format!("_{}_\n\n", info.description));
            }

            if let Some(ref repo) = info.repository {
                changelog.push_str(&format!("**Repository:** {}\n", repo));
            }

            if let Some(ref docs) = info.documentation {
                changelog.push_str(&format!("**Documentation:** {}\n", docs));
            }

            if let Some(downloads) = info.downloads {
                changelog.push_str(&format!(
                    "**Total Downloads:** {}\n",
                    format_downloads(downloads)
                ));
            }

            if !info.categories.is_empty() {
                changelog.push_str(&format!("**Categories:** {}\n", info.categories.join(", ")));
            }

            if !info.keywords.is_empty() {
                changelog.push_str(&format!("**Keywords:** {}\n", info.keywords.join(", ")));
            }

            changelog.push_str("\n---\n\n");
        }

        changelog.push_str("## Release History\n\n");

        for (i, ver) in versions.iter().take(20).enumerate() {
            let date = ver
                .created_at
                .as_ref()
                .map(|s| s.split('T').next().unwrap_or(s))
                .unwrap_or("Unknown date");

            if i == 0 {
                changelog.push_str(&format!("### v{} (Latest)\n", ver.num));
            } else if ver.yanked {
                changelog.push_str(&format!("### ~~v{}~~ (Yanked)\n", ver.num));
            } else {
                changelog.push_str(&format!("### v{}\n", ver.num));
            }
            changelog.push_str(&format!("*Released: {}*\n\n", date));
        }

        if versions.len() > 20 {
            changelog.push_str(&format!(
                "\n*...and {} more versions on crates.io*\n",
                versions.len() - 20
            ));
        }

        Ok(Some(changelog))
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // Use cargo search for basic results
        let output = Command::new("cargo")
            .args(["search", query, "--limit", "50"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run cargo search")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            // Format: "name = \"version\"    # description"
            let Some((name, rest)) = line.split_once(" = ") else {
                continue;
            };

            let version = rest.split('"').nth(1).unwrap_or("").to_string();
            let description = rest
                .split('#')
                .nth(1)
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            let name = name.trim();
            if name.is_empty() {
                continue;
            }

            packages.push(Package {
                name: name.to_string(),
                version,
                available_version: None,
                description,
                source: PackageSource::Cargo,
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

        // Enrich search results with additional metadata from crates.io
        // Limit enrichment to first 10 results to avoid rate limiting
        let enrichment_futures: Vec<_> = packages
            .iter()
            .take(10)
            .map(|pkg| self.fetch_crate_info(&pkg.name))
            .collect();

        let enrichments = futures::future::join_all(enrichment_futures).await;

        for (pkg, info_opt) in packages.iter_mut().take(10).zip(enrichments.into_iter()) {
            if let Some(info) = info_opt {
                if pkg.description.is_empty() || pkg.description.len() < info.description.len() {
                    pkg.description = info.description.clone();
                }
                pkg.homepage = info.homepage.clone().or(info.repository.clone());
                pkg.enrichment = Some(Self::create_enrichment(&info));
            }
        }

        Ok(packages)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Cargo
    }

    async fn get_package_commands(&self, name: &str) -> Result<Vec<(String, std::path::PathBuf)>> {
        let output = Command::new("cargo")
            .args(["install", "--list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list cargo packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commands = Vec::new();
        let mut current_crate: Option<&str> = None;

        for line in stdout.lines() {
            if !line.starts_with("    ") {
                let crate_name = line.split_whitespace().next().unwrap_or("");
                current_crate = Some(crate_name);
            } else if let Some(crate_name) = current_crate {
                if crate_name == name {
                    let bin_name = line.trim().trim_end_matches(':');
                    if let Ok(path) = which::which(bin_name) {
                        commands.push((bin_name.to_string(), path));
                    }
                }
            }
        }

        Ok(commands)
    }
}

fn format_downloads(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        // Basic version comparisons
        assert!(CargoBackend::is_newer_version("1.0.1", "1.0.0"));
        assert!(CargoBackend::is_newer_version("1.1.0", "1.0.0"));
        assert!(CargoBackend::is_newer_version("2.0.0", "1.9.9"));

        // Equal versions
        assert!(!CargoBackend::is_newer_version("1.0.0", "1.0.0"));

        // Older versions
        assert!(!CargoBackend::is_newer_version("1.0.0", "1.0.1"));
        assert!(!CargoBackend::is_newer_version("1.0.0", "2.0.0"));

        // Pre-release versions with numeric suffixes
        // Note: Our simple parser only handles numeric parts, so "beta" and "rc" are stripped
        assert!(CargoBackend::is_newer_version("1.0.0-2", "1.0.0-1"));
        assert!(CargoBackend::is_newer_version("1.0.0.2", "1.0.0.1"));

        // Build metadata
        assert!(CargoBackend::is_newer_version("1.0.1+build", "1.0.0+build"));

        // Different length versions
        assert!(CargoBackend::is_newer_version("1.0.0.1", "1.0.0"));
        assert!(!CargoBackend::is_newer_version("1.0.0", "1.0.0.1"));
    }

    #[test]
    fn test_format_downloads() {
        assert_eq!(format_downloads(500), "500");
        assert_eq!(format_downloads(1500), "1.5K");
        assert_eq!(format_downloads(1_500_000), "1.5M");
    }
}
