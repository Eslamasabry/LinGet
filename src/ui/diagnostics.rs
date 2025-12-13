use crate::backend::{detect_providers, ProviderStatus};
use crate::models::{Config, PackageSource};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub struct DiagnosticsDialog;

impl DiagnosticsDialog {
    pub fn show(
        config: Rc<RefCell<Config>>,
        enabled_sources: Rc<RefCell<HashSet<PackageSource>>>,
        available_sources: Rc<RefCell<HashSet<PackageSource>>>,
        parent: &impl IsA<gtk::Window>,
    ) {
        let dialog = adw::PreferencesWindow::builder()
            .title("Diagnostics")
            .default_width(720)
            .default_height(560)
            .modal(true)
            .transient_for(parent)
            .build();

        let providers_page = adw::PreferencesPage::builder()
            .title("Providers")
            .icon_name("applications-system-symbolic")
            .build();

        let providers_group = adw::PreferencesGroup::builder()
            .title("Detected Providers")
            .description(
                "What LinGet found on this system (available) vs what is enabled in LinGet.",
            )
            .build();

        let loading_row = adw::ActionRow::builder()
            .title("Detecting providers…")
            .subtitle("Please wait")
            .build();
        providers_group.add(&loading_row);
        providers_page.add(&providers_group);

        let system_page = adw::PreferencesPage::builder()
            .title("System")
            .icon_name("computer-symbolic")
            .build();

        let system_group = adw::PreferencesGroup::builder()
            .title("Configuration")
            .build();

        let interval_row = adw::ActionRow::builder()
            .title("Update interval (hours)")
            .subtitle(config.borrow().update_check_interval.to_string())
            .build();
        system_group.add(&interval_row);

        let startup_row = adw::ActionRow::builder()
            .title("Check updates on startup")
            .subtitle(config.borrow().check_updates_on_startup.to_string())
            .build();
        system_group.add(&startup_row);

        system_page.add(&system_group);

        dialog.add(&providers_page);
        dialog.add(&system_page);

        let header = adw::HeaderBar::new();
        let copy_btn = gtk::Button::builder()
            .label("Copy Diagnostics")
            .tooltip_text("Copy provider and config info to clipboard")
            .build();
        copy_btn.add_css_class("suggested-action");
        header.pack_end(&copy_btn);
        dialog.set_titlebar(Some(&header));

        let enabled_sources_for_copy = enabled_sources.clone();
        let available_sources_for_copy = available_sources.clone();
        let config_for_copy = config.clone();
        let providers_for_copy: Rc<RefCell<Vec<ProviderStatus>>> =
            Rc::new(RefCell::new(Vec::new()));

        copy_btn.connect_clicked({
            let providers_for_copy = providers_for_copy.clone();
            move |_| {
                let enabled = enabled_sources_for_copy.borrow().clone();
                let available = available_sources_for_copy.borrow().clone();
                let cfg = config_for_copy.borrow().clone();
                let providers = providers_for_copy.borrow().clone();

                let mut out = String::new();
                out.push_str("LinGet diagnostics\n");
                out.push_str(&format!(
                    "- update_check_interval_hours: {}\n",
                    cfg.update_check_interval
                ));
                out.push_str(&format!(
                    "- check_updates_on_startup: {}\n",
                    cfg.check_updates_on_startup
                ));
                out.push_str(&format!("- enabled_sources: {:?}\n", enabled));
                out.push_str(&format!("- available_sources: {:?}\n", available));
                out.push_str("\nProviders:\n");
                for p in providers {
                    out.push_str(&format!(
                        "- {}: available={}, enabled_in_app={}",
                        p.display_name,
                        p.available,
                        enabled.contains(&p.source)
                    ));
                    if !p.list_cmds.is_empty() {
                        out.push_str(&format!(", list_cmds={}", p.list_cmds.join("+")));
                    }
                    if !p.privileged_cmds.is_empty() {
                        out.push_str(&format!(
                            ", privileged_cmds={}",
                            p.privileged_cmds.join("+")
                        ));
                    }
                    if let Some(v) = p.version.as_ref() {
                        out.push_str(&format!(", version={}", v));
                    }
                    if let Some(r) = p.reason.as_ref() {
                        out.push_str(&format!(", reason={}", r));
                    }
                    if !p.found_paths.is_empty() {
                        out.push_str(&format!(
                            ", paths={}",
                            p.found_paths
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect::<Vec<_>>()
                                .join("; ")
                        ));
                    }
                    out.push('\n');
                }

                if let Some(display) = gtk::gdk::Display::default() {
                    display.clipboard().set_text(&out);
                    display.primary_clipboard().set_text(&out);
                }
            }
        });

        // Detect providers off the UI thread
        glib::spawn_future_local({
            let providers_group = providers_group.clone();
            let loading_row = loading_row.clone();
            let providers_for_copy = providers_for_copy.clone();
            let enabled_sources = enabled_sources.clone();
            let available_sources = available_sources.clone();
            async move {
                let handle = tokio::task::spawn_blocking(detect_providers);
                let providers = handle.await.unwrap_or_default();
                *providers_for_copy.borrow_mut() = providers.clone();

                providers_group.remove(&loading_row);

                let enabled = enabled_sources.borrow().clone();
                let available_set = available_sources.borrow().clone();
                for p in providers {
                    let enabled_in_app = enabled.contains(&p.source);
                    let available_in_app = available_set.contains(&p.source);

                    let mut subtitle = String::new();
                    if let Some(v) = p.version.as_ref() {
                        subtitle.push_str(v);
                    }
                    if let Some(r) = p.reason.as_ref() {
                        if !subtitle.is_empty() {
                            subtitle.push_str(" • ");
                        }
                        subtitle.push_str(r);
                    }
                    if subtitle.is_empty() {
                        subtitle = "OK".to_string();
                    }

                    let row = adw::ActionRow::builder()
                        .title(p.display_name)
                        .subtitle(subtitle)
                        .build();

                    let icon = gtk::Image::builder()
                        .icon_name(p.source.icon_name())
                        .build();
                    row.add_prefix(&icon);

                    let status = if p.available && available_in_app {
                        "Available"
                    } else {
                        "Unavailable"
                    };
                    let badge = gtk::Label::builder().label(status).build();
                    badge.add_css_class("caption");
                    badge.add_css_class(if p.available && available_in_app {
                        "success"
                    } else {
                        "dim-label"
                    });
                    row.add_suffix(&badge);

                    let enabled_badge = gtk::Label::builder()
                        .label(if enabled_in_app {
                            "Enabled"
                        } else {
                            "Disabled"
                        })
                        .build();
                    enabled_badge.add_css_class("caption");
                    enabled_badge.add_css_class(if enabled_in_app {
                        "accent"
                    } else {
                        "dim-label"
                    });
                    row.add_suffix(&enabled_badge);

                    providers_group.add(&row);
                }
            }
        });

        dialog.present();
    }
}
