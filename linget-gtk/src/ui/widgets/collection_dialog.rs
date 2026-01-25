use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;
use relm4::prelude::*;

#[allow(dead_code)]
pub struct CollectionDialogInit {
    pub parent: gtk::Window,
}

#[derive(Debug)]
pub enum CollectionDialogInput {
    Show,
    Hide,
    NameChanged(String),
    Confirm,
}

#[derive(Debug, Clone)]
pub enum CollectionDialogOutput {
    Created(String),
}

pub struct CollectionDialogModel {
    name: String,
    visible: bool,
}

pub struct CollectionDialogWidgets {
    dialog: adw::Dialog,
    entry: adw::EntryRow,
    create_btn: gtk::Button,
}

impl SimpleComponent for CollectionDialogModel {
    type Init = CollectionDialogInit;
    type Input = CollectionDialogInput;
    type Output = CollectionDialogOutput;
    type Root = adw::Dialog;
    type Widgets = CollectionDialogWidgets;

    fn init_root() -> Self::Root {
        adw::Dialog::builder()
            .title("New Collection")
            .content_width(360)
            .build()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = CollectionDialogModel {
            name: String::new(),
            visible: false,
        };

        let toolbar_view = adw::ToolbarView::new();

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .show_end_title_buttons(false)
            .build();

        let cancel_btn = gtk::Button::builder().label("Cancel").build();
        cancel_btn.add_css_class("flat");

        let create_btn = gtk::Button::builder()
            .label("Create")
            .sensitive(false)
            .build();
        create_btn.add_css_class("suggested-action");

        header.pack_start(&cancel_btn);
        header.pack_end(&create_btn);
        toolbar_view.add_top_bar(&header);

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(24)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let icon = gtk::Image::builder()
            .icon_name("folder-new-symbolic")
            .pixel_size(64)
            .build();
        icon.add_css_class("dim-label");

        let title = gtk::Label::builder()
            .label("Create a new collection")
            .build();
        title.add_css_class("title-2");

        let subtitle = gtk::Label::builder()
            .label("Collections help you organize your favorite apps")
            .build();
        subtitle.add_css_class("dim-label");

        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(vec!["boxed-list"])
            .build();

        let entry = adw::EntryRow::builder().title("Collection Name").build();

        list.append(&entry);

        content.append(&icon);
        content.append(&title);
        content.append(&subtitle);
        content.append(&list);

        toolbar_view.set_content(Some(&content));
        root.set_child(Some(&toolbar_view));

        let sender_entry = sender.clone();
        entry.connect_changed(move |e| {
            sender_entry.input(CollectionDialogInput::NameChanged(e.text().to_string()));
        });

        entry.connect_entry_activated({
            let sender = sender.clone();
            move |_| {
                sender.input(CollectionDialogInput::Confirm);
            }
        });

        let sender_cancel = sender.clone();
        cancel_btn.connect_clicked(move |_| {
            sender_cancel.input(CollectionDialogInput::Hide);
        });

        let sender_create = sender.clone();
        create_btn.connect_clicked(move |_| {
            sender_create.input(CollectionDialogInput::Confirm);
        });

        root.connect_closed({
            let sender = sender.clone();
            move |_| {
                sender.input(CollectionDialogInput::Hide);
            }
        });

        let widgets = CollectionDialogWidgets {
            dialog: root.clone(),
            entry,
            create_btn,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            CollectionDialogInput::Show => {
                self.visible = true;
                self.name.clear();
            }
            CollectionDialogInput::Hide => {
                self.visible = false;
                self.name.clear();
            }
            CollectionDialogInput::NameChanged(name) => {
                self.name = name;
            }
            CollectionDialogInput::Confirm => {
                let name = self.name.trim().to_string();
                if !name.is_empty() {
                    sender.output(CollectionDialogOutput::Created(name)).ok();
                    self.visible = false;
                    self.name.clear();
                }
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        widgets
            .create_btn
            .set_sensitive(!self.name.trim().is_empty());

        if !self.visible {
            widgets.entry.set_text("");
            widgets.dialog.close();
        }
    }
}

impl CollectionDialogModel {
    pub fn present(&self, widgets: &CollectionDialogWidgets, parent: &impl IsA<gtk::Widget>) {
        widgets.dialog.present(Some(parent));
        widgets.entry.grab_focus();
    }
}
