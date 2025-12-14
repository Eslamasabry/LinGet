use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
pub enum CommandEventKind {
    Info,
    Success,
    Error,
}

#[derive(Clone)]
pub struct CommandCenter {
    inner: Rc<Inner>,
}

struct Inner {
    root: gtk::Box,
    list: gtk::ListBox,
    empty: adw::StatusPage,
    stack: gtk::Stack,
    unread: RefCell<u32>,
    unread_badge: gtk::Label,
    external_badge: RefCell<Option<gtk::Label>>,
}

impl CommandCenter {
    pub fn new() -> Self {
        let header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(12)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let title = gtk::Label::builder()
            .label("Command Center")
            .hexpand(true)
            .xalign(0.0)
            .build();
        title.add_css_class("title-3");

        let unread_badge = gtk::Label::builder().label("0").visible(false).build();
        unread_badge.add_css_class("badge-accent");

        let clear_btn = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .tooltip_text("Clear history")
            .build();
        clear_btn.add_css_class("flat");
        clear_btn.add_css_class("circular");

        header.append(&title);
        header.append(&unread_badge);
        header.append(&clear_btn);

        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let empty = adw::StatusPage::builder()
            .icon_name("format-justify-fill-symbolic")
            .title("No recent activity")
            .description("Updates, installs, and removals will appear here")
            .build();

        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .transition_duration(150)
            .build();
        stack.add_named(&empty, Some("empty"));
        stack.add_named(&list, Some("list"));
        stack.set_visible_child_name("empty");

        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&stack)
            .build();

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(360)
            .build();
        root.add_css_class("command-center");
        root.append(&header);
        root.append(&scrolled);

        let this = Self {
            inner: Rc::new(Inner {
                root,
                list,
                empty,
                stack,
                unread: RefCell::new(0),
                unread_badge,
                external_badge: RefCell::new(None),
            }),
        };

        let center = this.clone();
        clear_btn.connect_clicked(move |_| {
            center.clear();
        });

        this
    }

    pub fn widget(&self) -> gtk::Box {
        self.inner.root.clone()
    }

    pub fn attach_badge(&self, badge: gtk::Label) {
        *self.inner.external_badge.borrow_mut() = Some(badge);
    }

    pub fn mark_read(&self) {
        *self.inner.unread.borrow_mut() = 0;
        self.inner.unread_badge.set_visible(false);
        if let Some(badge) = self.inner.external_badge.borrow().as_ref() {
            badge.set_visible(false);
        }
    }

    pub fn add_event(
        &self,
        kind: CommandEventKind,
        title: impl AsRef<str>,
        details: impl AsRef<str>,
        command: Option<String>,
    ) {
        let title = title.as_ref().trim().to_string();
        let details = details.as_ref().trim().to_string();

        let row = adw::ActionRow::builder()
            .title(&title)
            .subtitle(&details)
            .build();
        row.add_css_class("cmd-row");

        let icon_name = match kind {
            CommandEventKind::Info => "dialog-information-symbolic",
            CommandEventKind::Success => "emblem-ok-symbolic",
            CommandEventKind::Error => "dialog-error-symbolic",
        };
        let icon = gtk::Image::from_icon_name(icon_name);
        icon.add_css_class("dim-label");
        row.add_prefix(&icon);

        if let Some(command) = command {
            let copy_btn = gtk::Button::builder()
                .icon_name("edit-copy-symbolic")
                .tooltip_text("Copy command")
                .build();
            copy_btn.add_css_class("flat");
            copy_btn.add_css_class("circular");

            let cmd = command.clone();
            copy_btn.connect_clicked(move |_| {
                if let Some(display) = gtk::gdk::Display::default() {
                    display.clipboard().set_text(&cmd);
                    display.primary_clipboard().set_text(&cmd);
                }
            });

            row.add_suffix(&copy_btn);
        }

        let wrapper = gtk::ListBoxRow::new();
        wrapper.set_child(Some(&row));
        self.inner.list.prepend(&wrapper);

        self.inner.stack.set_visible_child_name("list");
        self.inner.empty.set_visible(false);

        let mut unread = self.inner.unread.borrow_mut();
        *unread = unread.saturating_add(1);
        self.inner.unread_badge.set_label(&unread.to_string());
        self.inner.unread_badge.set_visible(true);
        if let Some(badge) = self.inner.external_badge.borrow().as_ref() {
            badge.set_label(&unread.to_string());
            badge.set_visible(true);
        }

        // Ensure GTK processes the new row layout quickly.
        glib::idle_add_local_once({
            let list = self.inner.list.clone();
            move || list.queue_allocate()
        });
    }

    pub fn clear(&self) {
        while let Some(child) = self.inner.list.first_child() {
            self.inner.list.remove(&child);
        }
        self.inner.stack.set_visible_child_name("empty");
        self.inner.empty.set_visible(true);
        self.mark_read();
    }
}
