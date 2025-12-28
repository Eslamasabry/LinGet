use crate::models::{SchedulePreset, ScheduledOperation, ScheduledTask};

use chrono::{Local, NaiveTime, Utc};
use gtk4::prelude::*;
use gtk4::{self as gtk};
use relm4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct SelectionBarInit;

#[derive(Debug)]
pub enum SelectionBarInput {
    Show,
    Hide,
    SetCount(usize),
    SetHasUpdates(bool),
    /// Provide package info for bulk scheduling (id, name, source)
    SetSelectedPackages(Vec<(String, String, crate::models::PackageSource)>),
}

#[derive(Debug, Clone)]
pub enum SelectionBarOutput {
    SelectAll,
    DeselectAll,
    UpdateSelected,
    RemoveSelected,
    /// Schedule multiple packages for update at the given time
    ScheduleSelectedUpdates(Vec<ScheduledTask>),
}

pub struct SelectionBarModel {
    visible: bool,
    count: usize,
    has_updates: bool,
    selected_packages: Rc<RefCell<Vec<(String, String, crate::models::PackageSource)>>>,
}

pub struct SelectionBarWidgets {
    root: gtk::ActionBar,
    count_label: gtk::Label,
    update_btn: gtk::Button,
    schedule_btn: gtk::MenuButton,
}

impl SimpleComponent for SelectionBarModel {
    type Init = SelectionBarInit;
    type Input = SelectionBarInput;
    type Output = SelectionBarOutput;
    type Root = gtk::ActionBar;
    type Widgets = SelectionBarWidgets;

    fn init_root() -> Self::Root {
        let widget = gtk::ActionBar::builder().visible(false).build();
        widget.add_css_class("selection-bar");
        widget
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected_packages = Rc::new(RefCell::new(Vec::new()));

        let model = SelectionBarModel {
            visible: false,
            count: 0,
            has_updates: false,
            selected_packages: selected_packages.clone(),
        };

        let select_all_btn = gtk::Button::builder().label("Select All").build();
        select_all_btn.add_css_class("flat");
        select_all_btn.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.output(SelectionBarOutput::SelectAll).ok();
            }
        });

        let deselect_all_btn = gtk::Button::builder().label("Deselect All").build();
        deselect_all_btn.add_css_class("flat");
        deselect_all_btn.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.output(SelectionBarOutput::DeselectAll).ok();
            }
        });

        let count_label = gtk::Label::builder()
            .label("0 selected")
            .hexpand(true)
            .build();

        let update_btn = gtk::Button::builder()
            .label("Update Selected")
            .visible(false)
            .build();
        update_btn.add_css_class("suggested-action");
        update_btn.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.output(SelectionBarOutput::UpdateSelected).ok();
            }
        });

        let schedule_btn = gtk::MenuButton::builder()
            .icon_name("alarm-symbolic")
            .tooltip_text("Schedule updates for later")
            .visible(false)
            .build();
        schedule_btn.add_css_class("flat");

        let schedule_popover = build_bulk_schedule_popover(selected_packages.clone(), {
            let sender = sender.clone();
            move |tasks| {
                sender
                    .output(SelectionBarOutput::ScheduleSelectedUpdates(tasks))
                    .ok();
            }
        });
        schedule_btn.set_popover(Some(&schedule_popover));

        let remove_btn = gtk::Button::builder().label("Remove Selected").build();
        remove_btn.add_css_class("destructive-action");
        remove_btn.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender.output(SelectionBarOutput::RemoveSelected).ok();
            }
        });

        root.pack_start(&select_all_btn);
        root.pack_start(&deselect_all_btn);
        root.set_center_widget(Some(&count_label));
        root.pack_end(&remove_btn);
        root.pack_end(&schedule_btn);
        root.pack_end(&update_btn);

        let widgets = SelectionBarWidgets {
            root: root.clone(),
            count_label,
            update_btn,
            schedule_btn,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SelectionBarInput::Show => {
                self.visible = true;
            }
            SelectionBarInput::Hide => {
                self.visible = false;
            }
            SelectionBarInput::SetCount(count) => {
                self.count = count;
            }
            SelectionBarInput::SetHasUpdates(has_updates) => {
                self.has_updates = has_updates;
            }
            SelectionBarInput::SetSelectedPackages(packages) => {
                *self.selected_packages.borrow_mut() = packages;
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets.root.set_visible(self.visible);
        widgets
            .count_label
            .set_label(&format!("{} selected", self.count));
        let show_update_actions = self.has_updates && self.count > 0;
        widgets.update_btn.set_visible(show_update_actions);
        widgets.schedule_btn.set_visible(show_update_actions);
    }
}

fn build_bulk_schedule_popover<F>(
    selected_packages: Rc<RefCell<Vec<(String, String, crate::models::PackageSource)>>>,
    on_schedule: F,
) -> gtk::Popover
where
    F: Fn(Vec<ScheduledTask>) + 'static,
{
    let popover = gtk::Popover::new();
    popover.add_css_class("schedule-popover");

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();

    let header = gtk::Label::builder()
        .label("Schedule Updates")
        .css_classes(["heading"])
        .halign(gtk::Align::Start)
        .build();
    content.append(&header);

    let desc = gtk::Label::builder()
        .label("Selected packages will be updated at the chosen time")
        .css_classes(["dim-label", "caption"])
        .halign(gtk::Align::Start)
        .wrap(true)
        .build();
    content.append(&desc);

    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    content.append(&sep);

    let presets_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();

    let on_schedule = Rc::new(RefCell::new(Some(on_schedule)));
    let popover_ref = Rc::new(RefCell::new(None::<gtk::Popover>));

    for preset in SchedulePreset::quick_presets() {
        let preset_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["preset-row"])
            .build();

        let icon = gtk::Image::builder()
            .icon_name(preset.icon_name())
            .pixel_size(16)
            .build();

        let label = gtk::Label::builder()
            .label(preset.display_name())
            .hexpand(true)
            .halign(gtk::Align::Start)
            .build();

        if let Some(dt) = preset.to_datetime() {
            let time_str = dt.with_timezone(&Local).format("%I:%M %p").to_string();
            let time_label = gtk::Label::builder()
                .label(&time_str)
                .css_classes(["dim-label", "caption"])
                .build();
            preset_row.append(&icon);
            preset_row.append(&label);
            preset_row.append(&time_label);
        } else {
            preset_row.append(&icon);
            preset_row.append(&label);
        }

        let btn = gtk::Button::builder()
            .child(&preset_row)
            .css_classes(["flat"])
            .build();

        let selected_packages = selected_packages.clone();
        let on_schedule = on_schedule.clone();
        let popover_ref = popover_ref.clone();

        btn.connect_clicked(move |_| {
            if let Some(scheduled_at) = preset.to_datetime() {
                let packages = selected_packages.borrow();
                let tasks: Vec<ScheduledTask> = packages
                    .iter()
                    .map(|(id, name, source)| {
                        ScheduledTask::new(
                            id.clone(),
                            name.clone(),
                            *source,
                            ScheduledOperation::Update,
                            scheduled_at,
                        )
                    })
                    .collect();

                if let Some(ref callback) = *on_schedule.borrow() {
                    callback(tasks);
                }
                if let Some(p) = popover_ref.borrow().as_ref() {
                    p.popdown();
                }
            }
        });

        presets_box.append(&btn);
    }

    content.append(&presets_box);

    let custom_sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    custom_sep.set_margin_top(4);
    custom_sep.set_margin_bottom(4);
    content.append(&custom_sep);

    let custom_label = gtk::Label::builder()
        .label("Custom time")
        .css_classes(["dim-label", "caption"])
        .halign(gtk::Align::Start)
        .build();
    content.append(&custom_label);

    let time_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .build();

    let hour_spin = gtk::SpinButton::with_range(0.0, 23.0, 1.0);
    hour_spin.set_value(
        Local::now()
            .format("%H")
            .to_string()
            .parse()
            .unwrap_or(12.0),
    );
    hour_spin.set_width_chars(2);

    let colon = gtk::Label::new(Some(":"));

    let minute_spin = gtk::SpinButton::with_range(0.0, 59.0, 15.0);
    minute_spin.set_value(0.0);
    minute_spin.set_width_chars(2);

    let tomorrow_check = gtk::CheckButton::builder().label("Tomorrow").build();

    time_box.append(&hour_spin);
    time_box.append(&colon);
    time_box.append(&minute_spin);
    time_box.append(&tomorrow_check);
    content.append(&time_box);

    let schedule_btn = gtk::Button::builder()
        .label("Schedule All")
        .css_classes(["suggested-action", "pill"])
        .margin_top(8)
        .build();

    let selected_packages_btn = selected_packages.clone();
    let popover_ref_btn = popover_ref.clone();

    schedule_btn.connect_clicked(move |_| {
        let hour = hour_spin.value() as u32;
        let minute = minute_spin.value() as u32;
        let is_tomorrow = tomorrow_check.is_active();

        let now = Local::now();
        let target_date = if is_tomorrow {
            now.date_naive() + chrono::Duration::days(1)
        } else {
            now.date_naive()
        };

        if let Some(target_time) = NaiveTime::from_hms_opt(hour, minute, 0) {
            let local_dt = target_date.and_time(target_time);
            if let Some(dt) = local_dt.and_local_timezone(Local).single() {
                let scheduled_at = dt.with_timezone(&Utc);

                if scheduled_at <= Utc::now() {
                    return;
                }

                let packages = selected_packages_btn.borrow();
                let tasks: Vec<ScheduledTask> = packages
                    .iter()
                    .map(|(id, name, source)| {
                        ScheduledTask::new(
                            id.clone(),
                            name.clone(),
                            *source,
                            ScheduledOperation::Update,
                            scheduled_at,
                        )
                    })
                    .collect();

                if let Some(ref callback) = *on_schedule.borrow() {
                    callback(tasks);
                }
                if let Some(p) = popover_ref_btn.borrow().as_ref() {
                    p.popdown();
                }
            }
        }
    });

    content.append(&schedule_btn);

    popover.set_child(Some(&content));
    *popover_ref.borrow_mut() = Some(popover.clone());

    popover
}
