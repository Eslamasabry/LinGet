#![allow(dead_code)]

use crate::models::{HistoryEntry, HistoryFilter, HistoryOperation, OperationHistory};
use chrono::{Local, NaiveDate};
use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HistoryViewData {
    pub entries: Vec<HistoryEntry>,
    pub filter: HistoryFilter,
    pub search_query: String,
    pub is_loading: bool,
}

impl Default for HistoryViewData {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            filter: HistoryFilter::All,
            search_query: String::new(),
            is_loading: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum HistoryViewAction {
    Undo(String),
    Export,
    Refresh,
    FilterChanged(HistoryFilter),
    Search(String),
}

pub fn build_history_view<F>(data: &HistoryViewData, on_action: F) -> gtk::Box
where
    F: Fn(HistoryViewAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(0)
        .build();

    let header = build_header(&data.filter, &data.search_query, on_action.clone());
    container.append(&header);

    let scrolled = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();

    if data.is_loading {
        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .width_request(32)
            .height_request(32)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .margin_top(48)
            .build();
        scrolled.set_child(Some(&spinner));
    } else if data.entries.is_empty() {
        let empty = build_empty_state();
        scrolled.set_child(Some(&empty));
    } else {
        let timeline = build_timeline(&data.entries, on_action);
        scrolled.set_child(Some(&timeline));
    }

    container.append(&scrolled);
    container
}

fn build_header<F>(current_filter: &HistoryFilter, search_query: &str, on_action: F) -> gtk::Box
where
    F: Fn(HistoryViewAction) + Clone + 'static,
{
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(24)
        .margin_end(24)
        .build();

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search history...")
        .hexpand(true)
        .text(search_query)
        .build();

    let on_action_search = on_action.clone();
    search_entry.connect_search_changed(move |entry| {
        on_action_search(HistoryViewAction::Search(entry.text().to_string()));
    });

    header.append(&search_entry);

    let filter_dropdown = gtk::DropDown::from_strings(&[
        HistoryFilter::All.label(),
        HistoryFilter::Installs.label(),
        HistoryFilter::Removes.label(),
        HistoryFilter::Updates.label(),
        HistoryFilter::Today.label(),
        HistoryFilter::ThisWeek.label(),
    ]);

    let selected_idx = match current_filter {
        HistoryFilter::All => 0,
        HistoryFilter::Installs => 1,
        HistoryFilter::Removes => 2,
        HistoryFilter::Updates => 3,
        HistoryFilter::Today => 4,
        HistoryFilter::ThisWeek => 5,
    };
    filter_dropdown.set_selected(selected_idx);

    let on_action_filter = on_action.clone();
    filter_dropdown.connect_selected_notify(move |dropdown| {
        let filter = match dropdown.selected() {
            0 => HistoryFilter::All,
            1 => HistoryFilter::Installs,
            2 => HistoryFilter::Removes,
            3 => HistoryFilter::Updates,
            4 => HistoryFilter::Today,
            5 => HistoryFilter::ThisWeek,
            _ => HistoryFilter::All,
        };
        on_action_filter(HistoryViewAction::FilterChanged(filter));
    });

    header.append(&filter_dropdown);

    let export_btn = gtk::Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Export history")
        .build();
    export_btn.add_css_class("flat");

    let on_action_export = on_action.clone();
    export_btn.connect_clicked(move |_| {
        on_action_export(HistoryViewAction::Export);
    });

    header.append(&export_btn);

    let refresh_btn = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Refresh")
        .build();
    refresh_btn.add_css_class("flat");

    refresh_btn.connect_clicked(move |_| {
        on_action(HistoryViewAction::Refresh);
    });

    header.append(&refresh_btn);

    header
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
        .icon_name("document-open-recent-symbolic")
        .pixel_size(64)
        .build();
    icon.add_css_class("dim-label");

    let title = gtk::Label::builder().label("No History Yet").build();
    title.add_css_class("title-2");

    let subtitle = gtk::Label::builder()
        .label("Package operations will appear here as you install, update, and remove packages.")
        .wrap(true)
        .max_width_chars(40)
        .justify(gtk::Justification::Center)
        .build();
    subtitle.add_css_class("dim-label");

    container.append(&icon);
    container.append(&title);
    container.append(&subtitle);

    container
}

fn build_timeline<F>(entries: &[HistoryEntry], on_action: F) -> gtk::Box
where
    F: Fn(HistoryViewAction) + Clone + 'static,
{
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_top(8)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();

    let grouped = group_by_date(entries);
    let mut dates: Vec<NaiveDate> = grouped.keys().copied().collect();
    dates.sort_by(|a, b| b.cmp(a));

    let today = Local::now().date_naive();

    for date in dates {
        let date_label = if date == today {
            "Today".to_string()
        } else if date == today.pred_opt().unwrap_or(today) {
            "Yesterday".to_string()
        } else {
            date.format("%A, %B %d").to_string()
        };

        let group = adw::PreferencesGroup::builder().title(&date_label).build();

        if let Some(day_entries) = grouped.get(&date) {
            for entry in day_entries {
                let row = build_history_row(entry, on_action.clone());
                group.add(&row);
            }
        }

        container.append(group.upcast_ref::<gtk::Widget>());
    }

    container
}

fn build_history_row<F>(entry: &HistoryEntry, on_action: F) -> adw::ActionRow
where
    F: Fn(HistoryViewAction) + Clone + 'static,
{
    let time_str = entry.timestamp.format("%H:%M").to_string();

    let subtitle = if let Some(version_display) = entry.version_display() {
        format!("{} • {}", time_str, version_display)
    } else {
        time_str
    };

    let row = adw::ActionRow::builder()
        .title(&entry.package_name)
        .subtitle(&subtitle)
        .build();

    let icon = gtk::Image::builder()
        .icon_name(entry.operation.icon())
        .build();

    let icon_class = match entry.operation {
        HistoryOperation::Install | HistoryOperation::ExternalInstall => "success",
        HistoryOperation::Remove | HistoryOperation::ExternalRemove => "error",
        HistoryOperation::Update | HistoryOperation::ExternalUpdate => "accent",
        HistoryOperation::Downgrade => "warning",
        HistoryOperation::Cleanup => "dim-label",
    };
    icon.add_css_class(icon_class);
    row.add_prefix(&icon);

    if entry.operation.is_external() {
        let external_badge = gtk::Label::builder()
            .label("CLI")
            .tooltip_text("Changed outside LinGet")
            .build();
        external_badge.add_css_class("caption");
        external_badge.add_css_class("dim-label");
        row.add_suffix(&external_badge);
    }

    let source_label = gtk::Label::builder()
        .label(entry.package_source.to_string())
        .build();
    source_label.add_css_class("caption");
    source_label.add_css_class("dim-label");
    row.add_suffix(&source_label);

    if entry.is_reversible() {
        let undo_btn = gtk::Button::builder()
            .label(entry.operation.undo_label())
            .valign(gtk::Align::Center)
            .build();
        undo_btn.add_css_class("pill");

        let entry_id = entry.id.clone();
        undo_btn.connect_clicked(move |_| {
            on_action(HistoryViewAction::Undo(entry_id.clone()));
        });

        row.add_suffix(&undo_btn);
    } else if entry.undone {
        let undone_label = gtk::Label::builder().label("Undone").build();
        undone_label.add_css_class("caption");
        undone_label.add_css_class("dim-label");
        row.add_suffix(&undone_label);
    }

    row
}

fn group_by_date(entries: &[HistoryEntry]) -> HashMap<NaiveDate, Vec<&HistoryEntry>> {
    let mut groups: HashMap<NaiveDate, Vec<&HistoryEntry>> = HashMap::new();

    for entry in entries {
        let date = entry.timestamp.date_naive();
        groups.entry(date).or_default().push(entry);
    }

    for entries in groups.values_mut() {
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    }

    groups
}

pub fn filter_entries(
    history: &OperationHistory,
    filter: HistoryFilter,
    search_query: &str,
) -> Vec<HistoryEntry> {
    let filtered: Vec<&HistoryEntry> = match filter {
        HistoryFilter::All => history.entries.iter().collect(),
        HistoryFilter::Installs => history
            .entries
            .iter()
            .filter(|e| {
                matches!(
                    e.operation,
                    HistoryOperation::Install | HistoryOperation::ExternalInstall
                )
            })
            .collect(),
        HistoryFilter::Removes => history
            .entries
            .iter()
            .filter(|e| {
                matches!(
                    e.operation,
                    HistoryOperation::Remove | HistoryOperation::ExternalRemove
                )
            })
            .collect(),
        HistoryFilter::Updates => history
            .entries
            .iter()
            .filter(|e| {
                matches!(
                    e.operation,
                    HistoryOperation::Update | HistoryOperation::ExternalUpdate
                )
            })
            .collect(),
        HistoryFilter::Today => history.today_entries(),
        HistoryFilter::ThisWeek => {
            let now = Local::now();
            let week_ago = now - chrono::Duration::days(7);
            history
                .entries
                .iter()
                .filter(|e| e.timestamp >= week_ago)
                .collect()
        }
    };

    if search_query.is_empty() {
        filtered.into_iter().cloned().collect()
    } else {
        let query_lower = search_query.to_lowercase();
        filtered
            .into_iter()
            .filter(|e| e.package_name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }
}
