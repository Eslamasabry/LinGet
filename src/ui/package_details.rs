use crate::backend::PackageManager;
use crate::models::{Package, PackageStatus};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita::prelude::*;
use libadwaita as adw;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PackageDetailsDialog;

impl PackageDetailsDialog {
    pub fn show(
        package: &Package,
        parent: &impl IsA<gtk::Window>,
        pm: Arc<Mutex<PackageManager>>,
        toast_overlay: adw::ToastOverlay,
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
            .label(&package.source.to_string())
            .xalign(0.0)
            .build();
        source_badge.add_css_class("caption");
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
        let details_group = adw::PreferencesGroup::builder()
            .title("Details")
            .build();

        // Version row
        let version_row = adw::ActionRow::builder()
            .title("Version")
            .subtitle(&package.display_version())
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
            .subtitle(&package.status.to_string())
            .build();
        details_group.add(&status_row);

        // Size row
        let size_row = adw::ActionRow::builder()
            .title("Size")
            .subtitle(&package.size_display())
            .build();
        details_group.add(&size_row);

        // Source row
        let source_row = adw::ActionRow::builder()
            .title("Source")
            .subtitle(package.source.description())
            .build();
        details_group.add(&source_row);

        content.append(&details_group);

        // Action buttons
        let actions_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .halign(gtk::Align::Center)
            .margin_top(16)
            .build();

        if package.has_update() {
            let update_btn = gtk::Button::builder()
                .label("Update")
                .build();
            update_btn.add_css_class("suggested-action");
            update_btn.add_css_class("pill");
            
            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_clone = dialog.clone();
            
            update_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                
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
                            let t = adw::Toast::new(&format!("Update failed: {}", e));
                            t.set_timeout(5);
                            toast.add_toast(t);
                        }
                    }
                });
            });
            
            actions_box.append(&update_btn);
        }

        let remove_btn = gtk::Button::builder()
            .label(if package.status == PackageStatus::NotInstalled { "Close" } else { "Remove" })
            .build();
            
        if package.status == PackageStatus::Installed {
            remove_btn.add_css_class("destructive-action");
            remove_btn.add_css_class("pill");
            
            let pkg_clone = package.clone();
            let pm_clone = pm.clone();
            let toast_clone = toast_overlay.clone();
            let dialog_clone = dialog.clone();
            
            remove_btn.connect_clicked(move |btn| {
                let pkg = pkg_clone.clone();
                let pm = pm_clone.clone();
                let toast = toast_clone.clone();
                let dialog = dialog_clone.clone();
                let btn = btn.clone();
                
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
                            let t = adw::Toast::new(&format!("Remove failed: {}", e));
                            t.set_timeout(5);
                            toast.add_toast(t);
                        }
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
