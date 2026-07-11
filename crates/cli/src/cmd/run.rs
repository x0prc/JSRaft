use anyhow::Result;
use jsraft_core::{JsRuntime, RuntimeConfig};
use std::path::{Path, PathBuf};

pub async fn execute(
    file: &Path,
    cache: bool,
    cache_dir: Option<PathBuf>,
    _args: &[String],
) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let config = RuntimeConfig {
        cache_enabled: cache,
        cache_dir,
        ..Default::default()
    };

    let runtime = JsRuntime::new(config);

    runtime.run_file(file).await?;

    Ok(())
}
