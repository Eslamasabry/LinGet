use crate::backend::PackageManager;
use crate::cli::{OutputWriter, ScheduleAction};
use crate::models::Config;
use crate::scheduler_runtime::{
    execute_scheduled_task, sync_systemd_runtime, ScheduledTaskExecutionLock,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(
    pm: Arc<Mutex<PackageManager>>,
    action: ScheduleAction,
    writer: &OutputWriter,
) -> Result<()> {
    match action {
        ScheduleAction::RunDue => run_due(pm, writer).await,
    }
}

async fn run_due(pm: Arc<Mutex<PackageManager>>, writer: &OutputWriter) -> Result<()> {
    let Some(_lock) = ScheduledTaskExecutionLock::try_acquire()? else {
        writer.verbose("Another LinGet scheduler execution is already running");
        return Ok(());
    };

    let mut config = Config::load();
    let mut due_task_ids: Vec<String> = config
        .scheduler
        .due_tasks()
        .into_iter()
        .map(|task| task.id.clone())
        .collect();
    due_task_ids.sort_by_key(|task_id| {
        config
            .scheduler
            .tasks
            .iter()
            .find(|task| task.id == *task_id)
            .map(|task| task.scheduled_at)
    });

    if due_task_ids.is_empty() {
        writer.verbose("No scheduled tasks are due");
        return Ok(());
    }

    {
        let mut manager = pm.lock().await;
        manager.set_enabled_sources(config.enabled_sources.to_sources());
    }

    let mut completed = 0usize;
    let mut failed = 0usize;

    for task_id in due_task_ids {
        let task_opt = config
            .scheduler
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .cloned();
        let Some(task) = task_opt else {
            continue;
        };

        let result = {
            let manager = pm.lock().await;
            execute_scheduled_task(&manager, &task).await
        };

        match result {
            Ok(()) => {
                if let Some(task) = config
                    .scheduler
                    .tasks
                    .iter_mut()
                    .find(|entry| entry.id == task_id)
                {
                    task.mark_completed();
                }
                completed += 1;
            }
            Err(error) => {
                if let Some(task) = config
                    .scheduler
                    .tasks
                    .iter_mut()
                    .find(|entry| entry.id == task_id)
                {
                    task.mark_failed(error.to_string());
                }
                failed += 1;
            }
        }

        config.scheduler.cleanup_old_tasks();
        config.save()?;
    }

    if failed == 0 {
        writer.success(&format!(
            "Ran {} scheduled task{}",
            completed,
            if completed == 1 { "" } else { "s" }
        ));
    } else if completed == 0 {
        writer.warning(&format!(
            "{} scheduled task{} failed",
            failed,
            if failed == 1 { "" } else { "s" }
        ));
    } else {
        writer.warning(&format!(
            "Ran {} scheduled task{} with {} failure{}",
            completed,
            if completed == 1 { "" } else { "s" },
            failed,
            if failed == 1 { "" } else { "s" }
        ));
    }

    let _ = sync_systemd_runtime(config.scheduler.pending_count() > 0).await;

    Ok(())
}
