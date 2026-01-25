#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;

pub struct EmptyState {
    pub widget: adw::StatusPage,
}

impl EmptyState {
    pub fn new(icon_name: &str, title: &str, description: &str) -> Self {
        let status_page = adw::StatusPage::builder()
            .icon_name(icon_name)
            .title(title)
            .description(description)
            .vexpand(true)
            .hexpand(true)
            .build();

        status_page.add_css_class("empty-state");

        Self {
            widget: status_page,
        }
    }

    pub fn with_child(
        icon_name: &str,
        title: &str,
        description: &str,
        child: &impl IsA<gtk::Widget>,
    ) -> Self {
        let status_page = adw::StatusPage::builder()
            .icon_name(icon_name)
            .title(title)
            .description(description)
            .child(child)
            .vexpand(true)
            .hexpand(true)
            .build();

        status_page.add_css_class("empty-state");

        Self {
            widget: status_page,
        }
    }

    pub fn all_up_to_date() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Center)
            .build();

        let success_icon = gtk::Image::builder()
            .icon_name("emblem-ok-symbolic")
            .pixel_size(48)
            .build();
        success_icon.add_css_class("success-icon");
        child_box.append(&success_icon);

        let hint = gtk::Label::builder()
            .label("Your system is fresh and running the latest software. Nice work!")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        hint.add_css_class("dim-label");
        child_box.append(&hint);

        Self::with_child(
            "face-smile-big-symbolic",
            "Your System is Fresh! 🌿",
            "All packages are at their latest versions. You're all caught up!",
            &child_box,
        )
    }

    pub fn no_updates() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let tips_label = gtk::Label::builder()
            .label("💡 Tip: Press Ctrl+R to check for new updates")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        tips_label.add_css_class("dim-label");
        tips_label.add_css_class("caption");
        child_box.append(&tips_label);

        Self::with_child(
            "emblem-default-symbolic",
            "All Caught Up! ✨",
            "Nothing to update. Your packages are running their latest versions.",
            &child_box,
        )
    }

    pub fn search_packages() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Center)
            .build();

        let suggestions_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let suggestions = ["firefox", "vlc", "gimp", "vscode"];
        for suggestion in suggestions {
            let chip = gtk::Button::builder().label(suggestion).build();
            chip.add_css_class("pill");
            chip.add_css_class("flat");
            suggestions_box.append(&chip);
        }

        let try_label = gtk::Label::builder().label("Popular searches:").build();
        try_label.add_css_class("dim-label");
        try_label.add_css_class("caption");
        child_box.append(&try_label);
        child_box.append(&suggestions_box);

        let shortcut_hint = gtk::Label::builder()
            .label("Press / or Ctrl+F to start searching")
            .build();
        shortcut_hint.add_css_class("dim-label");
        child_box.append(&shortcut_hint);

        Self::with_child(
            "system-search-symbolic",
            "What Would You Like to Install? 🔍",
            "Search across APT, Flatpak, Snap, and more to find new software.",
            &child_box,
        )
    }

    /// No search results found
    pub fn no_results(query: &str) -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let tips_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .halign(gtk::Align::Center)
            .build();

        let tips_title = gtk::Label::builder().label("Search tips:").build();
        tips_title.add_css_class("heading");
        tips_box.append(&tips_title);

        let tips = [
            "• Try using fewer or different keywords",
            "• Check spelling of the package name",
            "• Enable more package sources in the sidebar",
        ];

        for tip in tips {
            let tip_label = gtk::Label::builder()
                .label(tip)
                .halign(gtk::Align::Start)
                .build();
            tip_label.add_css_class("dim-label");
            tips_box.append(&tip_label);
        }

        child_box.append(&tips_box);

        let escaped_query = query
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        Self::with_child(
            "edit-find-symbolic",
            "No Results Found",
            &format!("No packages match \"{}\"", escaped_query),
            &child_box,
        )
    }

    pub fn no_favorites() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Center)
            .build();

        let hint_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let star_icon = gtk::Image::builder()
            .icon_name("starred-symbolic")
            .pixel_size(16)
            .build();
        star_icon.add_css_class("dim-label");

        let hint_label = gtk::Label::builder()
            .label("Click the star icon on any package to add it here")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        hint_label.add_css_class("dim-label");

        hint_row.append(&star_icon);
        hint_row.append(&hint_label);
        child_box.append(&hint_row);

        let benefits_label = gtk::Label::builder()
            .label("Your VIP packages live here. Quick access to the ones you care about most.")
            .wrap(true)
            .max_width_chars(50)
            .justify(gtk::Justification::Center)
            .build();
        benefits_label.add_css_class("dim-label");
        benefits_label.add_css_class("caption");
        child_box.append(&benefits_label);

        Self::with_child(
            "starred-symbolic",
            "No Favorites Yet ⭐",
            "Star packages to keep them close. They'll show up right here.",
            &child_box,
        )
    }

    /// Empty library (no installed packages)
    pub fn empty_library() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let hint = gtk::Label::builder()
            .label("This could mean:\n• No packages are installed from enabled sources\n• The selected source filter has no packages\n• Try enabling more sources in the sidebar")
            .wrap(true)
            .justify(gtk::Justification::Left)
            .halign(gtk::Align::Center)
            .build();
        hint.add_css_class("dim-label");
        child_box.append(&hint);

        Self::with_child(
            "package-x-generic-symbolic",
            "No Packages Found",
            "Your package library appears empty with the current filters.",
            &child_box,
        )
    }

    /// Empty collection
    pub fn empty_collection(collection_name: &str) -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let hint = gtk::Label::builder()
            .label("To add packages to this collection:\n1. Go to Library or Updates view\n2. Click on a package to open details\n3. Use the \"Add to Collection\" button")
            .wrap(true)
            .justify(gtk::Justification::Left)
            .halign(gtk::Align::Center)
            .build();
        hint.add_css_class("dim-label");
        child_box.append(&hint);

        Self::with_child(
            "folder-symbolic",
            &format!("\"{}\" is Empty", collection_name),
            "Start organizing by adding packages to this collection.",
            &child_box,
        )
    }

    /// Provider not available
    pub fn provider_unavailable(provider_name: &str, install_hint: Option<&str>) -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        if let Some(hint) = install_hint {
            let install_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(8)
                .halign(gtk::Align::Center)
                .build();

            let install_label = gtk::Label::builder().label("To install:").build();
            install_label.add_css_class("heading");
            install_box.append(&install_label);

            let command_label = gtk::Label::builder().label(hint).selectable(true).build();
            command_label.add_css_class("monospace");
            command_label.add_css_class("dim-label");
            install_box.append(&command_label);

            child_box.append(&install_box);
        }

        Self::with_child(
            "dialog-warning-symbolic",
            &format!("{} Not Available", provider_name),
            &format!("{} is not installed on your system.", provider_name),
            &child_box,
        )
    }

    pub fn loading() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Center)
            .build();

        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(48)
            .height_request(48)
            .build();
        child_box.append(&spinner);

        let hint = gtk::Label::builder()
            .label("This may take a moment for large package lists...")
            .build();
        hint.add_css_class("dim-label");
        hint.add_css_class("caption");
        child_box.append(&hint);

        Self::with_child(
            "content-loading-symbolic",
            "Gathering Packages... ⏳",
            "Scanning your enabled sources. Almost there!",
            &child_box,
        )
    }

    /// Loading with progress info
    pub fn loading_with_progress(loaded: usize, total: usize) -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Center)
            .build();

        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(48)
            .height_request(48)
            .build();
        child_box.append(&spinner);

        let progress_label = gtk::Label::builder()
            .label(format!("Loaded {} of {} packages", loaded, total))
            .build();
        progress_label.add_css_class("dim-label");
        child_box.append(&progress_label);

        Self::with_child(
            "content-loading-symbolic",
            "Loading Packages...",
            "Fetching package information from your enabled sources.",
            &child_box,
        )
    }

    /// Generic error state
    pub fn error(message: &str) -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let help_label = gtk::Label::builder()
            .label("If this problem persists, try:\n• Checking your internet connection\n• Running 'linget providers' in terminal for diagnostics")
            .wrap(true)
            .justify(gtk::Justification::Left)
            .halign(gtk::Align::Center)
            .build();
        help_label.add_css_class("dim-label");
        help_label.add_css_class("caption");
        child_box.append(&help_label);

        Self::with_child(
            "dialog-error-symbolic",
            "Something Went Wrong",
            message,
            &child_box,
        )
    }

    /// Error with retry action
    pub fn error_with_retry<F>(message: &str, on_retry: F) -> Self
    where
        F: Fn() + 'static,
    {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Center)
            .build();

        let retry_btn = gtk::Button::builder()
            .label("Try Again")
            .halign(gtk::Align::Center)
            .build();
        retry_btn.add_css_class("pill");
        retry_btn.add_css_class("suggested-action");
        retry_btn.connect_clicked(move |_| on_retry());
        child_box.append(&retry_btn);

        let help_label = gtk::Label::builder()
            .label("If this keeps happening, check the terminal for detailed error messages.")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .build();
        help_label.add_css_class("dim-label");
        help_label.add_css_class("caption");
        child_box.append(&help_label);

        Self::with_child(
            "dialog-error-symbolic",
            "Something Went Wrong",
            message,
            &child_box,
        )
    }

    pub fn first_run() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(20)
            .halign(gtk::Align::Center)
            .build();

        let features_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(24)
            .halign(gtk::Align::Center)
            .build();

        let feature_items = [
            ("view-list-symbolic", "All packages\nin one place"),
            (
                "software-update-available-symbolic",
                "Easy updates\nacross sources",
            ),
            ("starred-symbolic", "Organize with\nfavorites"),
        ];

        for (icon, text) in feature_items {
            let feature_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(8)
                .halign(gtk::Align::Center)
                .build();

            let icon_widget = gtk::Image::builder().icon_name(icon).pixel_size(32).build();
            icon_widget.add_css_class("dim-label");
            feature_box.append(&icon_widget);

            let label = gtk::Label::builder()
                .label(text)
                .justify(gtk::Justification::Center)
                .build();
            label.add_css_class("caption");
            feature_box.append(&label);

            features_box.append(&feature_box);
        }

        child_box.append(&features_box);

        let shortcut_hint = gtk::Label::builder()
            .label("Press Ctrl+F to search • Ctrl+R to refresh • Ctrl+S for selection mode")
            .build();
        shortcut_hint.add_css_class("dim-label");
        shortcut_hint.add_css_class("caption");
        child_box.append(&shortcut_hint);

        Self::with_child(
            "io.github.linget",
            "Welcome to LinGet! 🚀",
            "Your unified package manager for Linux.\nAPT, Flatpak, Snap, npm, pip, cargo — all in one place.",
            &child_box,
        )
    }

    /// Offline state
    pub fn offline() -> Self {
        let child_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .build();

        let hint = gtk::Label::builder()
            .label("Some features like package search and updates require an internet connection.")
            .wrap(true)
            .justify(gtk::Justification::Center)
            .max_width_chars(50)
            .build();
        hint.add_css_class("dim-label");
        child_box.append(&hint);

        Self::with_child(
            "network-offline-symbolic",
            "You're Offline",
            "Check your internet connection and try again.",
            &child_box,
        )
    }

    pub fn set_icon(&self, icon_name: &str) {
        self.widget.set_icon_name(Some(icon_name));
    }

    pub fn set_title(&self, title: &str) {
        self.widget.set_title(title);
    }

    pub fn set_description(&self, description: &str) {
        self.widget.set_description(Some(description));
    }
}
