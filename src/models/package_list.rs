use super::{Config, Package, PackageSource, PackageStatus};
use chrono::Utc;
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedPackage {
    pub name: String,
    #[serde(
        serialize_with = "serialize_package_source",
        deserialize_with = "deserialize_package_source"
    )]
    pub source: PackageSource,
    #[serde(default)]
    pub version: String,
}

impl ExportedPackage {
    pub fn to_install_stub(&self) -> Package {
        Package {
            name: self.name.clone(),
            version: self.version.clone(),
            available_version: None,
            description: String::new(),
            source: self.source,
            status: PackageStatus::NotInstalled,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageListConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enabled_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignored_packages: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub favorite_packages: Vec<String>,
}

impl PackageListConfig {
    pub fn from_config(config: &Config) -> Self {
        let mut enabled_sources: Vec<String> = config
            .enabled_sources
            .to_sources()
            .into_iter()
            .map(|source| source.as_config_str().to_string())
            .collect();
        enabled_sources.sort();

        Self {
            enabled_sources,
            ignored_packages: config.ignored_packages.clone(),
            favorite_packages: config.favorite_packages.clone(),
        }
    }

    pub fn apply_preferences(&self, config: &mut Config) {
        config.ignored_packages = self.ignored_packages.clone();
        config.favorite_packages = self.favorite_packages.clone();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageListExport {
    pub packages: Vec<ExportedPackage>,
    #[serde(alias = "created_at")]
    pub exported_at: String,
    #[serde(default)]
    pub linget_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<PackageListConfig>,
}

impl PackageListExport {
    pub fn from_installed(packages: &[Package]) -> Self {
        Self::from_installed_with_config(packages, None)
    }

    pub fn from_installed_with_config(packages: &[Package], config: Option<&Config>) -> Self {
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
            exported_at: Utc::now().to_rfc3339(),
            linget_version: env!("CARGO_PKG_VERSION").to_string(),
            config: config.map(PackageListConfig::from_config),
        }
    }

    pub fn from_json_str(data: &str) -> serde_json::Result<ParsedPackageList> {
        match serde_json::from_str::<PackageListDocument>(data)? {
            PackageListDocument::Current(export) => Ok(ParsedPackageList {
                export,
                warnings: Vec::new(),
            }),
            PackageListDocument::Legacy(legacy) => Ok(legacy.into_parsed()),
        }
    }

    pub fn to_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    pub fn source_count(&self) -> usize {
        self.packages
            .iter()
            .map(|package| package.source)
            .collect::<HashSet<_>>()
            .len()
    }

    pub fn export_date_label(&self) -> &str {
        self.exported_at
            .split('T')
            .next()
            .unwrap_or(&self.exported_at)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPackageList {
    pub export: PackageListExport,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PackageListDocument {
    Current(PackageListExport),
    Legacy(LegacyPackageListExport),
}

#[derive(Debug, Deserialize)]
struct LegacyPackageListExport {
    created_at: String,
    #[serde(default)]
    config: Option<PackageListConfig>,
    packages: HashMap<String, Vec<LegacyPackageEntry>>,
}

#[derive(Debug, Deserialize)]
struct LegacyPackageEntry {
    name: String,
    #[serde(default)]
    version: String,
}

impl LegacyPackageListExport {
    fn into_parsed(self) -> ParsedPackageList {
        let mut grouped_packages: Vec<_> = self.packages.into_iter().collect();
        grouped_packages.sort_by(|left, right| left.0.cmp(&right.0));

        let mut packages = Vec::new();
        let mut warnings = Vec::new();

        for (source_key, source_packages) in grouped_packages {
            let Some(source) = PackageSource::from_str(&source_key) else {
                warnings.push(format!("Unknown source: {}, skipping", source_key));
                continue;
            };

            for package in source_packages {
                packages.push(ExportedPackage {
                    name: package.name,
                    source,
                    version: package.version,
                });
            }
        }

        packages.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then_with(|| left.name.cmp(&right.name))
        });

        ParsedPackageList {
            export: PackageListExport {
                packages,
                exported_at: self.created_at,
                linget_version: String::new(),
                config: self.config,
            },
            warnings,
        }
    }
}

fn serialize_package_source<S>(source: &PackageSource, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(source.as_config_str())
}

fn deserialize_package_source<'de, D>(deserializer: D) -> Result<PackageSource, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    PackageSource::from_str(&raw)
        .ok_or_else(|| D::Error::custom(format!("unknown package source: {}", raw)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed_package(name: &str, source: PackageSource) -> Package {
        Package {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            available_version: None,
            description: String::new(),
            source,
            status: PackageStatus::Installed,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: None,
            enrichment: None,
        }
    }

    #[test]
    fn exported_sources_use_config_keys() {
        let export =
            PackageListExport::from_installed(&[installed_package("ripgrep", PackageSource::Apt)]);
        let json = export.to_json_pretty().unwrap();

        assert!(json.contains("\"source\": \"apt\""));
    }

    #[test]
    fn parses_legacy_grouped_backup_and_keeps_config() {
        let json = r#"{
  "version": 1,
  "created_at": "2026-03-06T00:00:00Z",
  "config": {
    "enabled_sources": ["apt", "pip"],
    "ignored_packages": ["Apt:vim"],
    "favorite_packages": ["Flatpak:org.gnome.Calculator"]
  },
  "packages": {
    "apt": [{ "name": "vim", "version": "9.1" }],
    "flatpak": [{ "name": "org.gnome.Calculator", "version": "46.0" }],
    "unknown": [{ "name": "mystery", "version": "1.0" }]
  }
}"#;

        let parsed = PackageListExport::from_json_str(json).unwrap();

        assert_eq!(parsed.export.package_count(), 2);
        assert_eq!(parsed.export.source_count(), 2);
        assert_eq!(parsed.warnings, vec!["Unknown source: unknown, skipping"]);

        let config = parsed.export.config.unwrap();
        assert_eq!(config.enabled_sources, vec!["apt", "pip"]);
        assert_eq!(config.ignored_packages, vec!["Apt:vim"]);
        assert_eq!(
            config.favorite_packages,
            vec!["Flatpak:org.gnome.Calculator"]
        );
    }

    #[test]
    fn parses_current_exports_with_legacy_enum_names() {
        let json = r#"{
  "packages": [{ "name": "vim", "source": "Apt", "version": "9.1" }],
  "exported_at": "2026-03-06T00:00:00Z",
  "linget_version": "0.1.7"
}"#;

        let parsed = PackageListExport::from_json_str(json).unwrap();

        assert_eq!(parsed.export.package_count(), 1);
        assert_eq!(parsed.export.packages[0].source, PackageSource::Apt);
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn applying_exported_preferences_updates_lists_only() {
        let snapshot = PackageListConfig {
            enabled_sources: vec!["flatpak".to_string()],
            ignored_packages: vec!["Apt:vim".to_string()],
            favorite_packages: vec!["Flatpak:org.gnome.Calculator".to_string()],
        };

        let mut config = Config::default();
        config.enabled_sources.set(PackageSource::Apt, false);
        config.ignored_packages = vec!["old-ignore".to_string()];
        config.favorite_packages = vec!["old-favorite".to_string()];

        snapshot.apply_preferences(&mut config);

        assert!(!config.enabled_sources.get(PackageSource::Apt));
        assert_eq!(config.ignored_packages, vec!["Apt:vim"]);
        assert_eq!(
            config.favorite_packages,
            vec!["Flatpak:org.gnome.Calculator"]
        );
    }
}
