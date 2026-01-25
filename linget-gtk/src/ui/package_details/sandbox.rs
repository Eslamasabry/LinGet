use crate::backend::PackageManager;
use crate::models::{FlatpakMetadata, PrivacyLevel, SandboxRating, SandboxSummary};
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn build_sandbox_section(pm: Arc<Mutex<PackageManager>>, app_id: String) -> gtk::Box {
    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .build();
    section.add_css_class("sandbox-section");

    let loading_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk::Align::Center)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    let spinner = gtk::Spinner::builder().spinning(true).build();
    let loading_label = gtk::Label::new(Some("Loading sandbox info..."));
    loading_label.add_css_class("dim-label");
    loading_label.add_css_class("caption");
    loading_box.append(&spinner);
    loading_box.append(&loading_label);
    section.append(&loading_box);

    let (tx, rx) = async_channel::bounded::<Result<FlatpakMetadata, String>>(1);
    let section_clone = section.clone();

    relm4::spawn(async move {
        let result = {
            let manager = pm.lock().await;
            manager.get_flatpak_metadata(&app_id).await
        };
        let _ = tx.send(result.map_err(|e| e.to_string())).await;
    });

    glib::spawn_future_local(async move {
        if let Ok(result) = rx.recv().await {
            while let Some(child) = section_clone.first_child() {
                section_clone.remove(&child);
            }

            match result {
                Ok(metadata) => {
                    build_sandbox_content(&section_clone, &metadata);
                }
                Err(e) => {
                    let error_label = gtk::Label::builder()
                        .label(format!("Could not load sandbox info: {}", e))
                        .wrap(true)
                        .xalign(0.0)
                        .build();
                    error_label.add_css_class("dim-label");
                    section_clone.append(&error_label);
                }
            }
        }
    });

    section
}

fn build_sandbox_content(section: &gtk::Box, metadata: &FlatpakMetadata) {
    let summary = metadata.sandbox_summary();

    // Sandbox rating header
    let rating_box = build_rating_box(&summary);
    section.append(&rating_box);

    // Runtime info
    if let Some(ref runtime) = metadata.runtime {
        let runtime_row = adw::ActionRow::builder()
            .title("Runtime")
            .subtitle(runtime.to_string())
            .build();
        runtime_row.add_css_class("property");

        let runtime_icon = gtk::Image::builder()
            .icon_name("application-x-executable-symbolic")
            .build();
        runtime_icon.add_css_class("dim-label");
        runtime_row.add_prefix(&runtime_icon);

        section.append(&runtime_row);
    }

    // Installation info
    let install_row = adw::ActionRow::builder()
        .title("Installation")
        .subtitle(format!("{}", metadata.installation))
        .build();
    install_row.add_css_class("property");

    let install_icon = gtk::Image::builder()
        .icon_name(
            if metadata.installation == crate::models::InstallationType::System {
                "computer-symbolic"
            } else {
                "user-home-symbolic"
            },
        )
        .build();
    install_icon.add_css_class("dim-label");
    install_row.add_prefix(&install_icon);
    section.append(&install_row);

    // EOL warning if applicable
    if metadata.is_eol {
        let warning_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(8)
            .build();
        warning_box.add_css_class("warning-bar");

        let warning_icon = gtk::Image::builder()
            .icon_name("dialog-warning-symbolic")
            .build();
        warning_icon.add_css_class("error");

        let warning_label = gtk::Label::builder()
            .label(if let Some(ref reason) = metadata.eol_reason {
                format!("End of Life: {}", reason)
            } else {
                "This application has reached end of life".to_string()
            })
            .wrap(true)
            .xalign(0.0)
            .build();
        warning_label.add_css_class("error");

        warning_box.append(&warning_icon);
        warning_box.append(&warning_label);
        section.append(&warning_box);
    }

    // Permissions expander
    if !metadata.permissions.is_empty() {
        let permissions_expander = build_permissions_expander(metadata);
        section.append(&permissions_expander);
    } else {
        let no_perms_label = gtk::Label::builder()
            .label("No special permissions required")
            .xalign(0.0)
            .margin_top(8)
            .build();
        no_perms_label.add_css_class("success");
        no_perms_label.add_css_class("caption");
        section.append(&no_perms_label);
    }
}

fn build_rating_box(summary: &SandboxSummary) -> gtk::Box {
    let rating_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    rating_box.add_css_class("sandbox-rating-box");

    // Rating icon
    let rating_icon = gtk::Image::builder()
        .icon_name(summary.rating.icon_name())
        .pixel_size(32)
        .build();
    rating_icon.add_css_class(summary.rating.css_class());

    // Rating text
    let text_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .valign(gtk::Align::Center)
        .build();

    let rating_label = gtk::Label::builder()
        .label(format!("Sandbox: {}", summary.rating))
        .xalign(0.0)
        .build();
    rating_label.add_css_class("heading");
    rating_label.add_css_class(summary.rating.css_class());

    let desc_label = gtk::Label::builder()
        .label(&summary.description)
        .xalign(0.0)
        .wrap(true)
        .build();
    desc_label.add_css_class("caption");
    desc_label.add_css_class("dim-label");

    text_box.append(&rating_label);
    text_box.append(&desc_label);

    rating_box.append(&rating_icon);
    rating_box.append(&text_box);

    rating_box
}

fn build_permissions_expander(metadata: &FlatpakMetadata) -> gtk::Expander {
    let summary = metadata.sandbox_summary();

    let label_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let label_text = gtk::Label::new(Some(&format!(
        "Permissions ({} total, {} high-risk)",
        metadata.permissions.len(),
        metadata
            .permissions
            .iter()
            .filter(|p| p.privacy_level == PrivacyLevel::High && !p.negated)
            .count()
    )));

    let badge = build_sandbox_badge(summary.rating);

    label_box.append(&label_text);
    label_box.append(&badge);

    let expander = gtk::Expander::builder()
        .expanded(false)
        .margin_top(8)
        .build();
    expander.set_label_widget(Some(&label_box));
    expander.add_css_class("card");

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    // Group permissions by category
    let grouped = metadata.permissions_by_category();

    for (category, permissions) in grouped {
        let category_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_top(4)
            .build();

        // Category header
        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();

        let cat_icon = gtk::Image::builder()
            .icon_name(category.icon_name())
            .build();
        cat_icon.add_css_class("dim-label");

        let cat_label = gtk::Label::builder()
            .label(category.description())
            .xalign(0.0)
            .build();
        cat_label.add_css_class("caption");
        cat_label.add_css_class("heading");

        header.append(&cat_icon);
        header.append(&cat_label);
        category_box.append(&header);

        // Permission items
        for perm in permissions {
            let perm_row = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(8)
                .margin_start(24)
                .build();

            // Privacy level indicator
            let level_label = gtk::Label::new(Some("●"));
            level_label.add_css_class(perm.privacy_level.css_class());

            // Permission value
            let value_label = gtk::Label::builder()
                .label(if perm.negated {
                    format!("✗ {}", perm.value)
                } else {
                    perm.value.clone()
                })
                .xalign(0.0)
                .build();

            if perm.negated {
                value_label.add_css_class("dim-label");
            }

            // Description tooltip
            let desc_label = gtk::Label::builder()
                .label(&perm.description)
                .xalign(0.0)
                .hexpand(true)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            desc_label.add_css_class("dim-label");
            desc_label.add_css_class("caption");

            perm_row.append(&level_label);
            perm_row.append(&value_label);
            perm_row.append(&desc_label);
            perm_row.set_tooltip_text(Some(&perm.description));

            category_box.append(&perm_row);
        }

        content.append(&category_box);
    }

    // Legend
    let legend = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .margin_top(12)
        .halign(gtk::Align::Center)
        .build();

    let low_box = create_legend_item("●", PrivacyLevel::Low.css_class(), "Low");
    let med_box = create_legend_item("●", PrivacyLevel::Medium.css_class(), "Medium");
    let high_box = create_legend_item("●", PrivacyLevel::High.css_class(), "High");

    legend.append(&low_box);
    legend.append(&med_box);
    legend.append(&high_box);
    content.append(&legend);

    expander.set_child(Some(&content));
    expander
}

fn create_legend_item(icon: &str, class: &str, label: &str) -> gtk::Box {
    let item = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .build();

    let icon_label = gtk::Label::new(Some(icon));
    icon_label.add_css_class(class);

    let text_label = gtk::Label::new(Some(label));
    text_label.add_css_class("caption");
    text_label.add_css_class("dim-label");

    item.append(&icon_label);
    item.append(&text_label);
    item
}

/// Build a compact sandbox badge for the package row
pub fn build_sandbox_badge(rating: SandboxRating) -> gtk::Box {
    let badge = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(4)
        .build();
    badge.add_css_class("sandbox-badge");

    let compact_icon = match rating {
        SandboxRating::Strong => "security-high-symbolic",
        SandboxRating::Good | SandboxRating::Moderate => "security-medium-symbolic",
        SandboxRating::Weak => "security-low-symbolic",
    };

    let icon = gtk::Image::builder()
        .icon_name(compact_icon)
        .pixel_size(12)
        .build();
    icon.add_css_class(rating.css_class());

    badge.append(&icon);
    badge.set_tooltip_text(Some(&format!("Sandbox: {}", rating)));

    badge
}
