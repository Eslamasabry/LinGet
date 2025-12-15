use crate::backend::PackageManager;
use crate::models::{Config, PackageSource};
use crate::ui::RepositoriesDialog;
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PreferencesDialog;

impl PreferencesDialog {
    pub fn show(
        config: Rc<RefCell<Config>>,
        pm: Arc<Mutex<PackageManager>>,
        enabled_sources: Rc<RefCell<HashSet<PackageSource>>>,
        available_sources: Rc<RefCell<HashSet<PackageSource>>>,
        reload_packages: Rc<dyn Fn()>,
        parent: &impl IsA<gtk::Window>,
    ) {
        let dialog = adw::PreferencesWindow::builder()
            .title("Preferences")
            .default_width(600)
            .default_height(500)
            .modal(true)
            .transient_for(parent)
            .build();

        // General page
        let general_page = adw::PreferencesPage::builder()
            .title("General")
            .icon_name("preferences-system-symbolic")
            .build();

        // Startup group
        let startup_group = adw::PreferencesGroup::builder().title("Startup").build();

        let check_updates_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().check_updates_on_startup)
            .build();

        let check_updates_row = adw::ActionRow::builder()
            .title("Check for updates on startup")
            .subtitle("Automatically check for package updates when LinGet starts")
            .activatable_widget(&check_updates_switch)
            .build();
        check_updates_row.add_suffix(&check_updates_switch);

        let config_clone = config.clone();
        check_updates_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().check_updates_on_startup = switch.is_active();
            let _ = config_clone.borrow().save();
        });

        startup_group.add(&check_updates_row);

        let start_minimized_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().start_minimized)
            .build();

        let start_minimized_row = adw::ActionRow::builder()
            .title("Start minimized")
            .subtitle("Start LinGet minimized to the system tray")
            .activatable_widget(&start_minimized_switch)
            .build();
        start_minimized_row.add_suffix(&start_minimized_switch);

        startup_group.add(&start_minimized_row);

        let start_maximized_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().window_maximized)
            .build();

        let start_maximized_row = adw::ActionRow::builder()
            .title("Start maximized")
            .subtitle("Start LinGet with the window maximized")
            .activatable_widget(&start_maximized_switch)
            .build();
        start_maximized_row.add_suffix(&start_maximized_switch);

        // Make minimized and maximized mutually exclusive
        let config_clone = config.clone();
        let maximized_switch_for_minimized = start_maximized_switch.clone();
        start_minimized_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().start_minimized = switch.is_active();
            if switch.is_active() {
                maximized_switch_for_minimized.set_active(false);
            }
            let _ = config_clone.borrow().save();
        });

        let config_clone = config.clone();
        let minimized_switch_for_maximized = start_minimized_switch.clone();
        start_maximized_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().window_maximized = switch.is_active();
            if switch.is_active() {
                minimized_switch_for_maximized.set_active(false);
            }
            let _ = config_clone.borrow().save();
        });

        startup_group.add(&start_maximized_row);

        // Update check interval
        let interval_values: [u32; 6] = [0, 6, 12, 24, 48, 168];
        let interval_labels = gtk::StringList::new(&[
            "Disabled",
            "Every 6 hours",
            "Every 12 hours",
            "Every 24 hours",
            "Every 48 hours",
            "Weekly",
        ]);

        let interval_row = adw::ComboRow::builder()
            .title("Background update check")
            .subtitle("How often to check for updates automatically (requires restart)")
            .model(&interval_labels)
            .build();

        // Set current value
        let current_interval = config.borrow().update_check_interval;
        let selected_idx = interval_values
            .iter()
            .position(|&v| v == current_interval)
            .unwrap_or(3) as u32; // Default to 24 hours
        interval_row.set_selected(selected_idx);

        let config_clone = config.clone();
        interval_row.connect_selected_notify(move |row| {
            let idx = row.selected() as usize;
            if idx < interval_values.len() {
                config_clone.borrow_mut().update_check_interval = interval_values[idx];
                let _ = config_clone.borrow().save();
            }
        });

        startup_group.add(&interval_row);

        general_page.add(&startup_group);

        // Notifications group
        let notif_group = adw::PreferencesGroup::builder()
            .title("Notifications")
            .build();

        let notif_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().show_notifications)
            .build();

        let notif_row = adw::ActionRow::builder()
            .title("Show notifications")
            .subtitle("Display desktop notifications for updates and operations")
            .activatable_widget(&notif_switch)
            .build();
        notif_row.add_suffix(&notif_switch);

        let config_clone = config.clone();
        notif_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().show_notifications = switch.is_active();
            let _ = config_clone.borrow().save();
        });

        notif_group.add(&notif_row);

        general_page.add(&notif_group);

        // Appearance group
        let appearance_group = adw::PreferencesGroup::builder()
            .title("Appearance")
            .description("Adjust list density and visuals")
            .build();

        let compact_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().ui_compact)
            .build();
        let compact_row = adw::ActionRow::builder()
            .title("Compact list density")
            .subtitle("Use smaller rows for more packages on screen")
            .activatable_widget(&compact_switch)
            .build();
        compact_row.add_suffix(&compact_switch);
        compact_switch.connect_active_notify({
            let config = config.clone();
            let reload_packages = reload_packages.clone();
            move |s| {
                config.borrow_mut().ui_compact = s.is_active();
                let _ = config.borrow().save();
                reload_packages();
            }
        });
        appearance_group.add(&compact_row);

        let icons_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().ui_show_icons)
            .build();
        let icons_row = adw::ActionRow::builder()
            .title("Show app icons")
            .subtitle("Display application icons in package lists")
            .activatable_widget(&icons_switch)
            .build();
        icons_row.add_suffix(&icons_switch);
        icons_switch.connect_active_notify({
            let config = config.clone();
            let reload_packages = reload_packages.clone();
            move |s| {
                config.borrow_mut().ui_show_icons = s.is_active();
                let _ = config.borrow().save();
                reload_packages();
            }
        });
        appearance_group.add(&icons_row);

        general_page.add(&appearance_group);

        dialog.add(&general_page);

        // Sources page
        let sources_page = adw::PreferencesPage::builder()
            .title("Sources")
            .icon_name("application-x-addon-symbolic")
            .build();

        let sources_group = adw::PreferencesGroup::builder()
            .title("Package Sources")
            .description("Enable or disable package sources")
            .build();

        let apply_sources = {
            let config = config.clone();
            let pm = pm.clone();
            let enabled_sources = enabled_sources.clone();
            let reload = reload_packages.clone();

            move || {
                let sources = config.borrow().enabled_sources.to_sources();
                *enabled_sources.borrow_mut() = sources.clone();

                let pm = pm.clone();
                glib::spawn_future_local(async move {
                    pm.lock().await.set_enabled_sources(sources);
                });

                reload();
            }
        };

        let add_source_toggle = |title: &str,
                                 subtitle: &str,
                                 icon_name: &str,
                                 active: bool,
                                 sensitive: bool,
                                 on_toggle: Rc<dyn Fn(bool)>| {
            let switch = gtk::Switch::builder()
                .valign(gtk::Align::Center)
                .active(active)
                .sensitive(sensitive)
                .build();

            let row = adw::ActionRow::builder()
                .title(title)
                .subtitle(subtitle)
                .activatable_widget(&switch)
                .build();

            let icon = gtk::Image::builder().icon_name(icon_name).build();
            row.add_prefix(&icon);
            row.add_suffix(&switch);

            switch.connect_active_notify(move |s| on_toggle(s.is_active()));
            sources_group.add(&row);
        };

        // APT
        let available = available_sources.borrow().clone();

        add_source_toggle(
            "APT",
            "System packages (Debian/Ubuntu)",
            "package-x-generic-symbolic",
            config.borrow().enabled_sources.apt && available.contains(&PackageSource::Apt),
            available.contains(&PackageSource::Apt),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.apt = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "DNF",
            "System packages (Fedora/RHEL)",
            "system-software-install-symbolic",
            config.borrow().enabled_sources.dnf && available.contains(&PackageSource::Dnf),
            available.contains(&PackageSource::Dnf),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.dnf = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "Pacman",
            "System packages (Arch Linux)",
            "package-x-generic-symbolic",
            config.borrow().enabled_sources.pacman && available.contains(&PackageSource::Pacman),
            available.contains(&PackageSource::Pacman),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.pacman = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "Zypper",
            "System packages (openSUSE)",
            "system-software-install-symbolic",
            config.borrow().enabled_sources.zypper && available.contains(&PackageSource::Zypper),
            available.contains(&PackageSource::Zypper),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.zypper = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        // Flatpak
        add_source_toggle(
            "Flatpak",
            "Sandboxed applications",
            "system-software-install-symbolic",
            config.borrow().enabled_sources.flatpak && available.contains(&PackageSource::Flatpak),
            available.contains(&PackageSource::Flatpak),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.flatpak = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "Snap",
            "Snap packages (Ubuntu)",
            "package-x-generic-symbolic",
            config.borrow().enabled_sources.snap && available.contains(&PackageSource::Snap),
            available.contains(&PackageSource::Snap),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.snap = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        // npm
        add_source_toggle(
            "npm",
            "Node.js packages (global)",
            "text-x-script-symbolic",
            config.borrow().enabled_sources.npm && available.contains(&PackageSource::Npm),
            available.contains(&PackageSource::Npm),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.npm = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        // pip
        add_source_toggle(
            "pip",
            "Python packages",
            "text-x-python-symbolic",
            config.borrow().enabled_sources.pip && available.contains(&PackageSource::Pip),
            available.contains(&PackageSource::Pip),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.pip = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "pipx",
            "Python app packages (pipx)",
            "text-x-python-symbolic",
            config.borrow().enabled_sources.pipx && available.contains(&PackageSource::Pipx),
            available.contains(&PackageSource::Pipx),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.pipx = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "cargo",
            "Rust crates (cargo install)",
            "applications-development-symbolic",
            config.borrow().enabled_sources.cargo && available.contains(&PackageSource::Cargo),
            available.contains(&PackageSource::Cargo),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.cargo = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "brew",
            "Homebrew packages (Linuxbrew)",
            "application-x-executable-symbolic",
            config.borrow().enabled_sources.brew && available.contains(&PackageSource::Brew),
            available.contains(&PackageSource::Brew),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.brew = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "AUR",
            "Arch User Repository (AUR helper)",
            "package-x-generic-symbolic",
            config.borrow().enabled_sources.aur && available.contains(&PackageSource::Aur),
            available.contains(&PackageSource::Aur),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.aur = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "conda",
            "Conda packages (base env)",
            "text-x-python-symbolic",
            config.borrow().enabled_sources.conda && available.contains(&PackageSource::Conda),
            available.contains(&PackageSource::Conda),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.conda = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "mamba",
            "Mamba packages (base env)",
            "text-x-python-symbolic",
            config.borrow().enabled_sources.mamba && available.contains(&PackageSource::Mamba),
            available.contains(&PackageSource::Mamba),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.mamba = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "dart",
            "Dart/Flutter global tools (pub global)",
            "applications-development-symbolic",
            config.borrow().enabled_sources.dart && available.contains(&PackageSource::Dart),
            available.contains(&PackageSource::Dart),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.dart = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "Deb",
            "Local .deb packages",
            "package-x-generic-symbolic",
            config.borrow().enabled_sources.deb && available.contains(&PackageSource::Deb),
            available.contains(&PackageSource::Deb),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.deb = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        add_source_toggle(
            "AppImage",
            "Portable AppImage applications",
            "application-x-executable-symbolic",
            config.borrow().enabled_sources.appimage
                && available.contains(&PackageSource::AppImage),
            available.contains(&PackageSource::AppImage),
            Rc::new({
                let config = config.clone();
                let apply_sources = apply_sources.clone();
                move |active| {
                    config.borrow_mut().enabled_sources.appimage = active;
                    let _ = config.borrow().save();
                    apply_sources();
                }
            }),
        );

        sources_page.add(&sources_group);

        // Repository Management button
        let repo_group = adw::PreferencesGroup::builder()
            .title("Repositories")
            .description("Manage package source repositories")
            .build();

        let repo_row = adw::ActionRow::builder()
            .title("Manage Repositories")
            .subtitle("View and manage Flatpak remotes and other repositories")
            .activatable(true)
            .build();

        let arrow = gtk::Image::builder()
            .icon_name("go-next-symbolic")
            .build();
        repo_row.add_suffix(&arrow);

        let pm_for_repo = pm.clone();
        let dialog_for_repo = dialog.clone();
        repo_row.connect_activated(move |_| {
            RepositoriesDialog::show(pm_for_repo.clone(), &dialog_for_repo);
        });

        repo_group.add(&repo_row);
        sources_page.add(&repo_group);

        dialog.add(&sources_page);

        dialog.present();
    }
}
