use crate::models::PackageEnrichment;
use crate::ui::package_details::{DetailsPanelInput, DetailsPanelModel};

use gdk_pixbuf::Pixbuf;
use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use libadwaita as adw;
use relm4::prelude::*;

pub fn build_section(
    enrichment: &PackageEnrichment,
    sender: ComponentSender<DetailsPanelModel>,
) -> gtk::Box {
    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    section.add_css_class("enrichment-section");

    if let Some(ref summary) = enrichment.summary {
        if !summary.is_empty() {
            let summary_label = gtk::Label::builder()
                .label(summary)
                .wrap(true)
                .xalign(0.0)
                .build();
            summary_label.add_css_class("body");
            section.append(&summary_label);
        }
    }

    let stats_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(16)
        .halign(gtk::Align::Start)
        .build();

    if let Some(ref developer) = enrichment.developer {
        let dev_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        let dev_icon = gtk::Image::builder()
            .icon_name("avatar-default-symbolic")
            .build();
        dev_icon.add_css_class("dimmed");
        let dev_label = gtk::Label::new(Some(developer));
        dev_label.add_css_class("caption");
        dev_box.append(&dev_icon);
        dev_box.append(&dev_label);
        stats_box.append(&dev_box);
    }

    if let Some(downloads) = enrichment.downloads {
        let dl_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        let dl_icon = gtk::Image::builder()
            .icon_name("folder-download-symbolic")
            .build();
        dl_icon.add_css_class("dimmed");
        let dl_text = if downloads >= 1_000_000 {
            format!("{:.1}M", downloads as f64 / 1_000_000.0)
        } else if downloads >= 1_000 {
            format!("{:.1}K", downloads as f64 / 1_000.0)
        } else {
            downloads.to_string()
        };
        let dl_label = gtk::Label::new(Some(&dl_text));
        dl_label.add_css_class("caption");
        dl_box.append(&dl_icon);
        dl_box.append(&dl_label);
        stats_box.append(&dl_box);
    }

    if let Some(rating) = enrichment.rating {
        let rating_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .build();
        let star_icon = gtk::Image::builder().icon_name("starred-symbolic").build();
        star_icon.add_css_class("warning");
        let rating_label = gtk::Label::new(Some(&format!("{:.1}", rating)));
        rating_label.add_css_class("caption");
        rating_box.append(&star_icon);
        rating_box.append(&rating_label);
        stats_box.append(&rating_box);
    }

    if stats_box.first_child().is_some() {
        section.append(&stats_box);
    }

    let tags: Vec<&str> = enrichment
        .categories
        .iter()
        .chain(enrichment.keywords.iter())
        .map(|s| s.as_str())
        .take(8)
        .collect();

    if !tags.is_empty() {
        let tags_flow = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(6)
            .row_spacing(6)
            .column_spacing(6)
            .homogeneous(false)
            .build();
        tags_flow.add_css_class("enrichment-tags");

        for tag in tags {
            let chip = gtk::Label::builder().label(tag).build();
            chip.add_css_class("chip");
            chip.add_css_class("caption");
            tags_flow.insert(&chip, -1);
        }
        section.append(&tags_flow);
    }

    if let Some(ref repo) = enrichment.repository {
        if !repo.is_empty() {
            let link_btn = gtk::LinkButton::builder()
                .label("View Repository")
                .uri(repo)
                .halign(gtk::Align::Start)
                .build();
            link_btn.add_css_class("flat");
            section.append(&link_btn);
        }
    }

    if !enrichment.screenshots.is_empty() {
        let screenshots_label = gtk::Label::builder()
            .label("Screenshots")
            .xalign(0.0)
            .margin_top(8)
            .build();
        screenshots_label.add_css_class("heading");
        section.append(&screenshots_label);

        let carousel = adw::Carousel::builder()
            .hexpand(true)
            .height_request(200)
            .build();
        carousel.add_css_class("card");

        for url in enrichment.screenshots.iter().take(5) {
            let image = gtk::Picture::builder()
                .height_request(200)
                .can_shrink(true)
                .build();
            image.add_css_class("screenshot");
            image.set_cursor_from_name(Some("pointer"));

            let url = url.clone();
            let image_clone = image.clone();
            let (tx, rx) = async_channel::bounded::<Vec<u8>>(1);

            relm4::spawn(async move {
                if let Ok(bytes) = load_image_bytes(&url).await {
                    let _ = tx.send(bytes).await;
                }
            });

            let sender_clone = sender.clone();
            glib::spawn_future_local(async move {
                if let Ok(bytes) = rx.recv().await {
                    let bytes = glib::Bytes::from_owned(bytes);
                    let stream = gio::MemoryInputStream::from_bytes(&bytes);
                    if let Ok(pixbuf) =
                        Pixbuf::from_stream_at_scale_future(&stream, 800, 450, true).await
                    {
                        let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
                        image_clone.set_paintable(Some(&texture));

                        let click = gtk::GestureClick::new();
                        let texture_clone = texture.clone();
                        let s_clone = sender_clone.clone();
                        click.connect_released(move |_, _, _, _| {
                            s_clone
                                .input(DetailsPanelInput::PreviewScreenshot(texture_clone.clone()));
                        });
                        image_clone.add_controller(click);
                    }
                }
            });

            carousel.append(&image);
        }

        let carousel_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .build();
        carousel_box.append(&carousel);

        if enrichment.screenshots.len() > 1 {
            let indicators = adw::CarouselIndicatorDots::builder()
                .carousel(&carousel)
                .build();
            carousel_box.append(&indicators);
        }

        section.append(&carousel_box);
    }

    section
}

async fn load_image_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let bytes = client.get(url).send().await?.bytes().await?;
    Ok(bytes.to_vec())
}
