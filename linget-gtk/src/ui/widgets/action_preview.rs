use crate::models::Package;

use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Install,
    Remove,
    Update,
    Cleanup,
}

impl ActionType {
    fn heading(&self, package_count: usize) -> String {
        match self {
            ActionType::Install => {
                if package_count == 1 {
                    "Install Package?".to_string()
                } else {
                    format!("Install {} Packages?", package_count)
                }
            }
            ActionType::Remove => {
                if package_count == 1 {
                    "Remove Package?".to_string()
                } else {
                    format!("Remove {} Packages?", package_count)
                }
            }
            ActionType::Update => {
                if package_count == 1 {
                    "Update Package?".to_string()
                } else {
                    format!("Update {} Packages?", package_count)
                }
            }
            ActionType::Cleanup => "Clean Up?".to_string(),
        }
    }

    fn confirm_label(&self) -> &'static str {
        match self {
            ActionType::Install => "Install",
            ActionType::Remove => "Remove",
            ActionType::Update => "Update",
            ActionType::Cleanup => "Clean",
        }
    }

    fn icon_name(&self) -> &'static str {
        match self {
            ActionType::Install => "list-add-symbolic",
            ActionType::Remove => "user-trash-symbolic",
            ActionType::Update => "software-update-available-symbolic",
            ActionType::Cleanup => "edit-clear-all-symbolic",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ActionPreview {
    pub action_type: ActionType,
    pub packages: Vec<Package>,
    pub disk_change: i64,
    pub current_disk_usage: Option<u64>,
    pub new_commands: Vec<String>,
    pub new_services: Vec<String>,
    pub reverse_dependencies: Vec<String>,
}

#[allow(dead_code)]
impl ActionPreview {
    pub fn new(action_type: ActionType) -> Self {
        Self {
            action_type,
            packages: Vec::new(),
            disk_change: 0,
            current_disk_usage: None,
            new_commands: Vec::new(),
            new_services: Vec::new(),
            reverse_dependencies: Vec::new(),
        }
    }

    pub fn with_reverse_dependencies(mut self, deps: Vec<String>) -> Self {
        self.reverse_dependencies = deps;
        self
    }

    pub fn has_breaking_changes(&self) -> bool {
        self.action_type == ActionType::Remove && !self.reverse_dependencies.is_empty()
    }

    pub fn add_package(&mut self, pkg: Package) {
        if let Some(size) = pkg.size {
            match self.action_type {
                ActionType::Install | ActionType::Update => {
                    self.disk_change += size as i64;
                }
                ActionType::Remove | ActionType::Cleanup => {
                    self.disk_change -= size as i64;
                }
            }
        }
        self.packages.push(pkg);
    }

    pub fn set_disk_change(&mut self, bytes: i64) {
        self.disk_change = bytes;
    }

    pub fn set_current_disk_usage(&mut self, bytes: u64) {
        self.current_disk_usage = Some(bytes);
    }

    pub fn add_command(&mut self, cmd: String) {
        self.new_commands.push(cmd);
    }

    pub fn add_service(&mut self, service: String) {
        self.new_services.push(service);
    }

    pub fn show_dialog<W: IsA<gtk::Widget>>(
        &self,
        parent: &W,
        on_confirm: impl Fn() + 'static,
        on_cancel: impl Fn() + 'static,
    ) {
        let dialog = adw::MessageDialog::builder()
            .heading(self.action_type.heading(self.packages.len()))
            .build();

        let content = build_preview_content(self);
        dialog.set_extra_child(Some(&content));

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("confirm", self.action_type.confirm_label());
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let appearance = match self.action_type {
            ActionType::Remove | ActionType::Cleanup => adw::ResponseAppearance::Destructive,
            ActionType::Install | ActionType::Update => adw::ResponseAppearance::Suggested,
        };
        dialog.set_response_appearance("confirm", appearance);

        if let Some(window) = parent.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
            dialog.set_transient_for(Some(&window));
        }

        dialog.connect_response(None, move |_, response| {
            if response == "confirm" {
                on_confirm();
            } else {
                on_cancel();
            }
        });

        dialog.present();
    }
}

#[allow(dead_code)]
fn build_preview_content(preview: &ActionPreview) -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(8)
        .build();

    if preview.has_breaking_changes() {
        let warning_box = build_dependency_warning(&preview.reverse_dependencies);
        container.append(&warning_box);
    }

    let packages_group = adw::PreferencesGroup::builder().title("Packages").build();

    let package_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(vec!["boxed-list".to_string()])
        .build();

    let max_display = 10;
    for (i, pkg) in preview.packages.iter().enumerate() {
        if i >= max_display {
            let remaining = preview.packages.len() - max_display;
            let more_row = adw::ActionRow::builder()
                .title(format!("... and {} more", remaining))
                .css_classes(vec!["dim-label".to_string()])
                .build();
            package_list.append(&more_row);
            break;
        }

        let row = adw::ActionRow::builder()
            .title(&pkg.name)
            .subtitle(pkg.source.to_string())
            .build();

        let icon = gtk::Image::builder()
            .icon_name(preview.action_type.icon_name())
            .build();
        icon.add_css_class("dim-label");
        row.add_prefix(&icon);

        if let Some(size) = pkg.size {
            let size_label = gtk::Label::builder()
                .label(humansize::format_size(size, humansize::BINARY))
                .css_classes(vec!["dim-label".to_string()])
                .build();
            row.add_suffix(&size_label);
        }

        if pkg.has_update() {
            if let Some(ref new_version) = pkg.available_version {
                let version_label = gtk::Label::builder()
                    .label(format!("{} → {}", pkg.version, new_version))
                    .css_classes(vec!["caption".to_string(), "dim-label".to_string()])
                    .build();
                row.add_suffix(&version_label);
            }
        }

        package_list.append(&row);
    }

    packages_group.add(&package_list);
    container.append(&packages_group);

    if preview.disk_change != 0 || preview.current_disk_usage.is_some() {
        let disk_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .build();

        let disk_icon = gtk::Image::builder()
            .icon_name("drive-harddisk-symbolic")
            .build();
        disk_icon.add_css_class("dim-label");
        disk_box.append(&disk_icon);

        let disk_text = if let Some(current) = preview.current_disk_usage {
            let new_usage = (current as i64 + preview.disk_change) as u64;
            let change_str = format_disk_change(preview.disk_change);
            format!(
                "{} → {} ({})",
                humansize::format_size(current, humansize::BINARY),
                humansize::format_size(new_usage, humansize::BINARY),
                change_str
            )
        } else {
            format_disk_change(preview.disk_change)
        };

        let disk_label = gtk::Label::builder().label(&disk_text).build();
        disk_label.add_css_class("dim-label");
        disk_box.append(&disk_label);

        container.append(&disk_box);
    }

    if !preview.new_commands.is_empty() {
        let commands_group = adw::PreferencesGroup::builder()
            .title("New Commands")
            .description("These commands will be available after installation")
            .build();

        let commands_box = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .homogeneous(false)
            .max_children_per_line(5)
            .row_spacing(6)
            .column_spacing(6)
            .build();

        for cmd in &preview.new_commands {
            let chip = gtk::Label::builder()
                .label(cmd)
                .css_classes(vec!["chip".to_string()])
                .build();
            commands_box.append(&chip);
        }

        commands_group.add(&commands_box);
        container.append(&commands_group);
    }

    if !preview.new_services.is_empty() {
        let services_group = adw::PreferencesGroup::builder()
            .title("New Services")
            .description("System services that will be added")
            .build();

        let services_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list".to_string()])
            .build();

        for service in &preview.new_services {
            let row = adw::ActionRow::builder().title(service).build();
            let icon = gtk::Image::builder()
                .icon_name("system-run-symbolic")
                .build();
            icon.add_css_class("dim-label");
            row.add_prefix(&icon);
            services_list.append(&row);
        }

        services_group.add(&services_list);
        container.append(&services_group);
    }

    container
}

#[allow(dead_code)]
fn format_disk_change(bytes: i64) -> String {
    let abs_bytes = bytes.unsigned_abs();
    let size_str = humansize::format_size(abs_bytes, humansize::BINARY);
    if bytes > 0 {
        format!("+{}", size_str)
    } else if bytes < 0 {
        format!("-{}", size_str)
    } else {
        "No change".to_string()
    }
}

fn build_dependency_warning(reverse_deps: &[String]) -> gtk::Box {
    let warning_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();
    warning_box.add_css_class("warning-banner");

    let header_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let warning_icon = gtk::Image::builder()
        .icon_name("dialog-warning-symbolic")
        .pixel_size(24)
        .build();
    warning_icon.add_css_class("warning-icon");
    header_box.append(&warning_icon);

    let count = reverse_deps.len();
    let header_label = gtk::Label::builder()
        .label(format!(
            "<b>Warning:</b> {} package{} depend{} on this",
            count,
            if count == 1 { "" } else { "s" },
            if count == 1 { "s" } else { "" }
        ))
        .use_markup(true)
        .xalign(0.0)
        .hexpand(true)
        .build();
    header_box.append(&header_label);

    warning_box.append(&header_box);

    let desc_label = gtk::Label::builder()
        .label("Removing this package may break the following:")
        .xalign(0.0)
        .wrap(true)
        .build();
    desc_label.add_css_class("dim-label");
    warning_box.append(&desc_label);

    let deps_flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .homogeneous(false)
        .max_children_per_line(4)
        .row_spacing(4)
        .column_spacing(4)
        .build();

    let max_show = 12;
    for (i, dep) in reverse_deps.iter().enumerate() {
        if i >= max_show {
            let more_label = gtk::Label::builder()
                .label(format!("...and {} more", reverse_deps.len() - max_show))
                .build();
            more_label.add_css_class("chip");
            more_label.add_css_class("chip-muted");
            deps_flow.append(&more_label);
            break;
        }

        let chip = gtk::Label::builder().label(dep).build();
        chip.add_css_class("chip");
        chip.add_css_class("chip-danger");
        deps_flow.append(&chip);
    }

    warning_box.append(&deps_flow);

    warning_box
}
