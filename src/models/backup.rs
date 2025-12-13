use serde::{Deserialize, Serialize};
use crate::models::{Package, PackageSource};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupPackage {
    pub source: PackageSource,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageList {
    pub created_at: DateTime<Utc>,
    pub packages: Vec<BackupPackage>,
}

impl PackageList {
    pub fn new(packages: &[Package]) -> Self {
        let backup_items = packages.iter()
            .map(|p| BackupPackage {
                source: p.source,
                name: p.name.clone(),
            })
            .collect();

        Self {
            created_at: Utc::now(),
            packages: backup_items,
        }
    }
}
