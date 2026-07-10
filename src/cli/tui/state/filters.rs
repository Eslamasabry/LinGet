#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Installed,
    Updates,
    Favorites,
    SecurityUpdates,
    Duplicates,
}

impl Filter {
    pub fn from_config_value(value: Option<&str>) -> Self {
        match value.unwrap_or_default().to_lowercase().as_str() {
            "installed" => Self::Installed,
            "updates" => Self::Updates,
            "favorites" => Self::Favorites,
            "security" => Self::SecurityUpdates,
            "duplicates" => Self::Duplicates,
            _ => Self::All,
        }
    }

    pub fn as_config_value(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Installed => "installed",
            Self::Updates => "updates",
            Self::Favorites => "favorites",
            Self::SecurityUpdates => "security",
            Self::Duplicates => "duplicates",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sources,
    Packages,
    Queue,
}

impl Focus {
    pub fn from_config_value(value: Option<&str>) -> Self {
        match value.unwrap_or_default().to_lowercase().as_str() {
            "packages" => Self::Packages,
            "queue" => Self::Queue,
            _ => Self::Sources,
        }
    }

    pub fn as_config_value(self) -> &'static str {
        match self {
            Self::Sources => "sources",
            Self::Packages => "packages",
            Self::Queue => "queue",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DetailsTab {
    #[default]
    Info,
    Dependencies,
    Changelog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Today,
    Browse,
    Queue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutTier {
    Minimal,
    Compact,
    Standard,
    Wide,
}

impl LayoutTier {
    pub fn from_size(width: u16, height: u16) -> Self {
        if width < 80 || height < 24 {
            Self::Minimal
        } else if width < 100 {
            Self::Compact
        } else if width < 140 {
            Self::Standard
        } else {
            Self::Wide
        }
    }

    pub fn shows_sources(self) -> bool {
        matches!(self, Self::Standard | Self::Wide)
    }

    pub fn shows_inspector(self) -> bool {
        self == Self::Wide
    }
}
