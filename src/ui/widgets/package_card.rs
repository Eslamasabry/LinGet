use crate::models::{fetch_enrichment, get_package_icon, Package, PackageStatus};
use crate::ui::widgets::{PackageRowInit, PackageRowInput, PackageRowOutput};

use gtk4::prelude::*;
use gtk4::{self as gtk, glib, pango};
use relm4::prelude::*;

#[derive(Debug)]
pub enum PackageCardCmd {
    IconBytesLoaded(Vec<u8>),
    EnrichmentLoaded(Option<String>),
}

pub struct PackageCardModel {
    pub package: Package,
    pub is_favorite: bool,
    pub is_selected: bool,
    pub selection_mode: bool,
    pub show_icons: bool,
    pub is_loading: bool,
    pub icon_name: String,
    pub compact: bool,
    pub high_res_icon: Option<gtk::gdk::Texture>,
    pub is_scheduled: bool,
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

fn get_status_badge_text(status: PackageStatus) -> Option<&'static str> {
    match status {
        PackageStatus::UpdateAvailable => Some("Update"),
        PackageStatus::Installed => Some("Installed"),
        _ => None,
    }
}

fn get_status_badge_class(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::UpdateAvailable => "status-badge-update",
        PackageStatus::Installed => "status-badge-installed",
        _ => "status-badge",
    }
}

impl PackageCardModel {
    fn get_css_classes(&self) -> Vec<String> {
        let mut classes = vec!["pkg-card".to_string()];
        if self.is_selected {
            classes.push("selected".to_string());
        }
        if self.is_loading {
            classes.push("loading".to_string());
        }
        if self.compact {
            classes.push("compact-card".to_string());
        }
        match self.package.status {
            PackageStatus::UpdateAvailable => classes.push("status-update".to_string()),
            PackageStatus::Installed => classes.push("status-installed".to_string()),
            _ => {}
        }
        classes
    }

    fn card_dimensions(&self) -> (i32, i32) {
        if self.compact {
            (180, 220)
        } else {
            (260, 300)
        }
    }

    fn icon_size(&self) -> i32 {
        if self.compact {
            48
        } else {
            72
        }
    }

    fn icon_frame_size(&self) -> i32 {
        if self.compact {
            64
        } else {
            96
        }
    }

    fn truncated_description(&self) -> String {
        let desc = &self.package.description;
        if desc.is_empty() {
            return String::new();
        }
        let chars: Vec<char> = desc.chars().collect();
        if chars.len() > 120 {
            format!("{}…", chars[..120].iter().collect::<String>())
        } else {
            desc.clone()
        }
    }

    fn version_display(&self) -> String {
        let v = &self.package.version;
        let chars: Vec<char> = v.chars().collect();
        if chars.len() > 16 {
            format!("{}…", chars[..14].iter().collect::<String>())
        } else {
            v.clone()
        }
    }

    fn has_high_res(&self) -> bool {
        self.high_res_icon.is_some()
    }

    fn get_high_res_paintable(&self) -> Option<gtk::gdk::Paintable> {
        self.high_res_icon
            .as_ref()
            .map(|t| t.clone().upcast::<gtk::gdk::Paintable>())
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for PackageCardModel {
    type Init = PackageRowInit;
    type Input = PackageRowInput;
    type Output = PackageRowOutput;
    type CommandOutput = PackageCardCmd;
    type ParentWidget = gtk::FlowBox;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 0,
            set_focusable: true,
            set_cursor_from_name: Some("pointer"),
            #[watch]
            set_width_request: self.card_dimensions().0,
            #[watch]
            set_height_request: self.card_dimensions().1,
            #[watch]
            set_css_classes: &self.get_css_classes().iter().map(|s| s.as_str()).collect::<Vec<_>>(),

            gtk::Overlay {
                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,
                    #[watch]
                    set_margin_all: if self.compact { 6 } else { 8 },

                    gtk::Label {
                        #[watch]
                        set_label: get_status_badge_text(self.package.status).unwrap_or(""),
                        #[watch]
                        set_visible: get_status_badge_text(self.package.status).is_some(),
                        #[watch]
                        set_css_classes: &["status-badge", get_status_badge_class(self.package.status)],
                    },

                    gtk::Image {
                        set_icon_name: Some("alarm-symbolic"),
                        set_pixel_size: 14,
                        set_tooltip_text: Some("Scheduled for update"),
                        add_css_class: "scheduled-indicator",
                        add_css_class: "accent",
                        #[watch]
                        set_visible: self.is_scheduled,
                    },
                },

                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::End,
                    set_valign: gtk::Align::Start,
                    #[watch]
                    set_margin_all: if self.compact { 6 } else { 8 },
                    set_spacing: 4,

                    gtk::CheckButton {
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: self.selection_mode,
                        #[watch]
                        set_active: self.is_selected,
                        connect_toggled[sender, pkg = self.package.clone()] => move |btn| {
                            sender.output(PackageRowOutput::SelectionChanged(pkg.clone(), btn.is_active())).ok();
                        },
                    },

                    gtk::ToggleButton {
                        #[watch]
                        set_icon_name: if self.is_favorite { "starred-symbolic" } else { "non-starred-symbolic" },
                        add_css_class: "flat",
                        add_css_class: "circular",
                        add_css_class: "favorite-btn-small",
                        #[watch]
                        set_active: self.is_favorite,
                        connect_toggled[sender, pkg = self.package.clone()] => move |_| {
                            sender.output(PackageRowOutput::FavoriteToggled(pkg.clone())).ok();
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    #[watch]
                    set_spacing: if self.compact { 6 } else { 10 },
                    #[watch]
                    set_margin_top: if self.compact { 28 } else { 32 },
                    #[watch]
                    set_margin_bottom: if self.compact { 8 } else { 12 },
                    #[watch]
                    set_margin_start: if self.compact { 10 } else { 14 },
                    #[watch]
                    set_margin_end: if self.compact { 10 } else { 14 },

                    gtk::Overlay {
                        set_halign: gtk::Align::Center,
                        #[watch]
                        set_visible: self.show_icons,

                        gtk::Frame {
                            add_css_class: "card-icon-frame",
                            #[watch]
                            set_width_request: self.icon_frame_size(),
                            #[watch]
                            set_height_request: self.icon_frame_size(),

                            gtk::Image {
                                set_icon_name: Some(&self.icon_name),
                                #[watch]
                                set_pixel_size: self.icon_size(),
                                set_halign: gtk::Align::Center,
                                set_valign: gtk::Align::Center,
                                #[watch]
                                set_visible: !self.has_high_res(),
                            },
                        },

                        add_overlay = &gtk::Picture {
                            set_can_shrink: true,
                            #[watch]
                            set_width_request: self.icon_size(),
                            #[watch]
                            set_height_request: self.icon_size(),
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_visible: self.has_high_res(),
                            #[watch]
                            set_paintable: self.get_high_res_paintable().as_ref(),
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 4,
                        set_halign: gtk::Align::Center,
                        set_hexpand: true,

                        gtk::Label {
                            set_label: &self.package.name,
                            add_css_class: "card-title",
                            set_ellipsize: pango::EllipsizeMode::End,
                            #[watch]
                            set_max_width_chars: if self.compact { 16 } else { 28 },
                            set_halign: gtk::Align::Center,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_halign: gtk::Align::Center,
                            set_spacing: 6,

                            gtk::Label {
                                #[watch]
                                set_label: &self.version_display(),
                                add_css_class: "chip",
                                add_css_class: "chip-muted",
                                #[watch]
                                set_visible: !self.compact,
                            },

                            gtk::Label {
                                set_label: &self.package.source.to_string(),
                                add_css_class: "chip",
                                add_css_class: self.package.source.color_class(),
                            },
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &self.truncated_description(),
                            add_css_class: "card-description",
                            add_css_class: "dim-label",
                            set_wrap: true,
                            set_wrap_mode: pango::WrapMode::Word,
                            set_lines: 3,
                            set_ellipsize: pango::EllipsizeMode::End,
                            set_xalign: 0.5,
                            set_justify: gtk::Justification::Center,
                            #[watch]
                            set_visible: !self.compact && !self.package.description.is_empty(),
                            set_max_width_chars: 32,
                        },
                    },
                },

                add_overlay = &gtk::Box {
                    set_valign: gtk::Align::End,
                    set_halign: gtk::Align::End,
                    #[watch]
                    set_margin_all: if self.compact { 6 } else { 10 },

                    gtk::Button {
                        #[watch]
                        set_icon_name: get_action_icon(self.package.status),
                        #[watch]
                        set_tooltip_text: Some(get_action_tooltip(self.package.status)),
                        add_css_class: "circular",
                        add_css_class: "card-action-btn",
                        #[watch]
                        set_css_classes: if is_action_suggested(self.package.status) {
                            &["circular", "card-action-btn", "suggested-action"]
                        } else {
                            &["circular", "card-action-btn"]
                        },
                        #[watch]
                        set_sensitive: !self.is_loading,
                        connect_clicked[sender, pkg = self.package.clone()] => move |_| {
                            sender.output(PackageRowOutput::ActionClicked(pkg.clone())).ok();
                        },
                    }
                }
            },

            add_controller = gtk::GestureClick {
                connect_released[sender, pkg = self.package.clone()] => move |_, _, _, _| {
                    sender.output(PackageRowOutput::Clicked(pkg.clone())).ok();
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let icon_name = get_package_icon(&init.package.name, init.package.source);

        let pkg = init.package.clone();
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    let enrichment = fetch_enrichment(&pkg).await;
                    let icon_url = enrichment.and_then(|e| e.icon_url);
                    out.emit(PackageCardCmd::EnrichmentLoaded(icon_url));
                })
                .drop_on_shutdown()
        });

        Self {
            package: init.package,
            is_favorite: init.is_favorite,
            is_selected: false,
            selection_mode: init.selection_mode,
            show_icons: init.show_icons,
            is_loading: false,
            icon_name,
            compact: init.compact,
            high_res_icon: None,
            is_scheduled: init.is_scheduled,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            PackageRowInput::SetFavorite(fav) => self.is_favorite = fav,
            PackageRowInput::SetSelected(sel) => self.is_selected = sel,
            PackageRowInput::SetSelectionMode(mode) => self.selection_mode = mode,
            PackageRowInput::SetLoading(loading) => self.is_loading = loading,
            PackageRowInput::SetScheduled(scheduled) => self.is_scheduled = scheduled,
            PackageRowInput::UpdatePackage(pkg) => self.package = *pkg,
        }
    }

    fn update_cmd(&mut self, msg: Self::CommandOutput, sender: FactorySender<Self>) {
        match msg {
            PackageCardCmd::EnrichmentLoaded(icon_url) => {
                if let Some(url) = icon_url {
                    sender.command(move |out, shutdown| {
                        shutdown
                            .register(async move {
                                if let Ok(resp) = reqwest::get(&url).await {
                                    if let Ok(bytes) = resp.bytes().await {
                                        out.emit(PackageCardCmd::IconBytesLoaded(bytes.to_vec()));
                                    }
                                }
                            })
                            .drop_on_shutdown()
                    });
                }
            }
            PackageCardCmd::IconBytesLoaded(bytes) => {
                let gbytes = glib::Bytes::from_owned(bytes);
                if let Ok(texture) = gtk::gdk::Texture::from_bytes(&gbytes) {
                    self.high_res_icon = Some(texture);
                }
            }
        }
    }
}
