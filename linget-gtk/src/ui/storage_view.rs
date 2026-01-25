use crate::models::{Package, PackageSource};
use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    pub normalized_name: String,
    pub packages: Vec<Package>,
    pub total_size: u64,
    pub suggested_keep: Option<PackageSource>,
}

impl DuplicateGroup {
    pub fn size_savings(&self) -> u64 {
        if self.packages.len() <= 1 {
            return 0;
        }
        let max_size = self
            .packages
            .iter()
            .filter_map(|p| p.size)
            .max()
            .unwrap_or(0);
        self.total_size.saturating_sub(max_size)
    }
}

pub fn detect_duplicates(packages: &[Package]) -> Vec<DuplicateGroup> {
    let mut groups: HashMap<String, Vec<Package>> = HashMap::new();

    for pkg in packages {
        let normalized = normalize_package_name(&pkg.name, pkg.source);
        groups.entry(normalized).or_default().push(pkg.clone());
    }

    groups
        .into_iter()
        .filter(|(_, pkgs)| {
            pkgs.len() > 1
                && pkgs
                    .iter()
                    .map(|p| p.source)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    > 1
        })
        .map(|(name, pkgs)| {
            let total_size: u64 = pkgs.iter().filter_map(|p| p.size).sum();
            let suggested_keep = suggest_keep_source(&pkgs);
            DuplicateGroup {
                normalized_name: name,
                packages: pkgs,
                total_size,
                suggested_keep,
            }
        })
        .collect()
}

fn normalize_package_name(name: &str, source: PackageSource) -> String {
    let name_lower = name.to_lowercase();

    match source {
        PackageSource::Flatpak => {
            let parts: Vec<&str> = name_lower.split('.').collect();
            if parts.len() >= 3 {
                parts.last().copied().unwrap_or(&name_lower).to_string()
            } else {
                name_lower
            }
        }
        PackageSource::Snap => name_lower
            .trim_end_matches("-snap")
            .trim_end_matches("snap")
            .to_string(),
        _ => name_lower
            .replace("-bin", "")
            .replace("-git", "")
            .replace("-nightly", ""),
    }
}

fn suggest_keep_source(packages: &[Package]) -> Option<PackageSource> {
    let priority = [
        PackageSource::Apt,
        PackageSource::Dnf,
        PackageSource::Pacman,
        PackageSource::Zypper,
        PackageSource::Flatpak,
        PackageSource::Snap,
    ];

    for source in priority {
        if packages.iter().any(|p| p.source == source) {
            return Some(source);
        }
    }

    packages.first().map(|p| p.source)
}

pub struct StorageStats {
    pub total_size: u64,
    pub by_source: HashMap<PackageSource, u64>,
    pub largest_packages: Vec<Package>,
}

impl StorageStats {
    pub fn compute(packages: &[Package], top_n: usize) -> Self {
        let mut by_source: HashMap<PackageSource, u64> = HashMap::new();
        let mut total_size: u64 = 0;

        for pkg in packages {
            if let Some(size) = pkg.size {
                total_size += size;
                *by_source.entry(pkg.source).or_insert(0) += size;
            }
        }

        let mut sorted_packages: Vec<Package> = packages
            .iter()
            .filter(|p| p.size.is_some() && p.size.unwrap() > 0)
            .cloned()
            .collect();
        sorted_packages.sort_by(|a, b| b.size.cmp(&a.size));
        sorted_packages.truncate(top_n);

        Self {
            total_size,
            by_source,
            largest_packages: sorted_packages,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CleanupStats {
    pub cache_sizes: HashMap<PackageSource, u64>,
    pub orphaned_packages: HashMap<PackageSource, Vec<Package>>,
    pub total_recoverable: u64,
    pub total_orphaned: usize,
    pub is_loading: bool,
}

impl CleanupStats {
    pub fn total_recoverable_display(&self) -> String {
        humansize::format_size(self.total_recoverable, humansize::BINARY)
    }

    pub fn orphaned_count(&self, source: PackageSource) -> usize {
        self.orphaned_packages
            .get(&source)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone)]
pub enum CleanupAction {
    CleanAll,
    CleanSource(PackageSource),
    #[allow(dead_code)]
    RemoveOrphans(PackageSource),
    Refresh,
}

#[derive(Debug, Clone)]
pub enum DuplicateAction {
    RemovePackage(Box<Package>),
    #[allow(dead_code)]
    RemoveGroup(Box<DuplicateGroup>),
}

pub fn build_storage_view<F, D>(
    stats: &StorageStats,
    cleanup_stats: &CleanupStats,
    duplicates: &[DuplicateGroup],
    on_action: F,
    on_duplicate_action: D,
) -> gtk::Box
where
    F: Fn(CleanupAction) + Clone + 'static,
    D: Fn(DuplicateAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();

    let title = gtk::Label::builder()
        .label("Storage Usage")
        .xalign(0.0)
        .build();
    title.add_css_class("title-1");

    let total_label = gtk::Label::builder()
        .label(format!(
            "Total: {}",
            humansize::format_size(stats.total_size, humansize::BINARY)
        ))
        .xalign(0.0)
        .build();
    total_label.add_css_class("dim-label");

    header.append(&title);
    header.append(&total_label);
    container.append(&header);

    let cleanup_group = build_cleanup_section(cleanup_stats, on_action.clone());
    container.append(cleanup_group.upcast_ref::<gtk::Widget>());

    if !duplicates.is_empty() {
        let duplicates_group = build_duplicates_section(duplicates, on_duplicate_action);
        container.append(duplicates_group.upcast_ref::<gtk::Widget>());
    }

    let sources_group = adw::PreferencesGroup::builder()
        .title("Storage by Source")
        .description("Disk space used by each package source")
        .build();

    let mut source_sizes: Vec<(PackageSource, u64)> =
        stats.by_source.iter().map(|(k, v)| (*k, *v)).collect();
    source_sizes.sort_by(|a, b| b.1.cmp(&a.1));

    let max_size = source_sizes.first().map(|(_, s)| *s).unwrap_or(1);

    for (source, size) in &source_sizes {
        if *size == 0 {
            continue;
        }

        let row = adw::ActionRow::builder()
            .title(source.to_string())
            .subtitle(humansize::format_size(*size, humansize::BINARY))
            .build();

        let progress = gtk::ProgressBar::builder()
            .fraction(*size as f64 / max_size as f64)
            .valign(gtk::Align::Center)
            .width_request(120)
            .build();
        progress.add_css_class(source.color_class());

        let dot = gtk::Box::builder()
            .width_request(12)
            .height_request(12)
            .valign(gtk::Align::Center)
            .margin_end(8)
            .build();
        dot.add_css_class("source-dot");
        dot.add_css_class(source.color_class());

        row.add_prefix(&dot);
        row.add_suffix(&progress);

        sources_group.add(&row);
    }

    container.append(sources_group.upcast_ref::<gtk::Widget>());

    let packages_group = adw::PreferencesGroup::builder()
        .title("Largest Packages")
        .description("Top packages by disk usage")
        .build();

    for pkg in &stats.largest_packages {
        let size_str = pkg.size_display();
        let row = adw::ActionRow::builder()
            .title(&pkg.name)
            .subtitle(format!("{} • {}", pkg.source, size_str))
            .build();

        let chip = gtk::Label::builder()
            .label(&size_str)
            .valign(gtk::Align::Center)
            .build();
        chip.add_css_class("chip");
        chip.add_css_class("chip-muted");

        row.add_suffix(&chip);
        packages_group.add(&row);
    }

    container.append(packages_group.upcast_ref::<gtk::Widget>());

    container
}

fn build_cleanup_section<F>(cleanup_stats: &CleanupStats, on_action: F) -> adw::PreferencesGroup
where
    F: Fn(CleanupAction) + Clone + 'static,
{
    let group = adw::PreferencesGroup::builder()
        .title("Cleanup")
        .description("Reclaim disk space by removing unused data")
        .build();

    let summary_row = adw::ActionRow::builder()
        .title("Recoverable Space")
        .subtitle(if cleanup_stats.is_loading {
            "Calculating...".to_string()
        } else if cleanup_stats.total_recoverable == 0 {
            "Nothing to clean up".to_string()
        } else {
            format!(
                "{} from {} sources",
                cleanup_stats.total_recoverable_display(),
                cleanup_stats.cache_sizes.len()
            )
        })
        .build();

    let refresh_btn = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .valign(gtk::Align::Center)
        .tooltip_text("Refresh cleanup stats")
        .build();
    refresh_btn.add_css_class("flat");
    let on_action_refresh = on_action.clone();
    refresh_btn.connect_clicked(move |_| {
        on_action_refresh(CleanupAction::Refresh);
    });
    summary_row.add_suffix(&refresh_btn);

    if cleanup_stats.is_loading {
        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .valign(gtk::Align::Center)
            .build();
        summary_row.add_suffix(&spinner);
    } else if cleanup_stats.total_recoverable > 0 {
        let clean_all_btn = gtk::Button::builder()
            .label("Clean All")
            .valign(gtk::Align::Center)
            .build();
        clean_all_btn.add_css_class("suggested-action");
        clean_all_btn.add_css_class("pill");
        let on_action_all = on_action.clone();
        clean_all_btn.connect_clicked(move |_| {
            on_action_all(CleanupAction::CleanAll);
        });
        summary_row.add_suffix(&clean_all_btn);
    }

    group.add(&summary_row);

    let mut sources: Vec<_> = cleanup_stats.cache_sizes.keys().copied().collect();
    sources.sort();

    for source in sources {
        let cache_size = cleanup_stats.cache_sizes.get(&source).copied().unwrap_or(0);
        let orphan_count = cleanup_stats.orphaned_count(source);

        if cache_size == 0 && orphan_count == 0 {
            continue;
        }

        let mut subtitle_parts = Vec::new();
        if cache_size > 0 {
            subtitle_parts.push(format!(
                "{} cache",
                humansize::format_size(cache_size, humansize::BINARY)
            ));
        }
        if orphan_count > 0 {
            subtitle_parts.push(format!(
                "{} unused package{}",
                orphan_count,
                if orphan_count == 1 { "" } else { "s" }
            ));
        }

        let row = adw::ActionRow::builder()
            .title(source.to_string())
            .subtitle(subtitle_parts.join(" • "))
            .build();

        let dot = gtk::Box::builder()
            .width_request(12)
            .height_request(12)
            .valign(gtk::Align::Center)
            .margin_end(8)
            .build();
        dot.add_css_class("source-dot");
        dot.add_css_class(source.color_class());
        row.add_prefix(&dot);

        if cache_size > 0 {
            let clean_btn = gtk::Button::builder()
                .label("Clean")
                .valign(gtk::Align::Center)
                .build();
            clean_btn.add_css_class("flat");
            clean_btn.add_css_class("pill");
            let on_action_source = on_action.clone();
            clean_btn.connect_clicked(move |_| {
                on_action_source(CleanupAction::CleanSource(source));
            });
            row.add_suffix(&clean_btn);
        }

        group.add(&row);
    }

    group
}

fn build_duplicates_section<D>(duplicates: &[DuplicateGroup], on_action: D) -> adw::PreferencesGroup
where
    D: Fn(DuplicateAction) + Clone + 'static,
{
    let total_savings: u64 = duplicates.iter().map(|g| g.size_savings()).sum();

    let group = adw::PreferencesGroup::builder()
        .title("Duplicate Applications")
        .description(format!(
            "{} app{} installed from multiple sources • {} potential savings",
            duplicates.len(),
            if duplicates.len() == 1 { "" } else { "s" },
            humansize::format_size(total_savings, humansize::BINARY)
        ))
        .build();

    for dup_group in duplicates {
        let expander = adw::ExpanderRow::builder()
            .title(title_case(&dup_group.normalized_name))
            .subtitle(format!(
                "{} versions • {}",
                dup_group.packages.len(),
                humansize::format_size(dup_group.total_size, humansize::BINARY)
            ))
            .build();

        let savings_chip = gtk::Label::builder()
            .label(format!(
                "−{}",
                humansize::format_size(dup_group.size_savings(), humansize::BINARY)
            ))
            .valign(gtk::Align::Center)
            .build();
        savings_chip.add_css_class("chip");
        savings_chip.add_css_class("chip-warning");
        expander.add_suffix(&savings_chip);

        for pkg in &dup_group.packages {
            let is_suggested = dup_group.suggested_keep == Some(pkg.source);
            let pkg_row = adw::ActionRow::builder()
                .title(&pkg.name)
                .subtitle(pkg.size_display())
                .build();

            let source_chip = gtk::Label::builder()
                .label(pkg.source.to_string())
                .valign(gtk::Align::Center)
                .build();
            source_chip.add_css_class("chip");
            source_chip.add_css_class(pkg.source.color_class());
            pkg_row.add_prefix(&source_chip);

            if is_suggested {
                let keep_badge = gtk::Label::builder()
                    .label("Keep")
                    .valign(gtk::Align::Center)
                    .build();
                keep_badge.add_css_class("chip");
                keep_badge.add_css_class("chip-success");
                pkg_row.add_suffix(&keep_badge);
            } else {
                let remove_btn = gtk::Button::builder()
                    .label("Remove")
                    .valign(gtk::Align::Center)
                    .build();
                remove_btn.add_css_class("flat");
                remove_btn.add_css_class("pill");
                remove_btn.add_css_class("destructive-action");

                let pkg_clone = pkg.clone();
                let on_action_clone = on_action.clone();
                remove_btn.connect_clicked(move |_| {
                    on_action_clone(DuplicateAction::RemovePackage(Box::new(pkg_clone.clone())));
                });
                pkg_row.add_suffix(&remove_btn);
            }

            expander.add_row(&pkg_row);
        }

        group.add(&expander);
    }

    group
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
