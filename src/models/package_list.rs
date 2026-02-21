use crate::models::package::{Package, PackageSource, PackageStatus};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedPackage {
    pub name: String,
    pub source: PackageSource,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageListExport {
    pub packages: Vec<ExportedPackage>,
    pub exported_at: DateTime<Local>,
    pub linget_version: String,
}

impl PackageListExport {
    pub fn from_installed(packages: &[Package]) -> Self {
        let exported: Vec<ExportedPackage> = packages
            .iter()
            .filter(|p| {
                p.status == PackageStatus::Installed || p.status == PackageStatus::UpdateAvailable
            })
            .map(|p| ExportedPackage {
                name: p.name.clone(),
                source: p.source,
                version: p.version.clone(),
            })
            .collect();

        Self {
            packages: exported,
            exported_at: Local::now(),
            linget_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Returns packages from the export that are NOT currently installed locally.
    pub fn diff_against_installed<'a>(&'a self, installed: &[Package]) -> Vec<&'a ExportedPackage> {
        self.packages
            .iter()
            .filter(|ep| {
                !installed
                    .iter()
                    .any(|p| p.name == ep.name && p.source == ep.source)
            })
            .collect()
    }
}
