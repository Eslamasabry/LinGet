use crate::models::PackageSource;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProviderStatus {
    pub source: PackageSource,
    pub display_name: String,
    pub available: bool,
    pub list_cmds: Vec<String>,
    pub privileged_cmds: Vec<String>,
    pub found_paths: Vec<PathBuf>,
    pub version: Option<String>,
    pub reason: Option<String>,
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

pub fn detect_providers() -> Vec<ProviderStatus> {
    let mut rows: Vec<ProviderStatus> = vec![
        provider_row(PackageSource::Apt),
        provider_row(PackageSource::Dnf),
        provider_row(PackageSource::Pacman),
        provider_row(PackageSource::Zypper),
        provider_row(PackageSource::Flatpak),
        provider_row(PackageSource::Snap),
        provider_row(PackageSource::Npm),
        provider_row(PackageSource::Pip),
        provider_row(PackageSource::Pipx),
        provider_row(PackageSource::Cargo),
        provider_row(PackageSource::Brew),
        provider_row(PackageSource::Aur),
        provider_row(PackageSource::Conda),
        provider_row(PackageSource::Mamba),
        provider_row(PackageSource::Dart),
        provider_row(PackageSource::Deb),
        provider_row(PackageSource::AppImage),
    ];

    rows.sort_by(|a, b| {
        let a_key = (!a.available, a.display_name.to_lowercase());
        let b_key = (!b.available, b.display_name.to_lowercase());
        a_key.cmp(&b_key)
    });
    rows
}
