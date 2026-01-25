use crate::models::PackageSource;
use serde::Serialize;
use std::path::PathBuf;

/// Status information for a detected package manager provider.
///
/// This struct contains all the information about a package manager's
/// availability on the system, including version information and
/// the paths to relevant executables.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    /// The package source type (APT, DNF, Flatpak, etc.)
    pub source: PackageSource,
    /// Human-readable display name
    pub display_name: String,
    /// Whether this provider is available on the system
    pub available: bool,
    /// Commands used to list packages (e.g., ["apt", "dpkg-query"])
    pub list_cmds: Vec<String>,
    /// Commands that require elevated privileges (e.g., ["pkexec"])
    pub privileged_cmds: Vec<String>,
    /// Absolute paths to found executables
    #[serde(serialize_with = "serialize_paths")]
    pub found_paths: Vec<PathBuf>,
    /// Version string from the package manager (if available)
    pub version: Option<String>,
    /// Reason for unavailability (if not available)
    pub reason: Option<String>,
}

/// Custom serializer for PathBuf vectors to convert to strings
fn serialize_paths<S>(paths: &[PathBuf], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;
    let mut seq = serializer.serialize_seq(Some(paths.len()))?;
    for path in paths {
        seq.serialize_element(&path.display().to_string())?;
    }
    seq.end()
}

struct ProviderProbe {
    list_cmds: &'static [&'static str],
    privileged_cmds: &'static [&'static str],
    version_cmd: Option<(&'static str, &'static [&'static str])>,
}

fn which_all(cmds: &[&str]) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for cmd in cmds {
        if let Ok(path) = which::which(cmd) {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn cmd_version(cmd: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(cmd).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    let first_line = text.lines().next()?.trim();
    if first_line.is_empty() {
        None
    } else {
        Some(first_line.to_string())
    }
}

fn display_name(source: PackageSource) -> String {
    source.to_string()
}

fn provider_row(source: PackageSource) -> ProviderStatus {
    let probe = match source {
        PackageSource::Apt => ProviderProbe {
            list_cmds: &["apt", "dpkg-query"],
            privileged_cmds: &["pkexec"],
            version_cmd: Some(("apt", &["--version"])),
        },
        PackageSource::Dnf => ProviderProbe {
            list_cmds: &["dnf"],
            privileged_cmds: &["pkexec"],
            version_cmd: Some(("dnf", &["--version"])),
        },
        PackageSource::Pacman => ProviderProbe {
            list_cmds: &["pacman"],
            privileged_cmds: &["pkexec"],
            version_cmd: Some(("pacman", &["-V"])),
        },
        PackageSource::Zypper => ProviderProbe {
            list_cmds: &["zypper"],
            privileged_cmds: &["pkexec"],
            version_cmd: Some(("zypper", &["--version"])),
        },
        PackageSource::Flatpak => ProviderProbe {
            list_cmds: &["flatpak"],
            privileged_cmds: &[],
            version_cmd: Some(("flatpak", &["--version"])),
        },
        PackageSource::Snap => ProviderProbe {
            list_cmds: &["snap"],
            privileged_cmds: &["pkexec"],
            version_cmd: Some(("snap", &["version"])),
        },
        PackageSource::Npm => ProviderProbe {
            list_cmds: &["npm"],
            privileged_cmds: &[],
            version_cmd: Some(("npm", &["--version"])),
        },
        PackageSource::Pip => ProviderProbe {
            list_cmds: &["pip3", "pip"],
            privileged_cmds: &[],
            version_cmd: Some(("python3", &["--version"])),
        },
        PackageSource::Pipx => ProviderProbe {
            list_cmds: &["pipx"],
            privileged_cmds: &[],
            version_cmd: Some(("pipx", &["--version"])),
        },
        PackageSource::Cargo => ProviderProbe {
            list_cmds: &["cargo"],
            privileged_cmds: &[],
            version_cmd: Some(("cargo", &["--version"])),
        },
        PackageSource::Brew => ProviderProbe {
            list_cmds: &["brew"],
            privileged_cmds: &[],
            version_cmd: Some(("brew", &["--version"])),
        },
        PackageSource::Aur => ProviderProbe {
            list_cmds: &["yay", "paru"],
            privileged_cmds: &[],
            version_cmd: None,
        },
        PackageSource::Conda => ProviderProbe {
            list_cmds: &["conda"],
            privileged_cmds: &[],
            version_cmd: Some(("conda", &["--version"])),
        },
        PackageSource::Mamba => ProviderProbe {
            list_cmds: &["mamba"],
            privileged_cmds: &[],
            version_cmd: Some(("mamba", &["--version"])),
        },
        PackageSource::Dart => ProviderProbe {
            list_cmds: &["dart", "flutter"],
            privileged_cmds: &[],
            version_cmd: Some(("dart", &["--version"])),
        },
        PackageSource::Deb => ProviderProbe {
            list_cmds: &["dpkg"],
            privileged_cmds: &["pkexec"],
            version_cmd: Some(("dpkg", &["--version"])),
        },
        PackageSource::AppImage => ProviderProbe {
            list_cmds: &[],
            privileged_cmds: &[],
            version_cmd: None,
        },
    };

    let mut found_paths = which_all(probe.list_cmds);
    let privileged_paths = which_all(probe.privileged_cmds);
    found_paths.extend(privileged_paths);
    found_paths.sort();
    found_paths.dedup();

    let available = match source {
        PackageSource::Pip => which::which("pip3").is_ok() || which::which("pip").is_ok(),
        PackageSource::Aur => which::which("yay").is_ok() || which::which("paru").is_ok(),
        PackageSource::Dart => which::which("dart").is_ok() || which::which("flutter").is_ok(),
        PackageSource::AppImage => true,
        _ => probe.list_cmds.iter().all(|c| which::which(c).is_ok()),
    };

    let version = probe
        .version_cmd
        .and_then(|(cmd, args)| cmd_version(cmd, args));
    let reason = if available {
        None
    } else if probe.list_cmds.is_empty() {
        Some("Not available on this system".to_string())
    } else {
        let missing: Vec<&str> = probe
            .list_cmds
            .iter()
            .copied()
            .filter(|c| which::which(c).is_err())
            .collect();
        Some(format!("Missing: {}", missing.join(", ")))
    };

    ProviderStatus {
        source,
        display_name: display_name(source),
        available,
        list_cmds: probe.list_cmds.iter().map(|s| s.to_string()).collect(),
        privileged_cmds: probe
            .privileged_cmds
            .iter()
            .map(|s| s.to_string())
            .collect(),
        found_paths,
        version,
        reason,
    }
}

/// Detect all package manager providers on the system.
///
/// This function checks for all supported package managers (APT, DNF, Flatpak, etc.)
/// and returns their status including version information and executable paths.
///
/// The results are sorted with available providers first (alphabetically),
/// followed by unavailable providers (alphabetically).
///
/// # Example
///
/// ```no_run
/// use linget::backend::detect_providers;
///
/// let providers = detect_providers();
/// for provider in providers {
///     if provider.available {
///         println!("{}: {}", provider.display_name, provider.version.unwrap_or_default());
///     }
/// }
/// ```
pub fn detect_providers() -> Vec<ProviderStatus> {
    let mut rows: Vec<ProviderStatus> = PackageSource::ALL
        .iter()
        .map(|&source| provider_row(source))
        .collect();

    rows.sort_by(|a, b| {
        let a_key = (!a.available, a.display_name.to_lowercase());
        let b_key = (!b.available, b.display_name.to_lowercase());
        a_key.cmp(&b_key)
    });
    rows
}

/// Detect a single package manager provider.
///
/// This is useful when you only need to check one specific provider
/// Get only the available providers on the system.
///
/// This is a convenience function that filters `detect_providers()`
/// to return only providers that are actually available.
pub fn detect_available_providers() -> Vec<ProviderStatus> {
    detect_providers()
        .into_iter()
        .filter(|p| p.available)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_providers_returns_all_sources() {
        let providers = detect_providers();
        // Should return status for all 17 package sources
        assert_eq!(providers.len(), PackageSource::ALL.len());
    }

    #[test]
    fn test_provider_status_fields_populated() {
        let providers = detect_providers();
        for provider in &providers {
            // display_name should never be empty
            assert!(!provider.display_name.is_empty());
            // If not available, should have a reason (except for AppImage which is always available)
            if !provider.available && provider.source != PackageSource::AppImage {
                assert!(provider.reason.is_some());
            }
        }
    }

    #[test]
    fn test_detect_providers_sorted_correctly() {
        let providers = detect_providers();
        // Available providers should come before unavailable ones
        let mut found_unavailable = false;
        for provider in &providers {
            if !provider.available {
                found_unavailable = true;
            } else if found_unavailable {
                // If we found an available after unavailable, sorting is wrong
                panic!("Available provider found after unavailable providers");
            }
        }
    }

    #[test]
    fn test_detect_single_provider() {
        let providers = detect_providers();
        let apt_status = providers
            .iter()
            .find(|p| p.source == PackageSource::Apt)
            .expect("APT provider should be in list");
        assert_eq!(apt_status.source, PackageSource::Apt);
        assert_eq!(apt_status.display_name, "APT");
        // list_cmds should be populated for APT
        assert!(!apt_status.list_cmds.is_empty());
    }

    #[test]
    fn test_appimage_always_available() {
        let providers = detect_providers();
        let appimage_status = providers
            .iter()
            .find(|p| p.source == PackageSource::AppImage)
            .expect("AppImage provider should be in list");
        assert!(appimage_status.available);
        // AppImage has no version command, so version should be None
        assert!(appimage_status.version.is_none());
    }

    #[test]
    fn test_provider_status_serializable() {
        let providers = detect_providers();
        let provider = providers
            .iter()
            .find(|p| p.source == PackageSource::Flatpak)
            .expect("Flatpak provider should be in list");
        // Should be serializable to JSON without errors
        let json = serde_json::to_string(&provider);
        assert!(json.is_ok());
    }

    #[test]
    fn test_detect_available_providers_subset() {
        let all = detect_providers();
        let available = detect_available_providers();
        // Available should be a subset of all
        assert!(available.len() <= all.len());
        // All items in available should have available=true
        for p in &available {
            assert!(p.available);
        }
    }

    #[test]
    fn test_version_parsing() {
        // Test the cmd_version function indirectly through provider detection
        // At minimum, providers that are available should have attempted version detection
        let providers = detect_providers();
        for provider in &providers {
            if provider.available && provider.version.is_some() {
                // Version string should not be empty if present
                assert!(!provider.version.as_ref().unwrap().is_empty());
            }
        }
    }

    #[test]
    fn test_which_all_deduplicates() {
        // Test the which_all helper function
        let paths = which_all(&["ls", "ls"]); // Same command twice
                                              // Should deduplicate
        let unique_count = paths.len();
        let mut deduped = paths.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(unique_count, deduped.len());
    }
}
