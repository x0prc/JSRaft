//! Core runtime APIs for JSRaft.
//!
//! `jsraft-core` owns the QuickJS runtime integration, module resolution,
//! TypeScript transformation, runtime extensions, plugin loading, bytecode
//! snapshots, watch support, and the permission model used by privileged APIs.
//!
//! The primary entry point is [`JsRuntime`]:
//!
//! ```no_run
//! # async fn example() -> jsraft_core::Result<()> {
//! use jsraft_core::{JsRuntime, RuntimeConfig};
//!
//! let runtime = JsRuntime::new(RuntimeConfig::default());
//! runtime.run_file(std::path::Path::new("src/index.js")).await?;
//! # Ok(())
//! # }
//! ```
//!
//! For more architecture detail, see `docs/rust-architecture.md` in the
//! workspace root.

pub mod extensions;
pub mod io;
pub mod module;
pub mod ops;
pub mod plugin;
pub mod runtime;
pub mod snapshot;
pub mod transform;
pub mod watcher;

pub use io::read_text_mmap;
pub use module::ModuleLoader;
pub use runtime::{JsRuntime, ReplSession, RuntimeConfig, RuntimePermissions};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum JsRaftError {
    #[error("JavaScript error: {0}")]
    JsError(String),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("TypeScript error: {0}")]
    TypeScriptError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Extension error: {0}")]
    Extension(String),

    #[error("Package error: {0}")]
    Package(String),
}

pub type Result<T> = std::result::Result<T, JsRaftError>;
