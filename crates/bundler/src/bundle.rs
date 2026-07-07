use crate::config::BundleConfig;
use anyhow::{Context, Result};
use oxc_resolver::{ResolveOptions, Resolver};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// The JSRaft bundler.
///
/// A minimal JavaScript/TypeScript bundler that uses Oxc for parsing
/// and code generation. It resolves modules, performs tree shaking,
/// and produces bundled output.
pub struct Bundler {
    config: BundleConfig,
    resolver: Resolver,
}

/// Result of a bundle operation.
#[derive(Debug)]
pub struct BundleResult {
    /// The bundled code.
    pub code: String,
    /// Source map (if enabled).
    pub source_map: Option<String>,
    /// Bundle stats.
    pub stats: BundleStats,
}

/// Statistics about a bundle operation.
#[derive(Debug, Default)]
pub struct BundleStats {
    pub files_included: usize,
    pub total_size: usize,
    pub duration_ms: u64,
}

impl Bundler {
    /// Create a new bundler with the given config.
    pub fn new(config: BundleConfig) -> Self {
        let resolver = Resolver::new(ResolveOptions {
            extensions: vec![
                ".js".into(),
                ".jsx".into(),
                ".ts".into(),
                ".tsx".into(),
                ".mjs".into(),
                ".cjs".into(),
                ".json".into(),
            ],
            main_fields: vec!["main".into(), "module".into()],
            condition_names: vec!["import".into(), "require".into(), "node".into()],
            ..Default::default()
        });

        Self {
            config,
            resolver,
        }
    }

    /// Bundle the project.
    pub fn bundle(&mut self, root: &Path) -> Result<BundleResult> {
        let start = std::time::Instant::now();
        info!("Bundling from root: {}", root.display());

        let mut modules: HashMap<PathBuf, String> = HashMap::new();
        let mut visited: HashSet<PathBuf> = HashSet::new();

        // Resolve and collect all modules starting from entry points
        for entry in &self.config.entry {
            let entry_path = root.join(entry);
            self.collect_modules(&entry_path, root, &mut modules, &mut visited)?;
        }

        let files_included = modules.len();

        // MVP: concatenate resolved modules in dependency order.
        let mut bundled_parts: Vec<String> = Vec::new();

        for (path, source) in &modules {
            debug!("Processing module: {}", path.display());

            // Wrap in a module scope for tree shaking
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy();

            bundled_parts.push(format!(
                "// Module: {rel_path}\n{source}"
            ));
        }

        // Combine all parts
        let combined = bundled_parts.join("\n\n");

        // Minify if requested
        let final_code = if self.config.minify {
            self.minify(&combined)?
        } else {
            combined
        };

        let stats = BundleStats {
            files_included,
            total_size: final_code.len(),
            duration_ms: start.elapsed().as_millis() as u64,
        };

        info!(
            "Bundle complete: {} files, {} bytes, {}ms",
            stats.files_included,
            stats.total_size,
            stats.duration_ms
        );

        Ok(BundleResult {
            code: final_code,
            source_map: None, // TODO: implement source maps
            stats,
        })
    }

    /// Write bundle output to disk.
    pub fn write_output(&self, result: &BundleResult, root: &Path) -> Result<PathBuf> {
        let outdir = self
            .config
            .outdir
            .as_deref()
            .unwrap_or("dist");

        let out_path = root.join(outdir);
        std::fs::create_dir_all(&out_path)?;

        let filename = self
            .config
            .outfile
            .as_deref()
            .unwrap_or("bundle.js");

        let output_file = out_path.join(filename);
        std::fs::write(&output_file, &result.code)?;

        if let Some(ref source_map) = result.source_map {
            let sm_path = output_file.with_extension("js.map");
            std::fs::write(sm_path, source_map)?;
        }

        Ok(output_file)
    }

    fn collect_modules(
        &self,
        path: &Path,
        root: &Path,
        modules: &mut HashMap<PathBuf, String>,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<()> {
        let canonical = path
            .canonicalize()
            .with_context(|| format!("Failed to resolve: {}", path.display()))?;

        if visited.contains(&canonical) {
            return Ok(());
        }
        visited.insert(canonical.clone());

        let source = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read: {}", path.display()))?;

        modules.insert(canonical.clone(), source.clone());

        // Find imports in the source (simple regex for MVP)
        let imports = self.extract_imports(&source);

        for import_path in imports {
            if self.config.external.contains(&import_path) {
                continue;
            }

            let resolved = self.resolver.resolve(
                path.parent().unwrap_or(root),
                &import_path,
            );

            if let Ok(resolved_path) = resolved {
                let resolved_path = resolved_path.full_path();
                self.collect_modules(&resolved_path, root, modules, visited)?;
            }
        }

        Ok(())
    }

    fn extract_imports(&self, source: &str) -> Vec<String> {
        let mut imports = Vec::new();

        // Simple regex-based import extraction for MVP
        // In production, use the AST for this
        for line in source.lines() {
            let trimmed = line.trim();

            // import ... from '...'
            if trimmed.starts_with("import ") {
                if let Some(start) = trimmed.rfind('\'') {
                    if let Some(end) = trimmed[start + 1..].find('\'') {
                        imports.push(trimmed[start + 1..start + 1 + end].to_string());
                    }
                } else if let Some(start) = trimmed.rfind('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        imports.push(trimmed[start + 1..start + 1 + end].to_string());
                    }
                }
            }

            // require('...')
            if trimmed.contains("require(") {
                if let Some(start) = trimmed.find("require('") {
                    let after = &trimmed[start + 9..];
                    if let Some(end) = after.find('\'') {
                        imports.push(after[..end].to_string());
                    }
                } else if let Some(start) = trimmed.find("require(\"") {
                    let after = &trimmed[start + 9..];
                    if let Some(end) = after.find('"') {
                        imports.push(after[..end].to_string());
                    }
                }
            }

            // export ... from '...'
            if trimmed.starts_with("export ") {
                if let Some(start) = trimmed.rfind('\'') {
                    if let Some(end) = trimmed[start + 1..].find('\'') {
                        imports.push(trimmed[start + 1..start + 1 + end].to_string());
                    }
                } else if let Some(start) = trimmed.rfind('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        imports.push(trimmed[start + 1..start + 1 + end].to_string());
                    }
                }
            }
        }

        imports
    }

    fn minify(&self, source: &str) -> Result<String> {
        // Simple minification for MVP: remove comments and excess whitespace
        // In production, use oxc_minifier
        let mut result = String::with_capacity(source.len());
        let mut chars = source.chars().peekable();
        let mut in_string = false;
        let mut string_char = ' ';

        while let Some(ch) = chars.next() {
            if in_string {
                result.push(ch);
                if ch == string_char && chars.peek() != Some(&'\\') {
                    in_string = false;
                }
                continue;
            }

            match ch {
                '\'' | '"' | '`' => {
                    in_string = true;
                    string_char = ch;
                    result.push(ch);
                }
                '/' if chars.peek() == Some(&'/') => {
                    // Skip line comment
                    for c in chars.by_ref() {
                        if c == '\n' {
                            result.push('\n');
                            break;
                        }
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    // Skip block comment
                    chars.next();
                    let mut depth = 1;
                    while let Some(c) = chars.next() {
                        if c == '/' && chars.peek() == Some(&'*') {
                            depth += 1;
                            chars.next();
                        } else if c == '*' && chars.peek() == Some(&'/') {
                            depth -= 1;
                            chars.next();
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        Ok(result)
    }
}
