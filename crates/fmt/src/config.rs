use serde::{Deserialize, Serialize};

/// Configuration for the formatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FmtConfig {
    /// Print width.
    #[serde(default = "default_print_width")]
    pub print_width: usize,

    /// Tab width (spaces).
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,

    /// Use tabs instead of spaces.
    #[serde(default)]
    pub use_tabs: bool,

    /// Semicolons.
    #[serde(default = "default_semicolons")]
    pub semicolons: bool,

    /// Single quotes.
    #[serde(default)]
    pub single_quotes: bool,

    /// Trailing commas.
    #[serde(default = "default_trailing_commas")]
    pub trailing_commas: bool,

    /// Line ending: "lf", "crlf", "auto".
    #[serde(default = "default_line_ending")]
    pub line_ending: String,

    /// Ensure trailing newline.
    #[serde(default = "default_true")]
    pub trailing_newline: bool,

    /// Trim trailing whitespace.
    #[serde(default = "default_true")]
    pub trim_trailing_whitespace: bool,
}

fn default_print_width() -> usize {
    80
}

fn default_tab_width() -> usize {
    2
}

fn default_semicolons() -> bool {
    true
}

fn default_trailing_commas() -> bool {
    true
}

fn default_line_ending() -> String {
    "lf".into()
}

fn default_true() -> bool {
    true
}

impl Default for FmtConfig {
    fn default() -> Self {
        Self {
            print_width: default_print_width(),
            tab_width: default_tab_width(),
            use_tabs: false,
            semicolons: default_semicolons(),
            single_quotes: false,
            trailing_commas: default_trailing_commas(),
            line_ending: default_line_ending(),
            trailing_newline: default_true(),
            trim_trailing_whitespace: default_true(),
        }
    }
}

impl FmtConfig {
    /// Load config from a jsraft.toml file.
    pub fn from_toml(content: &str) -> anyhow::Result<Self> {
        let config: FmtConfig = toml::from_str(content)?;
        Ok(config)
    }
}
