use crate::module::ModuleLoader;
use crate::{JsRaftError, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tracing::debug;

/// Debounce interval for file change events.
const DEBOUNCE_MS: u64 = 100;

/// Watches a set of files for changes and notifies when they occur.
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<Event>>,
    watched: HashSet<PathBuf>,
}

impl FileWatcher {
    /// Create a new file watcher.
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let watcher = RecommendedWatcher::new(
            move |event| {
                let _ = tx.send(event);
            },
            notify::Config::default(),
        )
        .map_err(|e| JsRaftError::Runtime(format!("Failed to create file watcher: {e}")))?;

        Ok(Self {
            watcher,
            rx,
            watched: HashSet::new(),
        })
    }

    /// Watch the entry file and all its dependencies.
    pub fn watch_graph(
        &mut self,
        entry: &Path,
        module_loader: &ModuleLoader,
    ) -> Result<Vec<PathBuf>> {
        let modules = module_loader.load_graph(entry)?;
        let mut paths = Vec::new();

        for module in &modules {
            if self.watched.insert(module.path.clone()) {
                self.watcher
                    .watch(&module.path, RecursiveMode::NonRecursive)
                    .map_err(|e| {
                        JsRaftError::Runtime(format!(
                            "Failed to watch {}: {e}",
                            module.path.display()
                        ))
                    })?;
                debug!("Watching: {}", module.path.display());
            }
            paths.push(module.path.clone());
        }

        // Also watch the entry's parent directory for new files
        if let Some(parent) = entry.parent() {
            if parent.is_dir() && self.watched.insert(parent.to_path_buf()) {
                let _ = self.watcher.watch(parent, RecursiveMode::NonRecursive);
            }
        }

        Ok(paths)
    }

    /// Wait for a file change event with debouncing.
    ///
    /// Returns the set of changed file paths, or None on timeout/error.
    pub fn wait_for_change(&self) -> Option<HashSet<PathBuf>> {
        let mut changed = HashSet::new();
        let deadline = Duration::from_millis(DEBOUNCE_MS);

        // Drain events within the debounce window
        let start = std::time::Instant::now();
        loop {
            let remaining = deadline.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }

            match self.rx.recv_timeout(remaining) {
                Ok(Ok(event)) => {
                    if matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
                    ) {
                        for path in event.paths {
                            // Only track files we're watching
                            if self.watched.contains(&path) || path.extension().is_some_and(|e| {
                                matches!(e.to_str(), Some("js" | "ts" | "jsx" | "tsx" | "mjs" | "cjs"))
                            }) {
                                changed.insert(path);
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    debug!("Watch error: {e}");
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    break;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return None;
                }
            }
        }

        if changed.is_empty() {
            None
        } else {
            Some(changed)
        }
    }

    /// Get the set of currently watched paths.
    pub fn watched_paths(&self) -> &HashSet<PathBuf> {
        &self.watched
    }

    /// Clear all watched paths and re-initialize the watcher.
    pub fn reset(&mut self) -> Result<()> {
        for path in &self.watched {
            let _ = self.watcher.unwatch(path);
        }
        self.watched.clear();
        Ok(())
    }
}
