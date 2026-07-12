use anyhow::Result;
use jsraft_core::watcher::FileWatcher;
use jsraft_core::{JsRuntime, RuntimeConfig};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

pub async fn execute(
    file: &Path,
    cache: bool,
    cache_dir: Option<PathBuf>,
    watch: bool,
    _args: &[String],
) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let config = RuntimeConfig {
        cache_enabled: cache,
        cache_dir: cache_dir.clone(),
        ..Default::default()
    };

    // Run the file once
    let runtime = JsRuntime::new(config);
    if let Err(e) = runtime.run_file(file).await {
        error!("Error: {e}");
        if !watch {
            return Err(e.into());
        }
    }

    if !watch {
        return Ok(());
    }

    // Watch mode: monitor file changes and re-run
    info!("Watching for changes... (Ctrl+C to stop)");

    let mut watcher = FileWatcher::new()?;
    watcher.watch_graph(file, runtime.module_loader())?;

    // Wrap watcher in Arc<Mutex> for thread-safe access
    let watcher = Arc::new(Mutex::new(watcher));

    // Spawn Ctrl+C handler
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("\nStopping watch mode...");
        let _ = cancel_tx.send(());
    });

    // Watch loop
    let mut cancel_rx = cancel_rx;
    loop {
        let watcher_clone = Arc::clone(&watcher);
        tokio::select! {
            _ = &mut cancel_rx => {
                break;
            }
            change = tokio::task::spawn_blocking(move || {
                loop {
                    let changed = {
                        let w = watcher_clone.lock().unwrap();
                        w.wait_for_change()
                    };
                    if changed.is_some() {
                        return changed;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }) => {
                match change {
                    Ok(Some(changed)) => {
                        let files: Vec<_> = changed.iter().map(|p| p.display().to_string()).collect();
                        info!("Change detected in: {}", files.join(", "));

                        // Re-run the entry file
                        let runtime = JsRuntime::new(RuntimeConfig {
                            cache_enabled: cache,
                            cache_dir: cache_dir.clone(),
                            ..Default::default()
                        });
                        if let Err(e) = runtime.run_file(file).await {
                            error!("Error on re-run: {e}");
                        } else {
                            info!("Re-run complete. Watching for changes...");
                        }

                        // Re-watch the graph (dependencies may have changed)
                        {
                            let mut w = watcher.lock().unwrap();
                            if let Err(e) = w.reset() {
                                error!("Failed to reset watcher: {e}");
                            }
                            if let Err(e) = w.watch_graph(file, runtime.module_loader()) {
                                error!("Failed to re-watch: {e}");
                            }
                        }
                    }
                    Ok(None) => {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        error!("Watch task failed: {e}");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
