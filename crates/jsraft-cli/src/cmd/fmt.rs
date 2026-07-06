use anyhow::Result;
use jsraft_fmt::{FmtConfig, Formatter};
use std::path::Path;
use glob::glob;

pub async fn execute(paths: &[Path], check: bool, stdout: bool) -> Result<()> {
    let config = FmtConfig::default();
    let formatter = Formatter::new(config);

    let mut all_files = Vec::new();

    for path in paths {
        if path.is_dir() {
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
        println!("No files to format");
        return Ok(());
    }

    let mut changed_count = 0;

    for file in &all_files {
        let result = formatter.format_file(file, check)?;

        if result.changed {
            changed_count += 1;

            if stdout {
                println!("--- {}", file.display());
                println!("{}", result.formatted);
            } else if check {
                println!("Would reformat: {}", file.display());
            } else {
                println!("Formatted: {}", file.display());
            }
        }
    }

    if check && changed_count > 0 {
        println!("\n{changed_count} files would be reformatted");
        std::process::exit(1);
    }

    if !check {
        println!("\nFormatted {changed_count} files");
    }

    Ok(())
}
