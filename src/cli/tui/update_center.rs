use crate::models::{Package, PackageSource, UpdateCategory};
use std::collections::{BTreeMap, HashSet};

const RISKY_PACKAGE_KEYWORDS: [&str; 12] = [
    "linux", "kernel", "systemd", "glibc", "libc", "openssl", "gnutls", "firmware", "mesa", "grub",
    "nvidia", "llvm",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateLane {
    Security,
    Recommended,
    Optional,
    Risky,
}

impl UpdateLane {
    pub fn label(self) -> &'static str {
        match self {
            UpdateLane::Security => "Security",
            UpdateLane::Recommended => "Recommended",
            UpdateLane::Optional => "Optional",
            UpdateLane::Risky => "Risky",
        }
    }

    fn sort_key(self) -> u8 {
        match self {
            UpdateLane::Security => 0,
            UpdateLane::Recommended => 1,
            UpdateLane::Optional => 2,
            UpdateLane::Risky => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateCandidate {
    pub package: Package,
    pub lane: UpdateLane,
    pub category: UpdateCategory,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSummary {
    pub total: usize,
    pub security: usize,
    pub recommended: usize,
    pub optional: usize,
    pub risky: usize,
    pub by_source: Vec<(PackageSource, usize)>,
}

pub fn classify_updates(packages: &[Package]) -> Vec<UpdateCandidate> {
    let mut candidates: Vec<UpdateCandidate> = packages
        .iter()
        .filter(|pkg| pkg.has_update())
        .cloned()
        .map(|pkg| {
            let category = pkg
                .update_category
                .unwrap_or_else(|| pkg.detect_update_category());
            let lane = classify_lane(&pkg, category);
            UpdateCandidate {
                package: pkg,
                lane,
                category,
            }
        })
        .collect();

    candidates.sort_by(|a, b| {
        a.lane
            .sort_key()
            .cmp(&b.lane.sort_key())
            .then_with(|| a.package.source.cmp(&b.package.source))
            .then_with(|| {
                a.package
                    .name
                    .to_lowercase()
                    .cmp(&b.package.name.to_lowercase())
            })
    });

    candidates
}

fn classify_lane(pkg: &Package, category: UpdateCategory) -> UpdateLane {
    if category == UpdateCategory::Security {
        return UpdateLane::Security;
    }

    if is_risky_update(pkg, category) {
        return UpdateLane::Risky;
    }

    if category == UpdateCategory::Feature {
        return UpdateLane::Optional;
    }

    UpdateLane::Recommended
}

fn is_risky_update(pkg: &Package, category: UpdateCategory) -> bool {
    let name = pkg.name.to_lowercase();
    if RISKY_PACKAGE_KEYWORDS
        .iter()
        .any(|keyword| name.contains(keyword))
    {
        return true;
    }

    let system_source = matches!(
        pkg.source,
        PackageSource::Apt
            | PackageSource::Dnf
            | PackageSource::Pacman
            | PackageSource::Zypper
            | PackageSource::Deb
    );
    if system_source && category == UpdateCategory::Feature {
        return true;
    }

    has_major_version_jump(pkg)
}

fn has_major_version_jump(pkg: &Package) -> bool {
    let Some(available) = pkg.available_version.as_ref() else {
        return false;
    };

    let Some(current) = semver::Version::parse(&pkg.version).ok() else {
        return false;
    };
    let Some(next) = semver::Version::parse(available).ok() else {
        return false;
    };

    next.major > current.major
}

pub fn build_summary(candidates: &[UpdateCandidate]) -> UpdateSummary {
    let mut by_source = BTreeMap::new();
    let mut summary = UpdateSummary::default();

    for candidate in candidates {
        summary.total += 1;
        *by_source.entry(candidate.package.source).or_insert(0usize) += 1;

        match candidate.lane {
            UpdateLane::Security => summary.security += 1,
            UpdateLane::Recommended => summary.recommended += 1,
            UpdateLane::Optional => summary.optional += 1,
            UpdateLane::Risky => summary.risky += 1,
        }
    }

    summary.by_source = by_source.into_iter().collect();
    summary
}

pub fn recommended_packages(candidates: &[UpdateCandidate]) -> Vec<Package> {
    candidates
        .iter()
        .filter(|item| matches!(item.lane, UpdateLane::Security | UpdateLane::Recommended))
        .map(|item| item.package.clone())
        .collect()
}

pub fn all_packages(candidates: &[UpdateCandidate]) -> Vec<Package> {
    candidates.iter().map(|item| item.package.clone()).collect()
}

pub fn selected_packages(
    candidates: &[UpdateCandidate],
    selected_ids: &HashSet<String>,
) -> Vec<Package> {
    candidates
        .iter()
        .filter(|item| selected_ids.contains(&item.package.id()))
        .map(|item| item.package.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PackageEnrichment, PackageStatus};

    fn make_pkg(
        name: &str,
        source: PackageSource,
        version: &str,
        available_version: &str,
        category: Option<UpdateCategory>,
    ) -> Package {
        Package {
            name: name.to_string(),
            version: version.to_string(),
            available_version: Some(available_version.to_string()),
            description: String::from("test"),
            source,
            status: PackageStatus::UpdateAvailable,
            size: None,
            homepage: None,
            license: None,
            maintainer: None,
            dependencies: Vec::new(),
            install_date: None,
            update_category: category,
            enrichment: Some(PackageEnrichment::default()),
        }
    }

    #[test]
    fn classify_lanes_expected() {
        let security = make_pkg(
            "openssl",
            PackageSource::Apt,
            "3.0.0",
            "3.0.1",
            Some(UpdateCategory::Security),
        );
        let recommended = make_pkg(
            "ripgrep",
            PackageSource::Cargo,
            "14.0.0",
            "14.0.1",
            Some(UpdateCategory::Bugfix),
        );
        let optional = make_pkg(
            "bat",
            PackageSource::Cargo,
            "0.24.0",
            "0.25.0",
            Some(UpdateCategory::Feature),
        );
        let risky = make_pkg(
            "linux-image-generic",
            PackageSource::Apt,
            "6.8.0",
            "7.0.0",
            Some(UpdateCategory::Feature),
        );

        let candidates = classify_updates(&[optional, risky, recommended, security]);
        assert_eq!(candidates[0].lane, UpdateLane::Security);
        assert_eq!(candidates[1].lane, UpdateLane::Recommended);
        assert_eq!(candidates[2].lane, UpdateLane::Optional);
        assert_eq!(candidates[3].lane, UpdateLane::Risky);
    }

    #[test]
    fn recommended_filters_out_optional_and_risky() {
        let security = make_pkg(
            "openssl",
            PackageSource::Apt,
            "3.0.0",
            "3.0.1",
            Some(UpdateCategory::Security),
        );
        let recommended = make_pkg(
            "ripgrep",
            PackageSource::Cargo,
            "14.0.0",
            "14.0.1",
            Some(UpdateCategory::Bugfix),
        );
        let optional = make_pkg(
            "bat",
            PackageSource::Cargo,
            "0.24.0",
            "0.25.0",
            Some(UpdateCategory::Feature),
        );

        let candidates = classify_updates(&[security, recommended, optional]);
        let picked = recommended_packages(&candidates);
        assert_eq!(picked.len(), 2);
        assert!(picked.iter().any(|p| p.name == "openssl"));
        assert!(picked.iter().any(|p| p.name == "ripgrep"));
    }
}
