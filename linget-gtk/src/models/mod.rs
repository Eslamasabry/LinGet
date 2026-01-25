pub mod alias;
pub mod appearance;
pub mod changelog;
mod config;
mod enrichment;
mod flatpak_metadata;
pub mod health;
pub mod history;
mod icons;
pub mod insights;
mod package;
pub mod recommendations;
mod repository;
pub mod scheduler;

pub use changelog::ChangelogSummary;
pub use config::*;
pub use enrichment::*;
pub use flatpak_metadata::*;
pub use health::{HealthIssue, IssueSeverity, SystemHealth};
#[allow(unused_imports)]
pub use history::{
    HistoryEntry, HistoryFilter, HistoryOperation, OperationHistory, PackageSnapshot, SnapshotDiff,
};
pub use icons::*;
pub use insights::{guess_config_paths, guess_log_command, parse_install_date, PackageInsights};
pub use package::{Package, PackageEnrichment, PackageSource, PackageStatus, UpdateCategory};
pub use recommendations::{
    get_global_recommendations, get_package_recommendations, Recommendation,
};
pub use repository::*;
#[allow(unused_imports)]
pub use scheduler::{SchedulePreset, ScheduledOperation, ScheduledTask, SchedulerState};

#[allow(unused_imports)]
pub use appearance::{
    AppearanceConfig, BorderRadius, BorderStyle, CardSize, FontScale, GlowIntensity, GridColumns,
    ListDensity, SidebarWidth, SpacingLevel, TransitionSpeed,
};
