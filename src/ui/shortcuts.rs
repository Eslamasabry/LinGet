use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use std::rc::Rc;

pub struct ShortcutContext {
    pub search_entry: gtk::SearchEntry,
    pub select_button: gtk::ToggleButton,
    pub update_selected_btn: gtk::Button,
    pub remove_selected_btn: gtk::Button,
    pub refresh_fn: Rc<dyn Fn()>,
    pub close_details_panel: Rc<dyn Fn()>,
    pub details_flap: adw::Flap,
}

pub fn setup_keyboard_shortcuts(window: &gtk::ApplicationWindow, ctx: ShortcutContext) {
    let controller = gtk::EventControllerKey::new();

    controller.connect_key_pressed(move |_, key, _, modifier| {
        if modifier.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            match key {
                gtk::gdk::Key::f => {
                    ctx.search_entry.grab_focus();
                    return glib::Propagation::Stop;
                }
                gtk::gdk::Key::r => {
                    (ctx.refresh_fn)();
                    return glib::Propagation::Stop;
                }
                gtk::gdk::Key::s => {
                    ctx.select_button.set_active(!ctx.select_button.is_active());
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }
        match key {
            gtk::gdk::Key::slash => {
                ctx.search_entry.grab_focus();
                return glib::Propagation::Stop;
            }
            gtk::gdk::Key::Escape => {
                if ctx.details_flap.reveals_flap() {
                    (ctx.close_details_panel)();
                    return glib::Propagation::Stop;
                }
                if ctx.select_button.is_active() {
                    ctx.select_button.set_active(false);
                    return glib::Propagation::Stop;
                }
                if !ctx.search_entry.text().is_empty() {
                    ctx.search_entry.set_text("");
                    return glib::Propagation::Stop;
                }
            }
            gtk::gdk::Key::u | gtk::gdk::Key::U => {
                if ctx.select_button.is_active() && ctx.update_selected_btn.is_sensitive() {
                    ctx.update_selected_btn.emit_clicked();
                    return glib::Propagation::Stop;
                }
            }
            gtk::gdk::Key::Delete => {
                if ctx.select_button.is_active() && ctx.remove_selected_btn.is_sensitive() {
                    ctx.remove_selected_btn.emit_clicked();
                    return glib::Propagation::Stop;
                }
            }
            _ => {}
        }
        glib::Propagation::Proceed
    });

    window.add_controller(controller);
}
