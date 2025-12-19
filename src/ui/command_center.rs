use crate::models::Package;
use chrono::Local;
use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandEventKind {
    Info,
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub enum PackageOp {
    Install,
    Update,
    Remove,
    Downgrade,
    #[allow(dead_code)]
    DowngradeTo(String),
}

type RetryHandler = Rc<dyn Fn(RetrySpec)>;

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum RetrySpec {
    Package {
        package: Box<Package>,
        op: PackageOp,
    },
    BulkUpdate {
        packages: Vec<Package>,
    },
    BulkRemove {
        packages: Vec<Package>,
    },
}

#[derive(Clone)]
pub struct CommandCenter {
    inner: Rc<Inner>,
}

#[derive(Clone)]
pub struct CommandTask {
    inner: Rc<TaskInner>,
}

struct Inner {
    root: gtk::Box,
    active_section: gtk::Box,
    active_list: gtk::ListBox,
    history_list: gtk::ListBox,
    empty: adw::StatusPage,
    stack: gtk::Stack,
    unread: RefCell<u32>,
    unread_badge: gtk::Label,
    external_badge: RefCell<Option<gtk::Label>>,
    retry_handler: RefCell<Option<RetryHandler>>,
}

struct TaskInner {
    center: CommandCenter,
    wrapper: gtk::ListBoxRow,
    icon: gtk::Image,
    spinner: gtk::Spinner,
    row: adw::ActionRow,
    command_text: Rc<RefCell<String>>,
    retry_spec: Rc<RefCell<Option<RetrySpec>>>,
    copy_btn: gtk::Button,
    retry_btn: gtk::Button,
    finished: RefCell<bool>,
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

        let active_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .build();
        let active_label = gtk::Label::builder()
            .label("Active")
            .hexpand(true)
            .xalign(0.0)
            .build();
        active_label.add_css_class("caption");
        active_label.add_css_class("dim-label");
        active_header.append(&active_label);

        let active_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let history_header = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_top(18)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .build();
        let history_label = gtk::Label::builder()
            .label("History")
            .hexpand(true)
            .xalign(0.0)
            .build();
        history_label.add_css_class("caption");
        history_label.add_css_class("dim-label");
        history_header.append(&history_label);

        let history_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let active_section = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        active_section.append(&active_header);
        active_section.append(&active_list);
        active_section.set_visible(false);

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        content.append(&active_section);
        content.append(&history_header);
        content.append(&history_list);

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
        stack.add_named(&content, Some("list"));
        stack.set_visible_child_name("empty");

        let scrolled = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&stack)
            .build();

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .width_request(280)
            .build();
        root.add_css_class("command-center");
        root.append(&header);
        root.append(&scrolled);

        let this = Self {
            inner: Rc::new(Inner {
                root,
                active_section,
                active_list,
                history_list,
                empty,
                stack,
                unread: RefCell::new(0),
                unread_badge,
                external_badge: RefCell::new(None),
                retry_handler: RefCell::new(None),
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

    pub fn set_retry_handler(&self, handler: Rc<dyn Fn(RetrySpec)>) {
        *self.inner.retry_handler.borrow_mut() = Some(handler);
    }

    pub fn mark_read(&self) {
        *self.inner.unread.borrow_mut() = 0;
        self.inner.unread_badge.set_visible(false);
        if let Some(badge) = self.inner.external_badge.borrow().as_ref() {
            badge.set_visible(false);
        }
    }

    fn bump_unread(&self) {
        let mut unread = self.inner.unread.borrow_mut();
        *unread = unread.saturating_add(1);
        self.inner.unread_badge.set_label(&unread.to_string());
        self.inner.unread_badge.set_visible(true);
        if let Some(badge) = self.inner.external_badge.borrow().as_ref() {
            badge.set_label(&unread.to_string());
            badge.set_visible(true);
        }
    }

    fn now_stamp() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }

    fn format_subtitle(stamp: &str, details: &str) -> String {
        let details = details.trim();
        if details.is_empty() {
            stamp.to_string()
        } else {
            format!("{stamp} · {details}")
        }
    }

    pub fn begin_task(
        &self,
        title: impl AsRef<str>,
        details: impl AsRef<str>,
        retry: Option<RetrySpec>,
    ) -> CommandTask {
        let title = title.as_ref().trim().to_string();
        let details = details.as_ref().trim().to_string();

        let row = adw::ActionRow::builder()
            .title(&title)
            .subtitle(Self::format_subtitle(&Self::now_stamp(), &details))
            .build();
        row.add_css_class("cmd-row");

        let icon = gtk::Image::from_icon_name("content-loading-symbolic");
        icon.add_css_class("dim-label");
        row.add_prefix(&icon);

        let suffix = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .valign(gtk::Align::Center)
            .build();

        let spinner = gtk::Spinner::builder()
            .spinning(true)
            .visible(true)
            .valign(gtk::Align::Center)
            .build();
        spinner.add_css_class("row-spinner");

        let copy_btn = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy command")
            .visible(false)
            .build();
        copy_btn.add_css_class("flat");
        copy_btn.add_css_class("circular");

        let retry_btn = gtk::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Retry")
            .visible(retry.is_some())
            .build();
        retry_btn.add_css_class("flat");
        retry_btn.add_css_class("circular");

        let command_text = Rc::new(RefCell::new(String::new()));
        let retry_spec = Rc::new(RefCell::new(retry));

        let cmd_for_click = command_text.clone();
        copy_btn.connect_clicked(move |_| {
            let cmd = cmd_for_click.borrow().clone();
            if cmd.trim().is_empty() {
                return;
            }
            if let Some(display) = gtk::gdk::Display::default() {
                display.clipboard().set_text(&cmd);
                display.primary_clipboard().set_text(&cmd);
            }
        });

        let center_for_retry = self.clone();
        let retry_for_click = retry_spec.clone();
        retry_btn.connect_clicked(move |_| {
            let Some(handler) = center_for_retry.inner.retry_handler.borrow().clone() else {
                return;
            };
            let Some(spec) = retry_for_click.borrow().clone() else {
                return;
            };
            handler(spec);
        });

        suffix.append(&spinner);
        suffix.append(&retry_btn);
        suffix.append(&copy_btn);
        row.add_suffix(&suffix);

        let wrapper = gtk::ListBoxRow::new();
        wrapper.set_child(Some(&row));
        self.inner.active_list.prepend(&wrapper);
        self.inner.active_section.set_visible(true);

        self.inner.stack.set_visible_child_name("list");
        self.inner.empty.set_visible(false);

        glib::idle_add_local_once({
            let active_list = self.inner.active_list.clone();
            move || active_list.queue_allocate()
        });

        CommandTask {
            inner: Rc::new(TaskInner {
                center: self.clone(),
                wrapper,
                icon,
                spinner,
                row,
                command_text,
                retry_spec,
                copy_btn,
                retry_btn,
                finished: RefCell::new(false),
            }),
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
        let task = self.begin_task(&title, &details, None);
        task.finish(kind, &title, &details, command, true);
    }

    pub fn clear(&self) {
        while let Some(child) = self.inner.active_list.first_child() {
            self.inner.active_list.remove(&child);
        }
        while let Some(child) = self.inner.history_list.first_child() {
            self.inner.history_list.remove(&child);
        }
        self.inner.active_section.set_visible(false);
        self.inner.stack.set_visible_child_name("empty");
        self.inner.empty.set_visible(true);
        self.mark_read();
    }
}

impl CommandTask {
    pub fn finish(
        &self,
        kind: CommandEventKind,
        title: impl AsRef<str>,
        details: impl AsRef<str>,
        command: Option<String>,
        bump_unread: bool,
    ) {
        if *self.inner.finished.borrow() {
            return;
        }
        *self.inner.finished.borrow_mut() = true;

        let title = title.as_ref().trim();
        let details = details.as_ref().trim();
        let subtitle = CommandCenter::format_subtitle(&CommandCenter::now_stamp(), details);

        self.inner.row.set_title(title);
        self.inner.row.set_subtitle(&subtitle);

        let icon_name = match kind {
            CommandEventKind::Info => "dialog-information-symbolic",
            CommandEventKind::Success => "emblem-ok-symbolic",
            CommandEventKind::Error => "dialog-error-symbolic",
        };
        self.inner.icon.set_icon_name(Some(icon_name));

        self.inner.spinner.set_spinning(false);
        self.inner.spinner.set_visible(false);

        // Move from active to history on first finish.
        if let Some(list) = self
            .inner
            .wrapper
            .parent()
            .and_then(|p| p.downcast::<gtk::ListBox>().ok())
        {
            if list == self.inner.center.inner.active_list {
                self.inner
                    .center
                    .inner
                    .active_list
                    .remove(&self.inner.wrapper);
                self.inner
                    .center
                    .inner
                    .history_list
                    .prepend(&self.inner.wrapper);

                if self.inner.center.inner.active_list.first_child().is_none() {
                    self.inner.center.inner.active_section.set_visible(false);
                }
            }
        }

        if let Some(cmd) = command {
            *self.inner.command_text.borrow_mut() = cmd;
            self.inner.copy_btn.set_visible(true);
        } else {
            self.inner.copy_btn.set_visible(false);
        }

        // Retry is only meaningful for failures (or explicit info tasks).
        let can_retry = self.inner.retry_spec.borrow().is_some()
            && matches!(kind, CommandEventKind::Error | CommandEventKind::Info)
            && self.inner.center.inner.retry_handler.borrow().is_some();
        self.inner.retry_btn.set_visible(can_retry);

        if bump_unread {
            self.inner.center.bump_unread();
        }
    }
}
