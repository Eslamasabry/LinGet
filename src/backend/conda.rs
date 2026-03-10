use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use std::collections::HashSet;
use std::process::Stdio;
use tokio::process::Command;

pub struct CondaBackend;

impl CondaBackend {
    pub fn new() -> Self {
        Self
    }

    async fn conda_json(args: &[&str]) -> Result<serde_json::Value> {
        let output = Command::new("conda")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute conda")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).context("Failed to parse conda json")
    }

    async fn list_json() -> Result<Vec<serde_json::Value>> {
        // Prefer base env; fall back to current env.
        if let Ok(v) = Self::conda_json(&["list", "-n", "base", "--json"]).await {
            if let Some(arr) = v.as_array() {
                return Ok(arr.clone());
            }
        }
        let v = Self::conda_json(&["list", "--json"]).await?;
        Ok(v.as_array().cloned().unwrap_or_default())
    }

    fn resolve_search_records<'a>(
        json: &'a serde_json::Value,
        name: &str,
    ) -> Option<&'a [serde_json::Value]> {
        let obj = json.as_object()?;
        obj.get(name)
            .and_then(|value| value.as_array().map(Vec::as_slice))
            .or_else(|| {
                obj.iter().find_map(|(key, value)| {
                    if key.eq_ignore_ascii_case(name) {
                        value.as_array().map(Vec::as_slice)
                    } else {
                        None
                    }
                })
            })
    }

    fn release_timestamp(record: &serde_json::Value) -> Option<i64> {
        let raw = record.get("timestamp")?.as_i64()?;
        if raw > 10_000_000_000 {
            Some(raw / 1000)
        } else {
            Some(raw)
        }
    }

    fn release_date(record: &serde_json::Value) -> Option<String> {
        if let Some(ts_seconds) = Self::release_timestamp(record) {
            return Utc
                .timestamp_opt(ts_seconds, 0)
                .single()
                .map(|dt| dt.date_naive().to_string());
        }

        let raw = record.get("timestamp")?.as_str()?;
        if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
            return Some(dt.date_naive().to_string());
        }
        let date = raw.split('T').next().unwrap_or(raw).trim();
        if date.is_empty() {
            None
        } else {
            Some(date.to_string())
        }
    }
}

impl Default for CondaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for CondaBackend {
    fn is_available() -> bool {
        which::which("conda").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let items = Self::list_json().await?;
        let mut packages = Vec::new();
        for item in items {
            let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let version = item
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            packages.push(Package {
                name: name.to_string(),
                version,
                available_version: None,
                description: String::new(),
                source: PackageSource::Conda,
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
        // Get currently installed packages with their versions
        let installed = self.list_installed().await.unwrap_or_default();
        let installed_map: std::collections::HashMap<String, String> = installed
            .iter()
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect();

        // Run dry-run update to see what would be updated
        let output = Command::new("conda")
            .args(["update", "-n", "base", "--all", "--dry-run", "--json"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let Ok(output) = output else {
            tracing::warn!("Failed to check conda updates");
            return Ok(Vec::new());
        };

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut packages = Vec::new();

        // Parse "actions" -> "LINK" array for packages to be updated
        if let Some(actions) = json.get("actions") {
            if let Some(link) = actions.get("LINK").and_then(|v| v.as_array()) {
                for item in link {
                    let Some(name) = item.get("name").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let new_version = item
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Only include if we have the package installed with a different version
                    if let Some(current_version) = installed_map.get(name) {
                        if current_version != &new_version && !new_version.is_empty() {
                            packages.push(Package {
                                name: name.to_string(),
                                version: current_version.clone(),
                                available_version: Some(new_version),
                                description: String::new(),
                                source: PackageSource::Conda,
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
                }
            }
        }

        Ok(packages)
    }

    async fn install(&self, name: &str) -> Result<()> {
        let status = Command::new("conda")
            .args(["install", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to install conda package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install conda package {}", name)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        let status = Command::new("conda")
            .args(["remove", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to remove conda package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove conda package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        let status = Command::new("conda")
            .args(["update", "-n", "base", "-y", name])
            .status()
            .await
            .context("Failed to update conda package")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update conda package {}", name)
        }
    }

    async fn downgrade_to(&self, name: &str, version: &str) -> Result<()> {
        if version.trim().is_empty() {
            anyhow::bail!("Version is required");
        }
        let spec = format!("{}={}", name, version);
        let status = Command::new("conda")
            .args(["install", "-n", "base", "-y", &spec])
            .status()
            .await
            .context("Failed to install a specific conda package version")?;
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install {} via conda", spec)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = Command::new("conda")
            .args(["search", "--json", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search conda packages")?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value =
            serde_json::from_str(&stdout).unwrap_or(serde_json::Value::Null);

        let mut packages = Vec::new();

        if let Some(obj) = json.as_object() {
            for (name, versions) in obj {
                if let Some(arr) = versions.as_array() {
                    if let Some(latest) = arr.last() {
                        let version = latest
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let channel = latest
                            .get("channel")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        packages.push(Package {
                            name: name.clone(),
                            version,
                            available_version: None,
                            description: channel,
                            source: PackageSource::Conda,
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

        Ok(packages)
    }

    async fn get_changelog(&self, name: &str) -> Result<Option<String>> {
        let output = Command::new("conda")
            .args(["search", "--json", name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to search conda packages for changelog")?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value =
            serde_json::from_str(&stdout).unwrap_or(serde_json::Value::Null);
        let Some(records) = Self::resolve_search_records(&json, name) else {
            return Ok(None);
        };

        let mut releases: Vec<(String, Option<i64>, Option<String>)> = records
            .iter()
            .filter_map(|record| {
                let version = record.get("version").and_then(|value| value.as_str())?;
                if version.is_empty() {
                    return None;
                }
                Some((
                    version.to_string(),
                    Self::release_timestamp(record),
                    Self::release_date(record),
                ))
            })
            .collect();

        if releases.is_empty() {
            return Ok(None);
        }

        releases.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
        let mut seen_versions = HashSet::new();
        let unique_releases: Vec<(String, Option<String>)> = releases
            .into_iter()
            .filter_map(|(version, _timestamp, date)| {
                if seen_versions.insert(version.clone()) {
                    Some((version, date))
                } else {
                    None
                }
            })
            .collect();

        if unique_releases.is_empty() {
            return Ok(None);
        }

        let mut changelog = String::new();
        changelog.push_str(&format!("# {} Release History\n\n", name));
        changelog.push_str("## Version Timeline\n\n");

        for (index, (version, date)) in unique_releases.iter().take(25).enumerate() {
            if index == 0 {
                changelog.push_str(&format!("### v{} (Latest)\n", version));
            } else {
                changelog.push_str(&format!("### v{}\n", version));
            }
            changelog.push_str(&format!(
                "*Published: {}*\n\n",
                date.as_deref().unwrap_or("Unknown")
            ));
        }

        if unique_releases.len() > 25 {
            changelog.push_str(&format!(
                "*...and {} more versions on conda channels*\n",
                unique_releases.len() - 25
            ));
        }

        Ok(Some(changelog))
    }

    fn source(&self) -> PackageSource {
        PackageSource::Conda
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::TEST_PATH_ENV_LOCK;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(target_os = "linux")]
    fn create_fake_conda_script() -> (
        std::path::PathBuf,
        std::path::PathBuf,
        Option<std::ffi::OsString>,
    ) {
        let fixture_dir = std::env::temp_dir().join(format!(
            "linget-conda-dry-run-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&fixture_dir).expect("create temp fixture dir");

        let log_path = fixture_dir.join("args.txt");
        let script_path = fixture_dir.join("conda");
        let mut script = File::create(&script_path).expect("create fake conda script");
        writeln!(script, "#!/usr/bin/env sh").expect("write fake conda script");
        writeln!(script, "echo \"$*\" >> \"{}\"", log_path.to_string_lossy())
            .expect("write fake conda script");
        writeln!(script, "if [ \"$1\" = \"list\" ]; then").expect("write fake conda script");
        script
            .write_all(b"  echo '[{\"name\":\"pkg-orphan\",\"version\":\"1.0\"}]'\n")
            .expect("write fake conda script");
        writeln!(script, "elif [ \"$1\" = \"update\" ]; then").expect("write fake conda script");
        script
            .write_all(
                b"  echo '{\"actions\":{\"LINK\":[{\"name\":\"pkg-orphan\",\"version\":\"2.0\"}]}}'\n",
            )
            .expect("write fake conda script");
        writeln!(script, "elif [ \"$1\" = \"search\" ]; then").expect("write fake conda script");
        script
            .write_all(
                b"  echo '{\"demo\":[{\"version\":\"1.2.0\",\"timestamp\":1704067200000},{\"version\":\"1.1.0\",\"timestamp\":1701388800000},{\"version\":\"1.2.0\",\"timestamp\":1704067200000}]}'\n",
            )
            .expect("write fake conda script");
        writeln!(script, "else").expect("write fake conda script");
        script
            .write_all(b"  echo '{}'\n")
            .expect("write fake conda script");
        writeln!(script, "fi").expect("write fake conda script");

        let mut perms = script
            .metadata()
            .expect("read script metadata")
            .permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
        }
        fs::set_permissions(&script_path, perms).expect("chmod fake conda script");
        script.flush().expect("flush fake conda script");
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
        env::set_var("PATH", &joined_path);

        (fixture_dir, log_path, old_path)
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_check_updates_uses_dry_run_and_json() {
        let _path_env_guard = TEST_PATH_ENV_LOCK.lock().await;
        let (fixture_dir, log_path, old_path) = create_fake_conda_script();
        let script_path = fixture_dir.join("conda");

        let list_output = tokio::process::Command::new(&script_path)
            .args(["list", "-n", "base", "--json"])
            .output()
            .await
            .expect("script list command")
            .stdout;
        assert_eq!(
            String::from_utf8_lossy(&list_output),
            "[{\"name\":\"pkg-orphan\",\"version\":\"1.0\"}]\n"
        );

        let update_output = tokio::process::Command::new(&script_path)
            .args(["update", "-n", "base", "--all", "--dry-run", "--json"])
            .output()
            .await
            .expect("script update command")
            .stdout;
        assert_eq!(
            String::from_utf8_lossy(&update_output),
            "{\"actions\":{\"LINK\":[{\"name\":\"pkg-orphan\",\"version\":\"2.0\"}]}}\n"
        );

        let output_text = String::from_utf8_lossy(&update_output);
        let parsed_updates: serde_json::Value =
            serde_json::from_str(&output_text).expect("script update output should parse");
        let mut parsed_names = Vec::new();
        if let Some(actions) = parsed_updates.get("actions") {
            if let Some(link) = actions.get("LINK").and_then(|v| v.as_array()) {
                for item in link {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        parsed_names.push(name.to_string());
                    }
                }
            }
        }
        assert_eq!(parsed_names, vec!["pkg-orphan".to_string()]);

        let backend = CondaBackend::new();
        let installed = backend
            .list_installed()
            .await
            .expect("installed conda packages");
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].name, "pkg-orphan");
        let result = backend.check_updates().await;

        let args = fs::read_to_string(&log_path).unwrap_or_default();
        assert!(args.contains("list"));
        assert!(args.contains("update"));
        assert!(args.contains("--dry-run"));
        assert!(args.contains("--json"));
        let updates = result.expect("conda updates");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "pkg-orphan");

        if let Some(path) = old_path {
            env::set_var("PATH", path);
        } else {
            env::remove_var("PATH");
        }
        fs::remove_dir_all(&fixture_dir).ok();
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn test_get_changelog_builds_version_timeline_from_search() {
        let _path_env_guard = TEST_PATH_ENV_LOCK.lock().await;
        let (fixture_dir, log_path, old_path) = create_fake_conda_script();

        let backend = CondaBackend::new();
        let changelog = backend
            .get_changelog("demo")
            .await
            .expect("conda changelog call")
            .expect("expected synthetic conda changelog");

        assert!(changelog.contains("# demo Release History"));
        assert!(changelog.contains("## Version Timeline"));
        assert!(changelog.contains("### v1.2.0 (Latest)"));
        assert!(changelog.contains("### v1.1.0"));
        assert_eq!(changelog.matches("### v1.2.0").count(), 1);

        let args = fs::read_to_string(&log_path).unwrap_or_default();
        assert!(args.contains("search --json demo"));

        if let Some(path) = old_path {
            env::set_var("PATH", path);
        } else {
            env::remove_var("PATH");
        }
        fs::remove_dir_all(&fixture_dir).ok();
    }
}
