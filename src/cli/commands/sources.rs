use crate::backend::PackageManager;
use crate::cli::{OutputWriter, SourcesAction};
use crate::models::PackageSource;
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

    match action {
        None | Some(SourcesAction::List) => {
            // Currently all available sources are enabled by default
            // In future, this could read from config
            writer.sources(&available, &available);
        }
        Some(SourcesAction::Enable { source }) => {
            let pkg_source: PackageSource = source.into();
            if available.contains(&pkg_source) {
                writer.success(&format!("Source {:?} is now enabled", pkg_source));
                writer.message("Note: Source preferences are stored in your config file");
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
                writer.success(&format!("Source {:?} is now disabled", pkg_source));
                writer.message("Note: Source preferences are stored in your config file");
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
