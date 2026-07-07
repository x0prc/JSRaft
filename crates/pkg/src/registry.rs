use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

const NPM_REGISTRY: &str = "https://registry.npmjs.org";

/// npm package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmPackage {
    pub name: String,
    pub description: Option<String>,
    pub versions: HashMap<String, NpmVersion>,
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
}

/// npm package version metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmVersion {
    pub version: String,
    pub dist: NpmDist,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub peer_dependencies: HashMap<String, String>,
}

/// npm package distribution info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmDist {
    pub tarball: String,
    #[serde(default)]
    pub shasum: String,
    #[serde(default)]
    pub integrity: String,
}

/// npm registry client.
pub struct NpmRegistry {
    client: reqwest::Client,
}

impl NpmRegistry {
    /// Create a new registry client.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("jsraft-pkg/0.1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Fetch package metadata from npm.
    pub async fn get_package(&self, name: &str) -> Result<NpmPackage> {
        let url = format!("{NPM_REGISTRY}/{name}");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch package metadata")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Package '{}' not found (status: {})",
                name,
                response.status()
            );
        }

        let package: NpmPackage = response
            .json()
            .await
            .context("Failed to parse package metadata")?;

        Ok(package)
    }

    /// Download a tarball to a file.
    pub async fn download_tarball(&self, url: &str, dest: &Path) -> Result<()> {
        info!("Downloading tarball: {url}");

        let response = self
            .client
            .get(url)
            .send()
            .await
            .context("Failed to download tarball")?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download tarball: status {}", response.status());
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read tarball bytes")?;

        std::fs::write(dest, &bytes)?;

        Ok(())
    }

    /// Search for packages.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let url = format!("{NPM_REGISTRY}/-/v1/search?text={query}&size=10");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to search packages")?;

        let data: serde_json::Value = response.json().await?;

        let mut results = Vec::new();
        if let Some(objects) = data.get("objects").and_then(|o| o.as_array()) {
            for obj in objects {
                if let Some(package) = obj.get("package") {
                    let name = package
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let version = package
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let description = package
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("")
                        .to_string();

                    results.push(SearchResult {
                        name,
                        version,
                        description,
                    });
                }
            }
        }

        Ok(results)
    }
}

/// Search result from npm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
}
