use crate::models::alias::{AliasViewData, Shell, ShellAlias};
use crate::models::PackageSource;

use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum AliasViewAction {
    Refresh,
    Create {
        name: String,
        command: String,
        shells: HashSet<Shell>,
        description: Option<String>,
    },
    Delete(String),
    SearchChanged(String),
    ToggleShowExisting,
    FilterByShell(Option<Shell>),
    ExpandPackage {
        name: String,
        source: PackageSource,
    },
    CopyCommand(String),
}

pub struct AliasWidgets {
    pub container: gtk::Box,
    pub search_entry: gtk::SearchEntry,
    pub show_existing_btn: gtk::ToggleButton,
    pub shell_filter: gtk::DropDown,
    pub header_spinner: gtk::Spinner,
    pub scrolled: gtk::ScrolledWindow,
    pub spinner: gtk::Spinner,
    pub content_box: gtk::Box,
}

pub fn init_alias_view<F>(on_action: F) -> AliasWidgets
where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(24)
        .margin_end(24)
        .build();

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search aliases or commands...")
        .hexpand(true)
        .build();

    let on_action_search = on_action.clone();
    search_entry.connect_search_changed(move |entry| {
        on_action_search(AliasViewAction::SearchChanged(entry.text().to_string()));
    });

    header.append(&search_entry);

    let show_existing_btn = gtk::ToggleButton::builder()
        .icon_name("view-list-symbolic")
        .tooltip_text("Show all aliases (including existing)")
        .build();

    let on_action_toggle = on_action.clone();
    show_existing_btn.connect_toggled(move |_| {
        on_action_toggle(AliasViewAction::ToggleShowExisting);
    });

    header.append(&show_existing_btn);

    let shell_filter = gtk::DropDown::from_strings(&["All Shells", "Bash", "Zsh", "Fish"]);

    let on_action_filter = on_action.clone();
    shell_filter.connect_selected_notify(move |dropdown| {
        let shell = match dropdown.selected() {
            1 => Some(Shell::Bash),
            2 => Some(Shell::Zsh),
            3 => Some(Shell::Fish),
            _ => None,
        };
        on_action_filter(AliasViewAction::FilterByShell(shell));
    });

    header.append(&shell_filter);

    let add_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Create new alias")
        .build();
    add_btn.add_css_class("suggested-action");

    let on_action_add = on_action.clone();
    add_btn.connect_clicked(move |btn| {
        show_add_alias_dialog(btn, on_action_add.clone());
    });

    header.append(&add_btn);

    let header_spinner = gtk::Spinner::builder()
        .spinning(true)
        .visible(false)
        .build();
    header_spinner.add_css_class("dim-label");
    header.append(&header_spinner);

    let refresh_btn = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh")
        .build();
    refresh_btn.add_css_class("flat");

    refresh_btn.connect_clicked(move |_| {
        on_action(AliasViewAction::Refresh);
    });

    header.append(&refresh_btn);

    container.append(&header);

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    let spinner = gtk::Spinner::builder()
        .spinning(true)
        .width_request(32)
        .height_request(32)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .margin_top(48)
        .build();

    let content_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(8)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    container.append(&scrolled);

    AliasWidgets {
        container,
        search_entry,
        show_existing_btn,
        shell_filter,
        header_spinner,
        scrolled,
        spinner,
        content_box,
    }
}

pub fn update_alias_view<F>(widgets: &AliasWidgets, data: &AliasViewData, on_action: F)
where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    if widgets.search_entry.text() != data.search_query {
        widgets.search_entry.set_text(&data.search_query);
    }

    if widgets.show_existing_btn.is_active() != data.show_existing {
        widgets.show_existing_btn.set_active(data.show_existing);
    }

    let target_selected = match data.filter_shell {
        None => 0,
        Some(Shell::Bash) => 1,
        Some(Shell::Zsh) => 2,
        Some(Shell::Fish) => 3,
    };
    if widgets.shell_filter.selected() != target_selected {
        widgets.shell_filter.set_selected(target_selected);
    }

    let any_loading = data.is_loading || data.manager.lazy_packages.iter().any(|p| p.loading);
    widgets.header_spinner.set_visible(any_loading);
    widgets.header_spinner.set_spinning(any_loading);

    if data.is_loading {
        widgets.scrolled.set_child(Some(&widgets.spinner));
    } else {
        while let Some(child) = widgets.content_box.first_child() {
            widgets.content_box.remove(&child);
        }

        build_content_into(&widgets.content_box, data, on_action);
        widgets.scrolled.set_child(Some(&widgets.content_box));
    }
}

fn build_content_into<F>(container: &gtk::Box, data: &AliasViewData, on_action: F)
where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    let detected_shells = &data.manager.detected_shells;
    if !detected_shells.is_empty() {
        let shells_label = gtk::Label::builder()
            .label(format!(
                "Detected shells: {}",
                detected_shells
                    .iter()
                    .map(|s| s.display_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .xalign(0.0)
            .build();
        shells_label.add_css_class("dim-label");
        shells_label.add_css_class("caption");
        container.append(&shells_label);

        if let Some(default) = data.manager.default_shell {
            let default_label = gtk::Label::builder()
                .label(format!("Default shell: {}", default.display_name()))
                .xalign(0.0)
                .build();
            default_label.add_css_class("dim-label");
            default_label.add_css_class("caption");
            container.append(&default_label);
        }
    }

    let aliases = data.filtered_aliases();

    if aliases.is_empty() && data.manager.managed_aliases.is_empty() && !data.show_existing {
        let empty = build_empty_state();
        container.append(&empty);
    } else if aliases.is_empty() {
        let no_results = gtk::Label::builder()
            .label("No aliases match your search")
            .margin_top(24)
            .build();
        no_results.add_css_class("dim-label");
        container.append(&no_results);
    } else {
        let managed_group = adw::PreferencesGroup::builder()
            .title(if data.show_existing {
                "All Aliases"
            } else {
                "LinGet-Managed Aliases"
            })
            .build();

        for alias in aliases {
            let row = build_alias_row(alias, on_action.clone());
            managed_group.add(&row);
        }

        container.append(managed_group.upcast_ref::<gtk::Widget>());
    }

    let lazy_packages = data.filtered_lazy_packages();
    if !lazy_packages.is_empty() {
        let pkg_group = adw::PreferencesGroup::builder()
            .title("Installed Packages")
            .description("Expand to discover available commands")
            .build();

        for lazy_pkg in lazy_packages.iter().take(50) {
            let loaded_commands = data.get_package_commands_for(&lazy_pkg.name, lazy_pkg.source);
            let cmd_count = loaded_commands.map(|p| p.commands.len()).unwrap_or(0);

            let subtitle = if lazy_pkg.loading {
                format!("{} • Loading...", lazy_pkg.source)
            } else if lazy_pkg.loaded {
                format!("{} • {} command(s)", lazy_pkg.source, cmd_count)
            } else {
                format!("{} • Click to discover commands", lazy_pkg.source)
            };

            let expander = adw::ExpanderRow::builder()
                .title(&lazy_pkg.name)
                .subtitle(&subtitle)
                .build();

            let source_icon = get_source_icon(lazy_pkg.source);
            let icon = gtk::Image::builder().icon_name(source_icon).build();
            icon.add_css_class("dim-label");
            expander.add_prefix(&icon);

            if lazy_pkg.loading {
                let spinner = gtk::Spinner::builder()
                    .spinning(true)
                    .valign(gtk::Align::Center)
                    .build();
                expander.add_suffix(&spinner);
            }

            if let Some(pkg_cmds) = loaded_commands {
                for cmd in &pkg_cmds.commands {
                    let cmd_row = build_command_row(cmd, on_action.clone());
                    expander.add_row(&cmd_row);
                }
            }

            if !lazy_pkg.loaded && !lazy_pkg.loading {
                let pkg_name = lazy_pkg.name.clone();
                let pkg_source = lazy_pkg.source;
                let on_action_expand = on_action.clone();
                expander.connect_expanded_notify(move |exp| {
                    tracing::info!(
                        package = %pkg_name,
                        source = ?pkg_source,
                        is_expanded = exp.is_expanded(),
                        "Expander notify fired"
                    );
                    if exp.is_expanded() {
                        on_action_expand(AliasViewAction::ExpandPackage {
                            name: pkg_name.clone(),
                            source: pkg_source,
                        });
                    }
                });
            }

            pkg_group.add(&expander);
        }

        container.append(pkg_group.upcast_ref::<gtk::Widget>());
    }

    let commands = data.filtered_commands();
    if !commands.is_empty() && data.search_query.len() >= 2 && lazy_packages.is_empty() {
        let commands_group = adw::PreferencesGroup::builder()
            .title("Available Commands")
            .description("Commands from PATH that you can create aliases for")
            .build();

        for cmd in commands.iter().take(20) {
            let row = adw::ActionRow::builder()
                .title(*cmd)
                .activatable(true)
                .build();

            let create_btn = gtk::Button::builder()
                .label("Create Alias")
                .valign(gtk::Align::Center)
                .build();
            create_btn.add_css_class("pill");

            let cmd_clone = (*cmd).clone();
            let on_action_clone = on_action.clone();
            create_btn.connect_clicked(move |btn| {
                show_add_alias_dialog_with_command(btn, &cmd_clone, on_action_clone.clone());
            });

            row.add_suffix(&create_btn);
            commands_group.add(&row);
        }

        container.append(commands_group.upcast_ref::<gtk::Widget>());
    }
}

fn build_empty_state() -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .margin_top(48)
        .margin_bottom(48)
        .build();

    let icon = gtk::Image::builder()
        .icon_name("utilities-terminal-symbolic")
        .pixel_size(64)
        .build();
    icon.add_css_class("dim-label");

    let title = gtk::Label::builder().label("No Aliases Yet").build();
    title.add_css_class("title-2");

    let subtitle = gtk::Label::builder()
        .label("Create custom shortcuts for your favorite commands.\nAliases work across all your terminal sessions.")
        .wrap(true)
        .max_width_chars(50)
        .justify(gtk::Justification::Center)
        .build();
    subtitle.add_css_class("dim-label");

    container.append(&icon);
    container.append(&title);
    container.append(&subtitle);

    container
}

fn build_alias_row<F>(alias: &ShellAlias, on_action: F) -> adw::ActionRow
where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    let subtitle = format!("{} • {}", alias.command, alias.shells_display());

    let row = adw::ActionRow::builder()
        .title(&alias.name)
        .subtitle(&subtitle)
        .build();

    let icon = gtk::Image::builder()
        .icon_name(if alias.managed_by_linget {
            "emblem-default-symbolic"
        } else {
            "document-open-symbolic"
        })
        .build();

    if alias.managed_by_linget {
        icon.add_css_class("success");
    } else {
        icon.add_css_class("dim-label");
    }
    row.add_prefix(&icon);

    if alias.conflicts_with_command() {
        let warning = gtk::Image::builder()
            .icon_name("dialog-warning-symbolic")
            .tooltip_text("This alias shadows an existing command")
            .build();
        warning.add_css_class("warning");
        row.add_suffix(&warning);
    }

    if alias.managed_by_linget {
        let delete_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text("Delete alias")
            .build();
        delete_btn.add_css_class("flat");
        delete_btn.add_css_class("circular");

        let alias_name = alias.name.clone();
        delete_btn.connect_clicked(move |_| {
            on_action(AliasViewAction::Delete(alias_name.clone()));
        });

        row.add_suffix(&delete_btn);
    } else if let Some(source) = &alias.source_file {
        let source_label = gtk::Label::builder()
            .label(
                source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .as_ref(),
            )
            .tooltip_text(source.to_string_lossy().as_ref())
            .build();
        source_label.add_css_class("caption");
        source_label.add_css_class("dim-label");
        row.add_suffix(&source_label);
    }

    row
}

fn get_source_icon(source: PackageSource) -> &'static str {
    match source {
        PackageSource::Apt | PackageSource::Dnf => "package-x-generic-symbolic",
        PackageSource::Flatpak => "system-software-install-symbolic",
        PackageSource::Snap => "snap-symbolic",
        PackageSource::Pip | PackageSource::Pipx => "python-symbolic",
        PackageSource::Npm => "javascript-symbolic",
        PackageSource::Cargo => "rust-symbolic",
        _ => "application-x-executable-symbolic",
    }
}

fn build_command_row<F>(cmd: &crate::models::alias::CommandInfo, on_action: F) -> gtk::Widget
where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    if cmd.subcommands.is_empty() {
        let cmd_row = adw::ActionRow::builder()
            .title(&cmd.name)
            .subtitle(cmd.path.to_string_lossy())
            .build();

        let copy_btn = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy command")
            .valign(gtk::Align::Center)
            .build();
        copy_btn.add_css_class("flat");
        copy_btn.add_css_class("circular");

        let cmd_name = cmd.name.clone();
        let on_action_copy = on_action.clone();
        copy_btn.connect_clicked(move |_| {
            on_action_copy(AliasViewAction::CopyCommand(cmd_name.clone()));
        });

        let create_btn = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("Create alias for this command")
            .valign(gtk::Align::Center)
            .build();
        create_btn.add_css_class("flat");
        create_btn.add_css_class("circular");

        let cmd_name = cmd.name.clone();
        let on_action_create = on_action.clone();
        create_btn.connect_clicked(move |btn| {
            show_add_alias_dialog_with_command(btn, &cmd_name, on_action_create.clone());
        });

        cmd_row.add_suffix(&copy_btn);
        cmd_row.add_suffix(&create_btn);
        return cmd_row.upcast();
    }

    let expander = adw::ExpanderRow::builder()
        .title(&cmd.name)
        .subtitle(format!("{} • {} subcommands", cmd.path.to_string_lossy(), cmd.subcommands.len()))
        .build();

    let copy_btn = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy command")
        .valign(gtk::Align::Center)
        .build();
    copy_btn.add_css_class("flat");
    copy_btn.add_css_class("circular");

    let cmd_name = cmd.name.clone();
    let on_action_copy = on_action.clone();
    copy_btn.connect_clicked(move |_| {
        on_action_copy(AliasViewAction::CopyCommand(cmd_name.clone()));
    });

    let create_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Create alias for this command")
        .valign(gtk::Align::Center)
        .build();
    create_btn.add_css_class("flat");
    create_btn.add_css_class("circular");

    let cmd_name = cmd.name.clone();
    let on_action_create = on_action.clone();
    create_btn.connect_clicked(move |btn| {
        show_add_alias_dialog_with_command(btn, &cmd_name, on_action_create.clone());
    });

    expander.add_suffix(&copy_btn);
    expander.add_suffix(&create_btn);

    for subcmd in &cmd.subcommands {
        let subcmd_row = build_subcommand_row(subcmd, on_action.clone());
        expander.add_row(&subcmd_row);
    }

    expander.upcast()
}

fn build_subcommand_row<F>(subcmd: &crate::models::alias::SubcommandInfo, on_action: F) -> adw::ActionRow
where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    let description = subcmd.description.as_deref().unwrap_or("");
    let row = adw::ActionRow::builder()
        .title(&subcmd.full_command)
        .subtitle(description)
        .build();

    let icon = gtk::Image::builder()
        .icon_name("go-next-symbolic")
        .build();
    icon.add_css_class("dim-label");
    row.add_prefix(&icon);

    let copy_btn = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy command")
        .valign(gtk::Align::Center)
        .build();
    copy_btn.add_css_class("flat");
    copy_btn.add_css_class("circular");

    let full_cmd = subcmd.full_command.clone();
    let on_action_copy = on_action.clone();
    copy_btn.connect_clicked(move |_| {
        on_action_copy(AliasViewAction::CopyCommand(full_cmd.clone()));
    });

    let create_btn = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("Create alias for this command")
        .valign(gtk::Align::Center)
        .build();
    create_btn.add_css_class("flat");
    create_btn.add_css_class("circular");

    let full_cmd = subcmd.full_command.clone();
    let on_action_create = on_action.clone();
    create_btn.connect_clicked(move |btn| {
        show_add_alias_dialog_with_command(btn, &full_cmd, on_action_create.clone());
    });

    row.add_suffix(&copy_btn);
    row.add_suffix(&create_btn);
    row
}

fn show_add_alias_dialog<F>(parent: &impl IsA<gtk::Widget>, on_action: F)
where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    show_add_alias_dialog_with_command(parent, "", on_action);
}

fn show_add_alias_dialog_with_command<F>(
    parent: &impl IsA<gtk::Widget>,
    preset_command: &str,
    on_action: F,
) where
    F: Fn(AliasViewAction) + Clone + 'static,
{
    let dialog = adw::MessageDialog::builder()
        .heading("Create Alias")
        .body("Create a custom shortcut for a command")
        .build();

    if let Some(root) = parent.root() {
        if let Some(window) = root.downcast_ref::<gtk::Window>() {
            dialog.set_transient_for(Some(window));
        }
    }

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    let name_row = adw::EntryRow::builder().title("Alias name").build();

    let conflict_banner = adw::Banner::builder()
        .title("This name shadows an existing command")
        .revealed(false)
        .build();
    conflict_banner.add_css_class("warning");

    let name_row_clone = name_row.clone();
    let conflict_banner_clone = conflict_banner.clone();
    name_row.connect_changed(move |_| {
        let name = name_row_clone.text().to_string();
        let conflicts = !name.is_empty() && which::which(&name).is_ok();
        conflict_banner_clone.set_revealed(conflicts);
    });

    let command_row = adw::EntryRow::builder()
        .title("Command")
        .text(preset_command)
        .build();

    let desc_row = adw::EntryRow::builder()
        .title("Description (optional)")
        .build();

    let shells_group = adw::PreferencesGroup::builder()
        .title("Target Shells")
        .build();

    let bash_check = adw::SwitchRow::builder().title("Bash").active(true).build();

    let zsh_check = adw::SwitchRow::builder().title("Zsh").active(true).build();

    let fish_check = adw::SwitchRow::builder().title("Fish").active(true).build();

    shells_group.add(&bash_check);
    shells_group.add(&zsh_check);
    shells_group.add(&fish_check);

    content.append(&conflict_banner);
    content.append(&name_row);
    content.append(&command_row);
    content.append(&desc_row);
    content.append(shells_group.upcast_ref::<gtk::Widget>());

    dialog.set_extra_child(Some(&content));
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("create", "Create");
    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);

    dialog.connect_response(None, {
        let name_row = name_row.clone();
        let command_row = command_row.clone();
        let desc_row = desc_row.clone();
        move |_, response| {
            if response == "create" {
                let name = name_row.text().to_string();
                let command = command_row.text().to_string();
                let description = {
                    let text = desc_row.text().to_string();
                    if text.is_empty() {
                        None
                    } else {
                        Some(text)
                    }
                };

                if !name.is_empty() && !command.is_empty() {
                    let mut shells = HashSet::new();
                    if bash_check.is_active() {
                        shells.insert(Shell::Bash);
                    }
                    if zsh_check.is_active() {
                        shells.insert(Shell::Zsh);
                    }
                    if fish_check.is_active() {
                        shells.insert(Shell::Fish);
                    }

                    if !shells.is_empty() {
                        on_action(AliasViewAction::Create {
                            name,
                            command,
                            shells,
                            description,
                        });
                    }
                }
            }
        }
    });

    dialog.present();
}
