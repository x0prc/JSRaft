use crate::{read_text_mmap, JsRaftError, Result};
use rquickjs::{CatchResultExt, Ctx, Value};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// A JavaScript plugin discovered from disk.
#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
}

/// Discovers and loads JavaScript plugins.
pub struct PluginManager {
    dirs: Vec<PathBuf>,
}

impl PluginManager {
    /// Create a plugin manager from plugin directories.
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Self { dirs }
    }

    /// Discover JavaScript plugin files from configured directories.
    pub fn discover(&self) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for dir in &self.dirs {
            if !dir.exists() {
                debug!("Plugin directory does not exist: {}", dir.display());
                continue;
            }

            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if !is_plugin_file(&path) {
                    continue;
                }

                let source = read_text_mmap(&path)?;
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("plugin")
                    .to_string();

                plugins.push(Plugin { name, path, source });
            }
        }

        plugins.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(plugins)
    }

    /// Register the global plugin API and evaluate discovered plugins.
    pub fn load_into<'js>(&self, ctx: &Ctx<'js>) -> Result<Vec<String>> {
        register_plugin_api(ctx)?;

        let plugins = self.discover()?;
        let mut loaded = Vec::new();

        for plugin in plugins {
            info!("Loading plugin: {} ({})", plugin.name, plugin.path.display());
            let source = plugin_source(&plugin);
            let _: Value = ctx
                .eval(source.as_bytes())
                .catch(ctx)
                .map_err(|e| {
                    JsRaftError::Extension(format!(
                        "Plugin {} failed to load: {e}",
                        plugin.path.display()
                    ))
                })?;
            loaded.push(plugin.name);
        }

        Ok(loaded)
    }
}

fn register_plugin_api(ctx: &Ctx<'_>) -> Result<()> {
    let bootstrap = r#"
        globalThis.JSRaft = globalThis.JSRaft || {};
        JSRaft.plugins = JSRaft.plugins || [];
        JSRaft.registerPlugin = JSRaft.registerPlugin || function registerPlugin(name) {
            JSRaft.plugins.push(String(name));
        };
    "#;

    let _: Value = ctx.eval(bootstrap.as_bytes()).catch(ctx).map_err(|e| {
        JsRaftError::Extension(format!("Failed to initialize JSRaft plugin API: {e}"))
    })?;
    Ok(())
}

fn is_plugin_file(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|ext| {
            matches!(ext.to_str(), Some("js" | "mjs" | "cjs"))
        })
}

fn plugin_source(plugin: &Plugin) -> String {
    format!(
        "\n// Plugin: {}\n{}\n//# sourceURL={}\n",
        plugin.path.display(),
        plugin.source,
        plugin.path.display()
    )
}
