use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the linter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    /// Rules to enable/disable.
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,

    /// Rule categories to enable.
    #[serde(default = "default_categories")]
    pub categories: Vec<String>,

    /// Files/directories to ignore.
    #[serde(default)]
    pub ignore: Vec<String>,

    /// Maximum warnings to report.
    #[serde(default = "default_max_warnings")]
    pub max_warnings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    pub level: String, // "error", "warn", "off"
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            rules: HashMap::new(),
            categories: default_categories(),
            ignore: vec![
                "node_modules".into(),
                "dist".into(),
                ".jsraft".into(),
            ],
            max_warnings: 100,
        }
    }
}

fn default_categories() -> Vec<String> {
    vec![
        "correctness".into(),
        "suspicious".into(),
        "pedantic".into(),
    ]
}

fn default_max_warnings() -> usize {
    100
}

impl LintConfig {
    /// Load config from a jsraft.toml file.
    pub fn from_toml(content: &str) -> anyhow::Result<Self> {
        let config: LintConfig = toml::from_str(content)?;
        Ok(config)
    }
}
