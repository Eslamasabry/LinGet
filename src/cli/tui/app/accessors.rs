use super::*;

impl App {
    pub fn is_catalog_busy(&self) -> bool {
        self.catalog_activity.is_some()
    }

    pub fn catalog_activity_label(&self) -> Option<String> {
        match &self.catalog_activity {
            Some(CatalogActivity::RefreshingPackages) => Some("Refreshing packages".to_string()),
            Some(CatalogActivity::SearchingProviders { query }) => {
                Some(format!("Searching providers for '{}'", query))
            }
            None => None,
        }
    }

    pub fn catalog_loading_message(&self) -> String {
        match &self.catalog_activity {
            Some(CatalogActivity::RefreshingPackages) => "Loading packages...".to_string(),
            Some(CatalogActivity::SearchingProviders { query }) => {
                format!("Searching providers for '{}'...", query)
            }
            None => "Loading packages...".to_string(),
        }
    }

    pub fn catalog_busy_reason(&self) -> String {
        match &self.catalog_activity {
            Some(CatalogActivity::RefreshingPackages) => {
                "Please wait for the package refresh to finish".to_string()
            }
            Some(CatalogActivity::SearchingProviders { query }) => {
                format!(
                    "Please wait for the current provider search for '{}' to finish",
                    query
                )
            }
            None => "Please wait for the current operation to finish".to_string(),
        }
    }

    pub fn tui_mode_label(&self) -> &'static str {
        if self.showing_palette {
            "Palette"
        } else if self.showing_changelog {
            "Changelog"
        } else if self.showing_help {
            "Help"
        } else if self.confirming.is_some() {
            "Review Action"
        } else if self.showing_import_preview {
            "Import Preview"
        } else if self.queue_expanded && self.focus == Focus::Queue {
            "Queue Focus"
        } else if self.searching {
            "Search Input"
        } else if self.search_results.is_some() && !self.search.is_empty() {
            "Provider Results"
        } else if !self.search.is_empty() {
            "Local Filter"
        } else {
            match self.view_mode {
                ViewMode::Dashboard => "Dashboard",
                ViewMode::Queue => "Queue Overview",
                ViewMode::Browse => match self.focus {
                    Focus::Sources => "Source Browse",
                    Focus::Packages | Focus::Queue => "Package Browse",
                },
            }
        }
    }

    pub fn search_query_hint_label(&self) -> &'static str {
        if self.searching || !self.search.is_empty() {
            "edit query"
        } else {
            "search"
        }
    }

    pub fn search_escape_hint_label(&self) -> &'static str {
        if self.search_results.is_some() && !self.search.is_empty() {
            "local filter"
        } else {
            "clear search"
        }
    }

    pub fn search_current_scope_label(&self) -> String {
        if self.search_results.is_some() {
            self.provider_search_scope_label()
                .unwrap_or_else(|| "provider results".to_string())
        } else {
            "local package list".to_string()
        }
    }

    pub fn search_typing_hint_text(&self) -> &'static str {
        if self.search_results.is_some() {
            "Typing resumes filtering in the local package list"
        } else {
            "Typing filters the local package list"
        }
    }

    pub fn source_count(&self) -> usize {
        self.visible_sources().len() + 1
    }

    pub fn visible_sources(&self) -> Vec<PackageSource> {
        match self.filter {
            Filter::Updates | Filter::Favorites | Filter::SecurityUpdates | Filter::Duplicates => {
                let count_index = match self.filter {
                    Filter::Updates => FILTER_UPDATES_INDEX,
                    Filter::Favorites => FILTER_FAVORITES_INDEX,
                    Filter::SecurityUpdates => FILTER_SECURITY_INDEX,
                    Filter::Duplicates => FILTER_DUPLICATES_INDEX,
                    _ => unreachable!(),
                };
                self.available_sources
                    .iter()
                    .filter(|source| {
                        self.source_counts
                            .get(source)
                            .is_some_and(|counts| counts[count_index] > 0)
                    })
                    .copied()
                    .collect()
            }
            Filter::All | Filter::Installed => self.available_sources.clone(),
        }
    }

    pub fn retry_attempt_for_task(&self, task_id: &str) -> Option<usize> {
        self.retry_attempt.get(task_id).copied()
    }

    pub fn task_last_log_age_secs(&self, task_id: &str) -> Option<u64> {
        self.task_last_log_at
            .get(task_id)
            .map(|instant| instant.elapsed().as_secs())
    }

    pub fn queue_lane_for_task(&self, task: &TaskQueueEntry) -> QueueJourneyLane {
        match task.status {
            TaskQueueStatus::Running => QueueJourneyLane::Now,
            TaskQueueStatus::Queued => QueueJourneyLane::Next,
            TaskQueueStatus::Completed | TaskQueueStatus::Cancelled => QueueJourneyLane::Done,
            TaskQueueStatus::Failed => {
                let recovered = self
                    .recovery_state_for_task(&task.id)
                    .is_some_and(|state| state.last_outcome == Some(TaskQueueStatus::Completed));
                if recovered {
                    QueueJourneyLane::Done
                } else {
                    QueueJourneyLane::NeedsAttention
                }
            }
        }
    }

    pub fn queue_lane_counts(&self) -> (usize, usize, usize, usize) {
        let mut now = 0usize;
        let mut next = 0usize;
        let mut attention = 0usize;
        let mut done = 0usize;
        for task in &self.tasks {
            match self.queue_lane_for_task(task) {
                QueueJourneyLane::Now => now += 1,
                QueueJourneyLane::Next => next += 1,
                QueueJourneyLane::NeedsAttention => attention += 1,
                QueueJourneyLane::Done => done += 1,
            }
        }
        (now, next, attention, done)
    }

    pub fn queue_failure_filter_label(&self) -> &'static str {
        self.queue_failure_filter.label()
    }

    pub fn queue_visible_task_indices(&self) -> Vec<usize> {
        if self.queue_failure_filter == QueueFailureFilter::All {
            return (0..self.tasks.len()).collect();
        }

        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention)
            .filter(|(_, task)| {
                self.failure_category_for_task(task)
                    .is_some_and(|category| self.queue_failure_filter.matches(category))
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn queue_visible_cursor_position(&self, visible_indices: &[usize]) -> usize {
        visible_indices
            .iter()
            .position(|index| *index == self.task_cursor)
            .unwrap_or(0)
    }

    pub fn unresolved_failure_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention)
            .count()
    }

    pub fn retryable_failed_task_count(&self) -> usize {
        let mut seen = HashSet::new();
        self.tasks
            .iter()
            .filter(|task| {
                self.queue_lane_for_task(task) == QueueJourneyLane::NeedsAttention
                    && self
                        .failure_category_for_task(task)
                        .is_some_and(Self::safe_retry_category)
                    && !self.has_active_task_for_package(&task.package_id)
                    && seen.insert(task.package_id.clone())
            })
            .count()
    }

    pub fn current_package(&self) -> Option<&Package> {
        self.filtered
            .get(self.cursor)
            .and_then(|idx| self.packages.get(*idx))
    }

    pub(super) fn current_package_id(&self) -> Option<String> {
        self.current_package().map(Package::id)
    }

    fn package_by_id(&self, package_id: &str) -> Option<&Package> {
        self.packages
            .iter()
            .find(|package| package.id() == package_id)
    }

    pub fn changelog_target_package(&self) -> Option<&Package> {
        self.changelog_target_package_id
            .as_deref()
            .and_then(|package_id| self.package_by_id(package_id))
    }

    pub fn changelog_state_for_target(&self) -> Option<&ChangelogState> {
        self.changelog_target_package_id
            .as_ref()
            .and_then(|package_id| self.changelog_cache.get(package_id))
    }

    pub fn changelog_state_for_current_package(&self) -> Option<&ChangelogState> {
        self.current_package()
            .and_then(|package| self.changelog_cache.get(&package.id()))
    }

    pub fn changelog_supported_for_target(&self) -> bool {
        self.changelog_target_package()
            .is_some_and(|package| Self::changelog_supported_for_source(package.source))
    }

    pub fn visible_selected_count(&self) -> usize {
        self.filtered
            .iter()
            .filter(|idx| {
                self.packages
                    .get(**idx)
                    .is_some_and(|p| self.selected.contains(&p.id()))
            })
            .count()
    }

    pub fn hidden_selected_count(&self) -> usize {
        self.selected
            .len()
            .saturating_sub(self.visible_selected_count())
    }

    pub fn is_favorite_id(&self, package_id: &str) -> bool {
        self.favorite_packages.contains(package_id)
    }

    pub fn provider_search_scope_label(&self) -> Option<String> {
        self.search_results.as_ref()?;

        let source_count = self
            .search_provider_summaries
            .iter()
            .filter(|provider| provider.result_count > 0 && provider.error.is_none())
            .count();
        Some(if source_count == 0 {
            "provider results".to_string()
        } else {
            format!(
                "provider results · {} source{}",
                source_count,
                if source_count == 1 { "" } else { "s" }
            )
        })
    }

    pub fn provider_search_summary(&self) -> Option<String> {
        self.search_results.as_ref()?;
        if self.search_provider_summaries.is_empty() {
            return None;
        }

        let mut parts = Vec::new();
        for provider in self
            .search_provider_summaries
            .iter()
            .filter(|provider| provider.result_count > 0 && provider.error.is_none())
            .take(3)
        {
            parts.push(format!("{} {}", provider.source, provider.result_count));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    }

    pub fn package_source_badge(&self, package: &Package) -> String {
        let alternative_count = self
            .search_source_alternatives
            .get(&package.id())
            .map(|sources| sources.len())
            .unwrap_or(0);

        if alternative_count == 0 {
            package.source.to_string()
        } else {
            format!("{}+{}", package.source, alternative_count)
        }
    }

    pub fn package_source_note(&self, package: &Package) -> Option<String> {
        let alternative_sources = self.search_source_alternatives.get(&package.id())?;
        if alternative_sources.is_empty() {
            return None;
        }

        let mut labels: Vec<String> = alternative_sources
            .iter()
            .map(|source| source.to_string())
            .collect();
        labels.sort();
        Some(format!("Also available from {}", labels.join(", ")))
    }
}
