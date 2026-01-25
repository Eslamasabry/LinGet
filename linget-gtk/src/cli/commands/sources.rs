use crate::backend::PackageManager;
use crate::cli::{OutputWriter, SourcesAction};
use crate::models::{Config, PackageSource};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    action: Option<SourcesAction>,
    writer: &OutputWriter,
) -> Result<()> {
    let manager = pm.lock().await;
    let available: Vec<PackageSource> = manager.available_sources().into_iter().collect();
    let mut config = Config::load();

    match action {
        None | Some(SourcesAction::List) => {
            let enabled: Vec<PackageSource> = available
                .iter()
                .filter(|s| config.enabled_sources.get(**s))
                .copied()
                .collect();
            writer.sources(&available, &enabled);
        }
        Some(SourcesAction::Enable { source }) => {
            let pkg_source: PackageSource = source.into();
            if available.contains(&pkg_source) {
                config.enabled_sources.set(pkg_source, true);
                config.save()?;
                writer.success(&format!("Source {:?} is now enabled", pkg_source));
            } else {
                writer.error(&format!(
                    "Source {:?} is not available on this system",
                    pkg_source
                ));
            }
        }
        Some(SourcesAction::Disable { source }) => {
            let pkg_source: PackageSource = source.into();
            if available.contains(&pkg_source) {
                config.enabled_sources.set(pkg_source, false);
                config.save()?;
                writer.success(&format!("Source {:?} is now disabled", pkg_source));
            } else {
                writer.error(&format!(
                    "Source {:?} is not available on this system",
                    pkg_source
                ));
            }
        }
    }

    Ok(())
}
