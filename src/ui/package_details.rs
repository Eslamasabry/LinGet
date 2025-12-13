use crate::backend::PackageManager;
use crate::models::{Config, Package, PackageSource, PackageStatus};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PackageDetailsDialog;

fn parse_suggestion(message: &str) -> Option<(String, String)> {
    let marker = crate::backend::SUGGEST_PREFIX;
    let idx = message.find(marker)?;
    let command = message[idx + marker.len()..].trim();
    if command.is_empty() {
        return None;
    }
    Some((message[..idx].trim().to_string(), command.to_string()))
}

fn show_action_required_dialog(parent: &impl IsA<gtk::Window>, details: &str, command: &str) {
    let win = gtk::Window::builder()
        .title("Action required")
        .modal(true)
        .transient_for(parent)
        .default_width(640)
        .default_height(240)
        .build();

    let header = adw::HeaderBar::new();
    let copy_btn = gtk::Button::builder().label("Copy command").build();
    copy_btn.add_css_class("suggested-action");
    header.pack_end(&copy_btn);

    let close_btn = gtk::Button::builder().label("Close").build();
    header.pack_start(&close_btn);

    let details_label = gtk::Label::builder()
        .label(details)
        .wrap(true)
        .xalign(0.0)
        .build();
    details_label.add_css_class("dim-label");

    let command_entry = gtk::Entry::builder().text(command).editable(false).build();
    command_entry.add_css_class("monospace");

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    content.append(&details_label);
    content.append(&command_entry);

    let root = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    root.append(&header);
    root.append(&content);
    win.set_child(Some(&root));

    let cmd = command.to_string();
    copy_btn.connect_clicked({
        let command_entry = command_entry.clone();
        move |_| {
            command_entry.select_region(0, -1);
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&cmd);
                display.primary_clipboard().set_text(&cmd);
            }
        }
    });
    close_btn.connect_clicked({
        let win = win.clone();
        move |_| win.close()
    });

    win.present();
}

impl PackageDetailsDialog {
    pub fn show(
        package: &Package,
        parent: &impl IsA<gtk::Window>,
        pm: Arc<Mutex<PackageManager>>,
        toast_overlay: adw::ToastOverlay,
        config: Rc<RefCell<Config>>,
        reload_packages: Option<Rc<dyn Fn()>>,
    ) {
        let dialog = gtk::Window::builder()
            .title(&package.name)
            .default_width(500)
            .default_height(450)
            .modal(true)
            .transient_for(parent)
            .build();

        let header = adw::HeaderBar::new();

        // Content
        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .margin_start(24)
            .margin_end(24)
            .margin_top(16)
            .margin_bottom(24)
            .build();

        // Package icon and name header
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

        // Description
        if !package.description.is_empty() {
            let desc_label = gtk::Label::builder()
                .label(&package.description)
                .wrap(true)
                .xalign(0.0)
                .build();
            desc_label.add_css_class("dim-label");
            content.append(&desc_label);
        }

        // Details group
        let details_group = adw::PreferencesGroup::builder().title("Details").build();

        // Version row
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

        // Status row
        let status_row = adw::ActionRow::builder()
            .title("Status")
            .subtitle(package.status.to_string())
            .build();
        details_group.add(&status_row);

        // Size row
        let size_row = adw::ActionRow::builder()
            .title("Size")
            .subtitle(package.size_display())
            .build();
        details_group.add(&size_row);

        // Source row
        let source_row = adw::ActionRow::builder()
            .title("Source")
            .subtitle(package.source.description())
            .build();
        details_group.add(&source_row);

        // Ignore Updates row
        let ignore_row = adw::ActionRow::builder()
            .title("Ignore Updates")
            .subtitle("Prevent this package from being updated")
            .build();

        let ignore_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();

        // Set initial state
        let pkg_id = package.id();
        let is_ignored = config.borrow().ignored_packages.contains(&pkg_id);
        ignore_switch.set_active(is_ignored);

        let config_clone = config.clone();
        let pkg_id_clone = pkg_id.clone();
        let toast_clone = toast_overlay.clone();

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
            glib::Propagation::Proceed
        });

        ignore_row.add_suffix(&ignore_switch);
        details_group.add(&ignore_row);

        content.append(&details_group);

        // Action buttons
        let actions_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Center)
            .margin_top(16)
            .build();

        // Downgrade/Revert (source-specific)
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

            downgrade_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                let reload_packages_downgrade = reload_packages_downgrade.clone();

                btn.set_sensitive(false);
                btn.set_label(if matches!(pkg.source, PackageSource::Snap) {
                    "Reverting..."
                } else {
                    "Downgrading..."
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
                        }
                        Err(e) => {
                            let prefix = if matches!(pkg.source, PackageSource::Snap) {
                                "Revert"
                            } else {
                                "Downgrade"
                            };
                            let msg = format!("{} failed: {}", prefix, e);
                            if let Some((details, command)) = parse_suggestion(&msg) {
                                show_action_required_dialog(&dialog, &details, &command);
                                let t = adw::Toast::new("Action required");
                                t.set_timeout(5);
                                toast.add_toast(t);
                            } else {
                                let t = adw::Toast::new(&msg);
                                t.set_timeout(5);
                                toast.add_toast(t);
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

        // APT downgrade (version picker)
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
                    root.append(&header);
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
                        move |_| {
                            let version = version_entry.text().trim().to_string();
                            if version.is_empty() {
                                let t = adw::Toast::new("Enter a version first");
                                t.set_timeout(3);
                                toast.add_toast(t);
                                return;
                            }

                            version_entry.set_sensitive(false);

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
                                        }
                                        Err(e) => {
                                            let msg = format!("{} failed: {}", apply_label, e);
                                            if let Some((details, command)) = parse_suggestion(&msg)
                                            {
                                                show_action_required_dialog(
                                                    &parent_for_async,
                                                    &details,
                                                    &command,
                                                );
                                                let t = adw::Toast::new("Action required");
                                                t.set_timeout(5);
                                                toast.add_toast(t);
                                            } else {
                                                let t = adw::Toast::new(&msg);
                                                t.set_timeout(5);
                                                toast.add_toast(t);
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

        if package.has_update() {
            let update_btn = gtk::Button::builder().label("Update").build();
            update_btn.add_css_class("suggested-action");
            update_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_clone = dialog.clone();
            let reload_packages_update = reload_packages.clone();

            update_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                let reload_packages_update = reload_packages_update.clone();

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
                        }
                        Err(e) => {
                            let msg = format!("Update failed: {}", e);
                            if let Some((details, command)) = parse_suggestion(&msg) {
                                show_action_required_dialog(&dialog, &details, &command);
                                let t = adw::Toast::new("Action required");
                                t.set_timeout(5);
                                toast.add_toast(t);
                            } else {
                                let t = adw::Toast::new(&msg);
                                t.set_timeout(5);
                                toast.add_toast(t);
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

        if package.status == PackageStatus::Installed {
            remove_btn.add_css_class("destructive-action");
            remove_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_clone = dialog.clone();
            let reload_packages_remove = reload_packages.clone();

            remove_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                let reload_packages_remove = reload_packages_remove.clone();

                // Confirm dialog could be here, but for now direct action
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
                        }
                        Err(e) => {
                            let msg = format!("Remove failed: {}", e);
                            if let Some((details, command)) = parse_suggestion(&msg) {
                                show_action_required_dialog(&dialog, &details, &command);
                                let t = adw::Toast::new("Action required");
                                t.set_timeout(5);
                                toast.add_toast(t);
                            } else {
                                let t = adw::Toast::new(&msg);
                                t.set_timeout(5);
                                toast.add_toast(t);
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

        // Scrolled content
        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&content)
            .build();

        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        main_box.append(&header);
        main_box.append(&scrolled);

        dialog.set_child(Some(&main_box));
        dialog.present();
    }
}
