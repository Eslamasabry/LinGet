//! Bulk package operations (update all, update selected, remove selected)
//!
//! This module provides async helpers to run package operations in bulk with
//! progress tracking and Command Center integration.

use crate::backend::PackageManager;
use crate::models::{Package, PackageSource};
use crate::ui::{CommandCenter, CommandEventKind, RetrySpec};
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Kind of bulk operation
#[derive(Debug, Clone, Copy)]
pub enum BulkOpKind {
    Update,
    Remove,
}

impl BulkOpKind {
    fn verb(&self) -> &'static str {
        match self {
            BulkOpKind::Update => "Updating",
            BulkOpKind::Remove => "Removing",
        }
    }

    fn past_tense(&self) -> &'static str {
        match self {
            BulkOpKind::Update => "Updated",
            BulkOpKind::Remove => "Removed",
        }
    }

    fn task_title(&self, context: &str) -> String {
        format!("{} {}", self.verb(), context)
    }
}

/// Result of a bulk operation
pub struct BulkOpResult {
    pub success_count: usize,
    pub total_count: usize,
    pub blocked_snaps: Vec<String>,
}

impl BulkOpResult {
    pub fn is_full_success(&self) -> bool {
        self.success_count == self.total_count && self.blocked_snaps.is_empty()
    }

    pub fn format_message(&self, op: BulkOpKind) -> String {
        let base = format!(
            "{} {}/{} packages",
            op.past_tense(),
            self.success_count,
            self.total_count
        );

        if self.blocked_snaps.is_empty() {
            base
        } else {
            let mut snaps = self.blocked_snaps.clone();
            snaps.sort();
            snaps.dedup();
            let shown = snaps.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            let suffix = if snaps.len() > 3 { ", …" } else { "" };
            format!("{base}. Blocked snaps: {shown}{suffix} (close running apps and retry).")
        }
    }
}

pub struct BulkOpContext {
    pub pm: Arc<Mutex<PackageManager>>,
    pub progress_overlay: gtk::Box,
    pub progress_bar: gtk::ProgressBar,
    pub progress_label: gtk::Label,
    pub command_center: CommandCenter,
    pub toast_overlay: adw::ToastOverlay,
    pub reveal_command_center: Rc<dyn Fn(bool)>,
    pub reload_packages: Rc<dyn Fn()>,
}

/// Execute a bulk operation on packages
pub async fn execute_bulk_operation(
    ctx: BulkOpContext,
    packages: Vec<Package>,
    op: BulkOpKind,
    button: Option<gtk::Button>,
) {
    if packages.is_empty() {
        if let Some(btn) = button {
            btn.set_sensitive(true);
        }
        return;
    }

    let retry_spec = match op {
        BulkOpKind::Update => RetrySpec::BulkUpdate {
            packages: packages.clone(),
        },
        BulkOpKind::Remove => RetrySpec::BulkRemove {
            packages: packages.clone(),
        },
    };

    let task = ctx.command_center.begin_task(
        op.task_title(if packages.len() == 1 {
            "package"
        } else {
            "packages"
        }),
        format!("{} packages", packages.len()),
        Some(retry_spec),
    );

    ctx.progress_overlay.set_visible(true);
    ctx.progress_label
        .set_label(&format!("{} {} packages...", op.verb(), packages.len()));

    let result = run_operation(&ctx, &packages, op).await;

    ctx.progress_overlay.set_visible(false);
    if let Some(btn) = button {
        btn.set_sensitive(true);
    }

    let msg = result.format_message(op);
    let kind = if result.is_full_success() {
        CommandEventKind::Success
    } else {
        CommandEventKind::Info
    };

    let finish_title = match op {
        BulkOpKind::Update => "Bulk update finished",
        BulkOpKind::Remove => "Bulk remove finished",
    };
    task.finish(kind, finish_title, &msg, None, true);

    if kind != CommandEventKind::Success {
        (ctx.reveal_command_center)(true);
        let toast_msg = format!("{} (see Command Center)", finish_title);
        let t = adw::Toast::new(&toast_msg);
        t.set_timeout(5);
        ctx.toast_overlay.add_toast(t);
    }

    (ctx.reload_packages)();
}

async fn run_operation(ctx: &BulkOpContext, packages: &[Package], op: BulkOpKind) -> BulkOpResult {
    let total = packages.len();
    let mut success = 0;
    let mut blocked_snaps: Vec<String> = Vec::new();

    let manager = ctx.pm.lock().await;
    for (i, pkg) in packages.iter().enumerate() {
        ctx.progress_bar.set_fraction((i as f64) / (total as f64));
        ctx.progress_bar
            .set_text(Some(&format!("{}/{} - {}", i + 1, total, pkg.name)));

        let result = match op {
            BulkOpKind::Update => manager.update(pkg).await,
            BulkOpKind::Remove => manager.remove(pkg).await,
        };

        match result {
            Ok(_) => success += 1,
            Err(e) => {
                // Track blocked snaps specifically (only relevant for updates)
                if matches!(op, BulkOpKind::Update)
                    && pkg.source == PackageSource::Snap
                    && e.to_string().contains("because it is running")
                {
                    blocked_snaps.push(pkg.name.clone());
                }
            }
        }
    }

    BulkOpResult {
        success_count: success,
        total_count: total,
        blocked_snaps,
    }
}
