use super::PackageBackend;
use super::{run_pkexec, Suggest};
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

pub struct DebBackend;

impl DebBackend {
    pub fn new() -> Self {
        Self
    }

    fn get_downloads_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join("Downloads"));
            dirs.push(home.join("Desktop"));
        }

        // Also check /tmp for downloaded debs
        dirs.push(PathBuf::from("/tmp"));

        dirs
    }

    async fn get_deb_info(path: &PathBuf) -> Option<(String, String, String)> {
        // Use dpkg-deb to get package info
        let output = Command::new("dpkg-deb")
            .args([
                "--show",
                "--showformat=${Package}\n${Version}\n${Description}",
            ])
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if lines.len() >= 2 {
            let name = lines[0].to_string();
            let version = lines[1].to_string();
            let description = lines.get(2).map(|s| s.to_string()).unwrap_or_default();
            Some((name, version, description))
        } else {
            None
        }
    }
}

impl Default for DebBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for DebBackend {
    fn is_available() -> bool {
        which::which("dpkg").is_ok()
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        // Scan common download locations for .deb files
        for dir in Self::get_downloads_dirs() {
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "deb" {
                            if let Some((name, version, description)) =
                                Self::get_deb_info(&path).await
                            {
                                packages.push(Package {
                                    name,
                                    version,
                                    available_version: None,
                                    description,
                                    source: PackageSource::Deb,
                                    status: PackageStatus::NotInstalled,
                                    size: path.metadata().ok().map(|m| m.len()),
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

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // Local .deb files don't have updates
        Ok(Vec::new())
    }

    async fn install(&self, name: &str) -> Result<()> {
        // Find the .deb file
        for dir in Self::get_downloads_dirs() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "deb" {
                            if let Some((pkg_name, _, _)) = Self::get_deb_info(&path).await {
                                if pkg_name == name {
                                    // Install using dpkg
                                    run_pkexec(
                                        "dpkg",
                                        &["-i"],
                                        "Failed to install .deb package",
                                        Suggest {
                                            command: format!("sudo dpkg -i \"{}\"", path.display()),
                                        },
                                    )
                                    .await?;

                                    // Fix dependencies (best-effort)
                                    let _ = run_pkexec(
                                        "apt-get",
                                        &["install", "-f", "-y"],
                                        "Failed to fix dependencies",
                                        Suggest {
                                            command: "sudo apt-get install -f -y".to_string(),
                                        },
                                    )
                                    .await;

                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }

        anyhow::bail!(".deb file for '{}' not found in Downloads", name)
    }

    async fn remove(&self, name: &str) -> Result<()> {
        // Remove using dpkg
        run_pkexec(
            "dpkg",
            &["-r", "--", name],
            &format!("Failed to remove {}", name),
            Suggest {
                command: format!("sudo dpkg -r -- {}", name),
            },
        )
        .await
    }

    async fn update(&self, _name: &str) -> Result<()> {
        anyhow::bail!("Local .deb packages cannot be updated automatically")
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        anyhow::bail!("Search is not supported for local .deb files")
    }

    fn source(&self) -> PackageSource {
        PackageSource::Deb
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
    fn create_fake_dpkg_deb_script() -> (std::path::PathBuf, Option<std::ffi::OsString>) {
        let fixture_dir = std::env::temp_dir().join(format!(
            "linget-deb-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&fixture_dir).expect("create fixture dir");

        let script_path = fixture_dir.join("dpkg-deb");
        let mut script = File::create(&script_path).expect("create fake dpkg-deb");
        writeln!(script, "#!/usr/bin/env sh").expect("write fake dpkg-deb");
        writeln!(script, "echo 'demo-pkg'").expect("write fake dpkg-deb");
        writeln!(script, "echo '1.2.3'").expect("write fake dpkg-deb");
        writeln!(script, "echo 'Demo package description'").expect("write fake dpkg-deb");

        let mut perms = script.metadata().expect("metadata").permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o755);
        }
        fs::set_permissions(&script_path, perms).expect("chmod fake dpkg-deb");
        script.flush().expect("flush fake dpkg-deb");
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

    #[test]
    fn downloads_dirs_include_tmp() {
        assert!(DebBackend::get_downloads_dirs().contains(&PathBuf::from("/tmp")));
    }

    #[tokio::test]
    #[cfg(target_os = "linux")]
    async fn get_deb_info_parses_show_output() {
        let _path_env_guard = TEST_PATH_ENV_LOCK.lock().await;
        let (fixture_dir, old_path) = create_fake_dpkg_deb_script();
        let _restore_path = PathGuard::new(old_path);

        let info = DebBackend::get_deb_info(&PathBuf::from("/tmp/demo.deb"))
            .await
            .expect("expected deb info");
        assert_eq!(
            info,
            (
                "demo-pkg".to_string(),
                "1.2.3".to_string(),
                "Demo package description".to_string()
            )
        );
        fs::remove_dir_all(fixture_dir).ok();
    }
}
