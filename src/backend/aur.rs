use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

pub struct AurBackend {
    helper: String,
}

impl AurBackend {
    pub fn new() -> Self {
        let helper = if which::which("paru").is_ok() {
            "paru".to_string()
        } else {
            "yay".to_string()
        };
        Self { helper }
    }
}

impl Default for AurBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for AurBackend {
    fn is_available() -> bool {
        which::which("yay").is_ok() || which::which("paru").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        // -Qm lists "foreign" packages (often AUR).
        let output = Command::new(&self.helper)
            .args(["-Qm"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to list AUR packages via {}", self.helper))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    available_version: None,
                    description: String::new(),
                    source: PackageSource::Aur,
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
        // -Qua lists AUR updates.
        let output = Command::new(&self.helper)
            .args(["-Qua"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to check AUR updates via {}", self.helper))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        for line in stdout.lines() {
            // name old -> new
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                packages.push(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    available_version: Some(parts[3].to_string()),
                    description: String::new(),
                    source: PackageSource::Aur,
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
        // Use --noconfirm to skip prompts. This skips PKGBUILD review which is a security risk.
        // The user should have already reviewed the package on the AUR website.
        let status = Command::new(&self.helper)
            .args(["-S", "--noconfirm", "--needed", name])
            .status()
            .await
            .with_context(|| {
                format!("Failed to install AUR package {} via {}", name, self.helper)
            })?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to install AUR package {} via {}", name, self.helper)
        }
    }

    async fn remove(&self, name: &str) -> Result<()> {
        // Use pacman directly for removal (doesn't need AUR helper)
        // Requires pkexec for root access
        let status = Command::new("pkexec")
            .args(["pacman", "-R", "--noconfirm", name])
            .status()
            .await
            .context("Failed to remove AUR package")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to remove AUR package {}", name)
        }
    }

    async fn update(&self, name: &str) -> Result<()> {
        // Reinstall to get the latest version
        let status = Command::new(&self.helper)
            .args(["-S", "--noconfirm", name])
            .status()
            .await
            .with_context(|| {
                format!("Failed to update AUR package {} via {}", name, self.helper)
            })?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to update AUR package {} via {}", name, self.helper)
        }
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // Best-effort: use helper's -Ss which includes repo results too.
        let output = Command::new(&self.helper)
            .args(["-Ss", query])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .with_context(|| format!("Failed to search AUR via {}", self.helper))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        let lines: Vec<&str> = stdout.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let header = lines[i];
            if let Some((repo_name, rest)) = header.split_once(' ') {
                if let Some((_, name)) = repo_name.split_once('/') {
                    let version = rest.split_whitespace().next().unwrap_or("").to_string();
                    let description = if i + 1 < lines.len() {
                        lines[i + 1].trim().to_string()
                    } else {
                        String::new()
                    };
                    packages.push(Package {
                        name: name.to_string(),
                        version,
                        available_version: None,
                        description,
                        source: PackageSource::Aur,
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
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }

        Ok(packages)
    }

    fn source(&self) -> PackageSource {
        PackageSource::Aur
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
    fn create_fake_aur_helper() -> (std::path::PathBuf, Option<std::ffi::OsString>) {
        let fixture_dir = std::env::temp_dir().join(format!(
            "linget-aur-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&fixture_dir).expect("create fixture dir");

        for helper_name in ["yay", "paru"] {
            let script_path = fixture_dir.join(helper_name);
            let mut script = File::create(&script_path).expect("create fake aur helper");
            writeln!(script, "#!/usr/bin/env sh").expect("write fake aur helper");
            writeln!(script, "if [ \"$1\" = \"-Qm\" ]; then").expect("write fake aur helper");
            writeln!(script, "  echo 'yay 11.0.0'").expect("write fake aur helper");
            writeln!(script, "elif [ \"$1\" = \"-Qua\" ]; then").expect("write fake aur helper");
            writeln!(script, "  echo 'yay 11.0.0 -> 12.0.0'").expect("write fake aur helper");
            writeln!(script, "fi").expect("write fake aur helper");

            let mut perms = script.metadata().expect("metadata").permissions();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                perms.set_mode(0o755);
            }
            fs::set_permissions(&script_path, perms).expect("chmod fake aur helper");
            script.flush().expect("flush fake aur helper");
        }

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
    async fn list_installed_parses_helper_output() {
        let _path_env_guard = TEST_PATH_ENV_LOCK.lock().await;
        let (fixture_dir, old_path) = create_fake_aur_helper();
        let _restore_path = PathGuard::new(old_path);

        let packages = AurBackend::new()
            .list_installed()
            .await
            .expect("list installed aur packages");
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "yay");
        assert_eq!(packages[0].version, "11.0.0");
        fs::remove_dir_all(fixture_dir).ok();
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn check_updates_parses_arrow_format() {
        let _path_env_guard = TEST_PATH_ENV_LOCK.lock().await;
        let (fixture_dir, old_path) = create_fake_aur_helper();
        let _restore_path = PathGuard::new(old_path);

        let updates = AurBackend::new()
            .check_updates()
            .await
            .expect("check aur updates");
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].name, "yay");
        assert_eq!(updates[0].version, "11.0.0");
        assert_eq!(updates[0].available_version.as_deref(), Some("12.0.0"));
        fs::remove_dir_all(fixture_dir).ok();
    }
}
