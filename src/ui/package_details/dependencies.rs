use crate::models::PackageInsights;
use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;

pub fn build_dependencies_section(
    forward_deps: &[String],
    insights: Option<&PackageInsights>,
) -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .build();

    let expander = gtk::Expander::builder()
        .label("Dependencies")
        .expanded(false)
        .css_classes(vec!["card"])
        .build();

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .spacing(12)
        .build();

    if !forward_deps.is_empty() {
        let forward_group = adw::PreferencesGroup::builder()
            .title("Requires")
            .description(format!("{} packages", forward_deps.len()))
            .build();

        let forward_list = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .homogeneous(false)
            .max_children_per_line(5)
            .min_children_per_line(2)
            .row_spacing(4)
            .column_spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        for (i, dep) in forward_deps.iter().enumerate() {
            if i >= 20 {
                let more = gtk::Label::builder()
                    .label(format!("...and {} more", forward_deps.len() - 20))
                    .css_classes(vec!["dim-label", "caption"])
                    .build();
                forward_list.append(&more);
                break;
            }

            let dep_name = dep.split_whitespace().next().unwrap_or(dep);
            let chip = gtk::Label::builder()
                .label(dep_name)
                .css_classes(vec!["chip", "chip-muted"])
                .build();
            forward_list.append(&chip);
        }

        forward_group.add(&forward_list);
        content.append(&forward_group);
    } else {
        let no_deps = adw::ActionRow::builder()
            .title("No dependencies")
            .subtitle("This package is standalone")
            .build();
        no_deps.add_prefix(
            &gtk::Image::builder()
                .icon_name("emblem-ok-symbolic")
                .css_classes(vec!["success"])
                .build(),
        );
        content.append(&no_deps);
    }

    if let Some(insights) = insights {
        let reverse = &insights.reverse_dependencies;
        if !reverse.is_empty() {
            let reverse_group = adw::PreferencesGroup::builder()
                .title("Required By")
                .description(format!("{} packages depend on this", reverse.len()))
                .build();

            let reverse_list = gtk::FlowBox::builder()
                .selection_mode(gtk::SelectionMode::None)
                .homogeneous(false)
                .max_children_per_line(5)
                .min_children_per_line(2)
                .row_spacing(4)
                .column_spacing(4)
                .margin_top(8)
                .margin_bottom(8)
                .build();

            for (i, dep) in reverse.iter().enumerate() {
                if i >= 20 {
                    let more = gtk::Label::builder()
                        .label(format!("...and {} more", reverse.len() - 20))
                        .css_classes(vec!["dim-label", "caption"])
                        .build();
                    reverse_list.append(&more);
                    break;
                }

                let chip = gtk::Label::builder()
                    .label(dep)
                    .css_classes(vec!["chip", "dependency-chip"])
                    .build();
                reverse_list.append(&chip);
            }

            reverse_group.add(&reverse_list);
            content.append(&reverse_group);

            if !insights.is_safe_to_remove {
                let warning_row = adw::ActionRow::builder()
                    .title("Removing may break other packages")
                    .css_classes(vec!["warning"])
                    .build();
                warning_row.add_prefix(
                    &gtk::Image::builder()
                        .icon_name("dialog-warning-symbolic")
                        .css_classes(vec!["warning"])
                        .build(),
                );
                content.append(&warning_row);
            }
        } else {
            let safe_row = adw::ActionRow::builder()
                .title("Not required by other packages")
                .subtitle("Safe to remove")
                .build();
            safe_row.add_prefix(
                &gtk::Image::builder()
                    .icon_name("emblem-ok-symbolic")
                    .css_classes(vec!["success"])
                    .build(),
            );
            content.append(&safe_row);
        }
    }

    let summary = build_summary_text(forward_deps.len(), insights);
    expander.set_label(Some(&summary));

    expander.set_child(Some(&content));
    container.append(&expander);

    container
}

fn build_summary_text(forward_count: usize, insights: Option<&PackageInsights>) -> String {
    let reverse_count = insights.map(|i| i.reverse_dependencies.len()).unwrap_or(0);

    let parts: Vec<String> = [
        if forward_count > 0 {
            Some(format!("{} dependencies", forward_count))
        } else {
            None
        },
        if reverse_count > 0 {
            Some(format!("{} dependents", reverse_count))
        } else {
            None
        },
    ]
    .into_iter()
    .flatten()
    .collect();

    if parts.is_empty() {
        "Dependencies — Standalone".to_string()
    } else {
        format!("Dependencies — {}", parts.join(", "))
    }
}
