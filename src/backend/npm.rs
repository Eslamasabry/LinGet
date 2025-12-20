use super::PackageBackend;
use crate::backend::SUGGEST_PREFIX;
use crate::models::{Package, PackageEnrichment, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// npm backend for managing Node.js packages installed globally via `npm install -g`
pub struct NpmBackend {
    client: reqwest::Client,
}

impl NpmBackend {
    pub fn new() -> Self {
        // Create an HTTP client for npm registry API requests
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

    /// Fetch package metadata from npm registry API
    async fn fetch_package_info(&self, name: &str) -> Option<NpmRegistryPackage> {
        let url = format!("https://registry.npmjs.org/{}", name);
        let resp = self.client.get(&url).send().await.ok()?;

        if !resp.status().is_success() {
            return None;
        }

        resp.json::<NpmRegistryPackage>().await.ok()
    }

    /// Create package enrichment from registry info
    fn create_enrichment(info: &NpmRegistryPackage) -> PackageEnrichment {
        let latest_version = info
            .dist_tags
            .as_ref()
            .and_then(|dt| dt.latest.clone());

        let version_info = latest_version
            .as_ref()
            .and_then(|v| info.versions.as_ref().and_then(|vs| vs.get(v)));

        let keywords = version_info
            .and_then(|vi| vi.keywords.clone())
            .unwrap_or_default();

        PackageEnrichment {
            icon_url: None, // npm packages don't have standard icons
            screenshots: Vec::new(),
            categories: Vec::new(), // npm doesn't categorize like other registries
            developer: info.author.as_ref().map(|a| a.name.clone()),
            rating: None, // npm doesn't provide ratings
            downloads: None, // Would require separate API call to npm download counts
            summary: info.description.clone(),
            repository: info.repository.as_ref().and_then(|r| r.url.clone()),
            keywords,
            last_updated: info.time.as_ref().and_then(|t| t.modified.clone()),
        }
    }
}

impl Default for NpmBackend {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// npm CLI JSON output structures
// ============================================================================

#[derive(Debug, Deserialize)]
struct NpmListOutput {
    dependencies: Option<std::collections::HashMap<String, NpmPackageInfo>>,
}

#[derive(Debug, Deserialize)]
struct NpmPackageInfo {
    version: Option<String>,
    #[serde(rename = "resolved")]
    _resolved: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmOutdatedEntry {
    current: Option<String>,
    #[serde(rename = "wanted")]
    _wanted: Option<String>,
    latest: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmVersions {
    One(String),
    Many(Vec<String>),
}

// ============================================================================
// npm Registry API structures (https://registry.npmjs.org)
// ============================================================================

/// Package metadata from npm registry
#[derive(Debug, Deserialize)]
struct NpmRegistryPackage {
    name: Option<String>,
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<NpmDistTags>,
    versions: Option<std::collections::HashMap<String, NpmVersionInfo>>,
    time: Option<NpmTimeInfo>,
    author: Option<NpmAuthor>,
    repository: Option<NpmRepository>,
    license: Option<NpmLicense>,
    homepage: Option<String>,
    readme: Option<String>,
    maintainers: Option<Vec<NpmMaintainer>>,
}

#[derive(Debug, Deserialize)]
struct NpmDistTags {
    latest: Option<String>,
    #[allow(dead_code)]
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmVersionInfo {
    version: Option<String>,
    description: Option<String>,
    keywords: Option<Vec<String>>,
    homepage: Option<String>,
    license: Option<NpmLicense>,
    author: Option<NpmAuthor>,
    repository: Option<NpmRepository>,
    dependencies: Option<std::collections::HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<std::collections::HashMap<String, String>>,
    dist: Option<NpmDist>,
}

#[derive(Debug, Deserialize)]
struct NpmTimeInfo {
    created: Option<String>,
    modified: Option<String>,
    #[serde(flatten)]
    versions: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmAuthor {
    Object { name: String, email: Option<String>, url: Option<String> },
    String(String),
}

impl NpmAuthor {
    fn name(&self) -> String {
        match self {
            NpmAuthor::Object { name, .. } => name.clone(),
            NpmAuthor::String(s) => {
                // Parse "Name <email> (url)" format
                s.split('<').next().unwrap_or(s).trim().to_string()
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmRepository {
    Object { url: Option<String>, #[serde(rename = "type")] _type: Option<String> },
    String(String),
}

impl NpmRepository {
    fn url(&self) -> Option<String> {
        match self {
            NpmRepository::Object { url, .. } => url.clone().map(|u| {
                // Clean up git+https:// or git:// prefixes
                u.trim_start_matches("git+")
                    .trim_start_matches("git://")
                    .trim_end_matches(".git")
                    .to_string()
            }),
            NpmRepository::String(s) => Some(s.clone()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmLicense {
    Object { #[serde(rename = "type")] license_type: Option<String> },
    String(String),
}

impl NpmLicense {
    fn name(&self) -> String {
        match self {
            NpmLicense::Object { license_type } => license_type.clone().unwrap_or_default(),
            NpmLicense::String(s) => s.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct NpmMaintainer {
    name: Option<String>,
    #[allow(dead_code)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NpmDist {
    #[allow(dead_code)]
    tarball: Option<String>,
    #[allow(dead_code)]
    shasum: Option<String>,
    #[serde(rename = "unpackedSize")]
    unpacked_size: Option<u64>,
}

#[async_trait]
impl PackageBackend for NpmBackend {
    fn is_available() -> bool {
        which::which("npm").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["list", "-g", "--depth=0", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list npm packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) = serde_json::from_str::<NpmListOutput>(&stdout) {
            if let Some(deps) = parsed.dependencies {
                for (name, info) in deps {
                    packages.push(Package {
                        name,
                        version: info.version.unwrap_or_default(),
                        available_version: None,
                        description: String::new(),
                        source: PackageSource::Npm,
                        status: PackageStatus::Installed,
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
        }

        // Enrich packages with metadata from npm registry API
        // We do this in parallel for better performance
        let enrichment_futures: Vec<_> = packages
            .iter()
            .map(|pkg| self.fetch_package_info(&pkg.name))
            .collect();

        let enrichments = futures::future::join_all(enrichment_futures).await;

        for (pkg, info_opt) in packages.iter_mut().zip(enrichments.into_iter()) {
            if let Some(info) = info_opt {
                // Extract description
                if let Some(ref desc) = info.description {
                    pkg.description = desc.clone();
                }

                // Extract homepage
                pkg.homepage = info.homepage.clone().or_else(|| {
                    info.repository.as_ref().and_then(|r| r.url())
                });

                // Extract license
                pkg.license = info.license.as_ref().map(|l| l.name());

                // Extract maintainer (first one or author)
                pkg.maintainer = info.author.as_ref().map(|a| a.name()).or_else(|| {
                    info.maintainers
                        .as_ref()
                        .and_then(|m| m.first())
                        .and_then(|m| m.name.clone())
                });

                // Extract size from latest version
                let latest_version = info.dist_tags.as_ref().and_then(|dt| dt.latest.clone());
                if let Some(ref latest) = latest_version {
                    if let Some(version_info) = info.versions.as_ref().and_then(|vs| vs.get(latest)) {
                        pkg.size = version_info.dist.as_ref().and_then(|d| d.unpacked_size);
                    }
                }

                // Add enrichment
                pkg.enrichment = Some(Self::create_enrichment(&info));
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["outdated", "-g", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check npm updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        if let Ok(parsed) =
            serde_json::from_str::<std::collections::HashMap<String, NpmOutdatedEntry>>(&stdout)
        {
            for (name, info) in parsed {
                packages.push(Package {
                    name,
                    version: info.current.unwrap_or_default(),
                    available_version: info.latest,
                    description: String::new(),
                    source: PackageSource::Npm,
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
        let status = Command::new("npm")
            .args(["install", "-g", name])
            .status()
            .await
            .context("Failed to install npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install npm package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("npm")
            .args(["uninstall", "-g", name])
            .status()
            .await
            .context("Failed to remove npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove npm package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("npm")
            .args(["update", "-g", name])
            .status()
            .await
            .context("Failed to update npm package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update npm package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        let spec = format!("{}@{}", name, version);
        let output = Command::new("npm")
            .args(["install", "-g", &spec])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to install a specific npm package version")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let lowered = stderr.to_lowercase();
        if lowered.contains("eacces")
            || lowered.contains("permission")
            || lowered.contains("access")
        {
            anyhow::bail!(
                "Failed to install {}.\n\n{} sudo npm install -g {}\n",
                spec,
                SUGGEST_PREFIX,
                spec
            );
        }

        anyhow::bail!("Failed to install {}: {}", spec, stderr);
    }

    async fn available_downgrade_versions(&self, name: &str) -> Result<Vec<String>> {
        let output = Command::new("npm")
            .args(["view", name, "versions", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to query npm versions")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed = serde_json::from_str::<NpmVersions>(&stdout).ok();
        let mut versions = match parsed {
            Some(NpmVersions::One(v)) => vec![v],
            Some(NpmVersions::Many(v)) => v,
            None => Vec::new(),
        };

        // Present newest first.
        versions.reverse();
        Ok(versions)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("npm")
            .args(["search", query, "--json", "--long"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search npm packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        #[derive(Deserialize)]
        struct NpmSearchResult {
            name: String,
            version: Option<String>,
            description: Option<String>,
        }

        if let Ok(results) = serde_json::from_str::<Vec<NpmSearchResult>>(&stdout) {
            for result in results.into_iter().take(50) {
                packages.push(Package {
                    name: result.name,
                    version: result.version.unwrap_or_default(),
                    available_version: None,
                    description: result.description.unwrap_or_default(),
                    source: PackageSource::Npm,
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
}
