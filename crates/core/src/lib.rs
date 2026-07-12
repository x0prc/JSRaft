pub mod extensions;
pub mod io;
pub mod module;
pub mod ops;
pub mod runtime;
pub mod snapshot;
pub mod watcher;

pub use runtime::{JsRuntime, ReplSession, RuntimeConfig};
pub use module::ModuleLoader;
pub use io::read_text_mmap;

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
