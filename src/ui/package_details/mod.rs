pub mod enrichment;
pub mod panel;

pub use panel::PackageDetailsPanel;

use crate::backend::PackageManager;
use crate::models::{fetch_enrichment, Config, Package, PackageSource, PackageStatus};
use crate::ui::{CommandCenter, CommandEventKind, PackageOp, RetrySpec};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

#[allow(dead_code)]
pub struct PackageDetailsDialog;

#[allow(dead_code)]
fn parse_suggestion(message: &str) -> Option<(String, String)> {
    let marker = crate::backend::SUGGEST_PREFIX;
    let idx = message.find(marker)?;
    let command = message[idx + marker.len()..].trim();
    if command.is_empty() {
        return None;
    }
    Some((message[..idx].trim().to_string(), command.to_string()))
}

#[allow(dead_code)]
fn show_action_required_dialog(parent: &impl IsA<gtk::Window>, details: &str, command: &str) {
    let win = gtk::Window::builder()
        .title("Action Required")
        .modal(true)
        .transient_for(parent)
        .default_width(500)
        .default_height(300)
        .build();

    let header = adw::HeaderBar::new();
    win.set_titlebar(Some(&header));

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_start(24)
        .margin_end(24)
        .margin_top(16)
        .margin_bottom(24)
        .build();

    let details_label = gtk::Label::builder()
        .label(details)
        .wrap(true)
        .xalign(0.0)
        .build();
    content.append(&details_label);

    let command_entry = gtk::Entry::builder().text(command).editable(false).build();
    command_entry.add_css_class("monospace");
    content.append(&command_entry);

    let copy_btn = gtk::Button::builder().label("Copy Command").build();
    copy_btn.add_css_class("suggested-action");
    let cmd_for_copy = command.to_string();
    copy_btn.connect_clicked(move |btn| {
        if let Some(display) = gtk::gdk::Display::default() {
            display.clipboard().set_text(&cmd_for_copy);
            btn.set_label("Copied!");
        }
    });
    content.append(&copy_btn);

    win.set_child(Some(&content));

    let close_btn = gtk::Button::builder().label("Close").build();
    header.pack_end(&close_btn);
    close_btn.connect_clicked({
        let win = win.clone();
        move |_| win.close()
    });

    win.present();
}

#[allow(dead_code)]
impl PackageDetailsDialog {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        package: &Package,
        parent: &impl IsA<gtk::Window>,
        pm: Arc<Mutex<PackageManager>>,
        toast_overlay: adw::ToastOverlay,
        config: Rc<RefCell<Config>>,
        reload_packages: Option<Rc<dyn Fn()>>,
        command_center: Option<CommandCenter>,
        reveal_command_center: Option<Rc<dyn Fn(bool)>>,
    ) {
        tracing::debug!("Opening package details for: {}", package.name);
        let dialog = gtk::Window::builder()
            .title(&package.name)
            .default_width(500)
            .default_height(450)
            .modal(true)
            .transient_for(parent)
            .build();

        let header = adw::HeaderBar::new();
        dialog.set_titlebar(Some(&header));

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .margin_start(24)
            .margin_end(24)
            .margin_top(16)
            .margin_bottom(24)
            .build();

        let header_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(16)
            .build();

        let icon = gtk::Image::builder()
            .icon_name(package.source.icon_name())
            .pixel_size(64)
            .build();

        let title_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .valign(gtk::Align::Center)
            .build();

        let name_label = gtk::Label::builder()
            .label(&package.name)
            .xalign(0.0)
            .build();
        name_label.add_css_class("title-1");

        let source_badge = gtk::Label::builder()
            .label(package.source.to_string())
            .xalign(0.0)
            .build();
        source_badge.add_css_class("chip");
        source_badge.add_css_class(package.source.color_class());

        title_box.append(&name_label);
        title_box.append(&source_badge);

        header_box.append(&icon);
        header_box.append(&title_box);

        content.append(&header_box);

        let desc_label = gtk::Label::builder()
            .label(&package.description)
            .wrap(true)
            .xalign(0.0)
            .visible(!package.description.is_empty())
            .build();
        desc_label.add_css_class("dim-label");
        content.append(&desc_label);

        let enrichment_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .build();

        let loading_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        let spinner = gtk::Spinner::builder().spinning(true).build();
        let loading_label = gtk::Label::new(Some("Loading details…"));
        loading_label.add_css_class("dim-label");
        loading_label.add_css_class("caption");
        loading_box.append(&spinner);
        loading_box.append(&loading_label);
        enrichment_box.append(&loading_box);

        content.append(&enrichment_box);

        let pkg_for_enrich = package.clone();
        let desc_label_clone = desc_label.clone();
        let enrichment_box_clone = enrichment_box.clone();
        glib::spawn_future_local(async move {
            tracing::debug!("Starting enrichment fetch for {}", pkg_for_enrich.name);

            if let Some(enrichment) = fetch_enrichment(&pkg_for_enrich).await {
                tracing::debug!("Got enrichment for {}", pkg_for_enrich.name);
                while let Some(child) = enrichment_box_clone.first_child() {
                    enrichment_box_clone.remove(&child);
                }

                if enrichment.summary.is_some() {
                    desc_label_clone.set_visible(false);
                }

                let section = enrichment::build_section(&enrichment);
                enrichment_box_clone.append(&section);
            } else {
                tracing::debug!("No enrichment available for {}", pkg_for_enrich.name);
                while let Some(child) = enrichment_box_clone.first_child() {
                    enrichment_box_clone.remove(&child);
                }
            }
        });

        let details_group = adw::PreferencesGroup::builder().title("Details").build();

        let version_row = adw::ActionRow::builder()
            .title("Version")
            .subtitle(package.display_version())
            .build();
        if package.has_update() {
            let update_icon = gtk::Image::builder()
                .icon_name("software-update-available-symbolic")
                .build();
            update_icon.add_css_class("accent");
            version_row.add_suffix(&update_icon);
        }
        details_group.add(&version_row);

        let status_row = adw::ActionRow::builder()
            .title("Status")
            .subtitle(package.status.to_string())
            .build();
        details_group.add(&status_row);

        let size_row = adw::ActionRow::builder()
            .title("Size")
            .subtitle(package.size_display())
            .build();
        details_group.add(&size_row);

        let source_row = adw::ActionRow::builder()
            .title("Source")
            .subtitle(package.source.description())
            .build();
        details_group.add(&source_row);

        let ignore_row = adw::ActionRow::builder()
            .title("Ignore Updates")
            .subtitle("Prevent this package from being updated")
            .build();

        let ignore_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();

        let pkg_id = package.id();
        let is_ignored = config.borrow().ignored_packages.contains(&pkg_id);
        ignore_switch.set_active(is_ignored);

        let config_clone = config.clone();
        let pkg_id_clone = pkg_id.clone();
        let toast_clone = toast_overlay.clone();
        let reload_packages_ignore = reload_packages.clone();

        ignore_switch.connect_state_set(move |_, state| {
            let mut cfg = config_clone.borrow_mut();
            if state {
                if !cfg.ignored_packages.contains(&pkg_id_clone) {
                    cfg.ignored_packages.push(pkg_id_clone.clone());
                    let _ = cfg.save();
                    let t = adw::Toast::new("Package updates ignored");
                    toast_clone.add_toast(t);
                }
            } else if let Some(pos) = cfg.ignored_packages.iter().position(|x| x == &pkg_id_clone) {
                cfg.ignored_packages.remove(pos);
                let _ = cfg.save();
                let t = adw::Toast::new("Package updates enabled");
                toast_clone.add_toast(t);
            }
            drop(cfg);
            if let Some(reload) = reload_packages_ignore.as_ref() {
                reload();
            }
            glib::Propagation::Proceed
        });

        ignore_row.add_suffix(&ignore_switch);
        details_group.add(&ignore_row);

        content.append(&details_group);

        let changelog_expander = gtk::Expander::builder()
            .label("Release History")
            .expanded(false)
            .margin_top(8)
            .build();
        changelog_expander.add_css_class("card");

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
            .build();
        changelog_content.append(&changelog_spinner);

        changelog_expander.set_child(Some(&changelog_content));
        content.append(&changelog_expander);

        let pkg_for_changelog = package.clone();
        let pm_for_changelog = pm.clone();
        let changelog_content_clone = changelog_content.clone();
        let changelog_spinner_clone = changelog_spinner.clone();
        let changelog_fetched = Rc::new(RefCell::new(false));
        let changelog_fetched_clone = changelog_fetched.clone();

        changelog_expander.connect_expanded_notify(move |exp| {
            if !exp.is_expanded() {
                return;
            }
            if *changelog_fetched_clone.borrow() {
                return;
            }
            *changelog_fetched_clone.borrow_mut() = true;

            let pkg = pkg_for_changelog.clone();
            let pm = pm_for_changelog.clone();
            let content_box = changelog_content_clone.clone();
            let spinner = changelog_spinner_clone.clone();

            glib::spawn_future_local(async move {
                let changelog_result = {
                    let manager = pm.lock().await;
                    manager.get_changelog(&pkg).await
                };

                spinner.set_visible(false);

                match changelog_result {
                    Ok(Some(changelog)) => {
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
                            .build();
                        text_view.add_css_class("monospace");
                        text_view.buffer().set_text(&changelog);

                        scrolled.set_child(Some(&text_view));
                        content_box.append(&scrolled);
                    }
                    Ok(None) => {
                        let no_changelog_label = gtk::Label::builder()
                            .label("No release history available for this package.")
                            .xalign(0.0)
                            .margin_top(8)
                            .margin_bottom(8)
                            .build();
                        no_changelog_label.add_css_class("dim-label");
                        content_box.append(&no_changelog_label);
                    }
                    Err(e) => {
                        let error_label = gtk::Label::builder()
                            .label(format!("Failed to fetch changelog: {}", e))
                            .xalign(0.0)
                            .margin_top(8)
                            .margin_bottom(8)
                            .build();
                        error_label.add_css_class("dim-label");
                        error_label.add_css_class("error");
                        content_box.append(&error_label);
                    }
                }
            });
        });

        if let Some(warning) = package.source.gui_operation_warning() {
            let info_bar = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(8)
                .margin_top(8)
                .build();
            info_bar.add_css_class("card");
            info_bar.add_css_class("warning");

            let icon = gtk::Image::builder()
                .icon_name("dialog-warning-symbolic")
                .margin_start(12)
                .margin_top(8)
                .margin_bottom(8)
                .build();

            let label = gtk::Label::builder()
                .label(warning)
                .wrap(true)
                .xalign(0.0)
                .margin_end(12)
                .margin_top(8)
                .margin_bottom(8)
                .build();
            label.add_css_class("dim-label");

            info_bar.append(&icon);
            info_bar.append(&label);
            content.append(&info_bar);
        }

        let actions_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Center)
            .margin_top(16)
            .build();

        if matches!(package.source, PackageSource::Snap | PackageSource::Dnf)
            && matches!(
                package.status,
                PackageStatus::Installed | PackageStatus::UpdateAvailable
            )
        {
            let downgrade_label = if matches!(package.source, PackageSource::Snap) {
                "Revert"
            } else {
                "Downgrade"
            };
            let downgrade_btn = gtk::Button::builder().label(downgrade_label).build();
            downgrade_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_clone = dialog.clone();
            let reload_packages_downgrade = reload_packages.clone();
            let command_center_clone = command_center.clone();
            let reveal_command_center_clone = reveal_command_center.clone();

            downgrade_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                let reload_packages_downgrade = reload_packages_downgrade.clone();
                let command_center = command_center_clone.clone();
                let reveal_command_center = reveal_command_center_clone.clone();

                btn.set_sensitive(false);
                btn.set_label(if matches!(pkg.source, PackageSource::Snap) {
                    "Reverting..."
                } else {
                    "Downgrading..."
                });

                let task = command_center.as_ref().map(|c| {
                    let title = if matches!(pkg.source, PackageSource::Snap) {
                        format!("Reverting {}", pkg.name)
                    } else {
                        format!("Downgrading {}", pkg.name)
                    };
                    c.begin_task(
                        title,
                        format!("Source: {}", pkg.source),
                        Some(RetrySpec::Package {
                            package: Box::new(pkg.clone()),
                            op: PackageOp::Downgrade,
                        }),
                    )
                });

                glib::spawn_future_local(async move {
                    let manager = pm.lock().await;
                    let result = manager.downgrade(&pkg).await;
                    drop(manager);

                    dialog.close();

                    match result {
                        Ok(_) => {
                            let verb = if matches!(pkg.source, PackageSource::Snap) {
                                "Reverted"
                            } else {
                                "Downgraded"
                            };
                            let t = adw::Toast::new(&format!("{} {}", verb, pkg.name));
                            t.set_timeout(3);
                            toast.add_toast(t);
                            if let Some(task) = task.as_ref() {
                                task.finish(
                                    CommandEventKind::Success,
                                    format!("{} {}", verb, pkg.name),
                                    format!("Source: {}", pkg.source),
                                    None,
                                    true,
                                );
                            }
                        }
                        Err(e) => {
                            let prefix = if matches!(pkg.source, PackageSource::Snap) {
                                "Revert"
                            } else {
                                "Downgrade"
                            };
                            let msg = format!("{} failed: {}", prefix, e);
                            if let Some((details, command)) = parse_suggestion(&msg) {
                                if let Some(task) = task.as_ref() {
                                    task.finish(
                                        CommandEventKind::Error,
                                        "Action required",
                                        &details,
                                        Some(command.clone()),
                                        true,
                                    );
                                    if let Some(reveal) = reveal_command_center.as_ref() {
                                        reveal(true);
                                    }
                                } else {
                                    show_action_required_dialog(&dialog, &details, &command);
                                }
                                let t = adw::Toast::new("Action required");
                                t.set_timeout(5);
                                toast.add_toast(t);
                            } else {
                                let t = adw::Toast::new(&msg);
                                t.set_timeout(5);
                                toast.add_toast(t);
                                if let Some(task) = task.as_ref() {
                                    task.finish(
                                        CommandEventKind::Error,
                                        "Operation failed",
                                        &msg,
                                        None,
                                        true,
                                    );
                                    if let Some(reveal) = reveal_command_center.as_ref() {
                                        reveal(true);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(reload) = reload_packages_downgrade.as_ref() {
                        reload();
                    }
                });
            });

            actions_box.append(&downgrade_btn);
        }

        if matches!(
            package.source,
            PackageSource::Apt
                | PackageSource::Npm
                | PackageSource::Pip
                | PackageSource::Pipx
                | PackageSource::Cargo
                | PackageSource::Dart
                | PackageSource::Conda
                | PackageSource::Mamba
        ) && matches!(
            package.status,
            PackageStatus::Installed | PackageStatus::UpdateAvailable | PackageStatus::NotInstalled
        ) {
            let (button_label, title, subtitle, apply_label, apply_past) =
                if matches!(package.source, PackageSource::Apt)
                    && matches!(
                        package.status,
                        PackageStatus::Installed | PackageStatus::UpdateAvailable
                    )
                {
                    (
                        "Downgrade…",
                        "Downgrade package",
                        "Select (or type) an older version to install",
                        "Downgrade",
                        "Downgraded",
                    )
                } else {
                    (
                        "Install version…",
                        "Install specific version",
                        "Select (or type) a version to install",
                        "Install",
                        "Installed",
                    )
                };

            let version_btn = gtk::Button::builder().label(button_label).build();
            version_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_parent = dialog.clone();
            let reload_packages_change = reload_packages.clone();
            let title = title.to_string();
            let subtitle = subtitle.to_string();
            let apply_label = apply_label.to_string();
            let apply_past = apply_past.to_string();
            let command_center_clone = command_center.clone();
            let reveal_command_center_clone = reveal_command_center.clone();

            version_btn.connect_clicked(move |_| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let parent = dialog_parent.clone();
                let reload = reload_packages_change.clone();
                let title = title.clone();
                let subtitle = subtitle.clone();
                let apply_label = apply_label.clone();
                let apply_past = apply_past.clone();
                let command_center = command_center_clone.clone();
                let reveal_command_center = reveal_command_center_clone.clone();

                glib::spawn_future_local(async move {
                    let versions = {
                        let manager = pm.lock().await;
                        manager
                            .available_downgrade_versions(&pkg)
                            .await
                            .unwrap_or_default()
                    };

                    let win = gtk::Window::builder()
                        .title(&title)
                        .modal(true)
                        .transient_for(&parent)
                        .default_width(560)
                        .default_height(460)
                        .build();

                    let header = adw::HeaderBar::new();
                    win.set_titlebar(Some(&header));
                    let close_btn = gtk::Button::builder().label("Close").build();
                    header.pack_start(&close_btn);

                    let apply_btn = gtk::Button::builder().label(&apply_label).build();
                    apply_btn.add_css_class("suggested-action");
                    header.pack_end(&apply_btn);

                    let subtitle_lbl = gtk::Label::builder().label(&subtitle).xalign(0.0).build();
                    subtitle_lbl.add_css_class("dim-label");

                    let version_entry =
                        gtk::Entry::builder().placeholder_text("e.g. 1.2.3").build();
                    version_entry.add_css_class("monospace");

                    let entry_row = adw::ActionRow::builder()
                        .title("Version")
                        .subtitle("Leave blank to cancel")
                        .build();
                    entry_row.add_suffix(&version_entry);

                    let list = gtk::ListBox::builder()
                        .selection_mode(gtk::SelectionMode::Single)
                        .css_classes(vec!["boxed-list"])
                        .build();

                    for v in versions.iter().take(80) {
                        let row = gtk::ListBoxRow::new();
                        let label = gtk::Label::builder().label(v).xalign(0.0).build();
                        label.add_css_class("monospace");
                        label.set_margin_top(10);
                        label.set_margin_bottom(10);
                        label.set_margin_start(12);
                        label.set_margin_end(12);
                        row.set_child(Some(&label));
                        list.append(&row);
                    }

                    let list_hint = gtk::Label::builder()
                        .label(if list.first_child().is_some() {
                            "Pick from the list or type a version above"
                        } else {
                            "No versions found to list; type a version above"
                        })
                        .xalign(0.0)
                        .build();
                    list_hint.add_css_class("caption");
                    list_hint.add_css_class("dim-label");

                    list.connect_row_selected({
                        let version_entry = version_entry.clone();
                        let apply_btn = apply_btn.clone();
                        move |_, row| {
                            let Some(row) = row else {
                                return;
                            };
                            let Some(text) = row
                                .child()
                                .and_then(|w| w.downcast::<gtk::Label>().ok())
                                .map(|l| l.text().to_string())
                            else {
                                return;
                            };
                            version_entry.set_text(&text);
                            apply_btn.set_sensitive(!text.trim().is_empty());
                        }
                    });

                    version_entry.connect_changed({
                        let apply_btn = apply_btn.clone();
                        move |e| apply_btn.set_sensitive(!e.text().trim().is_empty())
                    });

                    apply_btn.set_sensitive(false);

                    let scrolled = gtk::ScrolledWindow::builder()
                        .vexpand(true)
                        .hscrollbar_policy(gtk::PolicyType::Never)
                        .child(&list)
                        .build();

                    let group = adw::PreferencesGroup::new();
                    group.add(&entry_row);

                    let content = gtk::Box::builder()
                        .orientation(gtk::Orientation::Vertical)
                        .spacing(12)
                        .margin_top(12)
                        .margin_bottom(12)
                        .margin_start(12)
                        .margin_end(12)
                        .build();
                    content.append(&subtitle_lbl);
                    content.append(&group);
                    content.append(&list_hint);
                    content.append(&scrolled);

                    let root = gtk::Box::builder()
                        .orientation(gtk::Orientation::Vertical)
                        .build();
                    root.append(&content);
                    win.set_child(Some(&root));

                    close_btn.connect_clicked({
                        let win = win.clone();
                        move |_| win.close()
                    });

                    let parent_for_click = parent.clone();
                    apply_btn.connect_clicked({
                        let pm = pm.clone();
                        let pkg = pkg.clone();
                        let toast = toast.clone();
                        let win = win.clone();
                        let reload = reload.clone();
                        let version_entry = version_entry.clone();
                        let apply_label = apply_label.clone();
                        let apply_past = apply_past.clone();
                        let command_center = command_center.clone();
                        let reveal_command_center = reveal_command_center.clone();
                        move |_| {
                            let version = version_entry.text().trim().to_string();
                            if version.is_empty() {
                                let t = adw::Toast::new("Enter a version first");
                                t.set_timeout(3);
                                toast.add_toast(t);
                                return;
                            }

                            version_entry.set_sensitive(false);

                            let task = command_center.as_ref().map(|c| {
                                c.begin_task(
                                    format!("{} {} {}", apply_label, pkg.name, version),
                                    format!("Source: {}", pkg.source),
                                    Some(RetrySpec::Package {
                                        package: Box::new(pkg.clone()),
                                        op: PackageOp::DowngradeTo(version.clone()),
                                    }),
                                )
                            });

                            glib::spawn_future_local({
                                let pm = pm.clone();
                                let pkg = pkg.clone();
                                let version = version.clone();
                                let toast = toast.clone();
                                let win = win.clone();
                                let reload = reload.clone();
                                let parent_for_async = parent_for_click.clone();
                                let apply_label = apply_label.clone();
                                let apply_past = apply_past.clone();
                                let task = task.clone();
                                let reveal_command_center = reveal_command_center.clone();
                                async move {
                                    let result = {
                                        let manager = pm.lock().await;
                                        manager.downgrade_to(&pkg, &version).await
                                    };

                                    win.close();

                                    match result {
                                        Ok(_) => {
                                            let t = adw::Toast::new(&format!(
                                                "{} {} to {}",
                                                apply_past, pkg.name, version
                                            ));
                                            t.set_timeout(5);
                                            toast.add_toast(t);
                                            if let Some(task) = task.as_ref() {
                                                task.finish(
                                                    CommandEventKind::Success,
                                                    format!(
                                                        "{} {} to {}",
                                                        apply_past, pkg.name, version
                                                    ),
                                                    format!("Source: {}", pkg.source),
                                                    None,
                                                    true,
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            let msg = format!("{} failed: {}", apply_label, e);
                                            if let Some((details, command)) = parse_suggestion(&msg)
                                            {
                                                if let Some(task) = task.as_ref() {
                                                    task.finish(
                                                        CommandEventKind::Error,
                                                        "Action required",
                                                        &details,
                                                        Some(command.clone()),
                                                        true,
                                                    );
                                                    if let Some(reveal) =
                                                        reveal_command_center.as_ref()
                                                    {
                                                        reveal(true);
                                                    }
                                                } else {
                                                    show_action_required_dialog(
                                                        &parent_for_async,
                                                        &details,
                                                        &command,
                                                    );
                                                }
                                                let t = adw::Toast::new("Action required");
                                                t.set_timeout(5);
                                                toast.add_toast(t);
                                            } else {
                                                let t = adw::Toast::new(&msg);
                                                t.set_timeout(5);
                                                toast.add_toast(t);
                                                if let Some(task) = task.as_ref() {
                                                    task.finish(
                                                        CommandEventKind::Error,
                                                        "Operation failed",
                                                        &msg,
                                                        None,
                                                        true,
                                                    );
                                                    if let Some(reveal) =
                                                        reveal_command_center.as_ref()
                                                    {
                                                        reveal(true);
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    if let Some(reload) = reload.as_ref() {
                                        reload();
                                    }
                                }
                            });
                        }
                    });

                    win.present();
                });
            });

            actions_box.append(&version_btn);
        }

        if package.has_update() && package.source.supports_gui_operations() {
            let update_btn = gtk::Button::builder().label("Update").build();
            update_btn.add_css_class("suggested-action");
            update_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_clone = dialog.clone();
            let reload_packages_update = reload_packages.clone();
            let command_center_clone = command_center.clone();
            let reveal_command_center_clone = reveal_command_center.clone();

            update_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                let reload_packages_update = reload_packages_update.clone();
                let command_center = command_center_clone.clone();
                let reveal_command_center = reveal_command_center_clone.clone();

                btn.set_sensitive(false);
                btn.set_label("Updating...");

                glib::spawn_future_local(async move {
                    let manager = pm.lock().await;
                    let result = manager.update(&pkg).await;
                    drop(manager);

                    dialog.close();

                    match result {
                        Ok(_) => {
                            let t = adw::Toast::new(&format!("Updated {}", pkg.name));
                            t.set_timeout(3);
                            toast.add_toast(t);
                            if let Some(center) = command_center.as_ref() {
                                center.add_event(
                                    CommandEventKind::Success,
                                    format!("Updated {}", pkg.name),
                                    format!("Source: {}", pkg.source),
                                    None,
                                );
                            }
                        }
                        Err(e) => {
                            let msg = format!("Update failed: {}", e);
                            if let Some((details, command)) = parse_suggestion(&msg) {
                                if let Some(center) = command_center.as_ref() {
                                    center.add_event(
                                        CommandEventKind::Error,
                                        "Action required",
                                        &details,
                                        Some(command.clone()),
                                    );
                                    if let Some(reveal) = reveal_command_center.as_ref() {
                                        reveal(true);
                                    }
                                } else {
                                    show_action_required_dialog(&dialog, &details, &command);
                                }
                                let t = adw::Toast::new("Action required");
                                t.set_timeout(5);
                                toast.add_toast(t);
                            } else {
                                let t = adw::Toast::new(&msg);
                                t.set_timeout(5);
                                toast.add_toast(t);
                                if let Some(center) = command_center.as_ref() {
                                    center.add_event(
                                        CommandEventKind::Error,
                                        "Update failed",
                                        &msg,
                                        None,
                                    );
                                    if let Some(reveal) = reveal_command_center.as_ref() {
                                        reveal(true);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(reload) = reload_packages_update.as_ref() {
                        reload();
                    }
                });
            });

            actions_box.append(&update_btn);
        }

        let remove_btn = gtk::Button::builder()
            .label(if package.status == PackageStatus::NotInstalled {
                "Close"
            } else {
                "Remove"
            })
            .build();

        if package.status == PackageStatus::Installed && package.source.supports_gui_operations() {
            remove_btn.add_css_class("destructive-action");
            remove_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_clone = dialog.clone();
            let reload_packages_remove = reload_packages.clone();
            let command_center_clone = command_center.clone();
            let reveal_command_center_clone = reveal_command_center.clone();

            remove_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                let reload_packages_remove = reload_packages_remove.clone();
                let command_center = command_center_clone.clone();
                let reveal_command_center = reveal_command_center_clone.clone();

                btn.set_sensitive(false);
                btn.set_label("Removing...");

                glib::spawn_future_local(async move {
                    let manager = pm.lock().await;
                    let result = manager.remove(&pkg).await;
                    drop(manager);

                    dialog.close();

                    match result {
                        Ok(_) => {
                            let t = adw::Toast::new(&format!("Removed {}", pkg.name));
                            t.set_timeout(3);
                            toast.add_toast(t);
                            if let Some(center) = command_center.as_ref() {
                                center.add_event(
                                    CommandEventKind::Success,
                                    format!("Removed {}", pkg.name),
                                    format!("Source: {}", pkg.source),
                                    None,
                                );
                            }
                        }
                        Err(e) => {
                            let msg = format!("Remove failed: {}", e);
                            if let Some((details, command)) = parse_suggestion(&msg) {
                                if let Some(center) = command_center.as_ref() {
                                    center.add_event(
                                        CommandEventKind::Error,
                                        "Action required",
                                        &details,
                                        Some(command.clone()),
                                    );
                                    if let Some(reveal) = reveal_command_center.as_ref() {
                                        reveal(true);
                                    }
                                } else {
                                    show_action_required_dialog(&dialog, &details, &command);
                                }
                                let t = adw::Toast::new("Action required");
                                t.set_timeout(5);
                                toast.add_toast(t);
                            } else {
                                let t = adw::Toast::new(&msg);
                                t.set_timeout(5);
                                toast.add_toast(t);
                                if let Some(center) = command_center.as_ref() {
                                    center.add_event(
                                        CommandEventKind::Error,
                                        "Remove failed",
                                        &msg,
                                        None,
                                    );
                                    if let Some(reveal) = reveal_command_center.as_ref() {
                                        reveal(true);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(reload) = reload_packages_remove.as_ref() {
                        reload();
                    }
                });
            });
        } else {
            let dialog_clone = dialog.clone();
            remove_btn.connect_clicked(move |_| {
                dialog_clone.close();
            });
        }

        actions_box.append(&remove_btn);

        content.append(&actions_box);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&content)
            .build();

        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        main_box.append(&scrolled);

        dialog.set_child(Some(&main_box));
        tracing::debug!("Presenting package details dialog");
        dialog.present();
    }
}
