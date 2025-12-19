use crate::backend::PackageManager;
use crate::models::{fetch_enrichment, Config, Package, PackageSource, PackageStatus};
use crate::ui::package_details::enrichment;
use crate::ui::{CommandCenter, CommandEventKind, PackageOp, RetrySpec};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct PackageDetailsPanel {
    inner: Rc<Inner>,
}

struct Inner {
    widget: gtk::Box,
    content_box: gtk::Box,
    header_bar: adw::HeaderBar,
    current_package: RefCell<Option<Package>>,
    on_close: RefCell<Option<Rc<dyn Fn()>>>,
}

impl PackageDetailsPanel {
    pub fn new() -> Self {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(400)
            .build();
        widget.add_css_class("details-panel");
        widget.add_css_class("background");

        let close_button = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Close")
            .build();
        close_button.add_css_class("flat");

        let header_bar = adw::HeaderBar::builder()
            .title_widget(&gtk::Label::new(Some("Package Details")))
            .build();
        header_bar.pack_start(&close_button);

        let content_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .vexpand(true)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&content_box)
            .build();

        widget.append(&header_bar);
        widget.append(&scrolled);

        let inner = Rc::new(Inner {
            widget,
            content_box,
            header_bar,
            current_package: RefCell::new(None),
            on_close: RefCell::new(None),
        });

        let on_close = inner.on_close.clone();
        close_button.connect_clicked(move |_| {
            if let Some(cb) = on_close.borrow().as_ref() {
                cb();
            }
        });

        Self { inner }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.inner.widget
    }

    pub fn set_on_close<F: Fn() + 'static>(&self, callback: F) {
        *self.inner.on_close.borrow_mut() = Some(Rc::new(callback));
    }

    #[allow(dead_code)]
    pub fn current_package(&self) -> Option<Package> {
        self.inner.current_package.borrow().clone()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show_package(
        &self,
        package: &Package,
        pm: Arc<Mutex<PackageManager>>,
        toast_overlay: adw::ToastOverlay,
        config: Rc<RefCell<Config>>,
        reload_packages: Option<Rc<dyn Fn()>>,
        command_center: Option<CommandCenter>,
        reveal_command_center: Option<Rc<dyn Fn(bool)>>,
        close_panel: Rc<dyn Fn()>,
    ) {
        *self.inner.current_package.borrow_mut() = Some(package.clone());

        self.inner
            .header_bar
            .set_title_widget(Some(&gtk::Label::new(Some(&package.name))));

        while let Some(child) = self.inner.content_box.first_child() {
            self.inner.content_box.remove(&child);
        }

        let content = gtk::Box::builder()
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

        let icon_frame = gtk::Frame::builder().build();
        icon_frame.add_css_class("icon-frame");
        icon_frame.add_css_class("card");

        let icon = gtk::Image::builder()
            .icon_name(package.source.icon_name())
            .pixel_size(64)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        icon_frame.set_child(Some(&icon));

        let title_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build();

        let name_label = gtk::Label::builder()
            .label(&package.name)
            .xalign(0.0)
            .wrap(true)
            .build();
        name_label.add_css_class("title-2");

        let source_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let source_dot = gtk::Box::builder()
            .width_request(10)
            .height_request(10)
            .valign(gtk::Align::Center)
            .build();
        source_dot.add_css_class("source-dot");
        source_dot.add_css_class(package.source.color_class());

        let source_label = gtk::Label::builder()
            .label(package.source.to_string())
            .xalign(0.0)
            .build();
        source_label.add_css_class("caption");
        source_label.add_css_class("dimmed");

        source_box.append(&source_dot);
        source_box.append(&source_label);

        title_box.append(&name_label);
        title_box.append(&source_box);

        header_box.append(&icon_frame);
        header_box.append(&title_box);

        content.append(&header_box);

        let desc_label = gtk::Label::builder()
            .label(&package.description)
            .wrap(true)
            .xalign(0.0)
            .visible(!package.description.is_empty())
            .build();
        desc_label.add_css_class("body");
        desc_label.add_css_class("dimmed");
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
            if let Some(enrichment_data) = fetch_enrichment(&pkg_for_enrich).await {
                while let Some(child) = enrichment_box_clone.first_child() {
                    enrichment_box_clone.remove(&child);
                }

                if enrichment_data.summary.is_some() {
                    desc_label_clone.set_visible(false);
                }

                let section = enrichment::build_section(&enrichment_data);
                enrichment_box_clone.append(&section);
            } else {
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
        version_row.add_css_class("property");
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
        status_row.add_css_class("property");
        details_group.add(&status_row);

        let size_row = adw::ActionRow::builder()
            .title("Size")
            .subtitle(package.size_display())
            .build();
        size_row.add_css_class("property");
        details_group.add(&size_row);

        let source_row = adw::ActionRow::builder()
            .title("Source")
            .subtitle(package.source.description())
            .build();
        source_row.add_css_class("property");
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
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_top(16)
            .build();

        self.build_action_buttons(
            &actions_box,
            package,
            pm,
            toast_overlay,
            reload_packages,
            command_center,
            reveal_command_center,
            close_panel,
        );

        content.append(&actions_box);

        self.inner.content_box.append(&content);
    }

    #[allow(clippy::too_many_arguments)]
    fn build_action_buttons(
        &self,
        actions_box: &gtk::Box,
        package: &Package,
        pm: Arc<Mutex<PackageManager>>,
        toast_overlay: adw::ToastOverlay,
        reload_packages: Option<Rc<dyn Fn()>>,
        command_center: Option<CommandCenter>,
        reveal_command_center: Option<Rc<dyn Fn(bool)>>,
        close_panel: Rc<dyn Fn()>,
    ) {
        let button_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        if package.has_update() && package.source.supports_gui_operations() {
            let update_btn = gtk::Button::builder().label("Update").build();
            update_btn.add_css_class("suggested-action");
            update_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let reload_packages_update = reload_packages.clone();
            let command_center_clone = command_center.clone();
            let reveal_command_center_clone = reveal_command_center.clone();
            let close_panel_clone = close_panel.clone();

            update_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let btn = btn.clone();
                let reload_packages_update = reload_packages_update.clone();
                let command_center = command_center_clone.clone();
                let reveal_command_center = reveal_command_center_clone.clone();
                let close_panel = close_panel_clone.clone();

                btn.set_sensitive(false);
                btn.set_label("Updating...");

                glib::spawn_future_local(async move {
                    let manager = pm.lock().await;
                    let result = manager.update(&pkg).await;
                    drop(manager);

                    close_panel();

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

                    if let Some(reload) = reload_packages_update.as_ref() {
                        reload();
                    }
                });
            });

            button_row.append(&update_btn);
        }

        if package.status == PackageStatus::Installed && package.source.supports_gui_operations() {
            let remove_btn = gtk::Button::builder().label("Remove").build();
            remove_btn.add_css_class("destructive-action");
            remove_btn.add_css_class("pill");

            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let reload_packages_remove = reload_packages.clone();
            let command_center_clone = command_center.clone();
            let reveal_command_center_clone = reveal_command_center.clone();
            let close_panel_clone = close_panel.clone();

            remove_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let btn = btn.clone();
                let reload_packages_remove = reload_packages_remove.clone();
                let command_center = command_center_clone.clone();
                let reveal_command_center = reveal_command_center_clone.clone();
                let close_panel = close_panel_clone.clone();

                btn.set_sensitive(false);
                btn.set_label("Removing...");

                glib::spawn_future_local(async move {
                    let manager = pm.lock().await;
                    let result = manager.remove(&pkg).await;
                    drop(manager);

                    close_panel();

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

                    if let Some(reload) = reload_packages_remove.as_ref() {
                        reload();
                    }
                });
            });

            button_row.append(&remove_btn);
        }

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
            let reload_packages_downgrade = reload_packages.clone();
            let command_center_clone = command_center.clone();
            let reveal_command_center_clone = reveal_command_center.clone();
            let close_panel_clone = close_panel.clone();

            downgrade_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let btn = btn.clone();
                let reload_packages_downgrade = reload_packages_downgrade.clone();
                let command_center = command_center_clone.clone();
                let reveal_command_center = reveal_command_center_clone.clone();
                let close_panel = close_panel_clone.clone();

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

                    close_panel();

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

                    if let Some(reload) = reload_packages_downgrade.as_ref() {
                        reload();
                    }
                });
            });

            button_row.append(&downgrade_btn);
        }

        actions_box.append(&button_row);
    }

    pub fn clear(&self) {
        *self.inner.current_package.borrow_mut() = None;
        while let Some(child) = self.inner.content_box.first_child() {
            self.inner.content_box.remove(&child);
        }
        self.inner
            .header_bar
            .set_title_widget(Some(&gtk::Label::new(Some("Package Details"))));
    }
}

impl Default for PackageDetailsPanel {
    fn default() -> Self {
        Self::new()
    }
}
