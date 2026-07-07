use serde::{Deserialize, Serialize};

/// Configuration for the bundler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleConfig {
    /// Entry point(s).
    pub entry: Vec<String>,
    /// Output directory.
    pub outdir: Option<String>,
    /// Output filename (for single entry).
    pub outfile: Option<String>,
    /// Bundle format: "esm", "iife", "cjs".
    pub format: String,
    /// Target environment.
    pub target: String,
    /// Enable minification.
    pub minify: bool,
    /// Enable source maps.
    pub sourcemap: bool,
    /// External modules (not bundled).
    pub external: Vec<String>,
    /// Define global constants.
    pub define: std::collections::HashMap<String, String>,
    /// Enable tree shaking.
    pub tree_shaking: bool,
}

impl Default for BundleConfig {
    fn default() -> Self {
        Self {
            entry: vec!["src/index.js".into()],
            outdir: Some("dist".into()),
            outfile: None,
            format: "esm".into(),
            target: "es2020".into(),
            minify: false,
            sourcemap: true,
            external: Vec::new(),
            define: std::collections::HashMap::new(),
            tree_shaking: true,
        }
    }
}

impl BundleConfig {
    /// Load config from a jsraft.toml file.
    pub fn from_toml(content: &str) -> anyhow::Result<Self> {
        let config: BundleConfig = toml::from_str(content)?;
        Ok(config)
    }
}
