use gtk4 as gtk;
use ksni::{self, menu::StandardItem, MenuItem, Tray, TrayService};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

/// Actions that can be triggered from the system tray
#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    ShowWindow,
    CheckUpdates,
    Quit,
}

#[allow(dead_code)]
pub struct TrayState {
    pub updates_count: AtomicU32,
    pub window_visible: AtomicBool,
    action_sender: std::sync::mpsc::Sender<TrayAction>,
}

impl TrayState {
    #[allow(dead_code)]
    pub fn set_updates_count(&self, count: u32) {
        self.updates_count.store(count, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn set_window_visible(&self, visible: bool) {
        self.window_visible.store(visible, Ordering::SeqCst);
    }
}

struct LinGetTray {
    state: Arc<TrayState>,
}

impl Tray for LinGetTray {
    fn icon_name(&self) -> String {
        // Try custom icon first, fall back to standard icon if not found
        let custom = "io.github.linget";
        if let Some(display) = gtk::gdk::Display::default() {
            if gtk::IconTheme::for_display(&display).has_icon(custom) {
                return custom.to_string();
            }
        }
        // Fallback to standard system icon
        "system-software-install".to_string()
    }

    fn title(&self) -> String {
        "LinGet".to_string()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let count = self.state.updates_count.load(Ordering::SeqCst);
        let description = if count > 0 {
            format!("{} updates available", count)
        } else {
            "Package manager".to_string()
        };

        ksni::ToolTip {
            icon_name: self.icon_name(),
            icon_pixmap: Vec::new(),
            title: "LinGet".to_string(),
            description,
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let window_visible = self.state.window_visible.load(Ordering::SeqCst);
        let updates_count = self.state.updates_count.load(Ordering::SeqCst);

        let show_label = if window_visible {
            "Hide Window"
        } else {
            "Show Window"
        };

        let updates_label = if updates_count > 0 {
            format!("Check Updates ({})", updates_count)
        } else {
            "Check Updates".to_string()
        };

        vec![
            StandardItem {
                label: show_label.to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.state.action_sender.send(TrayAction::ShowWindow);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: updates_label,
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.state.action_sender.send(TrayAction::CheckUpdates);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.state.action_sender.send(TrayAction::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.state.action_sender.send(TrayAction::ShowWindow);
    }
}

#[allow(dead_code)]
pub struct TrayHandle {
    pub state: Arc<TrayState>,
    pub action_receiver: std::sync::mpsc::Receiver<TrayAction>,
    _service: TrayService<LinGetTray>,
}

impl TrayHandle {
    /// Start the system tray service
    pub fn start() -> Option<Self> {
        let (sender, receiver) = std::sync::mpsc::channel();

        let state = Arc::new(TrayState {
            updates_count: AtomicU32::new(0),
            window_visible: AtomicBool::new(true),
            action_sender: sender,
        });

        let tray = LinGetTray {
            state: state.clone(),
        };

        // TrayService::new returns the service directly (panics on failure)
        // We catch_unwind to handle potential D-Bus connection failures gracefully
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| TrayService::new(tray)));

        match result {
            Ok(service) => {
                let handle = service.handle();
                handle.update(|_| {}); // Initial update

                Some(TrayHandle {
                    state,
                    action_receiver: receiver,
                    _service: service,
                })
            }
            Err(_) => {
                tracing::warn!("Failed to start system tray (no D-Bus session?)");
                None
            }
        }
    }
}
