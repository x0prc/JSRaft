use anyhow::Result;
use jsraft_pkg::PackageManager;
use std::path::PathBuf;

pub async fn execute(package: Option<&str>, version: Option<&str>, dev: bool) -> Result<()> {
    let root = std::env::current_dir()?;
    let manager = PackageManager::new(root.clone())?;

    if let Some(pkg) = package {
        // Install specific package
        println!("Installing {pkg}...");

        // Update config
        let config_path = root.join("jsraft.toml");
        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            jsraft_pkg::PkgConfig::from_toml(&content)?
        } else {
            jsraft_pkg::PkgConfig::default()
        };

        let version_req = version.unwrap_or("latest");

        if dev {
            config
                .dev_dependencies
                .insert(pkg.to_string(), version_req.to_string());
        } else {
            config
                .dependencies
                .insert(pkg.to_string(), version_req.to_string());
        }

        let toml_content = config.to_toml()?;
        std::fs::write(&config_path, toml_content)?;

        // Install
        let result = manager.install().await?;

        if result.errors.is_empty() {
            println!("Installed {pkg}@{version_req}");
        } else {
            for error in &result.errors {
                eprintln!("Error: {error}");
            }
        }
    } else {
        // Install all dependencies
        let result = manager.install().await?;

        println!("Installed {} packages", result.installed.len());

        if !result.errors.is_empty() {
            for error in &result.errors {
                eprintln!("Error: {error}");
            }
        }
    }

    Ok(())
}
