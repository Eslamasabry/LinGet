use crate::backend::PackageManager;
use crate::models::{PackageSource, Repository};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct RepositoriesDialog;

impl RepositoriesDialog {
    pub fn show(pm: Arc<Mutex<PackageManager>>, parent: &impl IsA<gtk::Window>) {
        let dialog = adw::Window::builder()
            .title("Repository Management")
            .default_width(500)
            .default_height(400)
            .modal(true)
            .transient_for(parent)
            .build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();

        // Header bar with Add button
        let header = adw::HeaderBar::builder().build();

        let add_button = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Add Flatpak Remote")
            .build();
        add_button.add_css_class("flat");
        header.pack_end(&add_button);

        content.append(&header);

        // Main content
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        let main_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        // Flatpak section
        let flatpak_group = adw::PreferencesGroup::builder()
            .title("Flatpak Remotes")
            .description("Manage Flatpak repositories")
            .build();

        let flatpak_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();

        flatpak_group.add(&flatpak_list);
        main_box.append(&flatpak_group.upcast::<gtk::Widget>());

        // Loading indicator
        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build();
        flatpak_list.append(&spinner);

        scroll.set_child(Some(&main_box));
        content.append(&scroll);

        dialog.set_content(Some(&content));

        // Add button click handler
        let pm_for_add = pm.clone();
        let flatpak_list_for_add = flatpak_list.clone();
        let dialog_for_add = dialog.clone();
        add_button.connect_clicked(move |_| {
            Self::show_add_dialog(
                pm_for_add.clone(),
                flatpak_list_for_add.clone(),
                &dialog_for_add,
            );
        });

        // Load repositories
        let flatpak_list_for_load = flatpak_list.clone();
        let pm_for_load = pm.clone();
        glib::spawn_future_local(async move {
            let repos = {
                let manager = pm_for_load.lock().await;
                manager
                    .list_repositories(PackageSource::Flatpak)
                    .await
                    .unwrap_or_default()
            };

            // Remove spinner
            while let Some(child) = flatpak_list_for_load.first_child() {
                flatpak_list_for_load.remove(&child);
            }

            if repos.is_empty() {
                let empty_label = gtk::Label::builder()
                    .label("No Flatpak remotes configured")
                    .css_classes(vec!["dim-label".to_string()])
                    .margin_top(12)
                    .margin_bottom(12)
                    .build();
                flatpak_list_for_load.append(&empty_label);
            } else {
                for repo in repos {
                    let row = Self::create_repo_row(&repo, pm_for_load.clone());
                    flatpak_list_for_load.append(&row);
                }
            }
        });

        dialog.present();
    }

    fn show_add_dialog(
        pm: Arc<Mutex<PackageManager>>,
        flatpak_list: gtk::ListBox,
        parent: &adw::Window,
    ) {
        let add_dialog = adw::Window::builder()
            .title("Add Flatpak Remote")
            .default_width(400)
            .default_height(200)
            .modal(true)
            .transient_for(parent)
            .build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();

        let header = adw::HeaderBar::builder().build();

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        let add_btn = gtk::Button::builder().label("Add").sensitive(false).build();
        add_btn.add_css_class("suggested-action");

        let add_dialog_for_cancel = add_dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            add_dialog_for_cancel.close();
        });

        header.pack_start(&cancel_btn);
        header.pack_end(&add_btn);
        content.append(&header);

        // Form content
        let form_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        // URL entry
        let url_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();
        let url_label = gtk::Label::builder()
            .label("Repository URL")
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading".to_string()])
            .build();
        let url_entry = gtk::Entry::builder()
            .placeholder_text("https://...")
            .input_hints(gtk::InputHints::NO_SPELLCHECK)
            .build();
        url_box.append(&url_label);
        url_box.append(&url_entry);
        form_box.append(&url_box);

        // Name entry
        let name_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .build();
        let name_label = gtk::Label::builder()
            .label("Name (optional)")
            .halign(gtk::Align::Start)
            .css_classes(vec!["heading".to_string()])
            .build();
        let name_entry = gtk::Entry::builder()
            .placeholder_text("custom-remote")
            .input_hints(gtk::InputHints::NO_SPELLCHECK)
            .build();
        name_box.append(&name_label);
        name_box.append(&name_entry);
        form_box.append(&name_box);

        // Enable Add button when URL is entered
        let add_btn_for_entry = add_btn.clone();
        url_entry.connect_changed(move |entry| {
            let text = entry.text();
            add_btn_for_entry.set_sensitive(!text.is_empty());
        });

        content.append(&form_box);
        add_dialog.set_content(Some(&content));

        // Add button click handler
        let add_dialog_for_add = add_dialog.clone();
        let url_entry_for_add = url_entry.clone();
        let name_entry_for_add = name_entry.clone();
        add_btn.connect_clicked(move |btn| {
            let url = url_entry_for_add.text().to_string();
            let name = {
                let text = name_entry_for_add.text();
                if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                }
            };

            btn.set_sensitive(false);
            let pm = pm.clone();
            let flatpak_list = flatpak_list.clone();
            let add_dialog = add_dialog_for_add.clone();

            glib::spawn_future_local(async move {
                let result = {
                    let manager = pm.lock().await;
                    manager
                        .add_repository(PackageSource::Flatpak, &url, name.as_deref())
                        .await
                };

                match result {
                    Ok(_) => {
                        tracing::info!("Added flatpak remote: {}", url);
                        add_dialog.close();

                        // Refresh the list
                        while let Some(child) = flatpak_list.first_child() {
                            flatpak_list.remove(&child);
                        }

                        let repos = {
                            let manager = pm.lock().await;
                            manager
                                .list_repositories(PackageSource::Flatpak)
                                .await
                                .unwrap_or_default()
                        };

                        if repos.is_empty() {
                            let empty_label = gtk::Label::builder()
                                .label("No Flatpak remotes configured")
                                .css_classes(vec!["dim-label".to_string()])
                                .margin_top(12)
                                .margin_bottom(12)
                                .build();
                            flatpak_list.append(&empty_label);
                        } else {
                            for repo in repos {
                                let row = Self::create_repo_row(&repo, pm.clone());
                                flatpak_list.append(&row);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to add flatpak remote: {}", e);
                    }
                }
            });
        });

        add_dialog.present();
    }

    fn create_repo_row(repo: &Repository, pm: Arc<Mutex<PackageManager>>) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(&repo.name)
            .subtitle(repo.url.as_deref().unwrap_or("No URL"))
            .build();

        // Status indicator
        let status_icon = if repo.enabled {
            gtk::Image::builder()
                .icon_name("emblem-ok-symbolic")
                .css_classes(vec!["success".to_string()])
                .tooltip_text("Enabled")
                .build()
        } else {
            gtk::Image::builder()
                .icon_name("action-unavailable-symbolic")
                .css_classes(vec!["warning".to_string()])
                .tooltip_text("Disabled")
                .build()
        };
        row.add_suffix(&status_icon);

        // Remove button (only for non-system repos)
        if repo.name != "flathub" && repo.name != "fedora" {
            let remove_btn = gtk::Button::builder()
                .icon_name("user-trash-symbolic")
                .css_classes(vec!["flat".to_string(), "circular".to_string()])
                .tooltip_text("Remove remote")
                .valign(gtk::Align::Center)
                .build();

            let repo_name = repo.name.clone();
            let pm_clone = pm.clone();
            remove_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                let repo_name = repo_name.clone();
                let pm = pm_clone.clone();
                glib::spawn_future_local(async move {
                    let result = {
                        let manager = pm.lock().await;
                        manager
                            .remove_repository(PackageSource::Flatpak, &repo_name)
                            .await
                    };
                    match result {
                        Ok(_) => {
                            tracing::info!("Removed flatpak remote: {}", repo_name);
                        }
                        Err(e) => {
                            tracing::error!("Failed to remove flatpak remote: {}", e);
                        }
                    }
                });
            });
            row.add_suffix(&remove_btn);
        }

        row
    }
}
