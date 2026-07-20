use crate::{read_text_mmap, transform::transform_typescript, JsRaftError, Result};
use rquickjs::{loader, Ctx, Module};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::debug;

/// A source file loaded as part of a runtime module graph.
#[derive(Debug, Clone)]
pub struct LoadedModule {
    pub path: PathBuf,
    pub source: String,
}

/// Module resolver and loader.
pub struct ModuleLoader {
    root: PathBuf,
}

/// QuickJS resolver backed by JSRaft module resolution.
pub struct QuickJsModuleResolver {
    loader: ModuleLoader,
}

/// QuickJS loader backed by JSRaft file loading.
pub struct QuickJsModuleLoader {
    loader: ModuleLoader,
}

impl ModuleLoader {
    /// Create a new module loader rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolve a module specifier to a file path.
    pub fn resolve(&self, specifier: &str, importer: Option<&Path>) -> Option<PathBuf> {
        let cwd = importer.and_then(|p| p.parent()).unwrap_or(&self.root);

        debug!("Resolving '{}' from '{}'", specifier, cwd.display());

        // Handle relative paths
        if specifier.starts_with('.') {
            let base = cwd.join(specifier);
            return self.try_resolve_file(&base);
        }

        // Handle absolute paths
        if specifier.starts_with('/') {
            let base = PathBuf::from(specifier);
            return self.try_resolve_file(&base);
        }

        // Handle node_modules
        self.resolve_node_modules(specifier, cwd)
    }

    /// Read a module's source code.
    pub fn read_module(&self, path: &Path) -> std::io::Result<String> {
        read_text_mmap(path)
    }

    /// Resolve and read a module in one step.
    pub fn load(
        &self,
        specifier: &str,
        importer: Option<&Path>,
    ) -> std::io::Result<(PathBuf, String)> {
        let path = self.resolve(specifier, importer).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Module not found: {specifier}"),
            )
        })?;
        let source = self.read_module(&path)?;
        Ok((path, source))
    }

    /// Load an entry file and all relative imports in dependency-first order.
    pub fn load_graph(&self, entry: &Path) -> Result<Vec<LoadedModule>> {
        let entry = if entry.is_absolute() {
            entry.to_path_buf()
        } else {
            self.root.join(entry)
        };

        let mut visited = HashSet::new();
        let mut modules = Vec::new();
        self.collect_graph(&entry, &mut visited, &mut modules)?;
        Ok(modules)
    }

    fn collect_graph(
        &self,
        path: &Path,
        visited: &mut HashSet<PathBuf>,
        modules: &mut Vec<LoadedModule>,
    ) -> Result<()> {
        let path = path.canonicalize()?;
        if !visited.insert(path.clone()) {
            return Ok(());
        }

        let source = self.read_module(&path)?;
        for specifier in extract_import_specifiers(&source) {
            if !(specifier.starts_with('.') || specifier.starts_with('/')) {
                continue;
            }

            let resolved = self.resolve(&specifier, Some(&path)).ok_or_else(|| {
                JsRaftError::ModuleNotFound(format!("{specifier} imported from {}", path.display()))
            })?;
            self.collect_graph(&resolved, visited, modules)?;
        }

        let source = transform_typescript(&path, &source)?;
        modules.push(LoadedModule { path, source });
        Ok(())
    }

    fn try_resolve_file(&self, base: &Path) -> Option<PathBuf> {
        // Try as-is
        if base.is_file() {
            return Some(base.to_path_buf());
        }

        // Try with extensions
        let extensions = [".js", ".ts", ".jsx", ".tsx", ".mjs", ".cjs", ".json"];
        for ext in extensions {
            let candidate = base.with_extension(ext.trim_start_matches('.'));
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        // Try as directory with index files
        for index in &["index.js", "index.ts", "index.mjs", "index.cjs"] {
            let candidate = base.join(index);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        None
    }

    fn resolve_node_modules(&self, specifier: &str, cwd: &Path) -> Option<PathBuf> {
        let mut current = Some(cwd.to_path_buf());

        while let Some(dir) = current {
            let candidate = dir.join("node_modules").join(specifier);
            if let Some(resolved) = self.try_resolve_file(&candidate) {
                return Some(resolved);
            }

            // Walk up
            current = dir.parent().map(|p| p.to_path_buf());

            // Stop at root
            if current
                .as_ref()
                .map(|p| p == Path::new("/"))
                .unwrap_or(false)
            {
                break;
            }
        }

        None
    }

    /// Get the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl QuickJsModuleResolver {
    /// Create a QuickJS resolver rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self {
            loader: ModuleLoader::new(root),
        }
    }
}

impl QuickJsModuleLoader {
    /// Create a QuickJS loader rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self {
            loader: ModuleLoader::new(root),
        }
    }
}

impl loader::Resolver for QuickJsModuleResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
    ) -> rquickjs::Result<String> {
        let importer = if base.is_empty() || base == "<input>" {
            None
        } else {
            Some(Path::new(base))
        };

        self.loader
            .resolve(name, importer)
            .and_then(|path| path.canonicalize().ok())
            .map(|path| path.to_string_lossy().into_owned())
            .ok_or_else(|| rquickjs::Error::new_resolving_message(base, name, "module not found"))
    }
}

impl loader::Loader for QuickJsModuleLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> rquickjs::Result<Module<'js>> {
        let path = Path::new(name);
        let source = self
            .loader
            .read_module(path)
            .map_err(|e| rquickjs::Error::new_loading_message(name, e.to_string()))?;

        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let source = format!("export default {};", source);
            return Module::declare(ctx.clone(), name, source);
        }

        let source = transform_typescript(path, &source)
            .map_err(|e| rquickjs::Error::new_loading_message(name, e.to_string()))?;

        Module::declare(ctx.clone(), name, source)
    }
}

/// Extract static ESM import/export specifiers from source.
pub fn extract_import_specifiers(source: &str) -> Vec<String> {
    let mut imports = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("import ") || trimmed.starts_with("export "))
            && trimmed.contains(['\'', '"'])
        {
            if let Some(specifier) = quoted_specifier(trimmed) {
                imports.push(specifier);
            }
        }
    }

    imports
}

fn quoted_specifier(input: &str) -> Option<String> {
    for quote in ['\'', '"'] {
        if let Some(start) = input.find(quote) {
            let rest = &input[start + 1..];
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_string());
            }
        }
    }

    None
}
