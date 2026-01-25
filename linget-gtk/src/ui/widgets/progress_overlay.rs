#![allow(dead_code)]

use gtk4::prelude::*;
use gtk4::{self as gtk};
use relm4::prelude::*;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ProgressOverlayInit;

#[derive(Debug)]
pub enum ProgressOverlayInput {
    Show,
    Hide,
    SetProgress {
        fraction: f64,
        text: Option<String>,
    },
    SetStepProgress {
        current: usize,
        total: usize,
        item_name: Option<String>,
    },
    SetLabel(String),
    Reset,
}

pub struct ProgressOverlayModel {
    visible: bool,
    fraction: f64,
    progress_text: Option<String>,
    label_text: String,
    current_step: usize,
    total_steps: usize,
    started_at: Option<Instant>,
}

pub struct ProgressOverlayWidgets {
    root: gtk::Box,
    progress_bar: gtk::ProgressBar,
    label: gtk::Label,
    step_label: gtk::Label,
    eta_label: gtk::Label,
}

impl SimpleComponent for ProgressOverlayModel {
    type Init = ProgressOverlayInit;
    type Input = ProgressOverlayInput;
    type Output = ();
    type Root = gtk::Box;
    type Widgets = ProgressOverlayWidgets;

    fn init_root() -> Self::Root {
        let widget = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Fill)
            .halign(gtk::Align::Fill)
            .vexpand(true)
            .hexpand(true)
            .visible(false)
            .build();
        widget.add_css_class("progress-scrim");
        widget
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ProgressOverlayModel {
            visible: false,
            fraction: 0.0,
            progress_text: None,
            label_text: "Working…".to_string(),
            current_step: 0,
            total_steps: 0,
            started_at: None,
        };

        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .halign(gtk::Align::Center)
            .spacing(8)
            .margin_start(24)
            .margin_end(24)
            .build();
        card.add_css_class("progress-card");

        let label = gtk::Label::builder()
            .label(&model.label_text)
            .wrap(true)
            .build();
        label.add_css_class("title-3");
        label.set_max_width_chars(60);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);

        let step_label = gtk::Label::builder().label("").visible(false).build();
        step_label.add_css_class("caption");
        step_label.add_css_class("dim-label");

        let progress_bar = gtk::ProgressBar::builder().show_text(true).build();
        progress_bar.add_css_class("osd");
        progress_bar.set_height_request(10);

        let eta_label = gtk::Label::builder().label("").visible(false).build();
        eta_label.add_css_class("caption");
        eta_label.add_css_class("dim-label");
        eta_label.add_css_class("progress-eta");

        card.append(&label);
        card.append(&step_label);
        card.append(&progress_bar);
        card.append(&eta_label);
        root.append(&card);

        let widgets = ProgressOverlayWidgets {
            root: root.clone(),
            progress_bar,
            label,
            step_label,
            eta_label,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ProgressOverlayInput::Show => {
                self.visible = true;
                if self.started_at.is_none() {
                    self.started_at = Some(Instant::now());
                }
            }
            ProgressOverlayInput::Hide => {
                self.visible = false;
            }
            ProgressOverlayInput::SetProgress { fraction, text } => {
                self.fraction = fraction;
                self.progress_text = text;
            }
            ProgressOverlayInput::SetStepProgress {
                current,
                total,
                item_name,
            } => {
                self.current_step = current;
                self.total_steps = total;

                if self.started_at.is_none() {
                    self.started_at = Some(Instant::now());
                }

                self.fraction = if total > 0 {
                    current as f64 / total as f64
                } else {
                    0.0
                };

                self.progress_text = Some(if let Some(name) = item_name {
                    format!("{}/{}: {}", current, total, name)
                } else {
                    format!("{}/{}", current, total)
                });
            }
            ProgressOverlayInput::SetLabel(text) => {
                self.label_text = text;
            }
            ProgressOverlayInput::Reset => {
                self.fraction = 0.0;
                self.progress_text = None;
                self.current_step = 0;
                self.total_steps = 0;
                self.started_at = None;
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.root.set_visible(self.visible);
        widgets.progress_bar.set_fraction(self.fraction);
        if let Some(ref text) = self.progress_text {
            widgets.progress_bar.set_text(Some(text));
        }
        widgets.label.set_label(&self.label_text);

        if self.total_steps > 0 {
            widgets.step_label.set_visible(true);
            widgets.step_label.set_label(&format!(
                "Step {} of {}",
                self.current_step, self.total_steps
            ));

            if let Some(eta) = self.calculate_eta() {
                widgets.eta_label.set_visible(true);
                widgets.eta_label.set_label(&eta);
            } else {
                widgets.eta_label.set_visible(false);
            }
        } else {
            widgets.step_label.set_visible(false);
            widgets.eta_label.set_visible(false);
        }
    }
}

impl ProgressOverlayModel {
    fn calculate_eta(&self) -> Option<String> {
        if self.current_step == 0 {
            return None;
        }

        let started_at = self.started_at?;
        let elapsed = started_at.elapsed().as_secs_f64();
        let avg_per_item = elapsed / self.current_step as f64;
        let remaining = self.total_steps.saturating_sub(self.current_step);
        let secs = (avg_per_item * remaining as f64) as u64;

        Some(if secs < 60 {
            format!("~{}s remaining", secs)
        } else {
            let mins = secs / 60;
            let remaining_secs = secs % 60;
            if remaining_secs > 0 {
                format!("~{}m {}s remaining", mins, remaining_secs)
            } else {
                format!("~{}m remaining", mins)
            }
        })
    }
}
