use crate::backend::SUGGEST_PREFIX;
use crate::models::{Package, PackageSource, PackageStatus};
use clap::ValueEnum;
use console::{style, Style};
use serde::Serialize;
use tabled::{
    settings::{object::Columns, Alignment, Modify, Style as TableStyle},
    Table, Tabled,
};

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
}

pub struct OutputWriter {
    format: OutputFormat,
    verbose: bool,
    quiet: bool,
}

impl OutputWriter {
    pub fn new(format: OutputFormat, verbose: bool, quiet: bool) -> Self {
        Self {
            format,
            verbose,
            quiet,
        }
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Get the output format
    pub fn format(&self) -> OutputFormat {
        self.format
    }

    /// Print a message (not printed in quiet mode or JSON mode)
    pub fn message(&self, msg: &str) {
        if !self.quiet && !self.is_json() {
            println!("{}", msg);
        }
    }

    /// Print a verbose message (only in verbose mode)
    pub fn verbose(&self, msg: &str) {
        if self.verbose && !self.quiet && !self.is_json() {
            println!("{} {}", style("▸").dim(), style(msg).dim());
        }
    }

    /// Print a success message
    pub fn success(&self, msg: &str) {
        if !self.quiet && !self.is_json() {
            println!("{} {}", style("✓").green().bold(), msg);
        }
    }

    /// Print an error message
    pub fn error(&self, msg: &str) {
        if self.is_json() {
            self.print_error_json(msg, None);
        } else {
            eprintln!("{} {}", style("✗").red().bold(), msg);
        }
    }

    /// Print a warning message
    pub fn warning(&self, msg: &str) {
        if !self.quiet && !self.is_json() {
            println!("{} {}", style("!").yellow().bold(), msg);
        }
    }

    /// Print an anyhow error with proper formatting
    pub fn anyhow_error(&self, error: &anyhow::Error) {
        let msg = error.to_string();

        // Check for LINGET_SUGGEST: prefix in the error message
        let (clean_msg, suggestion) = self.extract_suggestion(&msg);

        if self.is_json() {
            self.print_error_json(&clean_msg, suggestion.as_deref());
        } else {
            // Print the main error
            eprintln!("{} {}", style("✗").red().bold(), clean_msg);

            // Print the error chain for context (in verbose mode or if there are multiple causes)
            if self.verbose {
                let mut source = error.source();
                while let Some(cause) = source {
                    let cause_str = cause.to_string();
                    // Skip if it's the same as the main message
                    if cause_str != clean_msg {
                        eprintln!(
                            "  {} {}",
                            style("Caused by:").dim(),
                            style(&cause_str).dim()
                        );
                    }
                    source = cause.source();
                }
            }

            // Print suggestion if available
            if let Some(s) = suggestion {
                eprintln!();
                eprintln!("  {} {}", style("Try running:").yellow(), style(&s).cyan());
            }
        }
    }

    /// Extract suggestion from error message (looks for LINGET_SUGGEST: prefix)
    fn extract_suggestion(&self, message: &str) -> (String, Option<String>) {
        if let Some(idx) = message.find(SUGGEST_PREFIX) {
            let clean_msg = message[..idx].trim().to_string();
            let suggestion = message[idx + SUGGEST_PREFIX.len()..].trim().to_string();
            if suggestion.is_empty() {
                (clean_msg, None)
            } else {
                (clean_msg, Some(suggestion))
            }
        } else {
            (message.to_string(), None)
        }
    }

    /// Print error as JSON
    fn print_error_json(&self, message: &str, suggestion: Option<&str>) {
        #[derive(Serialize)]
        struct ErrorOutput {
            error: bool,
            message: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            suggestion: Option<String>,
        }

        let output = ErrorOutput {
            error: true,
            message: message.to_string(),
            suggestion: suggestion.map(|s| s.to_string()),
        };
        eprintln!("{}", serde_json::to_string_pretty(&output).unwrap());
    }

    /// Print a header/title
    pub fn header(&self, title: &str) {
        if !self.quiet && !self.is_json() {
            println!();
            println!("{}", style(title).bold().underlined());
            println!();
        }
    }

    /// Print package list
    pub fn packages(&self, packages: &[Package], title: Option<&str>) {
        match self.format {
            OutputFormat::Human => self.print_packages_human(packages, title),
            OutputFormat::Json => self.print_packages_json(packages),
        }
    }

    /// Print sources list
    pub fn sources(&self, available: &[PackageSource], enabled: &[PackageSource]) {
        match self.format {
            OutputFormat::Human => self.print_sources_human(available, enabled),
            OutputFormat::Json => self.print_sources_json(available, enabled),
        }
    }

    /// Print package info
    pub fn package_info(&self, package: &Package) {
        match self.format {
            OutputFormat::Human => self.print_package_info_human(package),
            OutputFormat::Json => self.print_package_json(package),
        }
    }

    fn print_packages_human(&self, packages: &[Package], title: Option<&str>) {
        if self.quiet {
            // Quiet mode: just print package names
            for pkg in packages {
                println!("{}", pkg.name);
            }
            return;
        }

        if let Some(t) = title {
            self.header(t);
        }

        if packages.is_empty() {
            println!("{}", style("No packages found").dim());
            return;
        }

        let rows: Vec<PackageRow> = packages.iter().map(PackageRow::from).collect();
        let mut table = Table::new(rows);
        table
            .with(TableStyle::rounded())
            .with(Modify::new(Columns::single(0)).with(Alignment::left()))
            .with(Modify::new(Columns::single(1)).with(Alignment::left()))
            .with(Modify::new(Columns::single(2)).with(Alignment::left()))
            .with(Modify::new(Columns::single(3)).with(Alignment::center()));

        println!("{}", table);
        println!();
        println!(
            "{}",
            style(format!("Total: {} packages", packages.len())).dim()
        );
    }

    fn print_packages_json(&self, packages: &[Package]) {
        #[derive(Serialize)]
        struct PackagesOutput {
            count: usize,
            packages: Vec<PackageJson>,
        }

        let output = PackagesOutput {
            count: packages.len(),
            packages: packages.iter().map(PackageJson::from).collect(),
        };

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }

    fn print_sources_human(&self, available: &[PackageSource], enabled: &[PackageSource]) {
        if self.quiet {
            for source in available {
                let status = if enabled.contains(source) {
                    "enabled"
                } else {
                    "disabled"
                };
                println!("{:?} {}", source, status);
            }
            return;
        }

        self.header("Package Sources");

        for source in available {
            let is_enabled = enabled.contains(source);
            let status_style = if is_enabled {
                Style::new().green()
            } else {
                Style::new().dim()
            };
            let icon = if is_enabled { "●" } else { "○" };
            let status = if is_enabled { "enabled" } else { "disabled" };

            println!(
                "  {} {:12} {}",
                status_style.apply_to(icon),
                format!("{:?}", source),
                status_style.apply_to(format!("({})", status))
            );
        }
    }

    fn print_sources_json(&self, available: &[PackageSource], enabled: &[PackageSource]) {
        #[derive(Serialize)]
        struct SourceInfo {
            name: String,
            enabled: bool,
        }

        #[derive(Serialize)]
        struct SourcesOutput {
            sources: Vec<SourceInfo>,
        }

        let sources: Vec<SourceInfo> = available
            .iter()
            .map(|s| SourceInfo {
                name: format!("{:?}", s).to_lowercase(),
                enabled: enabled.contains(s),
            })
            .collect();

        let output = SourcesOutput { sources };
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }

    fn print_package_info_human(&self, package: &Package) {
        if self.quiet {
            println!("{}", package.name);
            return;
        }

        println!();
        println!("{}", style(&package.name).bold().cyan());
        println!("{}", style("─".repeat(40)).dim());
        println!("  {:12} {}", style("Version:").bold(), package.version);
        if let Some(ref avail) = package.available_version {
            println!("  {:12} {}", style("Available:").bold(), avail);
        }
        println!("  {:12} {:?}", style("Source:").bold(), package.source);
        println!(
            "  {:12} {}",
            style("Status:").bold(),
            format_status(&package.status)
        );
        if !package.description.is_empty() {
            println!(
                "  {:12} {}",
                style("Description:").bold(),
                package.description
            );
        }
        if let Some(ref size) = package.size {
            println!(
                "  {:12} {}",
                style("Size:").bold(),
                humansize::format_size(*size, humansize::BINARY)
            );
        }
        if let Some(ref homepage) = package.homepage {
            println!("  {:12} {}", style("Homepage:").bold(), homepage);
        }
        if let Some(ref license) = package.license {
            println!("  {:12} {}", style("License:").bold(), license);
        }
        if let Some(ref maintainer) = package.maintainer {
            println!("  {:12} {}", style("Maintainer:").bold(), maintainer);
        }
        if let Some(ref date) = package.install_date {
            println!("  {:12} {}", style("Installed:").bold(), date);
        }
        println!();
    }

    fn print_package_json(&self, package: &Package) {
        let output = PackageJson::from(package);
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }
}

#[derive(Tabled)]
struct PackageRow {
    #[tabled(rename = "Package")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "Status")]
    status: String,
}

impl From<&Package> for PackageRow {
    fn from(pkg: &Package) -> Self {
        let version = if let Some(ref avail) = pkg.available_version {
            format!("{} → {}", pkg.version, avail)
        } else {
            pkg.version.clone()
        };

        Self {
            name: pkg.name.clone(),
            version,
            source: format!("{:?}", pkg.source).to_lowercase(),
            status: format_status_short(&pkg.status),
        }
    }
}

#[derive(Serialize)]
struct PackageJson {
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    available_version: Option<String>,
    source: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
}

impl From<&Package> for PackageJson {
    fn from(pkg: &Package) -> Self {
        Self {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            available_version: pkg.available_version.clone(),
            source: format!("{:?}", pkg.source).to_lowercase(),
            status: format!("{:?}", pkg.status).to_lowercase(),
            description: if pkg.description.is_empty() {
                None
            } else {
                Some(pkg.description.clone())
            },
            size: pkg.size,
            homepage: pkg.homepage.clone(),
            license: pkg.license.clone(),
        }
    }
}

fn format_status(status: &PackageStatus) -> String {
    match status {
        PackageStatus::Installed => style("Installed").green().to_string(),
        PackageStatus::UpdateAvailable => style("Update Available").yellow().to_string(),
        PackageStatus::NotInstalled => style("Not Installed").dim().to_string(),
        PackageStatus::Installing => style("Installing...").cyan().to_string(),
        PackageStatus::Removing => style("Removing...").red().to_string(),
        PackageStatus::Updating => style("Updating...").cyan().to_string(),
    }
}

fn format_status_short(status: &PackageStatus) -> String {
    match status {
        PackageStatus::Installed => style("✓").green().to_string(),
        PackageStatus::UpdateAvailable => style("↑").yellow().to_string(),
        PackageStatus::NotInstalled => style("○").dim().to_string(),
        PackageStatus::Installing => style("⟳").cyan().to_string(),
        PackageStatus::Removing => style("✗").red().to_string(),
        PackageStatus::Updating => style("⟳").cyan().to_string(),
    }
}
