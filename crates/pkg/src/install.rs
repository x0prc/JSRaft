use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::registry::{NpmPackage, NpmRegistry};
use crate::lockfile::{Lockfile, LockfileEntry};

/// Package manager configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkgConfig {
    /// Dependencies (name -> version requirement).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,

    /// Dev dependencies.
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,

    /// Package name.
    pub name: Option<String>,

    /// Package version.
    pub version: Option<String>,
}

impl Default for PkgConfig {
    fn default() -> Self {
        Self {
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            name: None,
            version: None,
        }
    }
}

impl PkgConfig {
    /// Load from a jsraft.toml file.
    pub fn from_toml(content: &str) -> Result<Self> {
        let config: PkgConfig = toml::from_str(content)?;
        Ok(config)
    }

    /// Save to a jsraft.toml file.
    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}

/// The package manager.
pub struct PackageManager {
    config: PkgConfig,
    registry: NpmRegistry,
    cache_dir: PathBuf,
    root: PathBuf,
}

impl PackageManager {
    /// Create a new package manager.
    pub fn new(root: PathBuf) -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("jsraft")
            .join("cache");

        std::fs::create_dir_all(&cache_dir)?;

        let config = Self::load_config(&root)?;
        let registry = NpmRegistry::new();

        Ok(Self {
            config,
            registry,
            cache_dir,
            root,
        })
    }

    /// Load package config from the project root.
    fn load_config(root: &Path) -> Result<PkgConfig> {
        let config_path = root.join("jsraft.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            PkgConfig::from_toml(&content)
        } else {
            Ok(PkgConfig::default())
        }
    }

    /// Install all dependencies.
    pub async fn install(&self) -> Result<InstallResult> {
        info!("Installing dependencies...");

        let mut installed = Vec::new();
        let mut errors = Vec::new();

        let all_deps: Vec<_> = self
            .config
            .dependencies
            .iter()
            .map(|(name, version)| (name.as_str(), version.as_str(), false))
            .chain(
                self.config
                    .dev_dependencies
                    .iter()
                    .map(|(name, version)| (name.as_str(), version.as_str(), true)),
            )
            .collect();

        for (name, version, is_dev) in all_deps {
            match self.install_package(name, version).await {
                Ok(path) => {
                    info!("Installed {name}@{version}");
                    installed.push(InstalledPackage {
                        name: name.to_string(),
                        version: version.to_string(),
                        path,
                        is_dev,
                    });
                }
                Err(e) => {
                    errors.push(format!("Failed to install {name}@{version}: {e}"));
                }
            }
        }

        // Update lockfile
        let lockfile = self.create_lockfile()?;
        lockfile.save(&self.root)?;

        // Create node_modules symlinks
        self.create_node_modules(&installed)?;

        Ok(InstallResult { installed, errors })
    }

    /// Install a specific package.
    async fn install_package(&self, name: &str, version: &str) -> Result<PathBuf> {
        // Check cache first
        let cache_key = format!("{name}@{version}");
        let cached_path = self.cache_dir.join(&cache_key);

        if cached_path.exists() {
            return Ok(cached_path);
        }

        // Fetch package metadata from npm
        let package = self.registry.get_package(name).await?;

        // Resolve version
        let resolved_version = self.resolve_version(&package, version)?;

        // Download tarball
        let version_meta = package
            .versions
            .get(&resolved_version)
            .ok_or_else(|| anyhow::anyhow!("Missing metadata for {name}@{resolved_version}"))?;
        let tarball_url = version_meta.dist.tarball.as_str();

        let tarball_path = self.cache_dir.join(format!("{name}@{resolved_version}.tgz"));

        self.registry
            .download_tarball(tarball_url, &tarball_path)
            .await?;

        // Extract tarball
        self.extract_tarball(&tarball_path, &cached_path)?;

        Ok(cached_path)
    }

    fn resolve_version(&self, package: &NpmPackage, requirement: &str) -> Result<String> {
        // Simple version resolution for MVP
        // In production, use semver crate
        let versions: Vec<&str> = package.versions.keys().map(|s| s.as_str()).collect();

        if requirement == "latest" || requirement == "*" {
            return versions
                .last()
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("No versions available"));
        }

        // Try exact match
        if package.versions.contains_key(requirement) {
            return Ok(requirement.to_string());
        }

        // Try prefix match (e.g., "^1.0.0" matches "1.x.x")
        for version in versions.iter().rev() {
            if version.starts_with(requirement.trim_start_matches(|c: char| c == '^' || c == '~')) {
                return Ok(version.to_string());
            }
        }

        // Fallback to latest
        versions
            .last()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No matching version for {requirement}"))
    }

    fn extract_tarball(&self, tarball: &Path, dest: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::fs::File;
        use tar::Archive;

        let file = File::open(tarball)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        std::fs::create_dir_all(dest)?;
        archive.unpack(dest)?;

        // npm tarballs extract to a "package" directory
        let package_dir = dest.join("package");
        if package_dir.exists() {
            // Move contents up
            for entry in std::fs::read_dir(&package_dir)? {
                let entry = entry?;
                let target = dest.join(entry.file_name());
                std::fs::rename(entry.path(), target)?;
            }
            std::fs::remove_dir_all(&package_dir)?;
        }

        Ok(())
    }

    fn create_lockfile(&self) -> Result<Lockfile> {
        let mut entries = HashMap::new();

        for (name, version) in &self.config.dependencies {
            entries.insert(
                name.clone(),
                LockfileEntry {
                    version: version.clone(),
                    resolved: version.clone(),
                    integrity: String::new(),
                },
            );
        }

        Ok(Lockfile { entries })
    }

    fn create_node_modules(&self, installed: &[InstalledPackage]) -> Result<()> {
        let node_modules = self.root.join("node_modules");
        std::fs::create_dir_all(&node_modules)?;

        for pkg in installed {
            let target = node_modules.join(&pkg.name);
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }

            // Create symlink to cache
            #[cfg(unix)]
            std::os::unix::fs::symlink(&pkg.path, &target)?;

            #[cfg(windows)]
            std::os::windows::fs::symlink_dir(&pkg.path, &target)?;
        }

        Ok(())
    }
}

/// Result of an install operation.
#[derive(Debug)]
pub struct InstallResult {
    pub installed: Vec<InstalledPackage>,
    pub errors: Vec<String>,
}

/// An installed package.
#[derive(Debug)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub is_dev: bool,
}
