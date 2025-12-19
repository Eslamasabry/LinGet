#![allow(dead_code)]

use crate::models::PackageSource;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Library,
    Updates,
    Discover,
    Favorites,
}

impl View {
    pub fn as_str(&self) -> &'static str {
        match self {
            View::Library => "library",
            View::Updates => "updates",
            View::Discover => "discover",
            View::Favorites => "favorites",
        }
    }
}

pub type StateCallback = Box<dyn Fn(&UIState)>;

#[derive(Default)]
pub struct UIStateInner {
    pub current_view: View,
    pub search_query: String,
    pub source_filter: Option<PackageSource>,
    pub installed_count: u32,
    pub updates_count: u32,
    pub favorites_count: u32,
    pub selection_mode: bool,
    pub selected_packages: HashSet<String>,
    pub unread_count: u32,
    pub command_center_visible: bool,
    pub is_loading: bool,
    pub loading_message: Option<String>,
    pub recent_searches: Vec<String>,
}

pub struct UIState {
    inner: RefCell<UIStateInner>,
    observers: RefCell<Vec<StateCallback>>,
}

impl Default for UIState {
    fn default() -> Self {
        Self::new()
    }
}

impl UIState {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(UIStateInner::default()),
            observers: RefCell::new(Vec::new()),
        }
    }

    pub fn subscribe<F>(&self, callback: F)
    where
        F: Fn(&UIState) + 'static,
    {
        self.observers.borrow_mut().push(Box::new(callback));
    }

    fn notify(&self) {
        for observer in self.observers.borrow().iter() {
            observer(self);
        }
    }

    pub fn current_view(&self) -> View {
        self.inner.borrow().current_view
    }

    pub fn set_view(&self, view: View) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.current_view == view {
                return;
            }
            inner.current_view = view;
        }
        self.notify();
    }

    pub fn search_query(&self) -> String {
        self.inner.borrow().search_query.clone()
    }

    pub fn set_search_query(&self, query: String) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.search_query == query {
                return;
            }
            inner.search_query = query;
        }
        self.notify();
    }

    pub fn source_filter(&self) -> Option<PackageSource> {
        self.inner.borrow().source_filter
    }

    pub fn set_source_filter(&self, source: Option<PackageSource>) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.source_filter == source {
                return;
            }
            inner.source_filter = source;
        }
        self.notify();
    }

    pub fn installed_count(&self) -> u32 {
        self.inner.borrow().installed_count
    }

    pub fn set_installed_count(&self, count: u32) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.installed_count == count {
                return;
            }
            inner.installed_count = count;
        }
        self.notify();
    }

    pub fn updates_count(&self) -> u32 {
        self.inner.borrow().updates_count
    }

    pub fn set_updates_count(&self, count: u32) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.updates_count == count {
                return;
            }
            inner.updates_count = count;
        }
        self.notify();
    }

    pub fn favorites_count(&self) -> u32 {
        self.inner.borrow().favorites_count
    }

    pub fn set_favorites_count(&self, count: u32) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.favorites_count == count {
                return;
            }
            inner.favorites_count = count;
        }
        self.notify();
    }

    pub fn set_counts(&self, installed: u32, updates: u32, favorites: u32) {
        {
            let mut inner = self.inner.borrow_mut();
            let changed = inner.installed_count != installed
                || inner.updates_count != updates
                || inner.favorites_count != favorites;
            if !changed {
                return;
            }
            inner.installed_count = installed;
            inner.updates_count = updates;
            inner.favorites_count = favorites;
        }
        self.notify();
    }

    pub fn is_selection_mode(&self) -> bool {
        self.inner.borrow().selection_mode
    }

    pub fn set_selection_mode(&self, enabled: bool) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.selection_mode == enabled {
                return;
            }
            inner.selection_mode = enabled;
            if !enabled {
                inner.selected_packages.clear();
            }
        }
        self.notify();
    }

    pub fn selected_packages(&self) -> HashSet<String> {
        self.inner.borrow().selected_packages.clone()
    }

    pub fn selected_count(&self) -> usize {
        self.inner.borrow().selected_packages.len()
    }

    pub fn toggle_selection(&self, package_id: &str) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.selected_packages.contains(package_id) {
                inner.selected_packages.remove(package_id);
            } else {
                inner.selected_packages.insert(package_id.to_string());
            }
        }
        self.notify();
    }

    pub fn select_package(&self, package_id: &str) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.selected_packages.contains(package_id) {
                return;
            }
            inner.selected_packages.insert(package_id.to_string());
        }
        self.notify();
    }

    pub fn deselect_package(&self, package_id: &str) {
        {
            let mut inner = self.inner.borrow_mut();
            if !inner.selected_packages.remove(package_id) {
                return;
            }
        }
        self.notify();
    }

    pub fn clear_selection(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.selected_packages.is_empty() {
                return;
            }
            inner.selected_packages.clear();
        }
        self.notify();
    }

    pub fn unread_count(&self) -> u32 {
        self.inner.borrow().unread_count
    }

    pub fn set_unread_count(&self, count: u32) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.unread_count == count {
                return;
            }
            inner.unread_count = count;
        }
        self.notify();
    }

    pub fn increment_unread(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.unread_count = inner.unread_count.saturating_add(1);
        }
        self.notify();
    }

    pub fn clear_unread(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.unread_count == 0 {
                return;
            }
            inner.unread_count = 0;
        }
        self.notify();
    }

    pub fn is_command_center_visible(&self) -> bool {
        self.inner.borrow().command_center_visible
    }

    pub fn set_command_center_visible(&self, visible: bool) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.command_center_visible == visible {
                return;
            }
            inner.command_center_visible = visible;
        }
        self.notify();
    }

    pub fn is_loading(&self) -> bool {
        self.inner.borrow().is_loading
    }

    pub fn loading_message(&self) -> Option<String> {
        self.inner.borrow().loading_message.clone()
    }

    pub fn set_loading(&self, loading: bool, message: Option<&str>) {
        {
            let mut inner = self.inner.borrow_mut();
            inner.is_loading = loading;
            inner.loading_message = message.map(String::from);
        }
        self.notify();
    }

    pub fn clear_loading(&self) {
        self.set_loading(false, None);
    }

    pub fn recent_searches(&self) -> Vec<String> {
        self.inner.borrow().recent_searches.clone()
    }

    pub fn add_recent_search(&self, query: &str) {
        if query.trim().is_empty() {
            return;
        }
        {
            let mut inner = self.inner.borrow_mut();
            inner.recent_searches.retain(|s| s != query);
            inner.recent_searches.insert(0, query.to_string());
            inner.recent_searches.truncate(10);
        }
        self.notify();
    }

    pub fn clear_recent_searches(&self) {
        {
            let mut inner = self.inner.borrow_mut();
            if inner.recent_searches.is_empty() {
                return;
            }
            inner.recent_searches.clear();
        }
        self.notify();
    }
}

pub fn create_ui_state() -> Rc<UIState> {
    Rc::new(UIState::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_changes() {
        let state = UIState::new();
        assert_eq!(state.current_view(), View::Library);

        state.set_view(View::Updates);
        assert_eq!(state.current_view(), View::Updates);
    }

    #[test]
    fn test_selection_mode() {
        let state = UIState::new();
        assert!(!state.is_selection_mode());

        state.set_selection_mode(true);
        assert!(state.is_selection_mode());

        state.select_package("pkg1");
        state.select_package("pkg2");
        assert_eq!(state.selected_count(), 2);

        state.set_selection_mode(false);
        assert_eq!(state.selected_count(), 0);
    }

    #[test]
    fn test_recent_searches() {
        let state = UIState::new();

        state.add_recent_search("firefox");
        state.add_recent_search("chrome");
        state.add_recent_search("firefox");

        let recent = state.recent_searches();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0], "firefox");
        assert_eq!(recent[1], "chrome");
    }
}
