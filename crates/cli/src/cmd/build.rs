use anyhow::Result;
use jsraft_bundler::{BundleConfig, Bundler};

pub async fn execute(
    entry: Option<&str>,
    outdir: Option<&str>,
    format: &str,
    minify: bool,
    sourcemap: bool,
) -> Result<()> {
    let root = std::env::current_dir()?;

    let mut config = BundleConfig::default();

    if let Some(entry) = entry {
        config.entry = vec![entry.into()];
    }

    if let Some(outdir) = outdir {
        config.outdir = Some(outdir.into());
    }

    config.format = format.into();
    config.minify = minify;
    config.sourcemap = sourcemap;

    let mut bundler = Bundler::new(config);

    println!("Bundling...");

    let result = bundler.bundle(&root)?;

    let output_path = bundler.write_output(&result, &root)?;

    println!(
        "Bundle complete: {} files, {} bytes, {}ms",
        result.stats.files_included,
        result.stats.total_size,
        result.stats.duration_ms
    );

    println!("Output: {}", output_path.display());

    Ok(())
}
