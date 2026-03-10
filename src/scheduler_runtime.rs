use crate::backend::PackageManager;
use crate::models::{Config, Package, PackageStatus, ScheduledOperation, ScheduledTask};
use anyhow::{bail, Context, Result};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::process::Command;

const SCHEDULER_SERVICE_NAME: &str = "linget-scheduler.service";
const SCHEDULER_TIMER_NAME: &str = "linget-scheduler.timer";
const SCHEDULER_LOCK_FILE: &str = "scheduler.lock";
const STALE_LOCK_AGE: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerRuntimeStatus {
    SystemdUser,
    InAppFallback { reason: String },
}

#[derive(Debug)]
pub struct ScheduledTaskExecutionLock {
    path: PathBuf,
}

impl ScheduledTaskExecutionLock {
    pub fn try_acquire() -> Result<Option<Self>> {
        let dir = Config::config_dir();
        std::fs::create_dir_all(&dir)
            .context("Failed to create config directory for scheduler lock")?;
        let path = dir.join(SCHEDULER_LOCK_FILE);

        if path.exists() && lock_is_stale(&path)? {
            let _ = std::fs::remove_file(&path);
        }

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                writeln!(
                    file,
                    "pid={}\ncreated_at={:?}",
                    std::process::id(),
                    SystemTime::now()
                )
                .context("Failed to write scheduler lock file")?;
                Ok(Some(Self { path }))
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
            Err(error) => Err(error).context("Failed to acquire scheduler execution lock"),
        }
    }
}

impl Drop for ScheduledTaskExecutionLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_is_stale(path: &Path) -> Result<bool> {
    let modified = path
        .metadata()
        .context("Failed to read scheduler lock metadata")?
        .modified()
        .context("Failed to read scheduler lock timestamp")?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    Ok(age >= STALE_LOCK_AGE)
}

fn systemd_user_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("systemd")
        .join("user")
}

fn quote_systemd_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn render_systemd_service(exec_path: &Path) -> String {
    format!(
        "[Unit]\nDescription=LinGet scheduled task runner\nAfter=network-online.target\n\n[Service]\nType=oneshot\nExecStart={} schedule run-due --quiet\n",
        quote_systemd_arg(&exec_path.display().to_string())
    )
}

fn render_systemd_timer() -> String {
    format!(
        "[Unit]\nDescription=Run LinGet scheduled tasks in the background\n\n[Timer]\nOnBootSec=2min\nOnUnitActiveSec=1min\nAccuracySec=30s\nPersistent=true\nUnit={}\n\n[Install]\nWantedBy=timers.target\n",
        SCHEDULER_SERVICE_NAME
    )
}

async fn run_systemctl_user(args: &[&str]) -> Result<String> {
    let output = Command::new("systemctl")
        .args(["--user"])
        .args(args)
        .output()
        .await
        .with_context(|| format!("Failed to run systemctl --user {}", args.join(" ")))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        bail!(
            "systemd --user scheduler unavailable: {}",
            if detail.is_empty() {
                "command failed".to_string()
            } else {
                detail
            }
        );
    }
}

async fn disable_scheduler_timer_if_present() -> Result<()> {
    match run_systemctl_user(&["disable", "--now", SCHEDULER_TIMER_NAME]).await {
        Ok(_) => Ok(()),
        Err(error) => {
            let message = error.to_string();
            if message.contains("not loaded")
                || message.contains("No such file")
                || message.contains("not found")
                || message.contains("does not exist")
            {
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

pub async fn sync_systemd_runtime(has_pending_tasks: bool) -> SchedulerRuntimeStatus {
    match sync_systemd_runtime_inner(has_pending_tasks).await {
        Ok(()) => SchedulerRuntimeStatus::SystemdUser,
        Err(error) => SchedulerRuntimeStatus::InAppFallback {
            reason: error.to_string(),
        },
    }
}

async fn sync_systemd_runtime_inner(has_pending_tasks: bool) -> Result<()> {
    if which::which("systemctl").is_err() {
        bail!("systemctl is not available");
    }

    run_systemctl_user(&["show-environment"]).await?;

    let unit_dir = systemd_user_dir();
    std::fs::create_dir_all(&unit_dir).context("Failed to create systemd user unit directory")?;

    let exec_path = std::env::current_exe().context("Failed to locate LinGet executable")?;
    std::fs::write(
        unit_dir.join(SCHEDULER_SERVICE_NAME),
        render_systemd_service(&exec_path),
    )
    .context("Failed to write scheduler service unit")?;
    std::fs::write(unit_dir.join(SCHEDULER_TIMER_NAME), render_systemd_timer())
        .context("Failed to write scheduler timer unit")?;

    run_systemctl_user(&["daemon-reload"]).await?;
    if has_pending_tasks {
        run_systemctl_user(&["enable", "--now", SCHEDULER_TIMER_NAME]).await?;
    } else {
        disable_scheduler_timer_if_present().await?;
    }

    Ok(())
}

fn task_package_stub(task: &ScheduledTask, status: PackageStatus) -> Package {
    Package {
        name: task.package_name.clone(),
        version: String::new(),
        available_version: None,
        description: String::new(),
        source: task.source,
        status,
        size: None,
        homepage: None,
        license: None,
        maintainer: None,
        dependencies: Vec::new(),
        install_date: None,
        update_category: None,
        enrichment: None,
    }
}

pub async fn execute_scheduled_task(manager: &PackageManager, task: &ScheduledTask) -> Result<()> {
    match task.operation {
        ScheduledOperation::Update => {
            let updates = manager.check_all_updates().await?;
            let package = updates
                .into_iter()
                .find(|package| package.id() == task.package_id)
                .with_context(|| {
                    format!(
                        "No update is currently available for {} from {}",
                        task.package_name, task.source
                    )
                })?;
            manager.update(&package).await
        }
        ScheduledOperation::Install => {
            let package = task_package_stub(task, PackageStatus::NotInstalled);
            manager.install(&package).await
        }
        ScheduledOperation::Remove => {
            let package = manager
                .list_all_installed()
                .await?
                .into_iter()
                .find(|package| package.id() == task.package_id)
                .unwrap_or_else(|| task_package_stub(task, PackageStatus::Installed));
            manager.remove(&package).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PackageSource;

    #[test]
    fn render_systemd_service_invokes_schedule_runner() {
        let service = render_systemd_service(Path::new("/tmp/linget"));
        assert!(service.contains("ExecStart=\"/tmp/linget\" schedule run-due --quiet"));
        assert!(service.contains("Type=oneshot"));
    }

    #[test]
    fn render_systemd_timer_targets_scheduler_service() {
        let timer = render_systemd_timer();
        assert!(timer.contains("OnUnitActiveSec=1min"));
        assert!(timer.contains("Persistent=true"));
        assert!(timer.contains("Unit=linget-scheduler.service"));
    }

    #[test]
    fn task_package_stub_preserves_identity_and_status() {
        let task = ScheduledTask::new(
            "APT:demo".to_string(),
            "demo".to_string(),
            PackageSource::Apt,
            ScheduledOperation::Install,
            chrono::Utc::now(),
        );
        let package = task_package_stub(&task, PackageStatus::NotInstalled);

        assert_eq!(package.id(), "APT:demo");
        assert_eq!(package.status, PackageStatus::NotInstalled);
        assert_eq!(package.source, PackageSource::Apt);
    }
}
