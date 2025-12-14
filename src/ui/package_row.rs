use crate::models::{get_package_icon, Package, PackageStatus};
use gtk4::prelude::*;
use gtk4::{self as gtk, pango};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A widget representing a single package in the list
pub struct PackageRow {
    pub widget: adw::ActionRow,
    pub package: Rc<RefCell<Package>>,
    pub checkbox: gtk::CheckButton,
    pub icon_frame: gtk::Box,
    pub action_button: gtk::Button,
    pub spinner: gtk::Spinner,
    pub progress_bar: gtk::ProgressBar,
    pub version_label: gtk::Label,
    pub source_button: gtk::Button,
    pub update_icon: gtk::Image,
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

    pub fn new(
        package: Package,
        _check_group: Option<&gtk::CheckButton>,
        show_icons: bool,
    ) -> Self {
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
        row.add_css_class("pkg-row");

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

        let icon_frame = gtk::Box::builder()
            .width_request(40)
            .height_request(40)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .build();
        icon_frame.add_css_class("icon-frame");
        icon_frame.append(&app_icon);
        icon_frame.set_visible(show_icons);
        row.add_prefix(&icon_frame);

        // Right side content box
        let suffix_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .valign(gtk::Align::Center)
            .build();

        // Version label
        let version_text = pkg.display_version();
        let version_label = gtk::Label::builder().label(&version_text).build();
        version_label.add_css_class("chip");
        version_label.add_css_class("chip-muted");
        version_label.set_max_width_chars(18);
        version_label.set_ellipsize(pango::EllipsizeMode::End);
        if !version_text.is_empty() {
            version_label.set_tooltip_text(Some(&version_text));
        }
        suffix_box.append(&version_label);

        // Keep the version chip compact; long versions still remain accessible via tooltip.

        // Source badge (clickable to filter)
        let source_button = gtk::Button::builder()
            .label(pkg.source.to_string())
            .valign(gtk::Align::Center)
            .tooltip_text(format!("Filter by {}", pkg.source))
            .build();
        source_button.add_css_class("flat");
        source_button.add_css_class("chip");
        source_button.add_css_class(pkg.source.color_class());
        suffix_box.append(&source_button);

        // Update indicator (kept for live UI updates; visibility toggled)
        let update_icon = gtk::Image::builder()
            .icon_name("software-update-available-symbolic")
            .tooltip_text("Update available")
            .visible(pkg.status == PackageStatus::UpdateAvailable)
            .build();
        update_icon.add_css_class("accent");
        suffix_box.append(&update_icon);

        row.add_suffix(&suffix_box);

        // Action button
        let action_button = Self::create_action_button(&pkg);
        action_button.add_css_class("row-action");
        row.add_suffix(&action_button);

        // Loading spinner (hidden by default)
        let spinner = gtk::Spinner::builder()
            .valign(gtk::Align::Center)
            .visible(false)
            .build();
        spinner.add_css_class("row-spinner");
        row.add_suffix(&spinner);

        // Per-row progress (hidden by default; used for inline operations)
        let progress_bar = gtk::ProgressBar::builder()
            .show_text(false)
            .valign(gtk::Align::Center)
            .visible(false)
            .build();
        progress_bar.add_css_class("row-progress");
        progress_bar.set_width_request(86);
        progress_bar.set_height_request(6);
        row.add_suffix(&progress_bar);

        // Navigate icon
        let nav_icon = gtk::Image::builder().icon_name("go-next-symbolic").build();
        nav_icon.add_css_class("dim-label");
        nav_icon.add_css_class("nav-chevron");
        nav_icon.add_css_class("row-chevron");
        row.add_suffix(&nav_icon);

        drop(pkg);

        // NOTE: GTK4 no longer exposes a stable `size-allocate` signal for this widget type.
        // Keep suffix chips always visible; long text is ellipsized with tooltips.

        Self {
            widget: row,
            package,
            checkbox,
            icon_frame,
            action_button,
            spinner,
            progress_bar,
            version_label,
            source_button,
            update_icon,
        }
    }

    pub fn update_from_package(&self, pkg: &Package, show_icons: bool) {
        *self.package.borrow_mut() = pkg.clone();

        self.widget.set_title(&pkg.name);
        let subtitle = if pkg.description.is_empty() {
            pkg.source.to_string()
        } else {
            Self::escape_markup(&pkg.description)
        };
        self.widget.set_subtitle(&subtitle);

        self.icon_frame.set_visible(show_icons);

        let version_text = pkg.display_version();
        self.version_label.set_label(&version_text);
        self.version_label
            .set_tooltip_text((!version_text.is_empty()).then_some(version_text.as_str()));

        self.source_button.set_label(&pkg.source.to_string());
        self.source_button
            .set_tooltip_text(Some(&format!("Filter by {}", pkg.source)));

        for source_class in [
            "source-apt",
            "source-dnf",
            "source-pacman",
            "source-zypper",
            "source-flatpak",
            "source-snap",
            "source-npm",
            "source-pip",
            "source-pipx",
            "source-cargo",
            "source-brew",
            "source-aur",
            "source-conda",
            "source-mamba",
            "source-dart",
            "source-deb",
            "source-appimage",
        ] {
            self.source_button.remove_css_class(source_class);
        }
        self.source_button.add_css_class(pkg.source.color_class());

        self.update_icon
            .set_visible(pkg.status == PackageStatus::UpdateAvailable);
        Self::apply_action_button_style(&self.action_button, pkg.status);
    }

    fn create_action_button(pkg: &Package) -> gtk::Button {
        match pkg.status {
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
        }
    }

    pub fn apply_action_button_style(button: &gtk::Button, status: PackageStatus) {
        button.add_css_class("flat");
        button.add_css_class("circular");

        // Clear suggested-action style by default.
        button.remove_css_class("suggested-action");

        match status {
            PackageStatus::Installed => {
                button.set_icon_name("user-trash-symbolic");
                button.set_tooltip_text(Some("Remove"));
                button.set_sensitive(true);
            }
            PackageStatus::UpdateAvailable => {
                button.set_icon_name("software-update-available-symbolic");
                button.set_tooltip_text(Some("Update"));
                button.add_css_class("suggested-action");
                button.set_sensitive(true);
            }
            PackageStatus::NotInstalled => {
                button.set_icon_name("list-add-symbolic");
                button.set_tooltip_text(Some("Install"));
                button.add_css_class("suggested-action");
                button.set_sensitive(true);
            }
            _ => {
                button.set_icon_name("content-loading-symbolic");
                button.set_tooltip_text(Some("Working…"));
                button.set_sensitive(false);
            }
        }
    }

    pub fn set_selection_mode(&self, enabled: bool) {
        self.checkbox.set_visible(enabled);
    }
}
