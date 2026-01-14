use crate::backends::PackageBackend;
use anyhow::Result;
use async_trait::async_trait;
use linget_backend_core::{Package, PackageSource, PackageStatus};
use std::fs;

pub struct AppImageBackend;

impl AppImageBackend {
    pub fn new() -> Self {
        Self
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
        true
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let mut packages = Vec::new();

        if let Ok(home) = std::env::var("HOME") {
            let appimage_dir = format!("{}/.local/bin", home);
            if let Ok(entries) = fs::read_dir(&appimage_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension() {
                        if ext == "AppImage" || ext == "appimage" {
                            let name = path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();

                            packages.push(Package {
                                name,
                                version: String::new(),
                                available_version: None,
                                description: format!("AppImage: {}", path.display()),
                                source: PackageSource::AppImage,
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
        }

        Ok(packages)
    }

    async fn check_updates(&self) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    async fn install(&self, _path: &str) -> Result<()> {
        Ok(())
    }

    async fn remove(&self, name: &str) -> Result<()> {
        if let Ok(home) = std::env::var("HOME") {
            let appimage_path = format!("{}/.local/bin/{}", home, name);
            if std::path::Path::new(&appimage_path).exists() {
                std::fs::remove_file(&appimage_path)?;
            }
        }
        Ok(())
    }

    async fn update(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn search(&self, _query: &str) -> Result<Vec<Package>> {
        Ok(Vec::new())
    }

    fn source(&self) -> PackageSource {
        PackageSource::AppImage
    }
}
