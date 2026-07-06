pub mod config;

pub use config::FmtConfig;

use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;

/// The JSRaft formatter.
///
/// For MVP, performs basic formatting operations.
/// In production, integrate Oxc codegen for AST-based formatting.
pub struct Formatter {
    config: FmtConfig,
}

impl Formatter {
    /// Create a new formatter with the given config.
    pub fn new(config: FmtConfig) -> Self {
        Self { config }
    }

    /// Format a file in-place.
    pub fn format_file(&self, path: &Path, check: bool) -> Result<FmtResult> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;

        let result = self.format_source(&source)?;

        if check {
            Ok(FmtResult {
                changed: result != source,
                original: source,
                formatted: result,
                file: path.to_path_buf(),
            })
        } else {
            if result != source {
                std::fs::write(path, &result)?;
            }
            Ok(FmtResult {
                changed: result != source,
                original: source,
                formatted: result,
                file: path.to_path_buf(),
            })
        }
    }

    /// Format source code directly.
    pub fn format_source(&self, source: &str) -> Result<String> {
        let mut result = source.to_string();

        // Normalize line endings
        if self.config.line_ending == "lf" {
            result = result.replace("\r\n", "\n");
        } else if self.config.line_ending == "crlf" {
            result = result.replace("\n", "\r\n");
        }

        // Remove trailing whitespace from each line
        if self.config.trim_trailing_whitespace {
            let lines: Vec<&str> = result.lines().collect();
            result = lines
                .iter()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>()
                .join("\n");
        }

        // Ensure single newline at end of file
        if self.config.trailing_newline {
            result = result.trim_end_matches('\n').to_string();
            result.push('\n');
        } else {
            result = result.trim_end_matches('\n').to_string();
        }

        // Normalize multiple blank lines to single
        while result.contains("\n\n\n") {
            result = result.replace("\n\n\n", "\n\n");
        }

        // Ensure space after keywords
        let keywords = ["if", "else", "for", "while", "switch", "return"];
        for keyword in &keywords {
            let pattern = format!("{keyword}(");
            let replacement = format!("{keyword} (");
            // Only replace if not already followed by space
            if result.contains(&pattern) && !result.contains(&replacement) {
                result = result.replace(&pattern, &replacement);
            }
        }

        Ok(result)
    }
}

/// Result of a format operation.
#[derive(Debug)]
pub struct FmtResult {
    pub changed: bool,
    pub original: String,
    pub formatted: String,
    pub file: std::path::PathBuf,
}
