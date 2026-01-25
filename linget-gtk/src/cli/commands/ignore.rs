use crate::cli::{IgnoreAction, OutputWriter};
use crate::models::{Config, PackageSource};
use anyhow::Result;

fn format_package_id(name: &str, source: Option<PackageSource>) -> String {
    match source {
        Some(s) => format!("{:?}:{}", s, name),
        None => name.to_string(),
    }
}

pub async fn run(action: IgnoreAction, writer: &OutputWriter) -> Result<()> {
    let mut config = Config::load();

    match action {
        IgnoreAction::List => {
            if config.ignored_packages.is_empty() {
                writer.message("No packages are currently ignored.");
            } else {
                writer.header("Ignored Packages");
                for pkg in &config.ignored_packages {
                    writer.message(&format!("  • {}", pkg));
                }
            }
        }
        IgnoreAction::Add { package, source } => {
            let pkg_id = format_package_id(&package, source.map(Into::into));

            if config.ignored_packages.contains(&pkg_id) {
                writer.warning(&format!("Package '{}' is already ignored", pkg_id));
            } else {
                config.ignored_packages.push(pkg_id.clone());
                config.save()?;
                writer.success(&format!(
                    "Package '{}' will be ignored from updates",
                    pkg_id
                ));
            }
        }
        IgnoreAction::Remove { package, source } => {
            let pkg_id = format_package_id(&package, source.map(Into::into));

            if let Some(pos) = config.ignored_packages.iter().position(|p| p == &pkg_id) {
                config.ignored_packages.remove(pos);
                config.save()?;
                writer.success(&format!("Package '{}' will no longer be ignored", pkg_id));
            } else {
                writer.warning(&format!("Package '{}' was not in the ignore list", pkg_id));
            }
        }
    }

    Ok(())
}
