use anyhow::Result;
use jsraft_core::watcher::FileWatcher;
use jsraft_core::{JsRuntime, RuntimeConfig, RuntimePermissions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

pub struct PermissionFlags {
    pub secure: bool,
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_net: bool,
    pub allow_env: bool,
    pub allow_process: bool,
}

pub async fn execute(
    file: &Path,
    cache: bool,
    cache_dir: Option<PathBuf>,
    plugins_dirs: Vec<PathBuf>,
    watch: bool,
    permission_flags: PermissionFlags,
    _args: &[String],
) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {}", file.display());
    }

    let permissions = runtime_permissions(&permission_flags);
    let config = RuntimeConfig {
        cache_enabled: cache,
        cache_dir: cache_dir.clone(),
        plugin_dirs: plugins_dirs.clone(),
        permissions: permissions.clone(),
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
                            plugin_dirs: plugins_dirs.clone(),
                            permissions: permissions.clone(),
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

pub(crate) fn runtime_permissions(flags: &PermissionFlags) -> RuntimePermissions {
    if !flags.secure {
        return RuntimePermissions::default();
    }

    RuntimePermissions {
        read: flags.allow_read,
        write: flags.allow_write,
        net: flags.allow_net,
        env: flags.allow_env,
        process: flags.allow_process,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_are_open_by_default_without_secure_mode() {
        let permissions = runtime_permissions(&PermissionFlags {
            secure: false,
            allow_read: false,
            allow_write: false,
            allow_net: false,
            allow_env: false,
            allow_process: false,
        });

        assert!(permissions.read);
        assert!(permissions.write);
        assert!(permissions.net);
        assert!(permissions.env);
        assert!(permissions.process);
    }

    #[test]
    fn secure_mode_respects_allow_flags() {
        let permissions = runtime_permissions(&PermissionFlags {
            secure: true,
            allow_read: true,
            allow_write: false,
            allow_net: true,
            allow_env: false,
            allow_process: true,
        });

        assert!(permissions.read);
        assert!(!permissions.write);
        assert!(permissions.net);
        assert!(!permissions.env);
        assert!(permissions.process);
    }
}
