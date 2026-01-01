use crate::backend::PackageManager;
use crate::models::{
    fetch_enrichment, get_package_recommendations, guess_config_paths, guess_log_command,
    parse_install_date, ChangelogSummary, Config, Package, PackageEnrichment, PackageInsights,
    PackageSource, PackageStatus, Recommendation, ScheduledOperation, ScheduledTask,
};
use crate::ui::package_details::{enrichment, sandbox};
use crate::ui::strip_html_tags;
use crate::ui::widgets::build_schedule_popover;

use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum DetailsPanelInput {
    ShowPackage(Box<Package>),
    Clear,
    EnrichmentLoaded(Option<PackageEnrichment>),
    InsightsLoaded(PackageInsights),
    RecommendationsLoaded(Vec<Recommendation>),
    IconLoaded(Vec<u8>),
    IconTextureReady(gtk::gdk::Texture),
    ChangelogLoaded(Result<Option<String>, String>),
    ChangelogExpanded,
    ToggleIgnoreUpdates(bool),
    ToggleCollection(String),
    PreviewScreenshot(gtk::gdk::Texture),
    InstallRecommendation(String),
    DismissRecommendation(String),
    UpdatePackage,
    RemovePackage,
    DowngradePackage,
    SchedulePackage(ScheduledTask),
    UpdateOperationProgress(bool, String),
    Close,
}

impl std::fmt::Debug for DetailsPanelInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShowPackage(pkg) => f.debug_tuple("ShowPackage").field(&pkg.name).finish(),
            Self::Clear => write!(f, "Clear"),
            Self::EnrichmentLoaded(_) => write!(f, "EnrichmentLoaded"),
            Self::InsightsLoaded(_) => write!(f, "InsightsLoaded"),
            Self::RecommendationsLoaded(r) => f
                .debug_tuple("RecommendationsLoaded")
                .field(&r.len())
                .finish(),
            Self::IconLoaded(_) => write!(f, "IconLoaded"),
            Self::IconTextureReady(_) => write!(f, "IconTextureReady"),
            Self::ChangelogLoaded(_) => write!(f, "ChangelogLoaded"),
            Self::ChangelogExpanded => write!(f, "ChangelogExpanded"),
            Self::ToggleIgnoreUpdates(b) => f.debug_tuple("ToggleIgnoreUpdates").field(b).finish(),
            Self::ToggleCollection(c) => f.debug_tuple("ToggleCollection").field(c).finish(),
            Self::PreviewScreenshot(_) => write!(f, "PreviewScreenshot"),
            Self::InstallRecommendation(n) => {
                f.debug_tuple("InstallRecommendation").field(n).finish()
            }
            Self::DismissRecommendation(n) => {
                f.debug_tuple("DismissRecommendation").field(n).finish()
            }
            Self::UpdatePackage => write!(f, "UpdatePackage"),
            Self::RemovePackage => write!(f, "RemovePackage"),
            Self::DowngradePackage => write!(f, "DowngradePackage"),
            Self::SchedulePackage(t) => f
                .debug_tuple("SchedulePackage")
                .field(&t.package_name)
                .finish(),
            Self::UpdateOperationProgress(_, _) => write!(f, "UpdateOperationProgress"),
            Self::Close => write!(f, "Close"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DetailsPanelOutput {
    Close,
    Reload,
    ShowToast(String),
    ShowImage(gtk::gdk::Texture),
    ToggleCollection { pkg_id: String, collection: String },
    ScheduleTask(ScheduledTask),
}

pub struct DetailsPanelInit {
    pub pm: Arc<Mutex<PackageManager>>,
    pub config: Rc<RefCell<Config>>,
}

pub struct DetailsPanelModel {
    package: Option<Package>,
    pm: Arc<Mutex<PackageManager>>,
    config: Rc<RefCell<Config>>,
    enrichment: Option<PackageEnrichment>,
    enrichment_loading: bool,
    insights: Option<PackageInsights>,
    insights_loading: bool,
    recommendations: Vec<Recommendation>,
    changelog: Option<String>,
    changelog_summary: Option<ChangelogSummary>,
    changelog_loading: bool,
    changelog_error: Option<String>,
    changelog_fetched: bool,
    operation_in_progress: bool,
    operation_label: String,
    displayed_package_id: Option<String>,
    high_res_icon: Option<gtk::gdk::Texture>,
    pending_sandbox_rebuild: Cell<bool>,
    pending_enrichment_rebuild: Cell<bool>,
    pending_insights_rebuild: Cell<bool>,
    pending_recommendations_rebuild: Cell<bool>,
    pending_collections_rebuild: Cell<bool>,
    pending_changelog_rebuild: Cell<bool>,
}

#[allow(dead_code)]
pub struct DetailsPanelWidgets {
    header_bar: adw::HeaderBar,
    title_label: gtk::Label,
    content_box: gtk::Box,
    icon_stack: gtk::Stack,
    icon_image: gtk::Image,
    high_res_image: gtk::Picture,
    name_label: gtk::Label,
    source_dot: gtk::Box,
    source_label: gtk::Label,
    description_label: gtk::Label,
    enrichment_box: gtk::Box,
    enrichment_spinner: gtk::Spinner,
    version_row: adw::ActionRow,
    version_update_icon: gtk::Image,
    status_row: adw::ActionRow,
    size_row: adw::ActionRow,
    source_row: adw::ActionRow,
    insights_group: adw::PreferencesGroup,
    install_date_row: adw::ActionRow,
    deps_row: adw::ActionRow,
    reverse_deps_row: adw::ActionRow,
    safe_remove_row: adw::ActionRow,
    safe_remove_icon: gtk::Image,
    config_row: adw::ActionRow,
    recommendations_group: adw::PreferencesGroup,
    recommendations_box: gtk::Box,
    ignore_switch: gtk::Switch,
    collections_popover_box: gtk::Box,
    sandbox_box: gtk::Box,
    dependencies_box: gtk::Box,
    changelog_expander: gtk::Expander,
    changelog_content: gtk::Box,
    changelog_spinner: gtk::Spinner,
    warning_box: gtk::Box,
    warning_label: gtk::Label,
    update_btn: gtk::Button,
    remove_btn: gtk::Button,
    downgrade_btn: gtk::Button,
    schedule_btn: gtk::MenuButton,
    scheduled_info_row: adw::ActionRow,
}

fn apply_changelog_formatting(buffer: &gtk::TextBuffer, log: &str) {
    let tag_table = buffer.tag_table();

    let title_tag = gtk::TextTag::builder()
        .weight(700)
        .scale(1.2)
        .foreground("white")
        .build();
    tag_table.add(&title_tag);

    let version_tag = gtk::TextTag::builder()
        .weight(700)
        .foreground("#3584e4")
        .build();
    tag_table.add(&version_tag);

    let bullet_tag = gtk::TextTag::builder().foreground("#999").build();
    tag_table.add(&bullet_tag);

    buffer.set_text("");
    let mut iter = buffer.start_iter();

    for line in log.lines() {
        if line.starts_with('#') || (line.len() < 40 && line.ends_with(':') && !line.contains("  "))
        {
            buffer.insert_with_tags(&mut iter, line, &[&title_tag]);
        } else if line.starts_with('v')
            && line
                .chars()
                .nth(1)
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            buffer.insert_with_tags(&mut iter, line, &[&version_tag]);
        } else if line.trim_start().starts_with('*') || line.trim_start().starts_with('-') {
            let leading_spaces = line.len() - line.trim_start().len();
            buffer.insert(&mut iter, &line[..leading_spaces]);
            buffer.insert_with_tags(
                &mut iter,
                &line[leading_spaces..leading_spaces + 1],
                &[&bullet_tag],
            );
            buffer.insert(&mut iter, &line[leading_spaces + 1..]);
        } else {
            buffer.insert(&mut iter, line);
        }
        buffer.insert(&mut iter, "\n");
    }
}

impl SimpleComponent for DetailsPanelModel {
    type Init = DetailsPanelInit;
    type Input = DetailsPanelInput;
    type Output = DetailsPanelOutput;
    type Root = gtk::Box;
    type Widgets = DetailsPanelWidgets;

    fn init_root() -> Self::Root {
        gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(400)
            .css_classes(vec!["details-panel", "background"])
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = DetailsPanelModel {
            package: None,
            pm: init.pm,
            config: init.config,
            enrichment: None,
            enrichment_loading: false,
            insights: None,
            insights_loading: false,
            recommendations: Vec::new(),
            changelog: None,
            changelog_summary: None,
            changelog_loading: false,
            changelog_error: None,
            changelog_fetched: false,
            operation_in_progress: false,
            operation_label: String::new(),
            displayed_package_id: None,
            high_res_icon: None,
            pending_sandbox_rebuild: Cell::new(false),
            pending_enrichment_rebuild: Cell::new(false),
            pending_insights_rebuild: Cell::new(false),
            pending_recommendations_rebuild: Cell::new(false),
            pending_collections_rebuild: Cell::new(false),
            pending_changelog_rebuild: Cell::new(false),
        };

        let close_button = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Close")
            .css_classes(vec!["flat"])
            .build();

        let sender_close = sender.clone();
        close_button.connect_clicked(move |_| {
            sender_close.input(DetailsPanelInput::Close);
        });

        let title_label = gtk::Label::new(Some("Package Details"));
        let header_bar = adw::HeaderBar::builder().title_widget(&title_label).build();
        header_bar.pack_start(&close_button);

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(24)
            .build();

        let header_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(16)
            .build();

        let icon_frame = gtk::Frame::builder()
            .css_classes(vec!["icon-frame", "card"])
            .build();

        let icon_stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(300)
            .build();

        let icon_image = gtk::Image::builder()
            .icon_name("package-x-generic")
            .pixel_size(64)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let high_res_image = gtk::Picture::builder()
            .can_shrink(true)
            .width_request(80)
            .height_request(80)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build();

        icon_stack.add_named(&icon_image, Some("generic"));
        icon_stack.add_named(&high_res_image, Some("high-res"));
        icon_stack.set_visible_child_name("generic");

        icon_frame.set_child(Some(&icon_stack));

        let title_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();

        let name_label = gtk::Label::builder()
            .label("")
            .xalign(0.0)
            .wrap(true)
            .css_classes(vec!["title-2"])
            .build();

        let source_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let source_dot = gtk::Box::builder()
            .width_request(10)
            .height_request(10)
            .valign(gtk::Align::Center)
            .css_classes(vec!["source-dot"])
            .build();

        let source_label = gtk::Label::builder()
            .label("")
            .xalign(0.0)
            .css_classes(vec!["caption", "dimmed"])
            .build();

        source_box.append(&source_dot);
        source_box.append(&source_label);
        title_box.append(&name_label);
        title_box.append(&source_box);
        header_box.append(&icon_frame);
        header_box.append(&title_box);
        content_box.append(&header_box);

        let description_label = gtk::Label::builder()
            .label("")
            .wrap(true)
            .xalign(0.0)
            .visible(false)
            .css_classes(vec!["body", "dimmed"])
            .build();
        content_box.append(&description_label);

        let enrichment_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .build();

        let enrichment_loading_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        let enrichment_spinner = gtk::Spinner::builder().spinning(true).build();
        let loading_label = gtk::Label::builder()
            .label("Loading details…")
            .css_classes(vec!["dim-label", "caption"])
            .build();
        enrichment_loading_box.append(&enrichment_spinner);
        enrichment_loading_box.append(&loading_label);
        enrichment_box.append(&enrichment_loading_box);
        content_box.append(&enrichment_box);

        let details_group = adw::PreferencesGroup::builder().title("Details").build();

        let version_row = adw::ActionRow::builder()
            .title("Version")
            .subtitle("")
            .css_classes(vec!["property"])
            .build();
        let version_update_icon = gtk::Image::builder()
            .icon_name("software-update-available-symbolic")
            .css_classes(vec!["accent"])
            .visible(false)
            .build();
        version_row.add_suffix(&version_update_icon);
        details_group.add(&version_row);

        let status_row = adw::ActionRow::builder()
            .title("Status")
            .subtitle("")
            .css_classes(vec!["property"])
            .build();
        details_group.add(&status_row);

        let size_row = adw::ActionRow::builder()
            .title("Size")
            .subtitle("")
            .css_classes(vec!["property"])
            .build();
        details_group.add(&size_row);

        let source_row = adw::ActionRow::builder()
            .title("Source")
            .subtitle("")
            .css_classes(vec!["property"])
            .build();
        details_group.add(&source_row);

        let ignore_row = adw::ActionRow::builder()
            .title("Ignore Updates")
            .subtitle("Prevent this package from being updated")
            .build();

        let ignore_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();

        let sender_ignore = sender.clone();
        ignore_switch.connect_state_set(move |_, state| {
            sender_ignore.input(DetailsPanelInput::ToggleIgnoreUpdates(state));
            glib::Propagation::Proceed
        });

        ignore_row.add_suffix(&ignore_switch);
        details_group.add(&ignore_row);

        let collections_row = adw::ActionRow::builder()
            .title("Collections")
            .subtitle("Add to a collection")
            .build();

        let collections_btn = gtk::MenuButton::builder()
            .icon_name("folder-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(vec!["flat"])
            .build();

        let collections_popover = gtk::Popover::new();
        let collections_popover_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(8)
            .margin_end(8)
            .build();
        collections_popover.set_child(Some(&collections_popover_box));
        collections_btn.set_popover(Some(&collections_popover));

        collections_row.add_suffix(&collections_btn);
        details_group.add(&collections_row);
        content_box.append(&details_group);

        let insights_group = adw::PreferencesGroup::builder()
            .title("Insights")
            .visible(false)
            .build();

        let install_date_row = adw::ActionRow::builder()
            .title("Installed")
            .subtitle("Unknown")
            .css_classes(vec!["property"])
            .build();
        let install_icon = gtk::Image::builder()
            .icon_name("document-open-recent-symbolic")
            .build();
        install_date_row.add_prefix(&install_icon);
        insights_group.add(&install_date_row);

        let deps_row = adw::ActionRow::builder()
            .title("Dependencies")
            .subtitle("Loading...")
            .css_classes(vec!["property"])
            .build();
        let deps_icon = gtk::Image::builder()
            .icon_name("application-x-addon-symbolic")
            .build();
        deps_row.add_prefix(&deps_icon);
        insights_group.add(&deps_row);

        let reverse_deps_row = adw::ActionRow::builder()
            .title("Required by")
            .subtitle("Loading...")
            .css_classes(vec!["property"])
            .build();
        let rdeps_icon = gtk::Image::builder()
            .icon_name("emblem-shared-symbolic")
            .build();
        reverse_deps_row.add_prefix(&rdeps_icon);
        insights_group.add(&reverse_deps_row);

        let safe_remove_row = adw::ActionRow::builder()
            .title("Safe to remove")
            .subtitle("Checking...")
            .css_classes(vec!["property"])
            .build();
        let safe_remove_icon = gtk::Image::builder()
            .icon_name("emblem-ok-symbolic")
            .build();
        safe_remove_row.add_prefix(&safe_remove_icon);
        insights_group.add(&safe_remove_row);

        let config_row = adw::ActionRow::builder()
            .title("Config location")
            .subtitle("None detected")
            .css_classes(vec!["property"])
            .visible(false)
            .build();
        let config_icon = gtk::Image::builder().icon_name("folder-symbolic").build();
        config_row.add_prefix(&config_icon);
        insights_group.add(&config_row);

        content_box.append(&insights_group);

        let recommendations_group = adw::PreferencesGroup::builder()
            .title("You might also like")
            .visible(false)
            .build();

        let recommendations_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .build();

        recommendations_group.add(&recommendations_box);
        content_box.append(&recommendations_group);

        let sandbox_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .visible(false)
            .build();
        content_box.append(&sandbox_box);

        let dependencies_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .visible(false)
            .build();
        content_box.append(&dependencies_box);

        let changelog_expander = gtk::Expander::builder()
            .label("Release History")
            .expanded(false)
            .margin_top(8)
            .css_classes(vec!["card"])
            .build();

        let changelog_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .build();

        let changelog_spinner = gtk::Spinner::builder()
            .spinning(true)
            .halign(gtk::Align::Center)
            .margin_top(12)
            .margin_bottom(12)
            .visible(false)
            .build();
        changelog_content.append(&changelog_spinner);
        changelog_expander.set_child(Some(&changelog_content));

        let sender_changelog = sender.clone();
        changelog_expander.connect_expanded_notify(move |exp| {
            if exp.is_expanded() {
                sender_changelog.input(DetailsPanelInput::ChangelogExpanded);
            }
        });
        content_box.append(&changelog_expander);

        let warning_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(8)
            .css_classes(vec!["card", "warning"])
            .visible(false)
            .build();

        let warning_icon = gtk::Image::builder()
            .icon_name("dialog-warning-symbolic")
            .margin_start(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let warning_label = gtk::Label::builder()
            .label("")
            .wrap(true)
            .xalign(0.0)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .css_classes(vec!["dim-label"])
            .build();

        warning_box.append(&warning_icon);
        warning_box.append(&warning_label);
        content_box.append(&warning_box);

        let action_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .margin_top(16)
            .build();

        let update_btn = gtk::Button::builder()
            .label("Update")
            .css_classes(vec!["suggested-action", "pill"])
            .visible(false)
            .build();
        let sender_update = sender.clone();
        update_btn.connect_clicked(move |_| {
            sender_update.input(DetailsPanelInput::UpdatePackage);
        });

        let remove_btn = gtk::Button::builder()
            .label("Remove")
            .css_classes(vec!["destructive-action", "pill"])
            .visible(false)
            .build();
        let sender_remove = sender.clone();
        remove_btn.connect_clicked(move |_| {
            sender_remove.input(DetailsPanelInput::RemovePackage);
        });

        let downgrade_btn = gtk::Button::builder()
            .label("Downgrade")
            .css_classes(vec!["pill"])
            .visible(false)
            .build();
        let sender_downgrade = sender.clone();
        downgrade_btn.connect_clicked(move |_| {
            sender_downgrade.input(DetailsPanelInput::DowngradePackage);
        });

        let schedule_btn = gtk::MenuButton::builder()
            .icon_name("alarm-symbolic")
            .tooltip_text("Schedule for later")
            .css_classes(vec!["flat", "circular"])
            .visible(false)
            .build();

        action_box.append(&update_btn);
        action_box.append(&remove_btn);
        action_box.append(&downgrade_btn);
        action_box.append(&schedule_btn);
        content_box.append(&action_box);

        let scheduled_info_row = adw::ActionRow::builder()
            .title("Scheduled")
            .subtitle("")
            .visible(false)
            .css_classes(vec!["property"])
            .build();
        let schedule_icon = gtk::Image::builder()
            .icon_name("alarm-symbolic")
            .css_classes(vec!["accent"])
            .build();
        scheduled_info_row.add_prefix(&schedule_icon);

        let cancel_schedule_btn = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Cancel scheduled operation")
            .css_classes(vec!["flat", "circular"])
            .valign(gtk::Align::Center)
            .build();
        scheduled_info_row.add_suffix(&cancel_schedule_btn);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(false)
            .propagate_natural_width(false)
            .vexpand(true)
            .child(&content_box)
            .build();

        root.append(&header_bar);
        root.append(&scrolled);

        let widgets = DetailsPanelWidgets {
            header_bar,
            title_label,
            content_box,
            icon_stack,
            icon_image,
            high_res_image,
            name_label,
            source_dot,
            source_label,
            description_label,
            enrichment_box,
            enrichment_spinner,
            version_row,
            version_update_icon,
            status_row,
            size_row,
            source_row,
            insights_group,
            install_date_row,
            deps_row,
            reverse_deps_row,
            safe_remove_row,
            safe_remove_icon,
            config_row,
            recommendations_group,
            recommendations_box,
            ignore_switch,
            collections_popover_box,
            sandbox_box,
            dependencies_box,
            changelog_expander,
            changelog_content,
            changelog_spinner,
            warning_box,
            warning_label,
            update_btn,
            remove_btn,
            downgrade_btn,
            schedule_btn,
            scheduled_info_row,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DetailsPanelInput::ShowPackage(package) => {
                let pkg = *package;
                let new_id = pkg.id();
                let changed = self.displayed_package_id.as_ref() != Some(&new_id);

                crate::ui::set_ui_marker(format!(
                    "DetailsInputShowPackage {} [{}]",
                    pkg.name, new_id
                ));
                tracing::debug!(
                    pkg_id = %new_id,
                    pkg_name = %pkg.name,
                    source = ?pkg.source,
                    changed,
                    "Details panel ShowPackage"
                );

                self.package = Some(pkg.clone());
                self.enrichment = None;
                self.enrichment_loading = true;
                self.insights = None;
                self.insights_loading = true;
                self.recommendations = Vec::new();
                self.high_res_icon = None;
                self.changelog = None;
                self.changelog_summary = None;
                self.changelog_loading = false;
                self.changelog_error = None;
                self.changelog_fetched = false;
                self.operation_in_progress = false;

                crate::ui::set_ui_marker(format!("DetailsShowPackageStateReset [{}]", new_id));

                if changed {
                    self.displayed_package_id = Some(new_id.clone());
                    self.pending_sandbox_rebuild.set(true);
                    self.pending_enrichment_rebuild.set(true);
                    self.pending_insights_rebuild.set(true);
                    self.pending_recommendations_rebuild.set(true);
                    self.pending_collections_rebuild.set(true);
                    self.pending_changelog_rebuild.set(true);
                    crate::ui::set_ui_marker(format!("DetailsShowPackageFlagsSet [{}]", new_id));
                } else {
                    crate::ui::set_ui_marker(format!("DetailsShowPackageNoChange [{}]", new_id));
                }

                let sender_enrichment = sender.clone();
                let pkg_for_enrichment = pkg.clone();
                crate::ui::set_ui_marker(format!("DetailsShowPackageSpawnEnrichment [{}]", new_id));
                relm4::spawn(async move {
                    let enrichment = fetch_enrichment(&pkg_for_enrichment).await;
                    sender_enrichment.input(DetailsPanelInput::EnrichmentLoaded(enrichment));
                });

                let sender_insights = sender.clone();
                let pkg_for_insights = pkg.clone();
                let pm_for_insights = self.pm.clone();
                relm4::spawn(async move {
                    let manager = pm_for_insights.lock().await;
                    let reverse_deps =
                        if let Some(backend) = manager.get_backend(pkg_for_insights.source) {
                            backend
                                .get_reverse_dependencies(&pkg_for_insights.name)
                                .await
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                    drop(manager);

                    let install_date = pkg_for_insights
                        .install_date
                        .as_ref()
                        .and_then(|d| parse_install_date(d));
                    let config_paths =
                        guess_config_paths(&pkg_for_insights.name, pkg_for_insights.source);
                    let log_command =
                        guess_log_command(&pkg_for_insights.name, pkg_for_insights.source);

                    let insights = PackageInsights::new()
                        .with_install_date(install_date)
                        .with_dependencies(pkg_for_insights.dependencies.len(), 0)
                        .with_reverse_deps(reverse_deps)
                        .with_config_paths(config_paths)
                        .with_log_command(log_command);

                    sender_insights.input(DetailsPanelInput::InsightsLoaded(insights));
                });

                let sender_recs = sender.clone();
                let pkg_name = pkg.name.clone();
                let dismissed = self.config.borrow().dismissed_recommendations.clone();
                relm4::spawn(async move {
                    let dismissed_set: std::collections::HashSet<String> =
                        dismissed.into_iter().collect();
                    let recs = get_package_recommendations(&pkg_name, &[], &dismissed_set, 4);
                    sender_recs.input(DetailsPanelInput::RecommendationsLoaded(recs));
                });

                crate::ui::set_ui_marker(format!("DetailsShowPackageDone [{}]", new_id));
            }

            DetailsPanelInput::Clear => {
                self.package = None;
                self.enrichment = None;
                self.enrichment_loading = false;
                self.insights = None;
                self.insights_loading = false;
                self.recommendations = Vec::new();
                self.high_res_icon = None;
                self.changelog = None;
                self.changelog_summary = None;
                self.changelog_loading = false;
                self.changelog_error = None;
                self.changelog_fetched = false;
                self.displayed_package_id = None;
                self.pending_sandbox_rebuild.set(false);
                self.pending_enrichment_rebuild.set(false);
                self.pending_insights_rebuild.set(false);
                self.pending_recommendations_rebuild.set(false);
                self.pending_collections_rebuild.set(false);
                self.pending_changelog_rebuild.set(false);
            }

            DetailsPanelInput::EnrichmentLoaded(enrichment) => {
                let has_enrichment = enrichment.is_some();
                self.enrichment = enrichment;
                self.enrichment_loading = false;
                self.pending_enrichment_rebuild.set(true);

                tracing::debug!(has_enrichment, "Details enrichment loaded");

                if let Some(ref e) = self.enrichment {
                    if let Some(ref icon_url) = e.icon_url {
                        let icon_url = icon_url.clone();
                        let sender = sender.clone();
                        relm4::spawn(async move {
                            if let Ok(resp) = reqwest::get(&icon_url).await {
                                if let Ok(bytes) = resp.bytes().await {
                                    sender.input(DetailsPanelInput::IconLoaded(bytes.to_vec()));
                                }
                            }
                        });
                    }
                }
            }

            DetailsPanelInput::InsightsLoaded(insights) => {
                self.insights = Some(insights);
                self.insights_loading = false;
                self.pending_insights_rebuild.set(true);
            }

            DetailsPanelInput::RecommendationsLoaded(recs) => {
                self.recommendations = recs;
                self.pending_recommendations_rebuild.set(true);
            }

            DetailsPanelInput::InstallRecommendation(name) => {
                sender
                    .output(DetailsPanelOutput::ShowToast(format!(
                        "Search for '{}' to install",
                        name
                    )))
                    .ok();
            }

            DetailsPanelInput::DismissRecommendation(name) => {
                {
                    let mut cfg = self.config.borrow_mut();
                    if !cfg.dismissed_recommendations.contains(&name) {
                        cfg.dismissed_recommendations.push(name.clone());
                        let _ = cfg.save();
                    }
                }
                self.recommendations.retain(|r| r.name != name);
                self.pending_recommendations_rebuild.set(true);
            }

            DetailsPanelInput::IconLoaded(bytes) => {
                let bytes = glib::Bytes::from_owned(bytes);
                let stream = gio::MemoryInputStream::from_bytes(&bytes);
                let sender = sender.clone();
                relm4::spawn_local(async move {
                    if let Ok(pixbuf) =
                        gdk_pixbuf::Pixbuf::from_stream_at_scale_future(&stream, 128, 128, true)
                            .await
                    {
                        let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
                        sender.input(DetailsPanelInput::IconTextureReady(texture));
                    }
                });
            }

            DetailsPanelInput::IconTextureReady(texture) => {
                self.high_res_icon = Some(texture);
            }

            DetailsPanelInput::ChangelogExpanded => {
                if self.changelog_fetched || self.changelog_loading {
                    return;
                }
                self.changelog_fetched = true;
                self.changelog_loading = true;

                if let Some(pkg) = &self.package {
                    let pkg = pkg.clone();
                    let pm = self.pm.clone();
                    let sender = sender.clone();
                    relm4::spawn(async move {
                        let result = {
                            let manager = pm.lock().await;
                            manager.get_changelog(&pkg).await
                        };
                        let mapped = result.map_err(|e| e.to_string());
                        sender.input(DetailsPanelInput::ChangelogLoaded(mapped));
                    });
                }
            }

            DetailsPanelInput::ChangelogLoaded(result) => {
                self.changelog_loading = false;
                self.pending_changelog_rebuild.set(true);
                match result {
                    Ok(Some(log)) => {
                        self.changelog_summary = Some(ChangelogSummary::parse(&log));
                        self.changelog = Some(log);
                    }
                    Ok(None) => {
                        self.changelog = None;
                        self.changelog_summary = None;
                    }
                    Err(e) => self.changelog_error = Some(e),
                }
            }

            DetailsPanelInput::ToggleIgnoreUpdates(ignored) => {
                if let Some(pkg) = &self.package {
                    let pkg_id = pkg.id();
                    let mut cfg = self.config.borrow_mut();
                    if ignored {
                        if !cfg.ignored_packages.contains(&pkg_id) {
                            cfg.ignored_packages.push(pkg_id);
                            let _ = cfg.save();
                            sender
                                .output(DetailsPanelOutput::ShowToast(
                                    "Package updates ignored".to_string(),
                                ))
                                .ok();
                        }
                    } else if let Some(pos) = cfg.ignored_packages.iter().position(|x| x == &pkg_id)
                    {
                        cfg.ignored_packages.remove(pos);
                        let _ = cfg.save();
                        sender
                            .output(DetailsPanelOutput::ShowToast(
                                "Package updates enabled".to_string(),
                            ))
                            .ok();
                    }
                }
            }

            DetailsPanelInput::ToggleCollection(collection_name) => {
                if let Some(pkg) = &self.package {
                    sender
                        .output(DetailsPanelOutput::ToggleCollection {
                            pkg_id: pkg.id(),
                            collection: collection_name,
                        })
                        .ok();
                }
            }

            DetailsPanelInput::PreviewScreenshot(texture) => {
                sender.output(DetailsPanelOutput::ShowImage(texture)).ok();
            }

            DetailsPanelInput::UpdatePackage => {
                if self.operation_in_progress {
                    return;
                }
                if let Some(pkg) = &self.package {
                    self.operation_in_progress = true;
                    self.operation_label = "Updating...".to_string();

                    let pkg = pkg.clone();
                    let pm = self.pm.clone();
                    let sender = sender.clone();
                    relm4::spawn(async move {
                        let result = {
                            let manager = pm.lock().await;
                            manager.update(&pkg).await
                        };
                        match result {
                            Ok(_) => {
                                sender
                                    .output(DetailsPanelOutput::ShowToast(format!(
                                        "Updated {}",
                                        pkg.name
                                    )))
                                    .ok();
                                sender.output(DetailsPanelOutput::Reload).ok();
                                sender.input(DetailsPanelInput::Close);
                            }
                            Err(e) => {
                                sender
                                    .output(DetailsPanelOutput::ShowToast(format!(
                                        "Update failed: {}",
                                        e
                                    )))
                                    .ok();
                            }
                        }
                    });
                }
            }

            DetailsPanelInput::RemovePackage => {
                if self.operation_in_progress {
                    return;
                }
                if let Some(pkg) = &self.package {
                    let pkg_name = pkg.name.clone();
                    let pkg_source = pkg.source;

                    let dialog = adw::MessageDialog::builder()
                        .heading("Remove Package")
                        .body(format!(
                            "Are you sure you want to remove {} from {}?",
                            pkg_name, pkg_source
                        ))
                        .build();

                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("remove", "Remove");
                    dialog.set_default_response(Some("cancel"));
                    dialog.set_close_response("cancel");
                    dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);

                    let pkg_for_dialog = pkg.clone();
                    let pm_for_dialog = self.pm.clone();
                    let sender_dialog = sender.clone();

                    dialog.connect_response(None, move |_, response| {
                        if response == "remove" {
                            let pkg = pkg_for_dialog.clone();
                            let pm = pm_for_dialog.clone();
                            let sender = sender_dialog.clone();
                            sender.input(DetailsPanelInput::UpdateOperationProgress(
                                true,
                                "Removing...".to_string(),
                            ));

                            relm4::spawn(async move {
                                let result = {
                                    let manager = pm.lock().await;
                                    manager.remove(&pkg).await
                                };
                                match result {
                                    Ok(_) => {
                                        sender
                                            .output(DetailsPanelOutput::ShowToast(format!(
                                                "Removed {}",
                                                pkg.name
                                            )))
                                            .ok();
                                        sender.output(DetailsPanelOutput::Reload).ok();
                                        sender.input(DetailsPanelInput::Close);
                                    }
                                    Err(e) => {
                                        sender
                                            .output(DetailsPanelOutput::ShowToast(format!(
                                                "Remove failed: {}",
                                                e
                                            )))
                                            .ok();
                                        sender.input(DetailsPanelInput::UpdateOperationProgress(
                                            false,
                                            String::new(),
                                        ));
                                    }
                                }
                            });
                        }
                    });

                    dialog.present();
                }
            }

            DetailsPanelInput::DowngradePackage => {
                if self.operation_in_progress {
                    return;
                }
                if let Some(pkg) = &self.package {
                    self.operation_in_progress = true;
                    let verb = if matches!(pkg.source, PackageSource::Snap) {
                        "Reverting"
                    } else {
                        "Downgrading"
                    };
                    self.operation_label = format!("{}...", verb);

                    let pkg = pkg.clone();
                    let pm = self.pm.clone();
                    let sender = sender.clone();
                    relm4::spawn(async move {
                        let result = {
                            let manager = pm.lock().await;
                            manager.downgrade(&pkg).await
                        };
                        let past = if matches!(pkg.source, PackageSource::Snap) {
                            "Reverted"
                        } else {
                            "Downgraded"
                        };
                        match result {
                            Ok(_) => {
                                sender
                                    .output(DetailsPanelOutput::ShowToast(format!(
                                        "{} {}",
                                        past, pkg.name
                                    )))
                                    .ok();
                                sender.output(DetailsPanelOutput::Reload).ok();
                                sender.input(DetailsPanelInput::Close);
                            }
                            Err(e) => {
                                sender
                                    .output(DetailsPanelOutput::ShowToast(format!("Failed: {}", e)))
                                    .ok();
                            }
                        }
                    });
                }
            }

            DetailsPanelInput::SchedulePackage(task) => {
                let time_display = task.scheduled_time_display();
                sender
                    .output(DetailsPanelOutput::ShowToast(format!(
                        "Scheduled {} for {}",
                        task.operation.display_name().to_lowercase(),
                        time_display
                    )))
                    .ok();
                sender.output(DetailsPanelOutput::ScheduleTask(task)).ok();
            }

            DetailsPanelInput::UpdateOperationProgress(in_progress, label) => {
                self.operation_in_progress = in_progress;
                self.operation_label = label;
            }

            DetailsPanelInput::Close => {
                sender.output(DetailsPanelOutput::Close).ok();
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        let start = Instant::now();

        let Some(pkg) = &self.package else {
            widgets.title_label.set_label("Package Details");
            widgets.name_label.set_label("");
            widgets.description_label.set_visible(false);
            widgets.enrichment_spinner.set_visible(false);
            widgets.update_btn.set_visible(false);
            widgets.remove_btn.set_visible(false);
            widgets.downgrade_btn.set_visible(false);
            widgets.schedule_btn.set_visible(false);
            widgets.scheduled_info_row.set_visible(false);
            widgets.warning_box.set_visible(false);
            widgets.sandbox_box.set_visible(false);
            return;
        };

        crate::ui::set_ui_marker("DetailsUpdateViewStart");

        widgets.title_label.set_label(&pkg.name);
        widgets.name_label.set_label(&pkg.name);

        crate::ui::set_ui_marker("DetailsUpdateViewSetIcon");
        if let Some(ref texture) = self.high_res_icon {
            widgets.high_res_image.set_paintable(Some(texture));
            widgets.icon_stack.set_visible_child_name("high-res");
        } else {
            widgets
                .icon_image
                .set_icon_name(Some(pkg.source.icon_name()));
            widgets.icon_stack.set_visible_child_name("generic");
        }

        for class in ["flatpak", "snap", "apt", "npm", "pip", "cargo"] {
            widgets.source_dot.remove_css_class(class);
        }
        widgets.source_dot.add_css_class(pkg.source.color_class());
        if self.enrichment_loading {
            widgets.source_dot.add_css_class("loading");
        } else {
            widgets.source_dot.remove_css_class("loading");
        }
        widgets.source_label.set_label(&pkg.source.to_string());

        let has_desc = !pkg.description.is_empty();

        crate::ui::set_ui_marker("DetailsUpdateViewSetDescription");
        widgets
            .description_label
            .set_label(&strip_html_tags(&pkg.description));
        widgets.description_label.set_visible(
            has_desc
                && self
                    .enrichment
                    .as_ref()
                    .and_then(|e| e.summary.as_ref())
                    .is_none(),
        );

        widgets
            .enrichment_spinner
            .set_visible(self.enrichment_loading);

        if self.pending_enrichment_rebuild.get() {
            self.pending_enrichment_rebuild.set(false);
            crate::ui::set_ui_marker("DetailsUpdateViewEnrichmentRebuild");

            let mut next = widgets.enrichment_box.first_child();
            while let Some(child) = next {
                next = child.next_sibling();
                if child.downcast_ref::<gtk::Spinner>().is_none() {
                    widgets.enrichment_box.remove(&child);
                }
            }

            if let Some(ref enrichment_data) = self.enrichment {
                crate::ui::set_ui_marker("DetailsUpdateViewEnrichmentBuildSection");
                let section = enrichment::build_section(enrichment_data, sender.clone());
                widgets.enrichment_box.append(&section);
                if enrichment_data.summary.is_some() {
                    widgets.description_label.set_visible(false);
                }
            }
        }

        widgets.version_row.set_subtitle(&pkg.display_version());
        widgets.version_update_icon.set_visible(pkg.has_update());
        widgets.status_row.set_subtitle(&pkg.status.to_string());
        widgets.size_row.set_subtitle(&pkg.size_display());
        widgets.source_row.set_subtitle(pkg.source.description());

        if self.pending_insights_rebuild.get() {
            self.pending_insights_rebuild.set(false);

            if let Some(ref insights) = self.insights {
                widgets.insights_group.set_visible(true);

                if let Some(age) = insights.install_age_display() {
                    widgets.install_date_row.set_subtitle(&age);
                } else {
                    widgets.install_date_row.set_subtitle("Unknown");
                }

                widgets.deps_row.set_subtitle(&insights.deps_display());
                widgets
                    .reverse_deps_row
                    .set_subtitle(&insights.reverse_deps_display());

                let (icon, label) = insights.safe_to_remove_display();
                widgets.safe_remove_icon.set_icon_name(Some(icon));
                widgets.safe_remove_row.set_subtitle(label);

                if insights.is_safe_to_remove {
                    widgets.safe_remove_icon.remove_css_class("warning");
                    widgets.safe_remove_icon.add_css_class("success");
                } else {
                    widgets.safe_remove_icon.remove_css_class("success");
                    widgets.safe_remove_icon.add_css_class("warning");
                }

                if !insights.config_paths.is_empty() {
                    widgets.config_row.set_visible(true);
                    widgets
                        .config_row
                        .set_subtitle(&insights.config_paths.join(", "));
                } else {
                    widgets.config_row.set_visible(false);
                }
            } else if self.insights_loading {
                widgets.insights_group.set_visible(true);
                widgets.install_date_row.set_subtitle("Loading...");
                widgets.deps_row.set_subtitle("Loading...");
                widgets.reverse_deps_row.set_subtitle("Loading...");
                widgets.safe_remove_row.set_subtitle("Checking...");
            } else {
                widgets.insights_group.set_visible(false);
            }

            while let Some(child) = widgets.dependencies_box.first_child() {
                widgets.dependencies_box.remove(&child);
            }

            if !pkg.dependencies.is_empty() || self.insights.is_some() {
                let deps_section = super::dependencies::build_dependencies_section(
                    &pkg.dependencies,
                    self.insights.as_ref(),
                );
                widgets.dependencies_box.append(&deps_section);
                widgets.dependencies_box.set_visible(true);
            } else {
                widgets.dependencies_box.set_visible(false);
            }
        }

        if self.pending_recommendations_rebuild.get() {
            self.pending_recommendations_rebuild.set(false);
            crate::ui::set_ui_marker("DetailsUpdateViewRecommendationsRebuild");

            while let Some(child) = widgets.recommendations_box.first_child() {
                widgets.recommendations_box.remove(&child);
            }

            if self.recommendations.is_empty() {
                widgets.recommendations_group.set_visible(false);
            } else {
                widgets.recommendations_group.set_visible(true);

                for rec in &self.recommendations {
                    let row = adw::ActionRow::builder()
                        .title(&rec.name)
                        .subtitle(strip_html_tags(&rec.description))
                        .build();

                    let cat_icon = gtk::Image::builder().icon_name(&rec.category_icon).build();
                    row.add_prefix(&cat_icon);

                    let button_box = gtk::Box::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .spacing(4)
                        .valign(gtk::Align::Center)
                        .build();

                    let install_btn = gtk::Button::builder()
                        .icon_name("list-add-symbolic")
                        .tooltip_text("Install")
                        .css_classes(vec!["flat", "circular"])
                        .build();

                    let dismiss_btn = gtk::Button::builder()
                        .icon_name("window-close-symbolic")
                        .tooltip_text("Dismiss")
                        .css_classes(vec!["flat", "circular"])
                        .build();

                    let name_for_install = rec.name.clone();
                    let sender_install = sender.clone();
                    install_btn.connect_clicked(move |_| {
                        sender_install.input(DetailsPanelInput::InstallRecommendation(
                            name_for_install.clone(),
                        ));
                    });

                    let name_for_dismiss = rec.name.clone();
                    let sender_dismiss = sender.clone();
                    dismiss_btn.connect_clicked(move |_| {
                        sender_dismiss.input(DetailsPanelInput::DismissRecommendation(
                            name_for_dismiss.clone(),
                        ));
                    });

                    button_box.append(&install_btn);
                    button_box.append(&dismiss_btn);
                    row.add_suffix(&button_box);

                    widgets.recommendations_box.append(&row);
                }
            }
        }

        let is_ignored = self.config.borrow().ignored_packages.contains(&pkg.id());
        crate::ui::set_ui_marker("DetailsUpdateViewSetIgnoreSwitch");
        widgets.ignore_switch.set_active(is_ignored);

        if self.pending_collections_rebuild.get() {
            self.pending_collections_rebuild.set(false);
            crate::ui::set_ui_marker("DetailsUpdateViewCollectionsRebuild");

            while let Some(child) = widgets.collections_popover_box.first_child() {
                widgets.collections_popover_box.remove(&child);
            }

            let config = self.config.borrow();
            let pkg_id = pkg.id();
            let mut collection_names: Vec<_> = config.collections.keys().cloned().collect();
            collection_names.sort();

            if collection_names.is_empty() {
                let empty_label = gtk::Label::builder()
                    .label("No collections yet")
                    .css_classes(vec!["dim-label"])
                    .build();
                widgets.collections_popover_box.append(&empty_label);
            } else {
                crate::ui::set_ui_marker("DetailsUpdateViewCollectionsBuildButtons");
                for name in collection_names {
                    let is_in_collection = config
                        .collections
                        .get(&name)
                        .map(|ids| ids.contains(&pkg_id))
                        .unwrap_or(false);

                    let check = gtk::CheckButton::builder()
                        .label(&name)
                        .active(is_in_collection)
                        .build();

                    let sender_clone = sender.clone();
                    let name_clone = name.clone();
                    check.connect_toggled(move |_| {
                        sender_clone.input(DetailsPanelInput::ToggleCollection(name_clone.clone()));
                    });

                    widgets.collections_popover_box.append(&check);
                }
            }
            drop(config);
        }

        if self.pending_sandbox_rebuild.get() {
            self.pending_sandbox_rebuild.set(false);
            crate::ui::set_ui_marker("DetailsUpdateViewSandboxRebuild");

            while let Some(child) = widgets.sandbox_box.first_child() {
                widgets.sandbox_box.remove(&child);
            }
            if pkg.source == PackageSource::Flatpak {
                crate::ui::set_ui_marker("DetailsUpdateViewSandboxBuildSection");
                let section = sandbox::build_sandbox_section(self.pm.clone(), pkg.name.clone());
                widgets.sandbox_box.append(&section);
                widgets.sandbox_box.set_visible(true);
            } else {
                widgets.sandbox_box.set_visible(false);
            }
        }

        widgets
            .changelog_spinner
            .set_visible(self.changelog_loading);

        if self.pending_changelog_rebuild.get() {
            self.pending_changelog_rebuild.set(false);
            crate::ui::set_ui_marker("DetailsUpdateViewChangelogClear");
            let mut next = widgets.changelog_content.first_child();
            while let Some(child) = next {
                next = child.next_sibling();
                if child.downcast_ref::<gtk::Spinner>().is_none() {
                    widgets.changelog_content.remove(&child);
                }
            }
            crate::ui::set_ui_marker("DetailsUpdateViewChangelogCleared");

            if let Some(ref summary) = self.changelog_summary {
                let label = format!("Release History — {}", summary.summary_text());
                widgets.changelog_expander.set_label(Some(&label));
            } else {
                widgets
                    .changelog_expander
                    .set_label(Some("Release History"));
            }

            if let Some(ref log) = self.changelog {
                crate::ui::set_ui_marker("DetailsUpdateViewChangelogBuildView");
                let scrolled = gtk::ScrolledWindow::builder()
                    .min_content_height(200)
                    .max_content_height(400)
                    .hscrollbar_policy(gtk::PolicyType::Never)
                    .build();

                let text_view = gtk::TextView::builder()
                    .editable(false)
                    .cursor_visible(false)
                    .wrap_mode(gtk::WrapMode::Word)
                    .margin_top(8)
                    .margin_bottom(8)
                    .margin_start(8)
                    .margin_end(8)
                    .css_classes(vec!["monospace"])
                    .build();
                crate::ui::set_ui_marker("DetailsUpdateViewChangelogSetText");
                apply_changelog_formatting(&text_view.buffer(), log);
                scrolled.set_child(Some(&text_view));
                widgets.changelog_content.append(&scrolled);
            } else if self.changelog_fetched && !self.changelog_loading {
                let label_text = self
                    .changelog_error
                    .as_deref()
                    .unwrap_or("No release history available for this package.");
                let label = gtk::Label::builder()
                    .label(label_text)
                    .xalign(0.0)
                    .margin_top(8)
                    .margin_bottom(8)
                    .css_classes(vec!["dim-label"])
                    .build();
                if self.changelog_error.is_some() {
                    label.add_css_class("error");
                }
                widgets.changelog_content.append(&label);
            }
        }

        if let Some(warning) = pkg.source.gui_operation_warning() {
            widgets.warning_label.set_label(warning);
            widgets.warning_box.set_visible(true);
        } else {
            widgets.warning_box.set_visible(false);
        }

        let supports_gui = pkg.source.supports_gui_operations();
        widgets
            .update_btn
            .set_visible(pkg.has_update() && supports_gui);
        widgets
            .remove_btn
            .set_visible(pkg.status == PackageStatus::Installed && supports_gui);

        let supports_downgrade = matches!(
            pkg.source,
            PackageSource::Snap | PackageSource::Dnf | PackageSource::Flatpak | PackageSource::Apt
        ) && matches!(
            pkg.status,
            PackageStatus::Installed | PackageStatus::UpdateAvailable
        );
        widgets.downgrade_btn.set_visible(supports_downgrade);
        let downgrade_label = match pkg.source {
            PackageSource::Snap => "Revert",
            PackageSource::Flatpak => "Rollback",
            _ => "Downgrade",
        };
        widgets.downgrade_btn.set_label(downgrade_label);

        let show_schedule = pkg.has_update() && supports_gui;
        widgets.schedule_btn.set_visible(show_schedule);
        if show_schedule {
            let pkg_clone = pkg.clone();
            let sender_schedule = sender.clone();
            let popover =
                build_schedule_popover(&pkg_clone, ScheduledOperation::Update, move |result| {
                    sender_schedule.input(DetailsPanelInput::SchedulePackage(result.task));
                });
            widgets.schedule_btn.set_popover(Some(&popover));
        }

        let pkg_id = pkg.id();
        let config = self.config.borrow();
        if let Some(task) = config.scheduler.get_pending_for_package(&pkg_id) {
            widgets.scheduled_info_row.set_subtitle(&format!(
                "{} scheduled for {}",
                task.operation.display_name(),
                task.scheduled_time_display()
            ));
            widgets.scheduled_info_row.set_visible(true);
        } else {
            widgets.scheduled_info_row.set_visible(false);
        }
        drop(config);

        if self.operation_in_progress {
            widgets.update_btn.set_sensitive(false);
            widgets.remove_btn.set_sensitive(false);
            widgets.downgrade_btn.set_sensitive(false);
            if widgets.update_btn.is_visible() {
                widgets.update_btn.set_label(&self.operation_label);
            } else if widgets.remove_btn.is_visible() {
                widgets.remove_btn.set_label(&self.operation_label);
            }
        } else {
            widgets.update_btn.set_sensitive(true);
            widgets.remove_btn.set_sensitive(true);
            widgets.downgrade_btn.set_sensitive(true);
            widgets.update_btn.set_label("Update");
            widgets.remove_btn.set_label("Remove");
        }

        crate::ui::set_ui_marker("DetailsUpdateViewDone");
        let elapsed = start.elapsed();
        if elapsed > Duration::from_millis(30) {
            tracing::warn!(
                elapsed_ms = elapsed.as_millis() as u64,
                pkg_id = %pkg.id(),
                pkg_name = %pkg.name,
                source = ?pkg.source,
                "Details panel update_view slow"
            );
        }
    }
}
