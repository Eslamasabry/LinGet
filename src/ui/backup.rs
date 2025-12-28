use crate::backend::PackageManager;
use crate::models::{Config, Package, PackageSource, PackageStatus};

use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
pub struct BackupData {
    pub version: u32,
    pub created_at: String,
    pub config: BackupConfig,
    pub packages: HashMap<String, Vec<PackageEntry>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BackupConfig {
    pub enabled_sources: Vec<String>,
    pub ignored_packages: Vec<String>,
    pub favorite_packages: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PackageEntry {
    pub name: String,
    pub version: String,
}

#[derive(Clone)]
struct ImportPackage {
    name: String,
    version: String,
    source: PackageSource,
    is_missing: bool,
    selected: bool,
}

pub fn show_export_dialog(window: &impl IsA<gtk::Window>, pm: Arc<Mutex<PackageManager>>) {
    let dialog = gtk::FileChooserNative::builder()
        .title("Export Packages")
        .action(gtk::FileChooserAction::Save)
        .modal(true)
        .transient_for(&window.clone().upcast::<gtk::Window>())
        .build();

    dialog.set_current_name("linget-backup.json");

    let json_filter = gtk::FileFilter::new();
    json_filter.set_name(Some("JSON files"));
    json_filter.add_mime_type("application/json");
    json_filter.add_pattern("*.json");
    dialog.add_filter(&json_filter);

    let window_clone = window.clone().upcast::<gtk::Window>();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    run_export(window_clone.clone(), pm.clone(), path);
                }
            }
        }
    });

    dialog.show();
}

fn run_export(window: gtk::Window, pm: Arc<Mutex<PackageManager>>, path: PathBuf) {
    let toast_overlay = find_toast_overlay(&window);

    glib::spawn_future_local(async move {
        let config = Config::load();

        let all_packages = {
            let manager = pm.lock().await;
            manager.list_all_installed().await
        };

        match all_packages {
            Ok(packages) => {
                let mut packages_by_source: HashMap<String, Vec<PackageEntry>> = HashMap::new();
                for pkg in packages {
                    let source_key = format!("{:?}", pkg.source).to_lowercase();
                    packages_by_source
                        .entry(source_key)
                        .or_default()
                        .push(PackageEntry {
                            name: pkg.name,
                            version: pkg.version,
                        });
                }

                let enabled_sources: Vec<String> = config
                    .enabled_sources
                    .to_sources()
                    .iter()
                    .map(|s| format!("{:?}", s).to_lowercase())
                    .collect();

                let backup = BackupData {
                    version: 1,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    config: BackupConfig {
                        enabled_sources,
                        ignored_packages: config.ignored_packages.clone(),
                        favorite_packages: config.favorite_packages.clone(),
                    },
                    packages: packages_by_source,
                };

                let total_packages: usize = backup.packages.values().map(|v| v.len()).sum();

                match serde_json::to_string_pretty(&backup) {
                    Ok(json) => match std::fs::write(&path, json) {
                        Ok(_) => {
                            if let Some(overlay) = &toast_overlay {
                                let filename = path
                                    .file_name()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let msg =
                                    format!("Exported {} packages to {}", total_packages, filename);
                                let toast = adw::Toast::builder().title(msg).timeout(5).build();
                                overlay.add_toast(toast);
                            }
                        }
                        Err(e) => {
                            show_error_toast(&toast_overlay, &format!("Failed to write file: {e}"))
                        }
                    },
                    Err(e) => {
                        show_error_toast(&toast_overlay, &format!("Failed to serialize: {}", e))
                    }
                }
            }
            Err(e) => show_error_toast(&toast_overlay, &format!("Failed to load packages: {}", e)),
        }
    });
}

pub fn show_import_dialog(window: &impl IsA<gtk::Window>, pm: Arc<Mutex<PackageManager>>) {
    let dialog = gtk::FileChooserNative::builder()
        .title("Import Packages")
        .action(gtk::FileChooserAction::Open)
        .modal(true)
        .transient_for(&window.clone().upcast::<gtk::Window>())
        .build();

    let json_filter = gtk::FileFilter::new();
    json_filter.set_name(Some("JSON files"));
    json_filter.add_mime_type("application/json");
    json_filter.add_pattern("*.json");
    dialog.add_filter(&json_filter);

    let window_clone = window.clone().upcast::<gtk::Window>();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    show_import_diff_dialog(window_clone.clone(), pm.clone(), path);
                }
            }
        }
    });

    dialog.show();
}

fn show_import_diff_dialog(window: gtk::Window, pm: Arc<Mutex<PackageManager>>, path: PathBuf) {
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            show_error_dialog(&window, &format!("Failed to read file: {}", e));
            return;
        }
    };

    let backup: BackupData = match serde_json::from_str(&content) {
        Ok(b) => b,
        Err(e) => {
            show_error_dialog(&window, &format!("Invalid backup file: {}", e));
            return;
        }
    };

    let pm_clone = pm.clone();
    let window_clone = window.clone();
    let backup_clone = backup.clone();

    glib::spawn_future_local(async move {
        let installed_packages = {
            let manager = pm_clone.lock().await;
            manager.list_all_installed().await.unwrap_or_default()
        };

        let installed_set: HashSet<(String, String)> = installed_packages
            .iter()
            .map(|p| (p.name.clone(), format!("{:?}", p.source).to_lowercase()))
            .collect();

        let mut import_packages: Vec<ImportPackage> = Vec::new();
        let mut missing_count = 0;
        let mut already_installed_count = 0;

        for (source_str, packages) in &backup_clone.packages {
            let Some(source) = PackageSource::from_str(source_str) else {
                continue;
            };

            for pkg in packages {
                let is_missing = !installed_set.contains(&(pkg.name.clone(), source_str.clone()));
                if is_missing {
                    missing_count += 1;
                } else {
                    already_installed_count += 1;
                }

                import_packages.push(ImportPackage {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    source,
                    is_missing,
                    selected: is_missing,
                });
            }
        }

        import_packages.sort_by(|a, b| {
            b.is_missing
                .cmp(&a.is_missing)
                .then_with(|| a.source.to_string().cmp(&b.source.to_string()))
                .then_with(|| a.name.cmp(&b.name))
        });

        build_import_dialog(
            window_clone,
            pm_clone,
            backup_clone,
            import_packages,
            missing_count,
            already_installed_count,
        );
    });
}

fn build_import_dialog(
    window: gtk::Window,
    pm: Arc<Mutex<PackageManager>>,
    backup: BackupData,
    import_packages: Vec<ImportPackage>,
    missing_count: usize,
    already_installed_count: usize,
) {
    let dialog = adw::Dialog::builder()
        .title("Import Packages")
        .content_width(600)
        .content_height(500)
        .build();

    let toolbar_view = adw::ToolbarView::new();

    let header = adw::HeaderBar::builder().show_title(true).build();
    toolbar_view.add_top_bar(&header);

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let summary_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .build();

    let missing_label = gtk::Label::builder()
        .label(format!("<b>{}</b> missing", missing_count))
        .use_markup(true)
        .css_classes(["accent"])
        .build();

    let installed_label = gtk::Label::builder()
        .label(format!("{} already installed", already_installed_count))
        .css_classes(["dim-label"])
        .build();

    let created_label = gtk::Label::builder()
        .label(format!(
            "Backup from {}",
            backup
                .created_at
                .split('T')
                .next()
                .unwrap_or(&backup.created_at)
        ))
        .css_classes(["dim-label"])
        .hexpand(true)
        .halign(gtk::Align::End)
        .build();

    summary_box.append(&missing_label);
    summary_box.append(&installed_label);
    summary_box.append(&created_label);
    content_box.append(&summary_box);

    let sources: Vec<PackageSource> = import_packages
        .iter()
        .map(|p| p.source)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let filter_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let filter_label = gtk::Label::builder()
        .label("Filter:")
        .css_classes(["dim-label"])
        .build();
    filter_box.append(&filter_label);

    let source_buttons: Rc<RefCell<Vec<(PackageSource, gtk::ToggleButton)>>> =
        Rc::new(RefCell::new(Vec::new()));

    for source in &sources {
        let btn = gtk::ToggleButton::builder()
            .label(source.to_string())
            .active(true)
            .css_classes(["flat", "chip"])
            .build();
        filter_box.append(&btn);
        source_buttons.borrow_mut().push((*source, btn));
    }

    content_box.append(&filter_box);

    let scrolled = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();

    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();

    let check_buttons: Rc<RefCell<Vec<(ImportPackage, gtk::CheckButton)>>> =
        Rc::new(RefCell::new(Vec::new()));

    for pkg in &import_packages {
        let row = adw::ActionRow::builder()
            .title(&pkg.name)
            .subtitle(format!("{} • {}", pkg.source, pkg.version))
            .build();

        if !pkg.is_missing {
            row.add_css_class("dim-label");
        }

        let check = gtk::CheckButton::builder()
            .active(pkg.selected)
            .valign(gtk::Align::Center)
            .build();

        if !pkg.is_missing {
            check.set_sensitive(false);
            check.set_active(false);
        }

        row.add_prefix(&check);

        let status_label = gtk::Label::builder()
            .label(if pkg.is_missing {
                "Missing"
            } else {
                "Installed"
            })
            .css_classes(if pkg.is_missing {
                vec!["chip", "accent"]
            } else {
                vec!["chip", "dim-label"]
            })
            .valign(gtk::Align::Center)
            .build();
        row.add_suffix(&status_label);

        list_box.append(&row);
        check_buttons.borrow_mut().push((pkg.clone(), check));
    }

    let source_buttons_clone = source_buttons.clone();
    let check_buttons_clone = check_buttons.clone();
    let list_box_clone = list_box.clone();

    for (_source, btn) in source_buttons.borrow().iter() {
        let source_buttons = source_buttons_clone.clone();
        let check_buttons = check_buttons_clone.clone();
        let list_box = list_box_clone.clone();

        btn.connect_toggled(move |_| {
            let active_sources: HashSet<PackageSource> = source_buttons
                .borrow()
                .iter()
                .filter(|(_, b)| b.is_active())
                .map(|(s, _)| *s)
                .collect();

            let mut idx = 0;
            let mut row = list_box.row_at_index(idx);
            for (pkg, _) in check_buttons.borrow().iter() {
                if let Some(r) = row {
                    r.set_visible(active_sources.contains(&pkg.source));
                    idx += 1;
                    row = list_box.row_at_index(idx);
                }
            }
        });
    }

    scrolled.set_child(Some(&list_box));
    content_box.append(&scrolled);

    let action_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::End)
        .build();

    let select_all_btn = gtk::Button::builder()
        .label("Select All Missing")
        .css_classes(["flat"])
        .build();

    let select_none_btn = gtk::Button::builder()
        .label("Select None")
        .css_classes(["flat"])
        .build();

    let check_buttons_select_all = check_buttons.clone();
    select_all_btn.connect_clicked(move |_| {
        for (pkg, check) in check_buttons_select_all.borrow().iter() {
            if pkg.is_missing {
                check.set_active(true);
            }
        }
    });

    let check_buttons_select_none = check_buttons.clone();
    select_none_btn.connect_clicked(move |_| {
        for (_, check) in check_buttons_select_none.borrow().iter() {
            check.set_active(false);
        }
    });

    action_box.append(&select_none_btn);
    action_box.append(&select_all_btn);
    content_box.append(&action_box);

    toolbar_view.set_content(Some(&content_box));

    let bottom_bar = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(12)
        .margin_bottom(12)
        .halign(gtk::Align::End)
        .build();

    let cancel_btn = gtk::Button::builder()
        .label("Cancel")
        .css_classes(["flat"])
        .build();

    let import_btn = gtk::Button::builder()
        .label("Import Selected")
        .css_classes(["suggested-action"])
        .build();

    bottom_bar.append(&cancel_btn);
    bottom_bar.append(&import_btn);
    toolbar_view.add_bottom_bar(&bottom_bar);

    dialog.set_child(Some(&toolbar_view));

    let dialog_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_cancel.close();
    });

    let dialog_import = dialog.clone();
    let check_buttons_import = check_buttons.clone();
    let window_import = window.clone();

    import_btn.connect_clicked(move |_| {
        let selected: Vec<ImportPackage> = check_buttons_import
            .borrow()
            .iter()
            .filter(|(_, check)| check.is_active())
            .map(|(pkg, _)| pkg.clone())
            .collect();

        if selected.is_empty() {
            return;
        }

        dialog_import.close();
        run_selective_import(window_import.clone(), pm.clone(), selected, backup.clone());
    });

    dialog.present(Some(&window));
}

fn run_selective_import(
    window: gtk::Window,
    pm: Arc<Mutex<PackageManager>>,
    packages: Vec<ImportPackage>,
    backup: BackupData,
) {
    let toast_overlay = find_toast_overlay(&window);
    let count = packages.len();

    if let Some(overlay) = &toast_overlay {
        let toast = adw::Toast::builder()
            .title(format!("Installing {} packages...", count))
            .timeout(2)
            .build();
        overlay.add_toast(toast);
    }

    glib::spawn_future_local(async move {
        let manager = pm.lock().await;
        let mut installed = 0;
        let mut failed = 0;

        for pkg in &packages {
            let stub_pkg = Package {
                name: pkg.name.clone(),
                version: String::new(),
                available_version: None,
                description: String::new(),
                source: pkg.source,
                status: PackageStatus::NotInstalled,
                size: None,
                homepage: None,
                license: None,
                maintainer: None,
                dependencies: Vec::new(),
                install_date: None,
                update_category: None,
                enrichment: None,
            };

            match manager.install(&stub_pkg).await {
                Ok(_) => installed += 1,
                Err(_) => failed += 1,
            }
        }

        drop(manager);

        let mut config = Config::load();
        config.ignored_packages = backup.config.ignored_packages;
        config.favorite_packages = backup.config.favorite_packages;
        let _ = config.save();

        if let Some(overlay) = &toast_overlay {
            let message = if failed > 0 {
                format!(
                    "Import complete: {} installed, {} failed",
                    installed, failed
                )
            } else {
                format!("Import complete: {} packages installed", installed)
            };
            let toast = adw::Toast::builder().title(&message).timeout(5).build();
            overlay.add_toast(toast);
        }
    });
}

fn find_toast_overlay(window: &gtk::Window) -> Option<adw::ToastOverlay> {
    window
        .first_child()
        .and_then(|c| c.first_child())
        .and_then(|c| c.downcast::<adw::ToastOverlay>().ok())
}

fn show_error_toast(toast_overlay: &Option<adw::ToastOverlay>, message: &str) {
    if let Some(overlay) = toast_overlay {
        let toast = adw::Toast::builder().title(message).timeout(5).build();
        overlay.add_toast(toast);
    }
}

fn show_error_dialog(window: &gtk::Window, message: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading("Error")
        .body(message)
        .build();
    dialog.add_response("ok", "OK");
    dialog.present(Some(window));
}
