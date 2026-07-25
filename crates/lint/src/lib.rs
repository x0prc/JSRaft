pub mod config;
pub mod rules;

pub use config::LintConfig;
pub use rules::{LintError, LintResult, LintWarning};

use anyhow::{Context, Result};
use std::path::Path;

/// The JSRaft linter.
///
/// For MVP, uses regex-based rules.
/// In production, integrate oxc_linter when available.
pub struct LinterEngine {
    _config: LintConfig,
}

impl LinterEngine {
    /// Create a new linter with the given config.
    pub fn new(config: LintConfig) -> Self {
        Self { _config: config }
    }

    /// Lint a file and return warnings/errors.
    pub fn lint_file(&self, path: &Path) -> Result<LintResult> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;

        self.lint_source(&source, Some(path))
    }

    /// Lint source code directly.
    pub fn lint_source(&self, source: &str, _path: Option<&Path>) -> Result<LintResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic lint rules for MVP
        for (line_num, line) in source.lines().enumerate() {
            let line_num = line_num + 1;

            // Rule: no-console (warn)
            if line.contains("console.log") && !line.trim().starts_with("//") {
                warnings.push(LintError {
                    message: "Unexpected console statement".into(),
                    line: line_num,
                    column: line.find("console.log").unwrap_or(0),
                    rule: "no-console".into(),
                });
            }

            // Rule: no-debugger (error)
            if line.contains("debugger") && !line.trim().starts_with("//") {
                errors.push(LintError {
                    message: "Unexpected debugger statement".into(),
                    line: line_num,
                    column: line.find("debugger").unwrap_or(0),
                    rule: "no-debugger".into(),
                });
            }

            // Rule: no-alert (warn)
            if (line.contains("alert(") || line.contains("confirm(") || line.contains("prompt("))
                && !line.trim().starts_with("//")
            {
                warnings.push(LintError {
                    message: "Unexpected alert/confirm/prompt".into(),
                    line: line_num,
                    column: 0,
                    rule: "no-alert".into(),
                });
            }

            // Rule: eqeqeq (warn) - == or != instead of === or !==
            let trimmed = line.trim();
            if !trimmed.starts_with("//") && !trimmed.starts_with("*") {
                if line.contains(" == ") && !line.contains(" === ") {
                    warnings.push(LintError {
                        message: "Expected '===' but found '=='".into(),
                        line: line_num,
                        column: line.find(" == ").unwrap_or(0),
                        rule: "eqeqeq".into(),
                    });
                }
                if line.contains(" != ") && !line.contains(" !== ") {
                    warnings.push(LintError {
                        message: "Expected '!==' but found '!='".into(),
                        line: line_num,
                        column: line.find(" != ").unwrap_or(0),
                        rule: "eqeqeq".into(),
                    });
                }
            }

            // Rule: no-var (warn)
            if line.contains("var ") && !line.trim().starts_with("//") {
                warnings.push(LintError {
                    message: "Unexpected var, use let or const".into(),
                    line: line_num,
                    column: line.find("var ").unwrap_or(0),
                    rule: "no-var".into(),
                });
            }

            // Rule: no-unused-vars (warn) - simple check
            if line.contains("let ") || line.contains("const ") {
                let var_name = if let Some(pos) = line.find("let ") {
                    &line[pos + 4..]
                } else if let Some(pos) = line.find("const ") {
                    &line[pos + 6..]
                } else {
                    ""
                };

                if let Some(name) = var_name.split_whitespace().next() {
                    let name = name.trim_end_matches('=');
                    // Check if variable is used elsewhere (simple check)
                    if !source.contains(name) || source.matches(name).count() <= 1 {
                        warnings.push(LintError {
                            message: format!("Unused variable '{name}'"),
                            line: line_num,
                            column: 0,
                            rule: "no-unused-vars".into(),
                        });
                    }
                }
            }
        }

        Ok(LintResult {
            errors,
            warnings,
            files_checked: 1,
        })
    }

    /// Lint multiple files.
    pub fn lint_files(&self, paths: &[&Path]) -> Result<LintResult> {
        let mut combined = LintResult {
            errors: Vec::new(),
            warnings: Vec::new(),
            files_checked: 0,
        };

        for path in paths {
            let result = self.lint_file(path)?;
            combined.errors.extend(result.errors);
            combined.warnings.extend(result.warnings);
            combined.files_checked += result.files_checked;
        }

        Ok(combined)
    }
}
