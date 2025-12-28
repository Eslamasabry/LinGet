use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangelogSummary {
    pub security_fixes: u32,
    pub bug_fixes: u32,
    pub new_features: u32,
    pub other_changes: u32,
    pub highlights: Vec<String>,
}

impl ChangelogSummary {
    pub fn parse(changelog: &str) -> Self {
        let mut summary = ChangelogSummary::default();
        let mut highlights = Vec::new();

        let lower = changelog.to_lowercase();
        let lines: Vec<&str> = lower.lines().collect();

        for line in &lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if Self::is_security_line(line) {
                summary.security_fixes += 1;
                if summary.security_fixes <= 2 {
                    if let Some(highlight) = Self::extract_highlight(line) {
                        highlights.push(format!("🔒 {}", highlight));
                    }
                }
            } else if Self::is_bugfix_line(line) {
                summary.bug_fixes += 1;
            } else if Self::is_feature_line(line) {
                summary.new_features += 1;
                if summary.new_features <= 2 && highlights.len() < 4 {
                    if let Some(highlight) = Self::extract_highlight(line) {
                        highlights.push(format!("✨ {}", highlight));
                    }
                }
            } else if Self::is_change_line(line) {
                summary.other_changes += 1;
            }
        }

        summary.highlights = highlights;
        summary
    }

    fn is_security_line(line: &str) -> bool {
        line.contains("security")
            || line.contains("cve-")
            || line.contains("vulnerability")
            || line.contains("exploit")
            || line.contains("injection")
            || line.contains("xss")
            || line.contains("csrf")
            || line.contains("authentication")
            || line.contains("authorization")
            || line.contains("privilege")
    }

    fn is_bugfix_line(line: &str) -> bool {
        (line.contains("fix") && !line.contains("prefix") && !line.contains("suffix"))
            || line.contains("bug")
            || line.contains("patch")
            || line.contains("issue")
            || line.contains("resolved")
            || line.contains("corrected")
            || line.contains("repair")
            || line.contains("crash")
            || line.contains("error")
    }

    fn is_feature_line(line: &str) -> bool {
        line.contains("add ")
            || line.contains("added")
            || line.contains("new ")
            || line.contains("feature")
            || line.contains("implement")
            || line.contains("introduce")
            || line.contains("support for")
            || line.contains("enable")
    }

    fn is_change_line(line: &str) -> bool {
        line.starts_with("*")
            || line.starts_with("-")
            || line.starts_with("•")
            || line.starts_with("[")
            || line.contains("update")
            || line.contains("change")
            || line.contains("improve")
            || line.contains("enhance")
            || line.contains("refactor")
            || line.contains("deprecate")
            || line.contains("remove")
    }

    fn extract_highlight(line: &str) -> Option<String> {
        let cleaned = line
            .trim_start_matches(|c: char| c == '*' || c == '-' || c == '•' || c.is_whitespace())
            .trim_start_matches('[')
            .split(']')
            .next_back()
            .unwrap_or(line)
            .trim();

        if cleaned.len() > 10 && cleaned.len() < 100 {
            let capitalized = cleaned
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default()
                + &cleaned[1..];
            Some(capitalized)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn total_changes(&self) -> u32 {
        self.security_fixes + self.bug_fixes + self.new_features + self.other_changes
    }

    pub fn summary_text(&self) -> String {
        let mut parts = Vec::new();

        if self.security_fixes > 0 {
            parts.push(format!(
                "{} security {}",
                self.security_fixes,
                if self.security_fixes == 1 {
                    "fix"
                } else {
                    "fixes"
                }
            ));
        }
        if self.bug_fixes > 0 {
            parts.push(format!(
                "{} bug {}",
                self.bug_fixes,
                if self.bug_fixes == 1 { "fix" } else { "fixes" }
            ));
        }
        if self.new_features > 0 {
            parts.push(format!(
                "{} new {}",
                self.new_features,
                if self.new_features == 1 {
                    "feature"
                } else {
                    "features"
                }
            ));
        }

        if parts.is_empty() {
            if self.other_changes > 0 {
                format!("{} changes", self.other_changes)
            } else {
                "No notable changes".to_string()
            }
        } else {
            parts.join(", ")
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.total_changes() == 0
    }

    #[allow(dead_code)]
    pub fn has_security_updates(&self) -> bool {
        self.security_fixes > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_security_fixes() {
        let log = r#"
        * Fixed security vulnerability CVE-2024-1234
        * Fixed authentication bypass issue
        "#;
        let summary = ChangelogSummary::parse(log);
        assert_eq!(summary.security_fixes, 2);
    }

    #[test]
    fn test_parse_bug_fixes() {
        let log = r#"
        - Fix crash on startup
        - Resolved issue with file loading
        - Bug fix for memory leak
        "#;
        let summary = ChangelogSummary::parse(log);
        assert_eq!(summary.bug_fixes, 3);
    }

    #[test]
    fn test_parse_features() {
        let log = r#"
        * Added new dark mode
        * Implemented drag and drop support
        * New feature: export to PDF
        "#;
        let summary = ChangelogSummary::parse(log);
        assert_eq!(summary.new_features, 3);
    }

    #[test]
    fn test_summary_text() {
        let summary = ChangelogSummary {
            security_fixes: 2,
            bug_fixes: 5,
            new_features: 1,
            other_changes: 10,
            highlights: Vec::new(),
        };
        assert_eq!(
            summary.summary_text(),
            "2 security fixes, 5 bug fixes, 1 new feature"
        );
    }
}
