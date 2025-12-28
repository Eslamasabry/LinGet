//! Shell alias management for bash, zsh, and fish.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::PackageSource;

/// Information about a command/executable provided by a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandInfo {
    /// The command name (e.g., "pip", "python3")
    pub name: String,
    /// Full path to the executable
    pub path: PathBuf,
    /// Optional description of what the command does
    pub description: Option<String>,
}

/// A package and the commands it provides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCommands {
    /// Package name
    pub package_name: String,
    /// Package source (apt, pip, cargo, etc.)
    pub source: PackageSource,
    /// Commands/executables provided by this package
    pub commands: Vec<CommandInfo>,
}

impl PackageCommands {
    #[allow(dead_code)]
    pub fn new(package_name: impl Into<String>, source: PackageSource) -> Self {
        Self {
            package_name: package_name.into(),
            source,
            commands: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_commands(mut self, commands: Vec<CommandInfo>) -> Self {
        self.commands = commands;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub const ALL: [Shell; 3] = [Shell::Bash, Shell::Zsh, Shell::Fish];

    pub fn display_name(&self) -> &'static str {
        match self {
            Shell::Bash => "Bash",
            Shell::Zsh => "Zsh",
            Shell::Fish => "Fish",
        }
    }

    pub fn primary_config_path(&self) -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        match self {
            Shell::Bash => home.join(".bashrc"),
            Shell::Zsh => home.join(".zshrc"),
            Shell::Fish => home.join(".config/fish/config.fish"),
        }
    }

    pub fn config_paths(&self) -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        match self {
            Shell::Bash => vec![
                home.join(".bashrc"),
                home.join(".bash_aliases"),
                home.join(".bash_profile"),
                home.join(".profile"),
            ],
            Shell::Zsh => vec![
                home.join(".zshrc"),
                home.join(".zsh_aliases"),
                home.join(".zprofile"),
            ],
            Shell::Fish => vec![
                home.join(".config/fish/config.fish"),
                home.join(".config/fish/functions"),
            ],
        }
    }

    pub fn is_installed(&self) -> bool {
        let shell_name = match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
        };
        which::which(shell_name).is_ok()
    }

    pub fn format_alias(&self, name: &str, command: &str) -> String {
        match self {
            Shell::Bash | Shell::Zsh => {
                let escaped = command.replace('\'', "'\\''");
                format!("alias {}='{}'", name, escaped)
            }
            Shell::Fish => {
                let escaped = command.replace('\'', "\\'");
                format!("alias {} '{}'", name, escaped)
            }
        }
    }

    pub fn parse_alias_line(&self, line: &str) -> Option<(String, String)> {
        let line = line.trim();

        match self {
            Shell::Bash | Shell::Zsh => {
                let rest = line.strip_prefix("alias ")?.trim();
                let eq_pos = rest.find('=')?;
                let name = rest[..eq_pos].trim().to_string();
                let value = rest[eq_pos + 1..].trim();
                let command = strip_quotes(value);
                Some((name, command))
            }
            Shell::Fish => {
                if line.starts_with("alias ") {
                    let rest = line.strip_prefix("alias ")?.trim();
                    let space_pos = rest.find(' ')?;
                    let name = rest[..space_pos].trim().to_string();
                    let value = rest[space_pos + 1..].trim();
                    let command = strip_quotes(value);
                    Some((name, command))
                } else if line.starts_with("function ") && line.contains(';') {
                    let rest = line.strip_prefix("function ")?.trim();
                    let semi_pos = rest.find(';')?;
                    let name = rest[..semi_pos].trim().to_string();
                    let after_semi = rest[semi_pos + 1..].trim();
                    let end_pos = after_semi.find("; end")?;
                    let command = after_semi[..end_pos].trim().to_string();
                    Some((name, command))
                } else {
                    None
                }
            }
        }
    }
}

fn strip_quotes(value: &str) -> String {
    if (value.starts_with('\'') && value.ends_with('\''))
        || (value.starts_with('"') && value.ends_with('"'))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShellAlias {
    pub name: String,
    pub command: String,
    pub shells: HashSet<Shell>,
    pub managed_by_linget: bool,
    pub source_file: Option<PathBuf>,
    pub description: Option<String>,
}

impl ShellAlias {
    pub fn new(
        name: impl Into<String>,
        command: impl Into<String>,
        shells: HashSet<Shell>,
    ) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            shells,
            managed_by_linget: true,
            source_file: None,
            description: None,
        }
    }

    pub fn from_config(
        name: impl Into<String>,
        command: impl Into<String>,
        shell: Shell,
        source_file: PathBuf,
    ) -> Self {
        let mut shells = HashSet::new();
        shells.insert(shell);
        Self {
            name: name.into(),
            command: command.into(),
            shells,
            managed_by_linget: false,
            source_file: Some(source_file),
            description: None,
        }
    }

    pub fn conflicts_with_command(&self) -> bool {
        which::which(&self.name).is_ok()
    }

    pub fn shells_display(&self) -> String {
        let mut shells: Vec<_> = self.shells.iter().map(|s| s.display_name()).collect();
        shells.sort();
        shells.join(", ")
    }
}

const LINGET_ALIAS_START: &str = "# >>> LinGet Aliases >>>";
const LINGET_ALIAS_END: &str = "# <<< LinGet Aliases <<<";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LazyPackage {
    pub name: String,
    pub source: PackageSource,
    pub loading: bool,
    pub loaded: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AliasManager {
    pub detected_shells: Vec<Shell>,
    pub default_shell: Option<Shell>,
    pub all_aliases: Vec<ShellAlias>,
    pub managed_aliases: Vec<ShellAlias>,
    pub available_commands: Vec<String>,
    pub package_commands: Vec<PackageCommands>,
    pub lazy_packages: Vec<LazyPackage>,
}

impl AliasManager {
    pub fn new() -> Self {
        let mut manager = Self::default();
        manager.detect_shells();
        manager
    }

    pub fn detect_shells(&mut self) {
        self.detected_shells = Shell::ALL
            .iter()
            .copied()
            .filter(|s| s.is_installed())
            .collect();

        if let Ok(shell_path) = std::env::var("SHELL") {
            self.default_shell = if shell_path.ends_with("bash") {
                Some(Shell::Bash)
            } else if shell_path.ends_with("zsh") {
                Some(Shell::Zsh)
            } else if shell_path.ends_with("fish") {
                Some(Shell::Fish)
            } else {
                None
            };
        }
    }

    pub fn load_existing_aliases(&mut self) -> Result<()> {
        self.all_aliases.clear();
        let mut seen: HashMap<String, usize> = HashMap::new();

        for shell in &self.detected_shells {
            for config_path in shell.config_paths() {
                if config_path.exists() && config_path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        let mut in_linget_section = false;

                        for line in content.lines() {
                            if line.trim() == LINGET_ALIAS_START {
                                in_linget_section = true;
                                continue;
                            }
                            if line.trim() == LINGET_ALIAS_END {
                                in_linget_section = false;
                                continue;
                            }

                            if let Some((name, command)) = shell.parse_alias_line(line) {
                                if let Some(&idx) = seen.get(&name) {
                                    self.all_aliases[idx].shells.insert(*shell);
                                } else {
                                    let mut alias = ShellAlias::from_config(
                                        &name,
                                        &command,
                                        *shell,
                                        config_path.clone(),
                                    );
                                    alias.managed_by_linget = in_linget_section;
                                    seen.insert(name, self.all_aliases.len());
                                    self.all_aliases.push(alias);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.managed_aliases = self
            .all_aliases
            .iter()
            .filter(|a| a.managed_by_linget)
            .cloned()
            .collect();

        Ok(())
    }

    pub fn scan_available_commands(&mut self) {
        self.available_commands.clear();

        if let Ok(path) = std::env::var("PATH") {
            let mut seen = HashSet::new();
            for dir in path.split(':') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() || file_type.is_symlink() {
                                if let Some(name) = entry.file_name().to_str() {
                                    if !seen.contains(name) {
                                        let path = entry.path();
                                        if is_executable(&path) {
                                            seen.insert(name.to_string());
                                            self.available_commands.push(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.available_commands.sort();
    }

    pub fn set_package_loading(&mut self, name: &str, source: PackageSource, loading: bool) {
        if let Some(pkg) = self
            .lazy_packages
            .iter_mut()
            .find(|p| p.name == name && p.source == source)
        {
            pkg.loading = loading;
        }
    }

    pub fn set_package_commands(
        &mut self,
        name: &str,
        source: PackageSource,
        commands: Vec<CommandInfo>,
    ) {
        if let Some(pkg) = self
            .lazy_packages
            .iter_mut()
            .find(|p| p.name == name && p.source == source)
        {
            pkg.loading = false;
            pkg.loaded = true;
        }

        if let Some(existing) = self
            .package_commands
            .iter_mut()
            .find(|p| p.package_name == name && p.source == source)
        {
            existing.commands = commands;
        } else {
            self.package_commands.push(PackageCommands {
                package_name: name.to_string(),
                source,
                commands,
            });
        }
    }

    pub fn get_lazy_package(&self, name: &str, source: PackageSource) -> Option<&LazyPackage> {
        self.lazy_packages
            .iter()
            .find(|p| p.name == name && p.source == source)
    }

    pub fn add_alias(&mut self, alias: ShellAlias) -> Result<()> {
        self.managed_aliases.retain(|a| a.name != alias.name);
        self.managed_aliases.push(alias.clone());
        self.write_aliases_to_shells()?;
        Ok(())
    }

    pub fn delete_alias(&mut self, name: &str) -> Result<()> {
        self.managed_aliases.retain(|a| a.name != name);
        self.write_aliases_to_shells()?;
        Ok(())
    }

    pub fn write_aliases_to_shells(&self) -> Result<()> {
        for shell in &self.detected_shells {
            self.write_aliases_to_shell(*shell)?;
        }
        Ok(())
    }

    fn write_aliases_to_shell(&self, shell: Shell) -> Result<()> {
        let config_path = shell.primary_config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        let existing_content = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {:?}", config_path))?
        } else {
            String::new()
        };

        let (new_content, found_linget_section) =
            remove_linget_section_from_content(&existing_content);

        let aliases_for_shell: Vec<_> = self
            .managed_aliases
            .iter()
            .filter(|a| a.shells.contains(&shell))
            .collect();

        let final_content = if !aliases_for_shell.is_empty() {
            build_content_with_aliases(&new_content, &aliases_for_shell, shell)
        } else if found_linget_section {
            new_content
        } else {
            return Ok(());
        };

        if config_path.exists() {
            let backup_path = config_path.with_extension("linget-backup");
            std::fs::copy(&config_path, &backup_path)
                .with_context(|| format!("Failed to backup {:?}", config_path))?;
        }

        std::fs::write(&config_path, &final_content)
            .with_context(|| format!("Failed to write {:?}", config_path))?;

        Ok(())
    }
}

fn remove_linget_section_from_content(content: &str) -> (String, bool) {
    let mut new_content = String::new();
    let mut in_linget_section = false;
    let mut found_section = false;

    for line in content.lines() {
        if line.trim() == LINGET_ALIAS_START {
            in_linget_section = true;
            found_section = true;
            continue;
        }
        if line.trim() == LINGET_ALIAS_END {
            in_linget_section = false;
            continue;
        }
        if !in_linget_section {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    (new_content, found_section)
}

fn build_content_with_aliases(base: &str, aliases: &[&ShellAlias], shell: Shell) -> String {
    let mut content = base.to_string();

    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    if !content.ends_with("\n\n") && !content.is_empty() {
        content.push('\n');
    }

    content.push_str(LINGET_ALIAS_START);
    content.push('\n');
    content.push_str("# Managed by LinGet - Do not edit manually\n");

    for alias in aliases {
        if let Some(desc) = &alias.description {
            content.push_str(&format!("# {}\n", desc));
        }
        content.push_str(&shell.format_alias(&alias.name, &alias.command));
        content.push('\n');
    }

    content.push_str(LINGET_ALIAS_END);
    content.push('\n');

    content
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> bool {
    true
}

#[derive(Debug, Clone, Default)]
pub struct AliasViewData {
    pub manager: AliasManager,
    pub search_query: String,
    pub is_loading: bool,
    pub show_existing: bool,
    pub filter_shell: Option<Shell>,
}

impl AliasViewData {
    pub fn filtered_aliases(&self) -> Vec<&ShellAlias> {
        let query = self.search_query.to_lowercase();

        let base = if self.show_existing {
            &self.manager.all_aliases
        } else {
            &self.manager.managed_aliases
        };

        base.iter()
            .filter(|a| {
                if !query.is_empty()
                    && !a.name.to_lowercase().contains(&query)
                    && !a.command.to_lowercase().contains(&query)
                {
                    return false;
                }

                if let Some(shell) = self.filter_shell {
                    if !a.shells.contains(&shell) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    pub fn filtered_commands(&self) -> Vec<&String> {
        let query = self.search_query.to_lowercase();

        self.manager
            .available_commands
            .iter()
            .filter(|c| query.is_empty() || c.to_lowercase().contains(&query))
            .take(100)
            .collect()
    }

    #[allow(dead_code)]
    pub fn filtered_package_commands(&self) -> Vec<&PackageCommands> {
        let query = self.search_query.to_lowercase();

        self.manager
            .package_commands
            .iter()
            .filter(|pkg| {
                if query.is_empty() {
                    return true;
                }
                pkg.package_name.to_lowercase().contains(&query)
                    || pkg
                        .commands
                        .iter()
                        .any(|c| c.name.to_lowercase().contains(&query))
            })
            .collect()
    }

    pub fn filtered_lazy_packages(&self) -> Vec<&LazyPackage> {
        let query = self.search_query.to_lowercase();

        self.manager
            .lazy_packages
            .iter()
            .filter(|pkg| query.is_empty() || pkg.name.to_lowercase().contains(&query))
            .collect()
    }

    pub fn get_package_commands_for(
        &self,
        name: &str,
        source: PackageSource,
    ) -> Option<&PackageCommands> {
        self.manager
            .package_commands
            .iter()
            .find(|p| p.package_name == name && p.source == source)
    }
}
