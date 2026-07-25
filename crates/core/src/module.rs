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
        let (package_name, subpath) = split_package_specifier(specifier)?;
        let mut current = Some(cwd.to_path_buf());

        while let Some(dir) = current {
            let package_dir = dir.join("node_modules").join(package_name);

            if let Some(subpath) = subpath {
                if let Some(resolved) = self.resolve_package_subpath(&package_dir, subpath) {
                    return Some(resolved);
                }
            } else if let Some(resolved) = self.resolve_package_entry(&package_dir) {
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

    fn resolve_package_entry(&self, package_dir: &Path) -> Option<PathBuf> {
        if !package_dir.is_dir() {
            return self.try_resolve_file(package_dir);
        }

        if let Some(package_json) = read_package_json(package_dir) {
            for target in package_entry_targets(&package_json) {
                if let Some(resolved) = self.try_resolve_file(&package_dir.join(target)) {
                    return Some(resolved);
                }
            }
        }

        for index in ["index.js", "index.mjs", "index.cjs", "index.ts"] {
            let candidate = package_dir.join(index);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        None
    }

    fn resolve_package_subpath(&self, package_dir: &Path, subpath: &str) -> Option<PathBuf> {
        if !package_dir.is_dir() {
            return None;
        }

        if let Some(package_json) = read_package_json(package_dir) {
            let export_key = format!("./{}", subpath.trim_start_matches("./"));
            for target in package_export_targets(&package_json, &export_key) {
                if let Some(resolved) = self.try_resolve_file(&package_dir.join(target)) {
                    return Some(resolved);
                }
            }
        }

        self.try_resolve_file(&package_dir.join(subpath))
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

fn split_package_specifier(specifier: &str) -> Option<(&str, Option<&str>)> {
    if specifier.is_empty() || specifier.starts_with('.') || specifier.starts_with('/') {
        return None;
    }

    if specifier.starts_with('@') {
        let mut parts = specifier.splitn(3, '/');
        let scope = parts.next()?;
        let name = parts.next()?;
        let package_len = scope.len() + 1 + name.len();
        let subpath = parts.next();
        return Some((&specifier[..package_len], subpath));
    }

    if let Some((package_name, subpath)) = specifier.split_once('/') {
        Some((package_name, Some(subpath)))
    } else {
        Some((specifier, None))
    }
}

fn read_package_json(package_dir: &Path) -> Option<serde_json::Value> {
    let content = read_text_mmap(&package_dir.join("package.json")).ok()?;
    serde_json::from_str(&content).ok()
}

fn package_entry_targets(package_json: &serde_json::Value) -> Vec<String> {
    let mut targets = Vec::new();

    if let Some(exports) = package_json.get("exports") {
        collect_root_export_targets(exports, &mut targets);
    }

    for field in ["module", "main", "browser"] {
        if let Some(target) = package_json.get(field).and_then(|value| value.as_str()) {
            push_package_target(&mut targets, target);
        }
    }

    targets
}

fn package_export_targets(package_json: &serde_json::Value, key: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let Some(exports) = package_json.get("exports") else {
        return targets;
    };

    if key == "." {
        collect_root_export_targets(exports, &mut targets);
        return targets;
    }

    if let Some(export_map) = exports.as_object() {
        if let Some(target) = export_map.get(key) {
            collect_package_targets(target, &mut targets);
        }
    }

    targets
}

fn collect_root_export_targets(exports: &serde_json::Value, targets: &mut Vec<String>) {
    if exports.is_string() || exports.is_array() {
        collect_package_targets(exports, targets);
        return;
    }

    let Some(object) = exports.as_object() else {
        return;
    };

    if let Some(root) = object.get(".") {
        collect_package_targets(root, targets);
    } else if object.keys().all(|key| !key.starts_with('.')) {
        collect_package_targets(exports, targets);
    }
}

fn collect_package_targets(value: &serde_json::Value, targets: &mut Vec<String>) {
    if let Some(target) = value.as_str() {
        push_package_target(targets, target);
        return;
    }

    if let Some(values) = value.as_array() {
        for value in values {
            collect_package_targets(value, targets);
        }
        return;
    }

    let Some(object) = value.as_object() else {
        return;
    };

    for condition in ["import", "module", "default", "require", "browser"] {
        if let Some(target) = object.get(condition) {
            collect_package_targets(target, targets);
        }
    }
}

fn push_package_target(targets: &mut Vec<String>, target: &str) {
    if target.is_empty() || target.starts_with('/') || target.contains("..") {
        return;
    }

    let target = target.trim_start_matches("./").to_string();
    if !targets.contains(&target) {
        targets.push(target);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolves_package_json_entry_fields() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let package = root.join("node_modules/pkg");
        fs::create_dir_all(package.join("dist")).unwrap();
        fs::write(
            package.join("package.json"),
            r#"{"module":"./dist/module.js","main":"./main.js"}"#,
        )
        .unwrap();
        fs::write(
            package.join("dist/module.js"),
            "export const value = 'module';",
        )
        .unwrap();
        fs::write(package.join("main.js"), "export const value = 'main';").unwrap();
        fs::write(root.join("app.js"), "import { value } from 'pkg';").unwrap();

        let loader = ModuleLoader::new(root.to_path_buf());
        let resolved = loader.resolve("pkg", Some(&root.join("app.js"))).unwrap();

        assert_eq!(resolved, package.join("dist/module.js"));
    }

    #[test]
    fn resolves_package_exports_and_subpaths() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let package = root.join("node_modules/pkg");
        fs::create_dir_all(package.join("lib")).unwrap();
        fs::write(
            package.join("package.json"),
            r#"{"exports":{".":{"import":"./lib/index.js"},"./feature":"./lib/feature.js"}}"#,
        )
        .unwrap();
        fs::write(package.join("lib/index.js"), "export const root = true;").unwrap();
        fs::write(
            package.join("lib/feature.js"),
            "export const feature = true;",
        )
        .unwrap();
        fs::write(root.join("app.js"), "import { root } from 'pkg';").unwrap();

        let loader = ModuleLoader::new(root.to_path_buf());

        assert_eq!(
            loader.resolve("pkg", Some(&root.join("app.js"))).unwrap(),
            package.join("lib/index.js")
        );
        assert_eq!(
            loader
                .resolve("pkg/feature", Some(&root.join("app.js")))
                .unwrap(),
            package.join("lib/feature.js")
        );
    }

    #[test]
    fn resolves_scoped_package_main() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let package = root.join("node_modules/@scope/pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("package.json"), r#"{"main":"./index.js"}"#).unwrap();
        fs::write(package.join("index.js"), "export const value = true;").unwrap();
        fs::write(root.join("app.js"), "import { value } from '@scope/pkg';").unwrap();

        let loader = ModuleLoader::new(root.to_path_buf());
        let resolved = loader
            .resolve("@scope/pkg", Some(&root.join("app.js")))
            .unwrap();

        assert_eq!(resolved, package.join("index.js"));
    }

    #[test]
    fn load_graph_transforms_typescript_sources() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("util.ts"), "export const value: number = 7;\n").unwrap();
        fs::write(
            root.join("main.ts"),
            "import { value } from './util.ts';\nconst result: number = value;\n",
        )
        .unwrap();

        let loader = ModuleLoader::new(root.to_path_buf());
        let modules = loader.load_graph(Path::new("main.ts")).unwrap();

        assert_eq!(modules.len(), 2);
        assert!(modules
            .iter()
            .any(|module| module.path.ends_with("util.ts")));
        assert!(modules
            .iter()
            .all(|module| !module.source.contains(": number")));
    }
}
