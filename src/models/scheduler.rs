use chrono::{DateTime, Duration, Local, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

use super::PackageSource;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledOperation {
    Update,
    Install,
    Remove,
}

#[allow(dead_code)]
impl ScheduledOperation {
    pub fn display_name(&self) -> &'static str {
        match self {
            ScheduledOperation::Update => "Update",
            ScheduledOperation::Install => "Install",
            ScheduledOperation::Remove => "Remove",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            ScheduledOperation::Update => "software-update-available-symbolic",
            ScheduledOperation::Install => "list-add-symbolic",
            ScheduledOperation::Remove => "user-trash-symbolic",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulePreset {
    Tonight,
    TomorrowMorning,
    TomorrowEvening,
    InOneHour,
    InThreeHours,
    Custom,
}

#[allow(dead_code)]
impl SchedulePreset {
    pub fn display_name(&self) -> &'static str {
        match self {
            SchedulePreset::Tonight => "Tonight (10 PM)",
            SchedulePreset::TomorrowMorning => "Tomorrow Morning (8 AM)",
            SchedulePreset::TomorrowEvening => "Tomorrow Evening (8 PM)",
            SchedulePreset::InOneHour => "In 1 Hour",
            SchedulePreset::InThreeHours => "In 3 Hours",
            SchedulePreset::Custom => "Custom Time...",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            SchedulePreset::Tonight => "weather-clear-night-symbolic",
            SchedulePreset::TomorrowMorning => "weather-clear-symbolic",
            SchedulePreset::TomorrowEvening => "weather-clear-night-symbolic",
            SchedulePreset::InOneHour => "alarm-symbolic",
            SchedulePreset::InThreeHours => "alarm-symbolic",
            SchedulePreset::Custom => "preferences-system-time-symbolic",
        }
    }

    pub fn to_datetime(self) -> Option<DateTime<Utc>> {
        let now = Local::now();
        let today = now.date_naive();

        let local_dt = match self {
            SchedulePreset::Tonight => {
                let target_time = NaiveTime::from_hms_opt(22, 0, 0)?;
                let target = today.and_time(target_time);
                if now.naive_local() >= target {
                    (today + Duration::days(1)).and_time(target_time)
                } else {
                    target
                }
            }
            SchedulePreset::TomorrowMorning => {
                let target_time = NaiveTime::from_hms_opt(8, 0, 0)?;
                (today + Duration::days(1)).and_time(target_time)
            }
            SchedulePreset::TomorrowEvening => {
                let target_time = NaiveTime::from_hms_opt(20, 0, 0)?;
                (today + Duration::days(1)).and_time(target_time)
            }
            SchedulePreset::InOneHour => {
                return Some(Utc::now() + Duration::hours(1));
            }
            SchedulePreset::InThreeHours => {
                return Some(Utc::now() + Duration::hours(3));
            }
            SchedulePreset::Custom => return None,
        };

        local_dt
            .and_local_timezone(Local)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
    }

    pub fn all() -> &'static [SchedulePreset] {
        &[
            SchedulePreset::Tonight,
            SchedulePreset::TomorrowMorning,
            SchedulePreset::TomorrowEvening,
            SchedulePreset::InOneHour,
            SchedulePreset::InThreeHours,
            SchedulePreset::Custom,
        ]
    }

    pub fn quick_presets() -> &'static [SchedulePreset] {
        &[
            SchedulePreset::Tonight,
            SchedulePreset::TomorrowMorning,
            SchedulePreset::InOneHour,
        ]
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub package_id: String,
    pub package_name: String,
    pub source: PackageSource,
    pub operation: ScheduledOperation,
    pub scheduled_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub completed: bool,
    pub error: Option<String>,
}

#[allow(dead_code)]
impl ScheduledTask {
    pub fn new(
        package_id: String,
        package_name: String,
        source: PackageSource,
        operation: ScheduledOperation,
        scheduled_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            package_id,
            package_name,
            source,
            operation,
            scheduled_at,
            created_at: Utc::now(),
            completed: false,
            error: None,
        }
    }

    pub fn is_due(&self) -> bool {
        !self.completed && Utc::now() >= self.scheduled_at
    }

    pub fn is_pending(&self) -> bool {
        !self.completed && Utc::now() < self.scheduled_at
    }

    pub fn time_until(&self) -> String {
        if self.completed {
            return "Completed".to_string();
        }

        let now = Utc::now();
        if now >= self.scheduled_at {
            return "Due now".to_string();
        }

        let duration = self.scheduled_at.signed_duration_since(now);
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;

        if hours > 24 {
            let days = hours / 24;
            format!("In {} day{}", days, if days == 1 { "" } else { "s" })
        } else if hours > 0 {
            format!(
                "In {}h {}m",
                hours,
                if minutes > 0 {
                    format!("{}m", minutes)
                } else {
                    String::new()
                }
            )
            .trim()
            .to_string()
        } else if minutes > 0 {
            format!(
                "In {} minute{}",
                minutes,
                if minutes == 1 { "" } else { "s" }
            )
        } else {
            "In less than a minute".to_string()
        }
    }

    pub fn scheduled_time_display(&self) -> String {
        let local = self.scheduled_at.with_timezone(&Local);
        local.format("%b %d, %I:%M %p").to_string()
    }

    pub fn mark_completed(&mut self) {
        self.completed = true;
    }

    pub fn mark_failed(&mut self, error: String) {
        self.completed = true;
        self.error = Some(error);
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerState {
    pub tasks: Vec<ScheduledTask>,
}

#[allow(dead_code)]
impl SchedulerState {
    pub fn add_task(&mut self, task: ScheduledTask) {
        self.tasks.retain(|t| {
            !(t.package_id == task.package_id && t.operation == task.operation && t.is_pending())
        });
        self.tasks.push(task);
    }

    pub fn remove_task(&mut self, task_id: &str) -> Option<ScheduledTask> {
        if let Some(pos) = self.tasks.iter().position(|t| t.id == task_id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    pub fn pending_tasks(&self) -> Vec<&ScheduledTask> {
        self.tasks.iter().filter(|t| t.is_pending()).collect()
    }

    pub fn due_tasks(&self) -> Vec<&ScheduledTask> {
        self.tasks.iter().filter(|t| t.is_due()).collect()
    }

    pub fn get_pending_for_package(&self, package_id: &str) -> Option<&ScheduledTask> {
        self.tasks
            .iter()
            .find(|t| t.package_id == package_id && t.is_pending())
    }

    pub fn has_pending_schedule(&self, package_id: &str) -> bool {
        self.tasks
            .iter()
            .any(|t| t.package_id == package_id && t.is_pending())
    }

    pub fn cleanup_old_tasks(&mut self) {
        let completed: Vec<_> = self.tasks.iter().filter(|t| t.completed).cloned().collect();

        if completed.len() > 50 {
            let to_remove: Vec<_> = completed
                .iter()
                .take(completed.len() - 50)
                .map(|t| t.id.clone())
                .collect();

            self.tasks.retain(|t| !to_remove.contains(&t.id));
        }
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.is_pending()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_presets() {
        for preset in SchedulePreset::all() {
            if *preset != SchedulePreset::Custom {
                let dt = preset.to_datetime();
                assert!(
                    dt.is_some(),
                    "Preset {:?} should produce a datetime",
                    preset
                );
                assert!(
                    dt.unwrap() > Utc::now(),
                    "Preset {:?} should be in the future",
                    preset
                );
            }
        }
    }

    #[test]
    fn test_scheduled_task_is_due() {
        let past_task = ScheduledTask::new(
            "test:pkg".to_string(),
            "pkg".to_string(),
            PackageSource::Apt,
            ScheduledOperation::Update,
            Utc::now() - Duration::hours(1),
        );
        assert!(past_task.is_due());

        let future_task = ScheduledTask::new(
            "test:pkg".to_string(),
            "pkg".to_string(),
            PackageSource::Apt,
            ScheduledOperation::Update,
            Utc::now() + Duration::hours(1),
        );
        assert!(!future_task.is_due());
        assert!(future_task.is_pending());
    }
}
