use std::path::{Path, PathBuf};
use tracing::debug;

/// Module resolver and loader.
pub struct ModuleLoader {
    root: PathBuf,
}

impl ModuleLoader {
    /// Create a new module loader rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Resolve a module specifier to a file path.
    pub fn resolve(&self, specifier: &str, importer: Option<&Path>) -> Option<PathBuf> {
        let cwd = importer
            .and_then(|p| p.parent())
            .unwrap_or(&self.root);

        debug!(
            "Resolving '{}' from '{}'",
            specifier,
            cwd.display()
        );

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
        std::fs::read_to_string(path)
    }

    /// Resolve and read a module in one step.
    pub fn load(&self, specifier: &str, importer: Option<&Path>) -> std::io::Result<(PathBuf, String)> {
        let path = self.resolve(specifier, importer)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Module not found: {specifier}"),
                )
            })?;
        let source = self.read_module(&path)?;
        Ok((path, source))
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
            if current.as_ref().map(|p| p == Path::new("/")).unwrap_or(false) {
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
