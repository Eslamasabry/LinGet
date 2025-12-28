use crate::models::{get_package_icon, Package, PackageStatus, UpdateCategory};

use gtk4::prelude::*;
use gtk4::{self as gtk, pango};
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub struct PackageRowModel {
    pub package: Package,
    pub is_favorite: bool,
    pub is_selected: bool,
    pub selection_mode: bool,
    pub show_icons: bool,
    pub is_loading: bool,
    pub icon_name: String,
    pub compact: bool,
    pub is_scheduled: bool,
}

#[derive(Debug, Clone)]
pub struct PackageRowInit {
    pub package: Package,
    pub is_favorite: bool,
    pub selection_mode: bool,
    pub show_icons: bool,
    pub compact: bool,
    pub is_scheduled: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum PackageRowInput {
    SetFavorite(bool),
    SetSelected(bool),
    SetSelectionMode(bool),
    SetLoading(bool),
    SetScheduled(bool),
    UpdatePackage(Box<Package>),
}

#[derive(Debug, Clone)]
pub enum PackageRowOutput {
    Clicked(Package),
    ActionClicked(Package),
    SourceFilterClicked(Package),
    FavoriteToggled(Package),
    SelectionChanged(Package, bool),
}

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn get_subtitle(pkg: &Package) -> String {
    if pkg.description.is_empty() {
        pkg.source.to_string()
    } else {
        escape_markup(&pkg.description)
    }
}

fn get_action_icon(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::Installed => "user-trash-symbolic",
        PackageStatus::UpdateAvailable => "software-update-available-symbolic",
        PackageStatus::NotInstalled => "list-add-symbolic",
        _ => "content-loading-symbolic",
    }
}

fn get_action_tooltip(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::Installed => "Remove",
        PackageStatus::UpdateAvailable => "Update",
        PackageStatus::NotInstalled => "Install",
        _ => "Working...",
    }
}

fn is_action_suggested(status: PackageStatus) -> bool {
    matches!(
        status,
        PackageStatus::UpdateAvailable | PackageStatus::NotInstalled
    )
}

fn get_update_category_classes(category: Option<UpdateCategory>) -> Vec<&'static str> {
    match category {
        Some(cat) => vec!["update-category-badge", cat.css_class()],
        None => vec!["update-category-badge", "update-minor"],
    }
}

fn get_update_category_icon(category: Option<UpdateCategory>) -> &'static str {
    category.map_or("software-update-available-symbolic", |c| c.icon_name())
}

fn get_update_category_label(category: Option<UpdateCategory>) -> &'static str {
    category.map_or("Update", |c| c.label())
}

#[relm4::factory(pub)]
impl FactoryComponent for PackageRowModel {
    type Init = PackageRowInit;
    type Input = PackageRowInput;
    type Output = PackageRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        adw::ActionRow {
            #[watch]
            set_title: &self.package.name,
            #[watch]
            set_subtitle: &get_subtitle(&self.package),
            set_activatable: true,
            add_css_class: "pkg-row",
            #[watch]
            set_css_classes: if self.compact {
                &["pkg-row", "compact-row"]
            } else {
                &["pkg-row"]
            },

            connect_activated[sender, pkg = self.package.clone()] => move |_| {
                sender.output(PackageRowOutput::Clicked(pkg.clone())).ok();
            },

            add_prefix = &gtk::CheckButton {
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: self.selection_mode,
                #[watch]
                set_active: self.is_selected,
                connect_toggled[sender, pkg = self.package.clone()] => move |btn| {
                    sender.output(PackageRowOutput::SelectionChanged(pkg.clone(), btn.is_active())).ok();
                },
            },

            add_prefix = &gtk::Box {
                set_width_request: 52,
                set_height_request: 52,
                set_valign: gtk::Align::Center,
                set_halign: gtk::Align::Center,
                add_css_class: "icon-frame",
                #[watch]
                set_visible: self.show_icons,

                gtk::Image {
                    set_icon_name: Some(&self.icon_name),
                    set_pixel_size: if self.icon_name == self.package.source.icon_name() { 36 } else { 48 },
                },
            },

            add_suffix = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,
                set_valign: gtk::Align::Center,

                gtk::Label {
                    #[watch]
                    set_label: &self.package.display_version(),
                    add_css_class: "chip",
                    add_css_class: "chip-muted",
                    set_max_width_chars: 18,
                    set_ellipsize: pango::EllipsizeMode::End,
                },

                gtk::Button {
                    set_label: &self.package.source.to_string(),
                    set_valign: gtk::Align::Center,
                    set_tooltip_text: Some(&format!("Filter by {}", self.package.source)),
                    add_css_class: "flat",
                    add_css_class: "chip",
                    add_css_class: "source-chip",
                    add_css_class: self.package.source.color_class(),
                    connect_clicked[sender, pkg = self.package.clone()] => move |_| {
                        sender.output(PackageRowOutput::SourceFilterClicked(pkg.clone())).ok();
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_visible: self.package.status == PackageStatus::UpdateAvailable,
                    #[watch]
                    set_css_classes: &get_update_category_classes(self.package.update_category),
                    #[watch]
                    set_tooltip_text: Some(get_update_category_label(self.package.update_category)),

                    gtk::Image {
                        #[watch]
                        set_icon_name: Some(get_update_category_icon(self.package.update_category)),
                        set_pixel_size: 14,
                    },

                    gtk::Label {
                        #[watch]
                        set_label: get_update_category_label(self.package.update_category),
                        add_css_class: "caption",
                    },
                },

                gtk::Image {
                    set_icon_name: Some("alarm-symbolic"),
                    set_pixel_size: 16,
                    set_valign: gtk::Align::Center,
                    set_tooltip_text: Some("Scheduled for update"),
                    add_css_class: "scheduled-indicator",
                    add_css_class: "accent",
                    #[watch]
                    set_visible: self.is_scheduled,
                },

                gtk::ToggleButton {
                    #[watch]
                    set_icon_name: if self.is_favorite { "starred-symbolic" } else { "non-starred-symbolic" },
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_tooltip_text: Some(if self.is_favorite { "Remove from favorites" } else { "Add to favorites" }),
                    add_css_class: "flat",
                    add_css_class: "circular",
                    add_css_class: "favorite-btn",
                    #[watch]
                    set_active: self.is_favorite,
                    #[watch]
                    set_css_classes: if self.is_favorite { &["flat", "circular", "favorite-btn", "favorited"] } else { &["flat", "circular", "favorite-btn"] },
                    connect_toggled[sender, pkg = self.package.clone()] => move |_| {
                        sender.output(PackageRowOutput::FavoriteToggled(pkg.clone())).ok();
                    },
                },
            },

            add_suffix = &gtk::Button {
                #[watch]
                set_icon_name: get_action_icon(self.package.status),
                #[watch]
                set_tooltip_text: Some(get_action_tooltip(self.package.status)),
                set_valign: gtk::Align::Center,
                #[watch]
                set_css_classes: if is_action_suggested(self.package.status) {
                    &["flat", "circular", "row-action", "suggested-action"]
                } else {
                    &["flat", "circular", "row-action"]
                },
                #[watch]
                set_sensitive: !self.is_loading && !matches!(self.package.status, PackageStatus::Installing | PackageStatus::Removing | PackageStatus::Updating),
                connect_clicked[sender, pkg = self.package.clone()] => move |_| {
                    sender.output(PackageRowOutput::ActionClicked(pkg.clone())).ok();
                },
            },

            add_suffix = &gtk::Spinner {
                set_valign: gtk::Align::Center,
                add_css_class: "row-spinner",
                #[watch]
                set_visible: self.is_loading,
                #[watch]
                set_spinning: self.is_loading,
            },

            add_suffix = &gtk::Image {
                set_icon_name: Some("go-next-symbolic"),
                add_css_class: "dim-label",
                add_css_class: "nav-chevron",
                add_css_class: "row-chevron",
            },
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let icon_name = get_package_icon(&init.package.name, init.package.source);
        Self {
            package: init.package,
            is_favorite: init.is_favorite,
            is_selected: false,
            selection_mode: init.selection_mode,
            show_icons: init.show_icons,
            is_loading: false,
            icon_name,
            compact: init.compact,
            is_scheduled: init.is_scheduled,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            PackageRowInput::SetFavorite(is_favorite) => {
                self.is_favorite = is_favorite;
            }
            PackageRowInput::SetSelected(selected) => {
                self.is_selected = selected;
            }
            PackageRowInput::SetSelectionMode(mode) => {
                self.selection_mode = mode;
            }
            PackageRowInput::SetLoading(loading) => {
                self.is_loading = loading;
            }
            PackageRowInput::SetScheduled(scheduled) => {
                self.is_scheduled = scheduled;
            }
            PackageRowInput::UpdatePackage(pkg) => {
                self.package = *pkg;
            }
        }
    }
}
