//! Bulk package operations (update all, update selected, remove selected)
//!
//! This module provides async helpers to run package operations in bulk with
//! progress tracking and Command Center integration.

use crate::backend::PackageManager;
use crate::models::{Package, PackageSource};
use crate::ui::{CommandCenter, CommandEventKind, ErrorDisplay, RetrySpec};
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

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

/// Details about a failed operation
#[derive(Debug, Clone)]
pub struct FailedOp {
    pub package_name: String,
    pub source: PackageSource,
    pub error: String,
}

/// Result of a bulk operation
pub struct BulkOpResult {
    pub success_count: usize,
    pub total_count: usize,
    pub blocked_snaps: Vec<String>,
    pub failed_ops: Vec<FailedOp>,
    pub auth_cancelled: bool,
}

impl BulkOpResult {
    pub fn is_full_success(&self) -> bool {
        self.success_count == self.total_count && self.blocked_snaps.is_empty()
    }

    pub fn is_all_cancelled(&self) -> bool {
        self.auth_cancelled && self.success_count == 0
    }

    pub fn format_message(&self, op: BulkOpKind) -> String {
        // If all operations were cancelled due to auth, give a clear message
        if self.is_all_cancelled() {
            return "Operation cancelled (authorization denied)".to_string();
        }

        let base = format!(
            "{} {}/{} packages",
            op.past_tense(),
            self.success_count,
            self.total_count
        );

        let mut messages = vec![base];

        // Add blocked snaps info
        if !self.blocked_snaps.is_empty() {
            let mut snaps = self.blocked_snaps.clone();
            snaps.sort();
            snaps.dedup();
            let shown = snaps.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            let suffix = if snaps.len() > 3 { ", …" } else { "" };
            messages.push(format!(
                "Blocked snaps: {shown}{suffix} (close running apps and retry)"
            ));
        }

        // Add failed operations summary
        if !self.failed_ops.is_empty() {
            let failed_count = self.failed_ops.len();
            if failed_count <= 3 {
                for fail in &self.failed_ops {
                    messages.push(format!(
                        "  - {} ({}): {}",
                        fail.package_name, fail.source, fail.error
                    ));
                }
            } else {
                messages.push(format!(
                    "{} packages failed (see logs for details)",
                    failed_count
                ));
            }
        }

        messages.join("\n")
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

    // Determine the result type and message
    let (kind, finish_title, show_command_center) = if result.is_all_cancelled() {
        // User cancelled the operation - just show a brief info
        (
            CommandEventKind::Info,
            "Operation cancelled",
            false, // Don't force open command center for cancellations
        )
    } else if result.is_full_success() {
        (
            CommandEventKind::Success,
            match op {
                BulkOpKind::Update => "Bulk update finished",
                BulkOpKind::Remove => "Bulk remove finished",
            },
            false,
        )
    } else {
        // Some failures occurred
        (
            CommandEventKind::Error,
            match op {
                BulkOpKind::Update => "Bulk update completed with errors",
                BulkOpKind::Remove => "Bulk remove completed with errors",
            },
            true, // Show command center for errors
        )
    };

    let msg = result.format_message(op);
    task.finish(kind, finish_title, &msg, None, !result.is_all_cancelled());

    if show_command_center {
        (ctx.reveal_command_center)(true);
        let toast_msg = format!("{} (see Command Center)", finish_title);
        let t = adw::Toast::new(&toast_msg);
        t.set_timeout(5);
        ctx.toast_overlay.add_toast(t);
    } else if result.is_all_cancelled() {
        // Show a brief toast for cancellations
        let t = adw::Toast::new("Operation cancelled");
        t.set_timeout(2);
        ctx.toast_overlay.add_toast(t);
    }

    (ctx.reload_packages)();
}

async fn run_operation(ctx: &BulkOpContext, packages: &[Package], op: BulkOpKind) -> BulkOpResult {
    let total = packages.len();
    let mut success = 0;
    let mut blocked_snaps: Vec<String> = Vec::new();
    let mut failed_ops: Vec<FailedOp> = Vec::new();
    let mut auth_cancelled = false;

    info!(
        operation = ?op,
        package_count = total,
        "Starting bulk operation"
    );

    let manager = ctx.pm.lock().await;
    for (i, pkg) in packages.iter().enumerate() {
        ctx.progress_bar.set_fraction((i as f64) / (total as f64));
        ctx.progress_bar
            .set_text(Some(&format!("{}/{} - {}", i + 1, total, pkg.name)));

        debug!(
            package = %pkg.name,
            source = ?pkg.source,
            progress = format!("{}/{}", i + 1, total),
            "Processing package"
        );

        let result = match op {
            BulkOpKind::Update => manager.update(pkg).await,
            BulkOpKind::Remove => manager.remove(pkg).await,
        };

        match result {
            Ok(_) => {
                success += 1;
                debug!(package = %pkg.name, "Package operation succeeded");
            }
            Err(e) => {
                let error_msg = e.to_string();
                let error_display = ErrorDisplay::from_anyhow(&e);

                // Check if this is an auth cancellation
                if error_display.is_cancelled {
                    auth_cancelled = true;
                    warn!(
                        package = %pkg.name,
                        "Operation cancelled by user"
                    );
                    // If auth was cancelled, we might want to stop the whole operation
                    // For now, we continue but mark it
                } else {
                    error!(
                        package = %pkg.name,
                        source = ?pkg.source,
                        error = %error_msg,
                        "Package operation failed"
                    );

                    // Track blocked snaps specifically (only relevant for updates)
                    if matches!(op, BulkOpKind::Update)
                        && pkg.source == PackageSource::Snap
                        && error_msg.contains("because it is running")
                    {
                        blocked_snaps.push(pkg.name.clone());
                    }

                    // Track all failures for detailed reporting
                    failed_ops.push(FailedOp {
                        package_name: pkg.name.clone(),
                        source: pkg.source,
                        error: error_display.title,
                    });
                }
            }
        }
    }

    info!(
        operation = ?op,
        success_count = success,
        failed_count = failed_ops.len(),
        blocked_snaps = blocked_snaps.len(),
        auth_cancelled = auth_cancelled,
        "Bulk operation completed"
    );

    BulkOpResult {
        success_count: success,
        total_count: total,
        blocked_snaps,
        failed_ops,
        auth_cancelled,
    }
}
