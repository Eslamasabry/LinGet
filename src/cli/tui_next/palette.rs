//! Command palette: every verb that isn't one of the 13 core keys.

use crate::cli::tui_next::app::{App, ConfirmAction, Filter, Overlay};
use crate::models::history::TaskQueueAction;
use crate::models::PackageStatus;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    QueueUpdate,
    QueueAllUpdates,
    Remove,
    Changelog,
    RetryFailed,
    ReapOrphans,
    Refresh,
    FilterUpdates,
    FilterSecurity,
    FilterInstalled,
    ToggleQueue,
    Help,
    Quit,
}

pub struct PaletteCommand {
    pub title: String,
    pub hint: &'static str,
    pub action: PaletteAction,
}

/// Context-sensitive command list. Only actions that can actually run right
/// now are shown — a palette full of greyed-out entries is a failure mode.
pub fn commands_for(app: &App) -> Vec<PaletteCommand> {
    let mut commands = Vec::new();
    let current = app.cursor_package();
    let has_selection = !app.selected.is_empty();

    let target = if has_selection {
        format!("{} selected", app.selected.len())
    } else {
        current
            .map(|package| package.name.clone())
            .unwrap_or_default()
    };

    let can_update = if has_selection {
        app.selected.iter().any(|id| {
            app.package(id)
                .map(|p| p.status == PackageStatus::UpdateAvailable)
                .unwrap_or(false)
        })
    } else {
        current
            .map(|p| p.status == PackageStatus::UpdateAvailable)
            .unwrap_or(false)
    };
    let can_remove = current
        .map(|p| {
            matches!(
                p.status,
                PackageStatus::Installed | PackageStatus::UpdateAvailable
            )
        })
        .unwrap_or(false)
        || has_selection;

    if can_update {
        commands.push(PaletteCommand {
            title: format!("Queue update · {target}"),
            hint: "u",
            action: PaletteAction::QueueUpdate,
        });
    }
    let (updates, _, _) = app.counts();
    if updates > 0 {
        commands.push(PaletteCommand {
            title: format!("Queue all {updates} updates"),
            hint: "a",
            action: PaletteAction::QueueAllUpdates,
        });
    }
    if can_remove {
        commands.push(PaletteCommand {
            title: format!("Remove · {target}"),
            hint: "",
            action: PaletteAction::Remove,
        });
    }
    if current.is_some() {
        commands.push(PaletteCommand {
            title: format!("Changelog · {target}"),
            hint: "",
            action: PaletteAction::Changelog,
        });
    }

    let retryable = app.retryable_failed_count();
    if retryable > 0 {
        commands.push(PaletteCommand {
            title: format!("Retry {retryable} failed package(s)"),
            hint: "",
            action: PaletteAction::RetryFailed,
        });
    }
    let orphans = app.queue.iter().filter(|e| App::is_orphan(e)).count();
    if orphans > 0 {
        commands.push(PaletteCommand {
            title: format!("Reap {orphans} orphaned task(s)"),
            hint: "",
            action: PaletteAction::ReapOrphans,
        });
    }

    commands.push(PaletteCommand {
        title: "Refresh catalog".to_string(),
        hint: "r",
        action: PaletteAction::Refresh,
    });
    for (title, filter, action) in [
        (
            "View: updates",
            Filter::Updates,
            PaletteAction::FilterUpdates,
        ),
        (
            "View: security only",
            Filter::Security,
            PaletteAction::FilterSecurity,
        ),
        (
            "View: installed",
            Filter::Installed,
            PaletteAction::FilterInstalled,
        ),
    ] {
        if app.filter != filter {
            commands.push(PaletteCommand {
                title: title.to_string(),
                hint: "",
                action,
            });
        }
    }
    commands.push(PaletteCommand {
        title: if app.queue_open {
            "Close queue drawer".to_string()
        } else {
            "Open queue drawer".to_string()
        },
        hint: "tab",
        action: PaletteAction::ToggleQueue,
    });
    commands.push(PaletteCommand {
        title: "Help".to_string(),
        hint: "?",
        action: PaletteAction::Help,
    });
    commands.push(PaletteCommand {
        title: "Quit".to_string(),
        hint: "q",
        action: PaletteAction::Quit,
    });
    commands
}

pub async fn run(app: &mut App, action: PaletteAction) -> Result<()> {
    match action {
        PaletteAction::QueueUpdate => {
            let ids: Vec<String> = if app.selected.is_empty() {
                app.cursor_package().map(|p| p.id()).into_iter().collect()
            } else {
                app.selected.iter().cloned().collect()
            };
            let queued = app.queue_action_for(ids, TaskQueueAction::Update).await?;
            if queued > 0 {
                app.set_status(format!("queued {queued} update(s)"));
                app.selected.clear();
            }
        }
        PaletteAction::QueueAllUpdates => {
            let ids: Vec<String> = app
                .packages
                .iter()
                .filter(|p| p.status == PackageStatus::UpdateAvailable)
                .map(|p| p.id())
                .collect();
            let queued = app.queue_action_for(ids, TaskQueueAction::Update).await?;
            if queued > 0 {
                app.set_status(format!("queued {queued} update(s)"));
            }
        }
        PaletteAction::Remove => {
            let target = if app.selected.is_empty() {
                app.cursor_package()
                    .map(|p| p.name.clone())
                    .unwrap_or_default()
            } else {
                format!("{} package(s)", app.selected.len())
            };
            app.overlay = Some(Overlay::Confirm {
                title: "Confirm removal".to_string(),
                body: format!("Remove {target}? This cannot be undone."),
                action: ConfirmAction::RemoveSelected,
            });
        }
        PaletteAction::Changelog => {
            open_changelog(app).await?;
        }
        PaletteAction::RetryFailed => {
            app.retry_failed().await?;
        }
        PaletteAction::ReapOrphans => {
            app.reap_orphans().await?;
        }
        PaletteAction::Refresh => {
            app.refresh();
        }
        PaletteAction::FilterUpdates => set_filter(app, Filter::Updates),
        PaletteAction::FilterSecurity => set_filter(app, Filter::Security),
        PaletteAction::FilterInstalled => set_filter(app, Filter::Installed),
        PaletteAction::ToggleQueue => {
            app.queue_open = !app.queue_open;
        }
        PaletteAction::Help => {
            app.overlay = Some(Overlay::Help);
        }
        PaletteAction::Quit => {
            app.request_quit();
        }
    }
    Ok(())
}

fn set_filter(app: &mut App, filter: Filter) {
    app.filter = filter;
    app.expanded = None;
    app.rebuild_rows();
}

async fn open_changelog(app: &mut App) -> Result<()> {
    let Some(package) = app.cursor_package() else {
        return Ok(());
    };
    let name = package.name.clone();
    let id = package.id();
    app.set_status(format!("fetching changelog for {name}…"));

    let pm = app.pm.clone();
    let package = match app.package(&id) {
        Some(package) => package.clone(),
        None => return Ok(()),
    };
    let result = {
        let guard = pm.read().await;
        guard.get_changelog(&package).await
    };
    match result {
        Ok(Some(text)) => {
            let lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();
            if lines.is_empty() {
                app.set_status(format!("no changelog for {name}"));
            } else {
                app.overlay = Some(Overlay::Changelog {
                    title: format!("Changelog · {name}"),
                    lines,
                    scroll: 0,
                });
            }
        }
        Ok(None) => {
            app.set_status(format!("no changelog for {name}"));
        }
        Err(error) => {
            app.set_status(format!("changelog failed: {error}"));
        }
    }
    Ok(())
}
