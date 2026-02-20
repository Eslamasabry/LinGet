use crate::models::{PackageSource, Repository};

#[derive(Debug, Default)]
pub struct SourceManagementState {
    pub repositories: Vec<Repository>,
    pub loading: bool,
    pub target_source: Option<PackageSource>,
}

impl SourceManagementState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.repositories.clear();
        self.target_source = None;
        self.loading = false;
    }
}
