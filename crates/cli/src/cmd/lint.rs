use anyhow::Result;
use jsraft_lint::{LintConfig, LinterEngine};
use std::path::Path;
use glob::glob;

pub async fn execute(paths: &[&Path], _fix: bool, _max_warnings: usize) -> Result<()> {
    let config = LintConfig::default();
    let linter = LinterEngine::new(config);

    let mut all_files = Vec::new();

    for path in paths.iter().copied() {
        if path.is_dir() {
            // Glob for JS/TS files
            let pattern = format!("{}/**/*.{{js,jsx,ts,tsx,mjs,cjs}}", path.display());
            for entry in glob(&pattern)? {
                let entry = entry?;
                all_files.push(entry);
            }
        } else {
            all_files.push(path.to_path_buf());
        }
    }

    if all_files.is_empty() {
        println!("No files to lint");
        return Ok(());
    }

    let file_refs: Vec<&Path> = all_files.iter().map(|p| p.as_path()).collect();
    let result = linter.lint_files(&file_refs)?;

    print!("{}", result.display());

    if result.has_errors() {
        std::process::exit(1);
    }

    Ok(())
}
