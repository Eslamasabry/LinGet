use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

pub struct BrewBackend;

#[derive(Debug, Clone, PartialEq, Eq)]
struct BrewFormulaMetadata {
    name: String,
    desc: Option<String>,
    homepage: Option<String>,
    stable: Option<String>,
    head: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BrewFormulaApiResponse {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    versions: Option<BrewFormulaApiVersions>,
}

#[derive(Debug, Deserialize)]
struct BrewFormulaApiVersions {
    #[serde(default)]
    stable: Option<String>,
    #[serde(default)]
    head: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    #[serde(default)]
    tag_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

fn normalize_release_body(body: &str) -> Option<String> {
    let lines: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn normalize_release_date(raw: &str) -> String {
    raw.split('T').next().unwrap_or(raw).trim().to_string()
}

impl BrewBackend {
    pub fn new() -> Self {
        Self
    }

    fn extract_github_repo(url: &str) -> Option<(String, String)> {
        let trimmed = url.trim().trim_end_matches('/');
        let without_scheme = trimmed
            .strip_prefix("https://")
            .or_else(|| trimmed.strip_prefix("http://"))?;
        let without_host = without_scheme
            .strip_prefix("github.com/")
            .or_else(|| without_scheme.strip_prefix("www.github.com/"))?;

        let mut segments = without_host
            .split('/')
            .filter(|segment| !segment.is_empty());
        let owner = segments.next()?.trim();
        let repo = segments.next()?.trim().trim_end_matches(".git");

        if owner.is_empty() || repo.is_empty() {
            None
        } else {
            Some((owner.to_string(), repo.to_string()))
        }
    }

    fn render_formula_overview(metadata: &BrewFormulaMetadata) -> String {
        let mut changelog = format!("# {} Release Overview\n\n", metadata.name);

        if let Some(desc) = metadata.desc.as_deref() {
            let desc = desc.trim();
            if !desc.is_empty() {
                changelog.push_str(desc);
                changelog.push_str("\n\n");
            }
        }

        if let Some(homepage) = metadata.homepage.as_deref() {
            let homepage = homepage.trim();
            if !homepage.is_empty() {
                changelog.push_str(&format!("*Homepage:* {}\n\n", homepage));
            }
        }

        changelog.push_str("## Version channels\n\n");
        if let Some(stable) = metadata.stable.as_deref() {
            changelog.push_str(&format!("- Stable: {}\n", stable));
        }
        if let Some(head) = metadata.head.as_deref() {
            changelog.push_str(&format!("- Head: {}\n", head));
        }
        if metadata.stable.is_none() && metadata.head.is_none() {
            changelog.push_str("- No version channel metadata available\n");
        }

        changelog
    }

    fn render_github_release_history(
        metadata: &BrewFormulaMetadata,
        releases: &[GithubRelease],
    ) -> Option<String> {
        let releases: Vec<&GithubRelease> =
            releases.iter().filter(|release| !release.draft).collect();
        if releases.is_empty() {
            return None;
        }

        let mut changelog = format!("# {} Release History\n\n", metadata.name);

        if let Some(desc) = metadata.desc.as_deref() {
            let desc = desc.trim();
            if !desc.is_empty() {
                changelog.push_str(desc);
                changelog.push_str("\n\n");
            }
        }

        if let Some(homepage) = metadata.homepage.as_deref() {
            let homepage = homepage.trim();
            if !homepage.is_empty() {
                changelog.push_str(&format!("*Homepage:* {}\n\n", homepage));
            }
        }

        changelog.push_str("## GitHub releases\n\n");
        for (index, release) in releases.iter().take(10).enumerate() {
            let version = release
                .tag_name
                .as_deref()
                .or(release.name.as_deref())
                .unwrap_or("Unknown release");
            let version = if release.prerelease {
                format!("{} (Pre-release)", version)
            } else if index == 0 {
                format!("{} (Latest)", version)
            } else {
                version.to_string()
            };
            changelog.push_str(&format!("### {}\n", version));

            if let Some(date) = release.published_at.as_deref() {
                changelog.push_str(&format!("*Released: {}*\n\n", normalize_release_date(date)));
            }

            if let Some(body) = release.body.as_deref().and_then(normalize_release_body) {
                changelog.push_str(&body);
                changelog.push_str("\n\n");
            }
        }

        Some(changelog)
    }

    fn parse_formula_metadata(payload: &str, name: &str) -> Option<BrewFormulaMetadata> {
        let formula = serde_json::from_str::<BrewFormulaApiResponse>(payload).ok()?;
        Some(BrewFormulaMetadata {
            name: formula.name.unwrap_or_else(|| name.to_string()),
            desc: formula.desc.and_then(|value| {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            }),
            homepage: formula.homepage.and_then(|value| {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            }),
            stable: formula
                .versions
                .as_ref()
                .and_then(|versions| versions.stable.clone()),
            head: formula
                .versions
                .as_ref()
                .and_then(|versions| versions.head.clone()),
        })
    }

    async fn fetch_formula_metadata(
        client: &reqwest::Client,
        name: &str,
    ) -> Result<Option<BrewFormulaMetadata>> {
        let response = client
            .get(format!(
                "https://formulae.brew.sh/api/formula/{}.json",
                name
            ))
            .send()
            .await
            .context("Failed to fetch Homebrew formula metadata")?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let payload = response
            .text()
            .await
            .context("Failed to read Homebrew formula metadata")?;

        Ok(Self::parse_formula_metadata(&payload, name))
    }

    async fn fetch_github_releases(
        client: &reqwest::Client,
        owner: &str,
        repo: &str,
    ) -> Result<Option<Vec<GithubRelease>>> {
        let response = client
            .get(format!(
                "https://api.github.com/repos/{}/{}/releases",
                owner, repo
            ))
            .header(reqwest::header::USER_AGENT, "LinGet")
            .send()
            .await
            .context("Failed to fetch GitHub releases")?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let releases = response
            .json::<Vec<GithubRelease>>()
            .await
            .context("Failed to parse GitHub releases")?;

        Ok(Some(releases))
    }
}

impl Default for BrewBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for BrewBackend {
    fn is_available() -> bool {
        which::which("brew").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let output = Command::new("brew")
            .args(["list", "--versions"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to list brew packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            let name = parts[0].to_string();
            let version = parts.last().unwrap_or(&"").to_string();

            packages.push(Package {
                name,
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Brew,
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

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // Prefer JSON output if available.
        let output = Command::new("brew")
            .args(["outdated", "--json=v2"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to check brew updates")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let mut packages = Vec::new();
            let formulae = json
                .get("formulae")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for item in formulae {
                let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                let installed = item
                    .get("installed_versions")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let current = item
                    .get("current_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if current.is_empty() {
                    continue;
                }

                packages.push(Package {
                    name: name.to_string(),
                    version: installed,
                    available_version: Some(current),
                    description: String::new(),
                    source: PackageSource::Brew,
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
            return Ok(packages);
        }

        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("brew")
            .args(["install", name])
            .status()
            .await
            .context("Failed to install brew package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install brew package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("brew")
            .args(["uninstall", name])
            .status()
            .await
            .context("Failed to remove brew package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove brew package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("brew")
            .args(["upgrade", name])
            .status()
            .await
            .context("Failed to update brew package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update brew package {}", name)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("brew")
            .args(["search", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search brew packages")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for name in stdout.lines().filter(|l| !l.trim().is_empty()) {
            packages.push(Package {
                name: name.trim().to_string(),
                version: String::new(),
                available_version: None,
                description: String::new(),
                source: PackageSource::Brew,
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
            if packages.len() >= 50 {
                break;
            }
        }

        Ok(packages)
    }

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let Some(metadata) = Self::fetch_formula_metadata(&client, name).await? else {
            return Ok(None);
        };

        if let Some((owner, repo)) = metadata
            .homepage
            .as_deref()
            .and_then(Self::extract_github_repo)
        {
            if let Ok(Some(releases)) = Self::fetch_github_releases(&client, &owner, &repo).await {
                if let Some(changelog) = Self::render_github_release_history(&metadata, &releases) {
                    return Ok(Some(changelog));
                }
            }
        }

        Ok(Some(Self::render_formula_overview(&metadata)))
    }

    fn source(&self) -> PackageSource {
        PackageSource::Brew
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::TEST_PATH_ENV_LOCK;
    use std::env;
    use std::ffi::OsString;
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct PathGuard {
        old_path: Option<OsString>,
    }

    impl PathGuard {
        fn new(old_path: Option<OsString>) -> Self {
            Self { old_path }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            if let Some(path) = self.old_path.take() {
                env::set_var("PATH", path);
            } else {
                env::remove_var("PATH");
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn create_fake_brew_script() -> (std::path::PathBuf, Option<std::ffi::OsString>) {
        let fixture_dir = std::env::temp_dir().join(format!(
            "linget-brew-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&fixture_dir).expect("create fixture dir");

        let script_path = fixture_dir.join("brew");
        let mut script = File::create(&script_path).expect("create fake brew");
        writeln!(script, "#!/usr/bin/env sh").expect("write fake brew");
        writeln!(script, "if [ \"$1\" = \"list\" ]; then").expect("write fake brew");
        writeln!(script, "  echo 'wget 1.0 1.1'").expect("write fake brew");
        writeln!(script, "elif [ \"$1\" = \"outdated\" ]; then").expect("write fake brew");
        writeln!(
            script,
            "  echo '{{\"formulae\":[{{\"name\":\"wget\",\"installed_versions\":[\"1.0\"],\"current_version\":\"1.1\"}}]}}'"
        )
        .expect("write fake brew");
        writeln!(script, "fi").expect("write fake brew");

        let mut perms = script.metadata().expect("metadata").permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
        }
        fs::set_permissions(&script_path, perms).expect("chmod fake brew");
        script.flush().expect("flush fake brew");
        drop(script);

        let old_path = env::var_os("PATH");
        let joined_path = match old_path.clone() {
            Some(path) => format!(
                "{}:{}",
                fixture_dir.to_string_lossy(),
                path.to_string_lossy()
            ),
            None => fixture_dir.to_string_lossy().into_owned(),
        };
        env::set_var("PATH", joined_path);

        (fixture_dir, old_path)
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn list_installed_uses_last_version_token() {
        let _path_env_guard = TEST_PATH_ENV_LOCK.lock().await;
        let (fixture_dir, old_path) = create_fake_brew_script();
        let _restore_path = PathGuard::new(old_path);

        let packages = BrewBackend::new()
            .list_installed()
            .await
            .expect("list brew packages");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "wget");
        assert_eq!(packages[0].version, "1.1");
        fs::remove_dir_all(fixture_dir).ok();
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn check_updates_reads_json_output() {
        let _path_env_guard = TEST_PATH_ENV_LOCK.lock().await;
        let (fixture_dir, old_path) = create_fake_brew_script();
        let _restore_path = PathGuard::new(old_path);

        let updates = BrewBackend::new()
            .check_updates()
            .await
            .expect("brew updates");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "wget");
        assert_eq!(updates[0].version, "1.0");
        assert_eq!(updates[0].available_version.as_deref(), Some("1.1"));
        fs::remove_dir_all(fixture_dir).ok();
    }

    #[test]
    fn extract_github_repo_handles_common_homepage_urls() {
        assert_eq!(
            BrewBackend::extract_github_repo("https://github.com/sharkdp/bat"),
            Some(("sharkdp".to_string(), "bat".to_string()))
        );
        assert_eq!(
            BrewBackend::extract_github_repo("https://github.com/sharkdp/bat/"),
            Some(("sharkdp".to_string(), "bat".to_string()))
        );
        assert_eq!(BrewBackend::extract_github_repo("https://bat.dev"), None);
    }

    #[test]
    fn parse_formula_metadata_reads_versions_and_homepage() {
        let metadata = BrewBackend::parse_formula_metadata(
            r#"{
                "name":"bat",
                "desc":"Clone of cat with wings",
                "homepage":"https://github.com/sharkdp/bat",
                "versions":{"stable":"0.24.0","head":"HEAD"}
            }"#,
            "bat",
        )
        .expect("metadata");

        assert_eq!(metadata.name, "bat");
        assert_eq!(metadata.desc.as_deref(), Some("Clone of cat with wings"));
        assert_eq!(
            metadata.homepage.as_deref(),
            Some("https://github.com/sharkdp/bat")
        );
        assert_eq!(metadata.stable.as_deref(), Some("0.24.0"));
        assert_eq!(metadata.head.as_deref(), Some("HEAD"));
    }

    #[test]
    fn render_github_release_history_marks_latest_release() {
        let metadata = BrewFormulaMetadata {
            name: "bat".to_string(),
            desc: Some("Clone of cat with wings".to_string()),
            homepage: Some("https://github.com/sharkdp/bat".to_string()),
            stable: Some("0.24.0".to_string()),
            head: None,
        };
        let releases = vec![
            GithubRelease {
                tag_name: Some("v0.24.0".to_string()),
                name: Some("v0.24.0".to_string()),
                body: Some("Added catppuccin themes\nImproved pager defaults".to_string()),
                published_at: Some("2026-03-01T12:00:00Z".to_string()),
                draft: false,
                prerelease: false,
            },
            GithubRelease {
                tag_name: Some("v0.23.0".to_string()),
                name: None,
                body: Some("Smaller startup improvements".to_string()),
                published_at: Some("2026-01-15T09:00:00Z".to_string()),
                draft: false,
                prerelease: true,
            },
        ];

        let changelog =
            BrewBackend::render_github_release_history(&metadata, &releases).expect("changelog");

        assert!(changelog.contains("# bat Release History"));
        assert!(changelog.contains("### v0.24.0 (Latest)"));
        assert!(changelog.contains("*Released: 2026-03-01*"));
        assert!(changelog.contains("Added catppuccin themes"));
        assert!(changelog.contains("### v0.23.0 (Pre-release)"));
    }
}
