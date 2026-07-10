use crate::module::{LoadedModule, ModuleLoader};
use crate::Result;
use rquickjs::{AsyncContext, AsyncRuntime, CatchResultExt, Value};
use std::fmt::Display;
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
    /// JavaScript engine backend.
    pub engine: EngineKind,
}

/// JavaScript engine backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    QuickJs,
    #[cfg(feature = "v8")]
    V8,
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
            engine: EngineKind::QuickJs,
        }
    }
}

/// The main JSRaft runtime.
pub struct JsRuntime {
    config: RuntimeConfig,
    module_loader: Arc<ModuleLoader>,
}

/// A persistent JavaScript context for interactive evaluation.
pub struct ReplSession {
    _runtime: AsyncRuntime,
    context: AsyncContext,
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

        let modules = self.module_loader.load_graph(path)?;
        let source = runtime_source(&modules, path);

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

            let _: Value = ctx.eval(source.as_bytes()).catch(&ctx)
                .map_err(|e| js_error(path, e))?;

            Ok::<(), crate::JsRaftError>(())
        }).await?;

        Ok(())
    }

    /// Evaluate a JavaScript string.
    pub async fn eval(&self, code: &str, _filename: &str) -> Result<String> {
        let rt = AsyncRuntime::new().map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create runtime: {e}"))
        })?;

        let ctx = AsyncContext::full(&rt).await.map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create context: {e}"))
        })?;

        let result = ctx.with(|ctx| {
            crate::extensions::console::register(&ctx)?;

            let val: Value = ctx
                .eval(code.as_bytes())
                .catch(&ctx)
                .map_err(|e| crate::JsRaftError::JsError(format!("<eval>: {e}")))?;

            let result = value_to_string(&val);

            Ok::<String, crate::JsRaftError>(result)
        }).await?;

        Ok(result)
    }

    /// Evaluate and get a typed result.
    pub async fn eval_as<T: for<'js> rquickjs::FromJs<'js>>(
        &self,
        code: &str,
        _filename: &str,
    ) -> Result<T> {
        let rt = AsyncRuntime::new().map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create runtime: {e}"))
        })?;

        let ctx = AsyncContext::full(&rt).await.map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create context: {e}"))
        })?;

        let result = ctx.with(|ctx| {
            crate::extensions::console::register(&ctx)?;

                let val: Value = ctx
                    .eval(code.as_bytes())
                    .catch(&ctx)
                    .map_err(|e| crate::JsRaftError::JsError(format!("<eval>: {e}")))?;

            T::from_js(&ctx, val)
                .map_err(|e| crate::JsRaftError::JsError(format!("{e}")))
        }).await?;

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

impl ReplSession {
    /// Create a persistent REPL session with built-in APIs registered once.
    pub async fn new() -> Result<Self> {
        let runtime = AsyncRuntime::new().map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create runtime: {e}"))
        })?;

        let context = AsyncContext::full(&runtime).await.map_err(|e| {
            crate::JsRaftError::Runtime(format!("Failed to create context: {e}"))
        })?;

        context.with(|ctx| {
            crate::extensions::console::register(&ctx)?;
            crate::extensions::fs::register(&ctx)?;
            crate::extensions::net::register(&ctx)?;
            crate::extensions::path::register(&ctx)?;
            crate::extensions::process::register(&ctx)?;
            crate::extensions::timers::register(&ctx)?;
            Ok::<(), crate::JsRaftError>(())
        }).await?;

        Ok(Self {
            _runtime: runtime,
            context,
        })
    }

    /// Evaluate a single input while preserving global context state.
    pub async fn eval(&self, code: &str) -> Result<String> {
        let result = self.context.with(|ctx| {
            let val: Value = ctx
                .eval(code.as_bytes())
                .catch(&ctx)
                .map_err(|e| crate::JsRaftError::JsError(format!("<repl>: {e}")))?;
            Ok::<String, crate::JsRaftError>(value_to_string(&val))
        }).await?;

        Ok(result)
    }
}

fn value_to_string(value: &Value<'_>) -> String {
    if value.is_undefined() {
        return String::new();
    }
    if value.is_null() {
        return "null".into();
    }
    if let Some(string) = value.as_string().and_then(|s| s.to_string().ok()) {
        return string;
    }
    if let Some(number) = value.as_number() {
        return number.to_string();
    }
    if let Some(boolean) = value.as_bool() {
        return boolean.to_string();
    }
    "[object]".into()
}

fn runtime_source(modules: &[LoadedModule], entry: &Path) -> String {
    let mut source = String::new();

    for module in modules {
        source.push_str("\n// Module: ");
        source.push_str(&module.path.display().to_string());
        source.push('\n');
        source.push_str(&strip_esm_syntax(&module.source));
        source.push('\n');
    }

    source.push_str("\n//# sourceURL=");
    source.push_str(&entry.display().to_string());
    source.push('\n');

    source
}

fn strip_esm_syntax(source: &str) -> String {
    let mut output = String::with_capacity(source.len());

    for line in source.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("import ") {
            output.push('\n');
            continue;
        }

        if trimmed.starts_with("export {") || trimmed.starts_with("export *") {
            output.push('\n');
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("export default ") {
            let indent_len = line.len() - trimmed.len();
            output.push_str(&line[..indent_len]);
            output.push_str("const __default = ");
            output.push_str(rest);
            output.push('\n');
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("export ") {
            let indent_len = line.len() - trimmed.len();
            output.push_str(&line[..indent_len]);
            output.push_str(rest);
            output.push('\n');
            continue;
        }

        output.push_str(line);
        output.push('\n');
    }

    output
}

fn js_error(path: &Path, error: impl Display) -> crate::JsRaftError {
    crate::JsRaftError::JsError(format!("{}: {error}", path.display()))
}

/// Run a file directly (convenience function).
pub async fn run_file(path: &Path) -> Result<()> {
    let config = RuntimeConfig::default();
    let runtime = JsRuntime::new(config);
    runtime.run_file(path).await
}
