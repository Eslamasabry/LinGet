use crate::models::{Package, SchedulePreset, ScheduledOperation, ScheduledTask};

use chrono::{DateTime, Local, NaiveTime, Utc};
use gtk4::prelude::*;
use gtk4::{self as gtk};
use std::cell::RefCell;
use std::rc::Rc;

pub struct SchedulePopoverResult {
    pub task: ScheduledTask,
}

pub fn build_schedule_popover(
    package: &Package,
    operation: ScheduledOperation,
    on_schedule: impl Fn(SchedulePopoverResult) + 'static,
) -> gtk::Popover {
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
        .label(format!("Schedule {}", operation.display_name()))
        .css_classes(["heading"])
        .halign(gtk::Align::Start)
        .build();
    content.append(&header);

    let package_label = gtk::Label::builder()
        .label(&package.name)
        .css_classes(["dim-label"])
        .halign(gtk::Align::Start)
        .build();
    content.append(&package_label);

    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    content.append(&sep);

    let presets_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(4)
        .build();

    let pkg_id = package.id();
    let pkg_name = package.name.clone();
    let pkg_source = package.source;
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

        let pkg_id = pkg_id.clone();
        let pkg_name = pkg_name.clone();
        let on_schedule = on_schedule.clone();
        let popover_ref = popover_ref.clone();

        btn.connect_clicked(move |_| {
            if let Some(scheduled_at) = preset.to_datetime() {
                let task = ScheduledTask::new(
                    pkg_id.clone(),
                    pkg_name.clone(),
                    pkg_source,
                    operation,
                    scheduled_at,
                );
                if let Some(callback) = on_schedule.borrow_mut().take() {
                    callback(SchedulePopoverResult { task });
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
        .label("Schedule")
        .css_classes(["suggested-action", "pill"])
        .margin_top(8)
        .build();

    let pkg_id = package.id();
    let pkg_name = package.name.clone();
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

                let task = ScheduledTask::new(
                    pkg_id.clone(),
                    pkg_name.clone(),
                    pkg_source,
                    operation,
                    scheduled_at,
                );
                if let Some(callback) = on_schedule.borrow_mut().take() {
                    callback(SchedulePopoverResult { task });
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

#[allow(dead_code)]
pub fn build_schedule_menu_button(
    package: &Package,
    operation: ScheduledOperation,
    on_schedule: impl Fn(SchedulePopoverResult) + 'static,
) -> gtk::MenuButton {
    let btn = gtk::MenuButton::builder()
        .icon_name("alarm-symbolic")
        .tooltip_text("Schedule for later")
        .css_classes(["flat", "circular"])
        .build();

    let popover = build_schedule_popover(package, operation, on_schedule);
    btn.set_popover(Some(&popover));

    btn
}

#[allow(dead_code)]
pub fn format_scheduled_time(scheduled_at: &DateTime<Utc>) -> String {
    let local = scheduled_at.with_timezone(&Local);
    let now = Local::now();

    if local.date_naive() == now.date_naive() {
        format!("Today at {}", local.format("%I:%M %p"))
    } else if local.date_naive() == (now.date_naive() + chrono::Duration::days(1)) {
        format!("Tomorrow at {}", local.format("%I:%M %p"))
    } else {
        local.format("%b %d at %I:%M %p").to_string()
    }
}
