use super::*;

impl App {
    pub(crate) fn catalog_state_consistent(&self) -> bool {
        let cursor_in_bounds = self.filtered.is_empty() || self.cursor < self.filtered.len();
        let filtered_indices_valid = self.filtered.iter().all(|idx| *idx < self.packages.len());
        let source_visible = self
            .source
            .is_none_or(|source| self.visible_sources().contains(&source));

        cursor_in_bounds && filtered_indices_valid && source_visible
    }

    pub(super) fn debug_assert_catalog_state(&self) {
        debug_assert!(
            self.catalog_state_consistent(),
            "catalog state invariants must hold after package navigation updates"
        );
        debug_assert!(
            self.filtered.iter().all(|idx| *idx < self.packages.len()),
            "filtered package indices must remain in bounds"
        );
        debug_assert!(
            self.filtered.is_empty() || self.cursor < self.filtered.len(),
            "package cursor must stay within filtered results"
        );
        debug_assert!(
            self.source
                .is_none_or(|source| self.visible_sources().contains(&source)),
            "selected source must remain visible for the active filter"
        );
    }

    pub(super) fn set_package_cursor(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.filtered.len().saturating_sub(1));
        self.debug_assert_catalog_state();
    }

    pub(super) fn clamp_package_cursor(&mut self) {
        self.set_package_cursor(self.cursor);
    }
}
