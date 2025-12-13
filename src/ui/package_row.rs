use crate::models::{get_package_icon, Package, PackageStatus};
use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

/// A widget representing a single package in the list
pub struct PackageRow {
    pub widget: adw::ActionRow,
    pub package: Rc<RefCell<Package>>,
    pub checkbox: gtk::CheckButton,
    pub action_button: gtk::Button,
    pub spinner: gtk::Spinner,
    pub source_button: gtk::Button,
}

impl PackageRow {
    /// Escape special characters for pango markup
    fn escape_markup(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    pub fn new(package: Package, _check_group: Option<&gtk::CheckButton>) -> Self {
        let package = Rc::new(RefCell::new(package));
        let pkg = package.borrow();

        let subtitle = if pkg.description.is_empty() {
            pkg.source.to_string()
        } else {
            Self::escape_markup(&pkg.description)
        };

        let row = adw::ActionRow::builder()
            .title(&pkg.name)
            .subtitle(&subtitle)
            .activatable(true)
            .build();

        // Checkbox for bulk selection
        let checkbox = gtk::CheckButton::builder()
            .valign(gtk::Align::Center)
            .visible(false)
            .build();

        row.add_prefix(&checkbox);

        // App icon - try to get actual icon, fall back to source icon
        let icon_name = get_package_icon(&pkg.name, pkg.source);
        let app_icon = gtk::Image::builder()
            .icon_name(&icon_name)
            .pixel_size(32)
            .build();

        // If the icon name is the same as source icon, it's a fallback
        // Use smaller size for generic icons
        if icon_name == pkg.source.icon_name() {
            app_icon.set_pixel_size(24);
        }

        row.add_prefix(&app_icon);

        // Right side content box
        let suffix_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .valign(gtk::Align::Center)
            .build();

        // Version label
        let version_label = gtk::Label::builder()
            .label(&pkg.display_version())
            .build();
        version_label.add_css_class("dim-label");
        version_label.add_css_class("caption");
        suffix_box.append(&version_label);

        // Source badge (clickable to filter)
        let source_button = gtk::Button::builder()
            .label(&pkg.source.to_string())
            .valign(gtk::Align::Center)
            .tooltip_text(&format!("Filter by {}", pkg.source))
            .build();
        source_button.add_css_class("flat");
        source_button.add_css_class("caption");
        source_button.add_css_class(pkg.source.color_class());
        suffix_box.append(&source_button);

        // Update indicator
        if pkg.status == PackageStatus::UpdateAvailable {
            let update_icon = gtk::Image::builder()
                .icon_name("software-update-available-symbolic")
                .tooltip_text("Update available")
                .build();
            update_icon.add_css_class("accent");
            suffix_box.append(&update_icon);
        }

        row.add_suffix(&suffix_box);

        // Action button
        let action_button = Self::create_action_button(&pkg);
        row.add_suffix(&action_button);

        // Loading spinner (hidden by default)
        let spinner = gtk::Spinner::builder()
            .valign(gtk::Align::Center)
            .visible(false)
            .build();
        row.add_suffix(&spinner);

        // Navigate icon
        let nav_icon = gtk::Image::builder()
            .icon_name("go-next-symbolic")
            .build();
        nav_icon.add_css_class("dim-label");
        row.add_suffix(&nav_icon);

        drop(pkg);

        Self {
            widget: row,
            package,
            checkbox,
            action_button,
            spinner,
            source_button,
        }
    }

    fn create_action_button(pkg: &Package) -> gtk::Button {
        let btn = match pkg.status {
            PackageStatus::Installed => {
                let b = gtk::Button::builder()
                    .icon_name("user-trash-symbolic")
                    .tooltip_text("Remove")
                    .valign(gtk::Align::Center)
                    .build();
                b.add_css_class("flat");
                b.add_css_class("circular");
                b
            }
            PackageStatus::UpdateAvailable => {
                let b = gtk::Button::builder()
                    .icon_name("software-update-available-symbolic")
                    .tooltip_text("Update")
                    .valign(gtk::Align::Center)
                    .build();
                b.add_css_class("flat");
                b.add_css_class("circular");
                b.add_css_class("suggested-action");
                b
            }
            PackageStatus::NotInstalled => {
                let b = gtk::Button::builder()
                    .icon_name("list-add-symbolic")
                    .tooltip_text("Install")
                    .valign(gtk::Align::Center)
                    .build();
                b.add_css_class("flat");
                b.add_css_class("circular");
                b.add_css_class("suggested-action");
                b
            }
            _ => {
                let b = gtk::Button::builder()
                    .icon_name("content-loading-symbolic")
                    .sensitive(false)
                    .valign(gtk::Align::Center)
                    .build();
                b.add_css_class("flat");
                b.add_css_class("circular");
                b
            }
        };
        btn
    }

    pub fn set_selection_mode(&self, enabled: bool) {
        self.checkbox.set_visible(enabled);
    }
}
