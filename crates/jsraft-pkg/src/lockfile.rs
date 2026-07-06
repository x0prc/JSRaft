use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Lock file entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileEntry {
    /// Version requirement.
    pub version: String,
    /// Resolved version.
    pub resolved: String,
    /// Integrity hash.
    pub integrity: String,
}

/// Lock file for deterministic installs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// Package entries.
    pub entries: HashMap<String, LockfileEntry>,
}

impl Lockfile {
    /// Load a lockfile from jsraft.lock.
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join("jsraft.lock");
        if !path.exists() {
            return Ok(Self {
                entries: HashMap::new(),
            });
        }

        let content = std::fs::read_to_string(&path)
            .context("Failed to read jsraft.lock")?;

        let lockfile: Self =
            serde_json::from_str(&content).context("Failed to parse jsraft.lock")?;

        Ok(lockfile)
    }

    /// Save the lockfile to jsraft.lock.
    pub fn save(&self, root: &Path) -> Result<()> {
        let path = root.join("jsraft.lock");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Check if a package is locked.
    pub fn is_locked(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Get a locked version.
    pub fn get_version(&self, name: &str) -> Option<&str> {
        self.entries.get(name).map(|e| e.resolved.as_str())
    }
}
