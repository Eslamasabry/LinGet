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
        let latest_version = info.dist_tags.as_ref().and_then(|dt| dt.latest.clone());

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
            developer: info.author.as_ref().map(|a| a.name()),
            rating: None,    // npm doesn't provide ratings
            downloads: None, // Would require separate API call to npm download counts
            summary: info.description.clone(),
            repository: info.repository.as_ref().and_then(|r| r.url()),
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
    description: Option<String>,
    #[serde(rename = "dist-tags")]
    dist_tags: Option<NpmDistTags>,
    versions: Option<std::collections::HashMap<String, NpmVersionInfo>>,
    time: Option<NpmTimeInfo>,
    author: Option<NpmAuthor>,
    repository: Option<NpmRepository>,
    license: Option<NpmLicense>,
    homepage: Option<String>,
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
    keywords: Option<Vec<String>>,
    dist: Option<NpmDist>,
}

#[derive(Debug, Deserialize)]
struct NpmTimeInfo {
    modified: Option<String>,
    #[serde(flatten)]
    versions: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NpmAuthor {
    Object { name: String },
    String(String),
}

impl NpmAuthor {
    fn name(&self) -> String {
        match self {
            NpmAuthor::Object { name } => name.clone(),
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
    Object {
        url: Option<String>,
        #[serde(rename = "type")]
        _type: Option<String>,
    },
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
    Object {
        #[serde(rename = "type")]
        license_type: Option<String>,
    },
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
                pkg.homepage = info
                    .homepage
                    .clone()
                    .or_else(|| info.repository.as_ref().and_then(|r| r.url()));

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
                    if let Some(version_info) = info.versions.as_ref().and_then(|vs| vs.get(latest))
                    {
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

        // Enrich packages with metadata from npm registry
        let enrichment_futures: Vec<_> = packages
            .iter()
            .map(|pkg| self.fetch_package_info(&pkg.name))
            .collect();

        let enrichments = futures::future::join_all(enrichment_futures).await;

        for (pkg, info_opt) in packages.iter_mut().zip(enrichments.into_iter()) {
            if let Some(info) = info_opt {
                if let Some(ref desc) = info.description {
                    pkg.description.clone_from(desc);
                }
                pkg.homepage = info
                    .homepage
                    .clone()
                    .or_else(|| info.repository.as_ref().and_then(|r| r.url()));
                pkg.license = info.license.as_ref().map(|l| l.name());
                pkg.maintainer = info.author.as_ref().map(|a| a.name());
                pkg.enrichment = Some(Self::create_enrichment(&info));
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let output = Command::new("npm")
            .args(["install", "-g", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run npm install")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let lowered = stderr.to_lowercase();

        // Permission errors
        if lowered.contains("eacces")
            || lowered.contains("permission denied")
            || lowered.contains("access")
        {
            anyhow::bail!(
                "Failed to install npm package '{}'.\n\n{} sudo npm install -g {}\n",
                name,
                SUGGEST_PREFIX,
                name
            );
        }

        // Package not found
        if lowered.contains("404") || lowered.contains("not found") || lowered.contains("e404") {
            anyhow::bail!(
                "Package '{}' not found on npm registry. Check the name and try again.",
                name
            );
        }

        // Network errors
        if lowered.contains("network")
            || lowered.contains("enotfound")
            || lowered.contains("etimedout")
        {
            anyhow::bail!(
                "Network error while installing '{}'. Check your internet connection and try again.\n\n{}",
                name,
                stderr.lines().take(5).collect::<Vec<_>>().join("\n")
            );
        }

        anyhow::bail!(
            "Failed to install npm package '{}': {}",
            name,
            stderr.trim()
        )
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let output = Command::new("npm")
            .args(["uninstall", "-g", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run npm uninstall")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let lowered = stderr.to_lowercase();

        // Permission errors
        if lowered.contains("eacces") || lowered.contains("permission") {
            anyhow::bail!(
                "Failed to remove npm package '{}'.\n\n{} sudo npm uninstall -g {}\n",
                name,
                SUGGEST_PREFIX,
                name
            );
        }

        // Package not installed
        if lowered.contains("not installed") {
            anyhow::bail!("Package '{}' is not installed globally.", name);
        }

        anyhow::bail!("Failed to remove npm package '{}': {}", name, stderr.trim())
    }

    async fn update(&self, name: &str) -> Result<()> {
        // npm update -g doesn't work well for specific packages
        // Use install -g to get the latest version
        let output = Command::new("npm")
            .args(["install", "-g", &format!("{}@latest", name)])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run npm install for update")?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let lowered = stderr.to_lowercase();

        // Permission errors
        if lowered.contains("eacces") || lowered.contains("permission") {
            anyhow::bail!(
                "Failed to update npm package '{}'.\n\n{} sudo npm install -g {}@latest\n",
                name,
                SUGGEST_PREFIX,
                name
            );
        }

        anyhow::bail!("Failed to update npm package '{}': {}", name, stderr.trim())
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

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        // Fetch package info from npm registry
        let Some(info) = self.fetch_package_info(name).await else {
            return Ok(None);
        };

        let mut changelog = String::new();
        changelog.push_str(&format!("# {} Release History\n\n", name));

        // Add description
        if let Some(ref desc) = info.description {
            if !desc.is_empty() {
                changelog.push_str(&format!("_{}_\n\n", desc));
            }
        }

        // Add repository link
        if let Some(ref repo) = info.repository {
            if let Some(url) = repo.url() {
                changelog.push_str(&format!("**Repository:** {}\n", url));
            }
        }

        // Add homepage
        if let Some(ref homepage) = info.homepage {
            if !homepage.is_empty() {
                changelog.push_str(&format!("**Homepage:** {}\n", homepage));
            }
        }

        // Add license
        if let Some(ref license) = info.license {
            let license_name = license.name();
            if !license_name.is_empty() {
                changelog.push_str(&format!("**License:** {}\n", license_name));
            }
        }

        // Add author/maintainers
        if let Some(ref author) = info.author {
            changelog.push_str(&format!("**Author:** {}\n", author.name()));
        }

        changelog.push_str("\n---\n\n");
        changelog.push_str("## Version History\n\n");

        // Get version timeline from time field
        if let Some(ref time) = info.time {
            // Collect versions with their release dates
            let mut version_dates: Vec<(&str, &str)> = time
                .versions
                .iter()
                .filter(|(k, _)| *k != "created" && *k != "modified")
                .map(|(v, d)| (v.as_str(), d.as_str()))
                .collect();

            // Sort by date (newest first)
            version_dates.sort_by(|a, b| b.1.cmp(a.1));

            // Get the latest version tag
            let latest_version = info.dist_tags.as_ref().and_then(|dt| dt.latest.as_ref());

            for (version, date) in version_dates.iter().take(25) {
                let date_part = date.split('T').next().unwrap_or(date);
                let is_latest = latest_version.is_some_and(|lv| lv == *version);

                if is_latest {
                    changelog.push_str(&format!("### v{} (Latest)\n", version));
                } else {
                    changelog.push_str(&format!("### v{}\n", version));
                }
                changelog.push_str(&format!("*Released: {}*\n\n", date_part));
            }

            if version_dates.len() > 25 {
                changelog.push_str(&format!(
                    "\n*...and {} more versions on npm*\n",
                    version_dates.len() - 25
                ));
            }
        }

        // Link to npm page
        changelog.push_str(&format!(
            "\n---\n\n[View on npm](https://www.npmjs.com/package/{})\n",
            name
        ));

        Ok(Some(changelog))
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

        // Enrich first 10 search results with additional metadata
        let enrichment_futures: Vec<_> = packages
            .iter()
            .take(10)
            .map(|pkg| self.fetch_package_info(&pkg.name))
            .collect();

        let enrichments = futures::future::join_all(enrichment_futures).await;

        for (pkg, info_opt) in packages.iter_mut().take(10).zip(enrichments.into_iter()) {
            if let Some(info) = info_opt {
                // Update description if empty or shorter
                if let Some(ref desc) = info.description {
                    if pkg.description.is_empty() || pkg.description.len() < desc.len() {
                        pkg.description.clone_from(desc);
                    }
                }
                pkg.homepage = info
                    .homepage
                    .clone()
                    .or_else(|| info.repository.as_ref().and_then(|r| r.url()));
                pkg.license = info.license.as_ref().map(|l| l.name());
                pkg.maintainer = info.author.as_ref().map(|a| a.name());
                pkg.enrichment = Some(Self::create_enrichment(&info));
            }
        }

        Ok(packages)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Format download count for display
#[allow(dead_code)]
fn format_downloads(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_author_parsing() {
        // Object format
        let author_obj = NpmAuthor::Object {
            name: "John Doe".to_string(),
        };
        assert_eq!(author_obj.name(), "John Doe");

        // String format
        let author_str = NpmAuthor::String("Jane Doe <jane@example.com>".to_string());
        assert_eq!(author_str.name(), "Jane Doe");

        // Simple string
        let author_simple = NpmAuthor::String("Bob Smith".to_string());
        assert_eq!(author_simple.name(), "Bob Smith");
    }

    #[test]
    fn test_npm_repository_url_parsing() {
        // Object format with git+ prefix
        let repo_obj = NpmRepository::Object {
            url: Some("git+https://github.com/user/repo.git".to_string()),
            _type: Some("git".to_string()),
        };
        assert_eq!(
            repo_obj.url(),
            Some("https://github.com/user/repo".to_string())
        );

        // String format
        let repo_str = NpmRepository::String("https://github.com/user/repo".to_string());
        assert_eq!(
            repo_str.url(),
            Some("https://github.com/user/repo".to_string())
        );
    }

    #[test]
    fn test_npm_license_parsing() {
        // Object format
        let license_obj = NpmLicense::Object {
            license_type: Some("MIT".to_string()),
        };
        assert_eq!(license_obj.name(), "MIT");

        // String format
        let license_str = NpmLicense::String("Apache-2.0".to_string());
        assert_eq!(license_str.name(), "Apache-2.0");
    }

    #[test]
    fn test_format_downloads() {
        assert_eq!(format_downloads(500), "500");
        assert_eq!(format_downloads(1500), "1.5K");
        assert_eq!(format_downloads(1_500_000), "1.5M");
        assert_eq!(format_downloads(0), "0");
    }
}
