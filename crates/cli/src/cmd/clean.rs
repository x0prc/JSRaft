use anyhow::Result;
use std::path::{Path, PathBuf};

/// Remove generated JSRaft artifacts.
pub async fn execute(all: bool) -> Result<()> {
    let root = std::env::current_dir()?;
    let mut targets = vec![root.join(".jsraft").join("cache")];

    if all {
        targets.push(root.join("dist"));
        targets.push(root.join("build"));
    }

    let mut removed = 0;
    for target in targets {
        if remove_path(&target)? {
            println!("Removed {}", target.display());
            removed += 1;
        }
    }

    if removed == 0 {
        println!("Nothing to clean");
    }

    Ok(())
}

fn remove_path(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }

    cleanup_empty_parent(path.parent().map(Path::to_path_buf))?;
    Ok(true)
}

fn cleanup_empty_parent(parent: Option<PathBuf>) -> Result<()> {
    let Some(parent) = parent else {
        return Ok(());
    };
    if parent.file_name().and_then(|name| name.to_str()) != Some(".jsraft") {
        return Ok(());
    }
    if parent.read_dir()?.next().is_none() {
        std::fs::remove_dir(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_path_removes_cache_and_empty_jsraft_dir() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join(".jsraft/cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("bytecode.bin"), b"cache").unwrap();

        assert!(remove_path(&cache).unwrap());
        assert!(!cache.exists());
        assert!(!temp.path().join(".jsraft").exists());
    }

    #[test]
    fn remove_path_returns_false_for_missing_path() {
        let temp = tempfile::tempdir().unwrap();
        assert!(!remove_path(&temp.path().join("missing")).unwrap());
    }
}
