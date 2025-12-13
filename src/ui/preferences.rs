use crate::models::Config;
use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PreferencesDialog;

impl PreferencesDialog {
    pub fn show(config: Rc<RefCell<Config>>, parent: &impl IsA<gtk::Window>) {
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
        let startup_group = adw::PreferencesGroup::builder()
            .title("Startup")
            .build();

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

        let config_clone = config.clone();
        start_minimized_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().start_minimized = switch.is_active();
            let _ = config_clone.borrow().save();
        });

        startup_group.add(&start_minimized_row);

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

        // APT
        let apt_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().enabled_sources.apt)
            .build();

        let apt_row = adw::ActionRow::builder()
            .title("APT")
            .subtitle("System packages (Debian/Ubuntu)")
            .activatable_widget(&apt_switch)
            .build();

        let apt_icon = gtk::Image::builder()
            .icon_name("package-x-generic-symbolic")
            .build();
        apt_row.add_prefix(&apt_icon);
        apt_row.add_suffix(&apt_switch);

        let config_clone = config.clone();
        apt_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().enabled_sources.apt = switch.is_active();
            let _ = config_clone.borrow().save();
        });

        sources_group.add(&apt_row);

        // Flatpak
        let flatpak_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().enabled_sources.flatpak)
            .build();

        let flatpak_row = adw::ActionRow::builder()
            .title("Flatpak")
            .subtitle("Sandboxed applications")
            .activatable_widget(&flatpak_switch)
            .build();

        let flatpak_icon = gtk::Image::builder()
            .icon_name("system-software-install-symbolic")
            .build();
        flatpak_row.add_prefix(&flatpak_icon);
        flatpak_row.add_suffix(&flatpak_switch);

        let config_clone = config.clone();
        flatpak_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().enabled_sources.flatpak = switch.is_active();
            let _ = config_clone.borrow().save();
        });

        sources_group.add(&flatpak_row);

        // npm
        let npm_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().enabled_sources.npm)
            .build();

        let npm_row = adw::ActionRow::builder()
            .title("npm")
            .subtitle("Node.js packages (global)")
            .activatable_widget(&npm_switch)
            .build();

        let npm_icon = gtk::Image::builder()
            .icon_name("text-x-script-symbolic")
            .build();
        npm_row.add_prefix(&npm_icon);
        npm_row.add_suffix(&npm_switch);

        let config_clone = config.clone();
        npm_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().enabled_sources.npm = switch.is_active();
            let _ = config_clone.borrow().save();
        });

        sources_group.add(&npm_row);

        // pip
        let pip_switch = gtk::Switch::builder()
            .valign(gtk::Align::Center)
            .active(config.borrow().enabled_sources.pip)
            .build();

        let pip_row = adw::ActionRow::builder()
            .title("pip")
            .subtitle("Python packages")
            .activatable_widget(&pip_switch)
            .build();

        let pip_icon = gtk::Image::builder()
            .icon_name("text-x-python-symbolic")
            .build();
        pip_row.add_prefix(&pip_icon);
        pip_row.add_suffix(&pip_switch);

        let config_clone = config.clone();
        pip_switch.connect_active_notify(move |switch| {
            config_clone.borrow_mut().enabled_sources.pip = switch.is_active();
            let _ = config_clone.borrow().save();
        });

        sources_group.add(&pip_row);

        sources_page.add(&sources_group);

        dialog.add(&sources_page);

        dialog.present();
    }
}
