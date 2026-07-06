use crate::module::ModuleLoader;
use crate::Result;
use rquickjs::{AsyncContext, AsyncRuntime, Module};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

/// Configuration for the JSRaft runtime.
pub struct RuntimeConfig {
    /// Root directory for module resolution.
    pub root: PathBuf,
    /// Enable TypeScript transpilation.
    pub typescript: bool,
    /// Enable source maps.
    pub source_maps: bool,
    /// Strict mode.
    pub strict: bool,
    /// Maximum stack size in bytes.
    pub max_stack_size: usize,
    /// Memory limit in bytes (0 = unlimited).
    pub memory_limit: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            typescript: true,
            source_maps: true,
            strict: true,
            max_stack_size: 512 * 1024,       // 512KB
            memory_limit: 128 * 1024 * 1024,  // 128MB
        }
    }
}

/// The main JSRaft runtime.
pub struct JsRuntime {
    config: RuntimeConfig,
    module_loader: Arc<ModuleLoader>,
}

impl JsRuntime {
    /// Create a new runtime with the given config.
    pub fn new(config: RuntimeConfig) -> Self {
        let module_loader = Arc::new(ModuleLoader::new(config.root.clone()));
        Self {
            config,
            module_loader,
        }
    }

    /// Run a JavaScript/TypeScript file.
    pub async fn run_file(&self, path: &Path) -> Result<()> {
        info!("Running file: {}", path.display());

        let rt = AsyncRuntime::new().map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create runtime: {e}"))
        })?;

        let ctx = AsyncContext::full(&rt).await.map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create context: {e}"))
        })?;

        ctx.with(|ctx| {
            // Register built-in extensions
            crate::extensions::console::register(&ctx)?;
            crate::extensions::fs::register(&ctx)?;
            crate::extensions::net::register(&ctx)?;
            crate::extensions::path::register(&ctx)?;
            crate::extensions::process::register(&ctx)?;
            crate::extensions::timers::register(&ctx)?;

            // Load and execute the entry file
            let path_str = path.to_string_lossy().to_string();
            let source = std::fs::read_to_string(path)
                .map_err(|e| crate::JsRaftError::Io(e))?;

            let module = Module::declare(ctx, path_str.as_str(), source.as_bytes())
                .map_err(|e| crate::JsRaftError::JsError(format!("{e}")))?;

            Ok(())
        }).await.map_err(|e| crate::JsRaftError::Runtime(format!("Context error: {e}")))?;

        Ok(())
    }

    /// Evaluate a JavaScript string.
    pub async fn eval(&self, code: &str, filename: &str) -> Result<String> {
        let rt = AsyncRuntime::new().map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create runtime: {e}"))
        })?;

        let ctx = AsyncContext::full(&rt).await.map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create context: {e}"))
        })?;

        let result = ctx.with(|ctx| {
            crate::extensions::console::register(&ctx)?;

            let val: rquickjs::Value = ctx
                .eval(code.as_bytes())
                .map_err(|e| crate::JsRaftError::JsError(format!("{e}")))?;

            let result = val
                .as_string()
                .and_then(|s| s.to_string().ok())
                .unwrap_or_default();

            Ok(result)
        }).await.map_err(|e| crate::JsRaftError::Runtime(format!("Context error: {e}")))?;

        Ok(result)
    }

    /// Evaluate and get a typed result.
    pub async fn eval_as<T: for<'js> rquickjs::FromJs<'js>>(
        &self,
        code: &str,
        filename: &str,
    ) -> Result<T> {
        let rt = AsyncRuntime::new().map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create runtime: {e}"))
        })?;

        let ctx = AsyncContext::full(&rt).await.map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create context: {e}"))
        })?;

        let result = ctx.with(|ctx| {
            crate::extensions::console::register(&ctx)?;

            let val: rquickjs::Value = ctx
                .eval(code.as_bytes())
                .map_err(|e| crate::JsRaftError::JsError(format!("{e}")))?;

            T::from_js(&ctx, val)
                .map_err(|e| crate::JsRaftError::JsError(format!("{e}")))
        }).await.map_err(|e| crate::JsRaftError::Runtime(format!("Context error: {e}")))?;

        Ok(result)
    }

    /// Get the module loader.
    pub fn module_loader(&self) -> &Arc<ModuleLoader> {
        &self.module_loader
    }

    /// Get the config.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
}

/// Run a file directly (convenience function).
pub async fn run_file(path: &Path) -> Result<()> {
    let config = RuntimeConfig::default();
    let runtime = JsRuntime::new(config);
    runtime.run_file(path).await
}
