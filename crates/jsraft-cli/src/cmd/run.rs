use anyhow::Result;
use jsraft_core::{JsRuntime, RuntimeConfig};
use std::path::Path;

pub async fn execute(file: &Path, args: &[String]) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let config = RuntimeConfig::default();
    let runtime = JsRuntime::new(config);

    runtime.run_file(file).await?;

    Ok(())
}
