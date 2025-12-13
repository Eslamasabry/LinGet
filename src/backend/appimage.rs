use super::PackageBackend;
use crate::models::{Package, PackageSource, PackageStatus};
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub struct AppImageBackend;

impl AppImageBackend {
    pub fn new() -> Self {
        Self
    }

    fn get_search_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // Common AppImage locations
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join("Applications"));
            dirs.push(home.join("apps"));
            dirs.push(home.join(".local/bin"));
            dirs.push(home.join("AppImages"));
        }

        // Also check /opt
        dirs.push(PathBuf::from("/opt"));

        dirs
    }

    fn is_appimage(path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower.ends_with(".appimage") {
                return true;
            }
            // Some AppImages don't have extension but are executable
            if path.is_file() {
                if let Ok(metadata) = path.metadata() {
                    let perms = metadata.permissions();
                    if perms.mode() & 0o111 != 0 {
                        // Check for AppImage magic bytes
                        if let Ok(mut file) = File::open(path) {
                            if file.seek(SeekFrom::Start(8)).is_ok() {
                                let mut magic = [0u8; 2];
                                if file.read_exact(&mut magic).is_ok() && magic == *b"AI" {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn extract_name(path: &Path) -> String {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| {
                let name = s.replace(['-', '_'], " ");
                // Capitalize first letter
                let mut chars = name.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                }
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }
}

impl Default for AppImageBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PackageBackend for AppImageBackend {
    fn is_available() -> bool {
        // AppImages are always "available" - just need to find them
        true
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        for dir in Self::get_search_dirs() {
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if Self::is_appimage(&path) {
                        let name = Self::extract_name(&path);
                        let version = "local".to_string();

                        packages.push(Package {
                            name,
                            version,
                            available_version: None,
                            description: format!("AppImage at {}", path.display()),
                            source: PackageSource::AppImage,
                            status: PackageStatus::Installed,
                            size: path.metadata().ok().map(|m| m.len()),
                            homepage: None,
                            license: None,
                            maintainer: None,
                            dependencies: Vec::new(),
                            install_date: None,
                        });
                    }
                }
            }
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        // AppImages don't have a central update mechanism
        Ok(Vec::new())
    }

    async fn install(&self, _name: &str) -> Result<()> {
        anyhow::bail!("AppImage installation not supported. Download the .AppImage file and place it in ~/Applications")
    }

    async fn remove(&self, name: &str) -> Result<()> {
        // Find and remove the AppImage
        for dir in Self::get_search_dirs() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if Self::is_appimage(&path) {
                        let pkg_name = Self::extract_name(&path);
                        if pkg_name.to_lowercase().contains(&name.to_lowercase()) {
                            std::fs::remove_file(&path)
                                .context(format!("Failed to remove {}", path.display()))?;
                            return Ok(());
                        }
                    }
                }
            }
        }
        anyhow::bail!("AppImage '{}' not found", name)
    }

    async fn update(&self, _name: &str) -> Result<()> {
        anyhow::bail!("AppImage updates must be done manually by downloading the new version")
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        // Can't search AppImages - they're local files
        Ok(Vec::new())
    }
}
