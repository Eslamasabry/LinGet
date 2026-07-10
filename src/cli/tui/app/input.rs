use super::*;
impl App {
    async fn handle_changelog_key(&mut self, key: KeyEvent) {
        const CHANGELOG_STEP: usize = 3;
        const CHANGELOG_PAGE: usize = 14;

        match key.code {
            _ if key.code == KeyCode::Char('d')
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.changelog_scroll = self.changelog_scroll.saturating_add(CHANGELOG_PAGE);
            }
            _ if key.code == KeyCode::Char('u')
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.changelog_scroll = self.changelog_scroll.saturating_sub(CHANGELOG_PAGE);
            }
            KeyCode::Esc | KeyCode::Char('c') => self.close_changelog_overlay(),
            KeyCode::Char('r') => self.refresh_changelog_overlay().await,
            KeyCode::Char('u') | KeyCode::Char('U') => {
                self.queue_changelog_action(TaskQueueAction::Update);
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                self.queue_changelog_action(TaskQueueAction::Install);
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Char('x') | KeyCode::Char('X') => {
                self.queue_changelog_action(TaskQueueAction::Remove);
            }
            KeyCode::Char('v') => {
                self.changelog_diff_only = !self.changelog_diff_only;
                self.changelog_scroll = 0;
                if self.changelog_diff_only {
                    self.set_status("Changelog mode: version delta", true);
                } else {
                    self.set_status("Changelog mode: full history", true);
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.changelog_scroll = self.changelog_scroll.saturating_add(CHANGELOG_STEP);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.changelog_scroll = self.changelog_scroll.saturating_sub(CHANGELOG_STEP);
            }
            KeyCode::PageDown => {
                self.changelog_scroll = self.changelog_scroll.saturating_add(CHANGELOG_PAGE);
            }
            KeyCode::PageUp => {
                self.changelog_scroll = self.changelog_scroll.saturating_sub(CHANGELOG_PAGE);
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.changelog_scroll = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.changelog_scroll = usize::MAX / 2;
            }
            _ => {}
        }
    }

    fn queue_changelog_action(&mut self, action: TaskQueueAction) {
        let Some(package) = self.changelog_target_package().cloned() else {
            self.set_status("Package details are no longer available", true);
            return;
        };

        let had_confirming = self.confirming.is_some();
        self.prepare_action_for_targets(action, vec![package], false);
        if !had_confirming && self.confirming.is_some() {
            self.close_changelog_overlay();
        }
    }

    pub async fn execute_command(&mut self, command: CommandId) {
        let allow_invalid_execution = matches!(
            command,
            CommandId::Install | CommandId::Remove | CommandId::Update
        );

        if !allow_invalid_execution && !self.command_enabled(command) {
            if let Some(reason) = self.command_disabled_reason(command) {
                self.set_status(reason, true);
            }
            return;
        }

        match command {
            CommandId::Quit => {
                let armed_recently = self
                    .quit_armed_at
                    .is_some_and(|at| at.elapsed() < super::QUIT_CONFIRM_WINDOW);
                if self.has_active_queue_tasks() && !armed_recently {
                    self.quit_armed_at = Some(Instant::now());
                    self.set_status(
                        "Tasks are still running — quit again within 3s to quit anyway",
                        true,
                    );
                } else {
                    self.should_quit = true;
                }
            }
            CommandId::ShowHelp => {
                self.showing_help = true;
                self.help_scroll = 0;
            }
            CommandId::OpenPalette => self.open_palette(),
            CommandId::CycleFocus => {
                let effective_view = if self.is_queue_view() {
                    ViewMode::Queue
                } else {
                    self.view_mode
                };
                self.focus = match effective_view {
                    ViewMode::Queue => Focus::Queue,
                    ViewMode::Today => Focus::Packages,
                    ViewMode::Browse if self.layout_tier().shows_sources() => match self.focus {
                        Focus::Sources => Focus::Packages,
                        Focus::Packages | Focus::Queue => Focus::Sources,
                    },
                    ViewMode::Browse => Focus::Packages,
                };
            }
            CommandId::MoveUp => {
                if self.queue_focus_active() {
                    self.queue_prev();
                } else {
                    match self.focus {
                        Focus::Sources => self.prev_source(),
                        Focus::Packages | Focus::Queue => self.prev_package(),
                    }
                }
            }
            CommandId::MoveDown => {
                if self.queue_focus_active() {
                    self.queue_next();
                } else {
                    match self.focus {
                        Focus::Sources => self.next_source(),
                        Focus::Packages | Focus::Queue => self.next_package(),
                    }
                }
            }
            CommandId::MoveTop => {
                if self.queue_focus_active() {
                    self.queue_top();
                } else {
                    match self.focus {
                        Focus::Sources => self.set_source_by_index(0),
                        Focus::Packages | Focus::Queue => self.top(),
                    }
                }
            }
            CommandId::MoveBottom => {
                if self.queue_focus_active() {
                    self.queue_bottom();
                } else {
                    match self.focus {
                        Focus::Sources => {
                            self.set_source_by_index(self.source_count().saturating_sub(1))
                        }
                        Focus::Packages | Focus::Queue => self.bottom(),
                    }
                }
            }
            CommandId::PageUp => {
                if self.queue_focus_active() {
                    self.queue_page_up();
                } else if self.focus == Focus::Sources {
                    self.source_page_up();
                } else {
                    self.page_up();
                }
            }
            CommandId::PageDown => {
                if self.queue_focus_active() {
                    self.queue_page_down();
                } else if self.focus == Focus::Sources {
                    self.source_page_down();
                } else {
                    self.page_down();
                }
            }
            CommandId::FilterAll => {
                self.filter = Filter::All;
                self.apply_filters();
            }
            CommandId::FilterInstalled => {
                self.filter = Filter::Installed;
                self.apply_filters();
            }
            CommandId::FilterUpdates => {
                self.filter = Filter::Updates;
                self.apply_filters();
            }
            CommandId::FilterFavorites => {
                self.filter = Filter::Favorites;
                self.apply_filters();
            }
            CommandId::FilterSecurityUpdates => {
                self.filter = Filter::SecurityUpdates;
                self.apply_filters();
            }
            CommandId::FilterDuplicates => {
                self.filter = Filter::Duplicates;
                self.apply_filters();
            }
            CommandId::ToggleFavorite => self.toggle_favorite_on_cursor(),
            CommandId::BulkToggleFavorite => self.bulk_toggle_favorites(),
            CommandId::ToggleFavoritesUpdatesOnly => self.toggle_favorites_updates_only(),
            CommandId::ToggleSelection => self.toggle_selection_on_cursor(),
            CommandId::SelectAll => self.select_all_visible(),
            CommandId::Install => self.prepare_action(TaskQueueAction::Install),
            CommandId::Remove => self.prepare_action(TaskQueueAction::Remove),
            CommandId::Update => self.prepare_action(TaskQueueAction::Update),
            CommandId::RunRecommended => self.run_recommended_action().await,
            CommandId::ViewChangelog => self.open_changelog_overlay(false).await,
            CommandId::Search => self.enter_search_mode(),
            CommandId::Refresh => {
                if !self.start_loading() {
                    self.set_status(self.catalog_busy_reason(), true);
                }
            }
            CommandId::ToggleQueue => self.toggle_queue_view(),
            CommandId::QueueCancel => self.cancel_selected_task().await,
            CommandId::QueueRetry => self.retry_selected_task().await,
            CommandId::QueueRetrySafe => self.retry_safe_failed_tasks().await,
            CommandId::QueueRemediate => self.apply_selected_task_remediation().await,
            CommandId::QueueLogOlder => self.queue_log_scroll_up(),
            CommandId::QueueLogNewer => self.queue_log_scroll_down(),
            CommandId::ExportPackages => self.export_packages().await,
            CommandId::ImportPackages => self.import_packages().await,
            CommandId::CycleTheme => {
                let name = crate::cli::tui::theme::cycle_theme();
                self.set_status(format!("Theme: {}", name), true);
            }
        }
    }

    async fn handle_palette_key(&mut self, key: KeyEvent) {
        use crate::cli::tui::line_edit;

        // Ctrl+C closes palette, never quits app
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.close_palette();
            return;
        }

        self.palette_edit_cursor = line_edit::clamp(&self.palette_query, self.palette_edit_cursor);

        match key.code {
            KeyCode::Esc => self.close_palette(),
            KeyCode::Enter => {
                let Some(entry) = self.palette_selected_entry() else {
                    return;
                };

                if !entry.enabled {
                    if let Some(reason) = entry.disabled_reason {
                        self.set_status(reason, true);
                    }
                    return;
                }

                self.close_palette();
                self.execute_command(entry.id).await;
            }
            // List navigation uses the arrow keys only — plain j/k must
            // insert into the query (command labels contain them).
            KeyCode::Up => {
                self.palette_cursor = self.palette_cursor.saturating_sub(1);
            }
            KeyCode::Down => {
                let len = self.palette_entries().len();
                if len > 0 {
                    self.palette_cursor = (self.palette_cursor + 1).min(len - 1);
                }
            }
            KeyCode::Home => {
                self.palette_cursor = 0;
            }
            KeyCode::End => {
                let len = self.palette_entries().len();
                if len > 0 {
                    self.palette_cursor = len - 1;
                }
            }
            KeyCode::PageUp => {
                self.palette_cursor = self.palette_cursor.saturating_sub(5);
            }
            KeyCode::PageDown => {
                let len = self.palette_entries().len();
                if len > 0 {
                    self.palette_cursor = (self.palette_cursor + 5).min(len - 1);
                }
            }
            KeyCode::Left => line_edit::move_left(&mut self.palette_edit_cursor),
            KeyCode::Right => {
                line_edit::move_right(&self.palette_query, &mut self.palette_edit_cursor);
            }
            KeyCode::Backspace => {
                line_edit::backspace(&mut self.palette_query, &mut self.palette_edit_cursor);
                self.clamp_palette_cursor();
            }
            KeyCode::Delete => {
                line_edit::delete_forward(&mut self.palette_query, &mut self.palette_edit_cursor);
                self.clamp_palette_cursor();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                line_edit::clear(&mut self.palette_query, &mut self.palette_edit_cursor);
                self.clamp_palette_cursor();
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                line_edit::delete_word_back(&mut self.palette_query, &mut self.palette_edit_cursor);
                self.clamp_palette_cursor();
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                line_edit::move_home(&mut self.palette_edit_cursor);
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                line_edit::move_end(&self.palette_query, &mut self.palette_edit_cursor);
            }
            KeyCode::Char(ch)
                if !ch.is_control()
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                line_edit::insert(&mut self.palette_query, &mut self.palette_edit_cursor, ch);
                self.clamp_palette_cursor();
            }
            _ => {}
        }

        self.clamp_palette_cursor();
    }

    async fn handle_normal_key(&mut self, key: KeyEvent) {
        // Ctrl+C cancels running task when in queue view, otherwise is ignored
        // We never quit on Ctrl+C - use 'q' to quit instead
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            let can_cancel = self.is_queue_view()
                && self.focus == Focus::Queue
                && self
                    .tasks
                    .get(self.task_cursor)
                    .is_some_and(|t| t.status == TaskQueueStatus::Running);

            if can_cancel {
                self.execute_command(CommandId::QueueCancel).await;
            } else {
                // Never a silent dead-end: tell the user what quits the app.
                self.set_status("Nothing to cancel — press q to quit LinGet", true);
            }
            return;
        }

        if key.code == KeyCode::Char(':')
            || (key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            self.execute_command(CommandId::OpenPalette).await;
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                self.execute_command(CommandId::Quit).await;
                return;
            }
            KeyCode::Char('?') => {
                self.execute_command(CommandId::ShowHelp).await;
                return;
            }
            _ => {}
        }

        match key.code {
            KeyCode::F(1) => {
                self.navigate_to(ViewMode::Today);
                return;
            }
            KeyCode::F(2) => {
                self.navigate_to(ViewMode::Browse);
                return;
            }
            KeyCode::F(3) => {
                self.navigate_to(ViewMode::Queue);
                return;
            }
            _ => {}
        }

        if self.view_mode == ViewMode::Today {
            match key.code {
                KeyCode::Enter => self.open_today_recommendation(),
                KeyCode::Char('w') => self.execute_command(CommandId::RunRecommended).await,
                KeyCode::Char('r') => self.execute_command(CommandId::Refresh).await,
                KeyCode::Char('l') => self.navigate_to(ViewMode::Queue),
                _ => {}
            }
            return;
        }

        if self.is_queue_view() && self.focus == Focus::Queue {
            match key.code {
                KeyCode::Tab => {
                    self.execute_command(CommandId::CycleFocus).await;
                    return;
                }
                KeyCode::Esc | KeyCode::Char('l') => {
                    self.execute_command(CommandId::ToggleQueue).await;
                    return;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.queue_next();
                    return;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.queue_prev();
                    return;
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    self.queue_top();
                    return;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    self.queue_bottom();
                    return;
                }
                KeyCode::PageDown => {
                    self.queue_page_down();
                    return;
                }
                KeyCode::PageUp => {
                    self.queue_page_up();
                    return;
                }
                KeyCode::Char('[') => {
                    self.execute_command(CommandId::QueueLogOlder).await;
                    return;
                }
                KeyCode::Char(']') => {
                    self.execute_command(CommandId::QueueLogNewer).await;
                    return;
                }
                _ if key.code == KeyCode::Char('d')
                    && key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.queue_page_down();
                    return;
                }
                _ if key.code == KeyCode::Char('u')
                    && key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.queue_page_up();
                    return;
                }
                _ => {
                    self.handle_queue_shortcuts(key).await;
                    return;
                }
            }
        }

        match key.code {
            KeyCode::Tab => self.execute_command(CommandId::CycleFocus).await,
            KeyCode::Char('j') | KeyCode::Down => match self.focus {
                Focus::Sources => self.next_source(),
                Focus::Packages | Focus::Queue => self.next_package(),
            },
            KeyCode::Char('k') | KeyCode::Up => match self.focus {
                Focus::Sources => self.prev_source(),
                Focus::Packages | Focus::Queue => self.prev_package(),
            },
            KeyCode::Char('g') | KeyCode::Home => match self.focus {
                Focus::Sources => self.set_source_by_index(0),
                Focus::Packages | Focus::Queue => self.top(),
            },
            KeyCode::Char('G') | KeyCode::End => match self.focus {
                Focus::Sources => self.set_source_by_index(self.source_count().saturating_sub(1)),
                Focus::Packages | Focus::Queue => self.bottom(),
            },
            KeyCode::PageDown => self.execute_command(CommandId::PageDown).await,
            KeyCode::PageUp => self.execute_command(CommandId::PageUp).await,
            _ if key.code == KeyCode::Char('d')
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.execute_command(CommandId::PageDown).await;
            }
            _ if key.code == KeyCode::Char('u')
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.execute_command(CommandId::PageUp).await;
            }
            _ if key.code == KeyCode::Char('b')
                && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.show_sidebar = !self.show_sidebar;
            }
            KeyCode::Enter => {
                if self.focus == Focus::Packages {
                    self.prepare_default_action_for_cursor();
                } else if self.focus == Focus::Sources {
                    // "Open" the highlighted source: jump into its package list.
                    self.focus = Focus::Packages;
                }
            }
            KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.active_details_tab = crate::cli::tui::state::filters::DetailsTab::Info;
            }
            KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.active_details_tab = crate::cli::tui::state::filters::DetailsTab::Dependencies;
            }
            KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => {
                self.active_details_tab = crate::cli::tui::state::filters::DetailsTab::Changelog;
            }
            KeyCode::Char('1') => self.execute_command(CommandId::FilterAll).await,
            KeyCode::Char('2') => self.execute_command(CommandId::FilterInstalled).await,
            KeyCode::Char('3') => self.execute_command(CommandId::FilterUpdates).await,
            KeyCode::Char('4') => self.execute_command(CommandId::FilterFavorites).await,
            KeyCode::Char('5') => self.execute_command(CommandId::FilterSecurityUpdates).await,
            KeyCode::Char('6') => self.execute_command(CommandId::FilterDuplicates).await,
            KeyCode::Char('f') => self.execute_command(CommandId::ToggleFavorite).await,
            KeyCode::Char('F') => self.execute_command(CommandId::BulkToggleFavorite).await,
            KeyCode::Char('v') => {
                self.execute_command(CommandId::ToggleFavoritesUpdatesOnly)
                    .await;
            }
            KeyCode::Char(' ') => self.execute_command(CommandId::ToggleSelection).await,
            KeyCode::Char('a') => self.execute_command(CommandId::SelectAll).await,
            KeyCode::Char('i') => self.execute_command(CommandId::Install).await,
            KeyCode::Char('d') | KeyCode::Char('x') => {
                self.execute_command(CommandId::Remove).await
            }
            KeyCode::Char('D') => {
                if self.filter == Filter::Duplicates {
                    self.dismiss_duplicate_keep_cursor();
                }
            }
            KeyCode::Char('u') => self.execute_command(CommandId::Update).await,
            KeyCode::Char('w') => self.execute_command(CommandId::RunRecommended).await,
            KeyCode::Char('c') => self.execute_command(CommandId::ViewChangelog).await,
            KeyCode::Char('/') => self.execute_command(CommandId::Search).await,
            KeyCode::Char('r') => self.execute_command(CommandId::Refresh).await,
            KeyCode::Char('l') => self.execute_command(CommandId::ToggleQueue).await,
            KeyCode::Esc => {
                if self.is_queue_view() {
                    self.execute_command(CommandId::ToggleQueue).await;
                } else if self.search_results.is_some() && !self.search.is_empty() {
                    self.restore_local_catalog_with_current_search();
                    self.set_status("Provider results hidden; local filter kept", true);
                } else if !self.search.is_empty() {
                    self.search.clear();
                    self.search_cursor = 0;
                    self.apply_filters();
                    self.set_status("Search cleared", true);
                } else if !self.selected.is_empty() {
                    self.clear_selection();
                    self.set_status("Selection cleared", true);
                }
            }
            KeyCode::Char('C') => {
                if !self.is_queue_view()
                    && self
                        .tasks
                        .iter()
                        .any(|t| t.status == TaskQueueStatus::Failed)
                {
                    self.dismiss_all_failed_tasks();
                } else {
                    self.execute_command(CommandId::QueueCancel).await;
                }
            }
            KeyCode::Char('R') => self.execute_command(CommandId::QueueRetry).await,
            KeyCode::Char('A') => self.execute_command(CommandId::QueueRetrySafe).await,
            KeyCode::Char('M') => self.execute_command(CommandId::QueueRemediate).await,
            KeyCode::Char('E') => self.execute_command(CommandId::ExportPackages).await,
            KeyCode::Char('I') => self.execute_command(CommandId::ImportPackages).await,
            KeyCode::Char('T') => self.execute_command(CommandId::CycleTheme).await,
            _ => {}
        }
    }

    pub async fn handle_key(&mut self, key: KeyEvent) {
        self.clear_status_if_needed();

        if self.showing_import_preview {
            self.handle_import_preview_key(key).await;
            return;
        }
        if self.showing_changelog {
            self.handle_changelog_key(key).await;
            return;
        }
        if self.showing_palette {
            self.handle_palette_key(key).await;
            return;
        }
        if self.showing_help {
            self.handle_help_key(key).await;
            return;
        }
        if self.showing_onboarding {
            match key.code {
                KeyCode::Enter => {
                    self.dismiss_tui_onboarding();
                    self.navigate_to(ViewMode::Today);
                }
                KeyCode::Char('?') => {
                    self.showing_help = true;
                    self.help_scroll = 0;
                }
                KeyCode::Char('q') => self.should_quit = true,
                _ => {}
            }
            return;
        }
        if self.confirming.is_some() {
            self.handle_confirm_key(key).await;
            return;
        }
        if self.searching {
            self.handle_search_key(key);
            return;
        }
        self.handle_normal_key(key).await;
    }

    async fn handle_import_preview_key(&mut self, key: KeyEvent) {
        const PREVIEW_PAGE: usize = 10;
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.showing_import_preview = false;
                self.import_preview.clear();
                self.import_preview_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.import_preview_scroll = self.import_preview_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.import_preview_scroll = self.import_preview_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.import_preview_scroll =
                    self.import_preview_scroll.saturating_add(PREVIEW_PAGE);
            }
            KeyCode::PageUp => {
                self.import_preview_scroll =
                    self.import_preview_scroll.saturating_sub(PREVIEW_PAGE);
            }
            KeyCode::Char('g') | KeyCode::Home => self.import_preview_scroll = 0,
            KeyCode::Enter => {
                use crate::models::history::TaskQueueAction;

                let packages = self
                    .import_preview
                    .iter()
                    .map(|ep| ep.to_install_stub())
                    .collect();

                self.showing_import_preview = false;
                self.import_preview.clear();
                self.queue_tasks(packages, TaskQueueAction::Install).await;
            }
            _ => {}
        }
    }

    pub async fn handle_mouse(&mut self, event: MouseEvent, regions: &LayoutRegions) {
        const SCROLL_STEP: usize = 3;
        const DOUBLE_CLICK_MS: u128 = 400;

        let pos = (event.column, event.row);

        if self.showing_palette {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    self.palette_cursor = self.palette_cursor.saturating_sub(1);
                    self.clamp_palette_cursor();
                }
                MouseEventKind::ScrollDown => {
                    let len = self.palette_entries().len();
                    if len > 0 {
                        self.palette_cursor = (self.palette_cursor + 1).min(len - 1);
                    }
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    let col = event.column;
                    let row = event.row;
                    let is_double = self.last_click.take().is_some_and(|(lc, lr, lt)| {
                        lc == col && lr == row && lt.elapsed().as_millis() < DOUBLE_CLICK_MS
                    });
                    self.last_click = Some((col, row, Instant::now()));
                    self.handle_mouse_palette_click(col, row, is_double, &regions.palette)
                        .await;
                }
                _ => {}
            }
            return;
        }

        if self.showing_changelog {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    self.changelog_scroll = self.changelog_scroll.saturating_sub(SCROLL_STEP);
                }
                MouseEventKind::ScrollDown => {
                    self.changelog_scroll = self.changelog_scroll.saturating_add(SCROLL_STEP);
                }
                MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Down(MouseButton::Right) => {
                    self.close_changelog_overlay();
                }
                _ => {}
            }
            return;
        }

        // While a modal overlay is up, no mouse input may leak through to the
        // lists underneath — scrolling a hidden list is disorienting.
        if self.showing_help {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    self.help_scroll = self.help_scroll.saturating_sub(SCROLL_STEP);
                }
                MouseEventKind::ScrollDown => {
                    self.help_scroll = self.help_scroll.saturating_add(SCROLL_STEP);
                }
                MouseEventKind::Down(_) => {
                    self.showing_help = false;
                    self.help_scroll = 0;
                }
                _ => {}
            }
            return;
        }

        if self.confirming.is_some() {
            if let MouseEventKind::Down(MouseButton::Left) = event.kind {
                self.handle_mouse_confirm(event.column, event.row, &regions.preflight_modal)
                    .await;
            }
            return;
        }

        if self.showing_import_preview {
            match event.kind {
                MouseEventKind::ScrollUp => {
                    self.import_preview_scroll = self.import_preview_scroll.saturating_sub(1);
                }
                MouseEventKind::ScrollDown => {
                    self.import_preview_scroll = self.import_preview_scroll.saturating_add(1);
                }
                _ => {}
            }
            return;
        }

        match event.kind {
            MouseEventKind::ScrollUp => {
                if rect_contains(regions.expanded_queue_logs, pos) {
                    self.focus = Focus::Queue;
                    self.queue_log_scroll_up();
                } else if rect_contains(regions.expanded_queue_tasks, pos) {
                    self.focus = Focus::Queue;
                    self.queue_prev();
                } else if rect_contains(regions.packages, pos) {
                    self.focus = Focus::Packages;
                    for _ in 0..SCROLL_STEP {
                        self.prev_package();
                    }
                } else if rect_contains(regions.sources, pos) {
                    self.focus = Focus::Sources;
                    let idx = self.source_index();
                    if idx > 0 {
                        self.set_source_by_index(idx - 1);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if rect_contains(regions.expanded_queue_logs, pos) {
                    self.focus = Focus::Queue;
                    self.queue_log_scroll_down();
                } else if rect_contains(regions.expanded_queue_tasks, pos) {
                    self.focus = Focus::Queue;
                    self.queue_next();
                } else if rect_contains(regions.packages, pos) {
                    self.focus = Focus::Packages;
                    for _ in 0..SCROLL_STEP {
                        self.next_package();
                    }
                } else if rect_contains(regions.sources, pos) {
                    self.focus = Focus::Sources;
                    let idx = self.source_index();
                    if idx + 1 < self.source_count() {
                        self.set_source_by_index(idx + 1);
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let col = event.column;
                let row = event.row;

                let is_double = self.last_click.take().is_some_and(|(lc, lr, lt)| {
                    lc == col && lr == row && lt.elapsed().as_millis() < DOUBLE_CLICK_MS
                });
                self.last_click = Some((col, row, Instant::now()));

                if rect_contains(regions.header_filter_row, pos) {
                    self.handle_mouse_header(col, row, regions);
                } else if rect_contains(regions.filter_panel, pos) {
                    self.handle_mouse_filter_panel(col, row, regions);
                } else if rect_contains(regions.sources, pos) {
                    self.handle_mouse_sources(row, &regions.sources);
                } else if rect_contains(regions.packages, pos) {
                    self.handle_mouse_packages_click(col, row, is_double, &regions.packages);
                } else if rect_contains(regions.details, pos) {
                    self.handle_mouse_details_click(col, row, &regions.details);
                } else if rect_contains(regions.queue_bar, pos) {
                    self.toggle_queue_view();
                } else if rect_contains(regions.expanded_queue, pos) {
                    self.handle_mouse_expanded_queue_click(col, row, regions)
                        .await;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if rect_contains(regions.packages, pos) {
                    self.handle_mouse_packages_drag(event.row, &regions.packages);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag_select_anchor = None;
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if rect_contains(regions.packages, pos) {
                    self.handle_mouse_packages_right_click(event.row, &regions.packages);
                }
            }
            _ => {}
        }
    }

    fn handle_mouse_filter_panel(&mut self, col: u16, row: u16, regions: &LayoutRegions) {
        self.navigate_to(ViewMode::Browse);
        if let Some(filter) = crate::cli::tui::components::workspace::filter_panel_hit_test(
            self,
            regions.filter_panel,
            col,
            row,
        ) {
            self.filter = filter;
            self.apply_filters();
        } else if crate::cli::tui::components::workspace::filter_panel_search_hit_test(
            regions.filter_panel,
            col,
            row,
        ) {
            self.enter_search_mode();
        }
    }

    fn palette_index_from_mouse_row(&self, row: u16, palette_rect: &Rect) -> Option<usize> {
        if palette_rect.width <= 2 || palette_rect.height <= 4 {
            return None;
        }

        let inner_y = palette_rect.y.saturating_add(1);
        let inner_height = palette_rect.height.saturating_sub(2);
        if inner_height < 3 {
            return None;
        }

        let list_top = inner_y.saturating_add(1);
        let visible_rows = inner_height.saturating_sub(2) as usize;
        if visible_rows == 0 {
            return None;
        }
        if row < list_top || row >= list_top.saturating_add(visible_rows as u16) {
            return None;
        }

        let entries = self.palette_entries();
        if entries.is_empty() {
            return None;
        }

        let start = ui::window_start(entries.len(), visible_rows, self.palette_cursor);
        let clicked = start + row.saturating_sub(list_top) as usize;
        (clicked < entries.len()).then_some(clicked)
    }

    async fn handle_mouse_palette_click(
        &mut self,
        col: u16,
        row: u16,
        is_double: bool,
        palette_rect: &Rect,
    ) {
        if !rect_contains(*palette_rect, (col, row)) {
            self.close_palette();
            return;
        }

        let Some(index) = self.palette_index_from_mouse_row(row, palette_rect) else {
            return;
        };

        self.palette_cursor = index;

        if !is_double {
            return;
        }

        let Some(entry) = self.palette_entries().get(index).cloned() else {
            return;
        };

        if !entry.enabled {
            if let Some(reason) = entry.disabled_reason {
                self.set_status(reason, true);
            }
            return;
        }

        self.close_palette();
        self.execute_command(entry.id).await;
    }

    fn handle_mouse_header(&mut self, col: u16, row: u16, regions: &LayoutRegions) {
        if let Some(action) = ui::header_filter_hit_test(self, regions.header_filter_row, col, row)
        {
            self.apply_header_action(action);
            return;
        }

        if !self.searching {
            self.enter_search_mode();
        }
    }

    fn apply_header_action(&mut self, action: crate::cli::tui::components::header::HeaderAction) {
        use crate::cli::tui::components::header::HeaderAction;

        match action {
            HeaderAction::Today => self.navigate_to(ViewMode::Today),
            HeaderAction::Browse => {
                self.navigate_to(ViewMode::Browse);
                self.filter = Filter::All;
                self.apply_filters();
            }
            HeaderAction::Queue => {
                self.navigate_to(ViewMode::Queue);
            }
        }
    }

    fn source_index_from_mouse_row(&self, row: u16, sources_rect: &Rect) -> Option<usize> {
        if sources_rect.width <= 2 || sources_rect.height <= 2 {
            return None;
        }

        let top = sources_rect.y.saturating_add(1);
        let visible_rows = sources_rect.height.saturating_sub(2) as usize;
        if visible_rows == 0 {
            return None;
        }

        if row < top || row >= top.saturating_add(visible_rows as u16) {
            return None;
        }

        let total = self.visible_sources().len() + 1;
        let start = ui::window_start(total, visible_rows, self.source_index());
        let clicked_index = start + row.saturating_sub(top) as usize;
        (clicked_index < total).then_some(clicked_index)
    }

    fn handle_mouse_sources(&mut self, row: u16, sources_rect: &Rect) {
        self.focus = Focus::Sources;
        if let Some(clicked_index) = self.source_index_from_mouse_row(row, sources_rect) {
            self.set_source_by_index(clicked_index);
        }
    }

    fn package_index_from_mouse_row(&self, row: u16, packages_rect: &Rect) -> Option<usize> {
        if packages_rect.width <= 2 || packages_rect.height <= 4 || self.filtered.is_empty() {
            return None;
        }

        let first_row = packages_rect.y.saturating_add(3);
        let visible_rows = packages_rect.height.saturating_sub(6) as usize;
        if visible_rows == 0 {
            return None;
        }

        if row < first_row || row >= first_row.saturating_add(visible_rows as u16) {
            return None;
        }

        let start = ui::window_start(self.filtered.len(), visible_rows.max(1), self.cursor);
        let clicked_index = start + row.saturating_sub(first_row) as usize;
        (clicked_index < self.filtered.len()).then_some(clicked_index)
    }

    fn prepare_default_action_for_cursor(&mut self) {
        let Some(package) = self.current_package() else {
            return;
        };
        let action = match package.status {
            PackageStatus::NotInstalled => Some(TaskQueueAction::Install),
            PackageStatus::UpdateAvailable => Some(TaskQueueAction::Update),
            PackageStatus::Installed => Some(TaskQueueAction::Remove),
            _ => None,
        };

        if let Some(action) = action {
            self.prepare_action(action);
        } else {
            self.set_status("No primary action for this package", true);
        }
    }

    fn handle_mouse_packages_click(
        &mut self,
        col: u16,
        row: u16,
        is_double: bool,
        packages_rect: &Rect,
    ) {
        let Some(clicked_index) = self.package_index_from_mouse_row(row, packages_rect) else {
            return;
        };

        self.focus = Focus::Packages;
        self.cursor = clicked_index;
        self.drag_select_anchor = Some(clicked_index);

        // Column zones: marker (selection toggle), then the status badge
        // (favorite toggle); clicks on the name only move the cursor.
        let inner_col = col.saturating_sub(packages_rect.x.saturating_add(1));
        if inner_col < 2 {
            self.toggle_selection_on_cursor();
            return;
        }
        if (2..=5).contains(&inner_col) {
            self.toggle_favorite_on_cursor();
            return;
        }

        if is_double {
            self.prepare_default_action_for_cursor();
        }
    }

    fn select_package_range(&mut self, start: usize, end: usize) {
        let (from, to) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        for row_index in from..=to {
            let Some(package_index) = self.filtered.get(row_index).copied() else {
                continue;
            };
            let Some(package) = self.packages.get(package_index) else {
                continue;
            };
            self.selected.insert(package.id());
        }
    }

    fn handle_mouse_packages_drag(&mut self, row: u16, packages_rect: &Rect) {
        let Some(anchor) = self.drag_select_anchor else {
            return;
        };
        let Some(clicked_index) = self.package_index_from_mouse_row(row, packages_rect) else {
            return;
        };

        self.focus = Focus::Packages;
        self.cursor = clicked_index;
        self.select_package_range(anchor, clicked_index);
    }

    fn handle_mouse_packages_right_click(&mut self, row: u16, packages_rect: &Rect) {
        let Some(clicked_index) = self.package_index_from_mouse_row(row, packages_rect) else {
            return;
        };
        self.focus = Focus::Packages;
        self.cursor = clicked_index;
        self.prepare_default_action_for_cursor();
    }

    async fn handle_mouse_expanded_queue_click(
        &mut self,
        col: u16,
        row: u16,
        regions: &LayoutRegions,
    ) {
        use crate::cli::tui::components::queue_board::{queue_click_target, RowTarget};

        // Any click inside the queue panel focuses it.
        self.focus = Focus::Queue;

        match queue_click_target(self, regions.expanded_queue_tasks, col, row) {
            Some(RowTarget::Task(id)) => {
                if let Some(index) = self.tasks.iter().position(|task| task.id == id) {
                    self.set_task_cursor(index);
                }
            }
            Some(RowTarget::RetrySafeAll) => {
                self.execute_command(CommandId::QueueRetrySafe).await;
            }
            None => {}
        }
    }

    async fn handle_mouse_confirm(&mut self, col: u16, row: u16, modal_rect: &Rect) {
        match ui::preflight_modal_hit_test(*modal_rect, col, row) {
            Some(true) => {
                if let Some(confirming) = self.confirming.as_mut() {
                    if confirming.preflight.risk_level == PreflightRiskLevel::High
                        && !confirming.risk_acknowledged
                    {
                        confirming.risk_acknowledged = true;
                        self.set_status(
                            "High-risk operation acknowledged. Click confirm again to queue.",
                            true,
                        );
                        return;
                    }
                }

                if let Some(action) = self.confirming.take() {
                    self.clear_preflight_verification_tracking();
                    let queued = self.queue_tasks(action.packages, action.action).await;
                    self.clear_selection();
                    self.set_status(Self::queued_result_message(action.action, queued), true);
                }
            }
            Some(false) => {
                self.confirming = None;
                self.clear_preflight_verification_tracking();
                self.set_status("Cancelled", true);
            }
            None => {
                if !rect_contains(*modal_rect, (col, row)) {
                    self.confirming = None;
                    self.clear_preflight_verification_tracking();
                    self.set_status("Cancelled", true);
                }
            }
        }
    }

    pub fn handle_mouse_details_click(
        &mut self,
        col: u16,
        row: u16,
        details_rect: &ratatui::layout::Rect,
    ) {
        self.focus = crate::cli::tui::state::filters::Focus::Packages;

        if let Some(tab) =
            crate::cli::tui::components::details::details_tab_hit_test(*details_rect, col, row)
        {
            self.active_details_tab = tab;
        }
    }
}
