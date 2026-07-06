use serde::{Deserialize, Serialize};

/// A lint error (severity: error).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub rule: String,
}

/// A lint warning (severity: warning).
pub type LintWarning = LintError;

/// Result of a lint operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    pub errors: Vec<LintError>,
    pub warnings: Vec<LintWarning>,
    pub files_checked: usize,
}

impl LintResult {
    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any issues (errors or warnings).
    pub fn has_issues(&self) -> bool {
        self.has_errors() || !self.warnings.is_empty()
    }

    /// Total count of issues.
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// Format results for display.
    pub fn display(&self) -> String {
        let mut output = String::new();

        for error in &self.errors {
            output.push_str(&format!(
                "error[{}] at {}:{}: {}\n",
                error.rule, error.line, error.column, error.message
            ));
        }

        for warning in &self.warnings {
            output.push_str(&format!(
                "warn[{}] at {}:{}: {}\n",
                warning.rule, warning.line, warning.column, warning.message
            ));
        }

        output.push_str(&format!(
            "\n{} files checked, {} errors, {} warnings\n",
            self.files_checked,
            self.errors.len(),
            self.warnings.len()
        ));

        output
    }
}
