use crate::module::LoadedModule;
use crate::{JsRaftError, Result};
use rquickjs::qjs;
use rquickjs::Ctx;
use sha2::{Digest, Sha256};
use std::ffi::CString;
use std::fs;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Bytecode cache manifest stored alongside cached bytecode.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CacheManifest {
    /// Content hash of all module sources in the graph.
    pub hash: String,
    /// The entry file path.
    pub entry: String,
    /// Per-module metadata.
    pub modules: Vec<ModuleMeta>,
}

/// Metadata for a single cached module.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ModuleMeta {
    /// Canonical path of the module.
    pub path: String,
    /// SHA256 hash of this module's source code.
    pub source_hash: String,
}

/// Manages bytecode caching for JSRaft runtime.
pub struct BytecodeCache {
    /// Directory where cached bytecode files are stored.
    pub cache_dir: PathBuf,
}

impl BytecodeCache {
    /// Create a new cache rooted at the given directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Default cache directory under the project root.
    pub fn default_for(root: &Path) -> Self {
        Self::new(root.join(".jsraft").join("cache"))
    }

    /// Compute a deterministic content hash of all module sources.
    pub fn hash_modules(modules: &[LoadedModule]) -> String {
        let mut hasher = Sha256::new();
        for module in modules {
            // Include path to detect renamed/moved files
            hasher.update(module.path.to_string_lossy().as_bytes());
            hasher.update(b"\0");
            hasher.update(module.source.as_bytes());
            hasher.update(b"\n");
        }
        hex::encode(hasher.finalize())
    }

    /// Path to the cached bytecode file for a given hash.
    pub fn bytecode_path(&self, hash: &str) -> PathBuf {
        self.cache_dir.join(hash).join("bytecode.bin")
    }

    /// Path to the manifest file for a given hash.
    pub fn manifest_path(&self, hash: &str) -> PathBuf {
        self.cache_dir.join(hash).join("manifest.json")
    }

    /// Check if a valid cache entry exists for the given hash.
    pub fn is_cached(&self, hash: &str) -> bool {
        self.bytecode_path(hash).exists() && self.manifest_path(hash).exists()
    }

    /// Save compiled bytecode and manifest to disk.
    pub fn save(
        &self,
        hash: &str,
        bytecode: &[u8],
        entry: &Path,
        modules: &[LoadedModule],
    ) -> Result<()> {
        let dir = self.cache_dir.join(hash);
        fs::create_dir_all(&dir)?;

        fs::write(self.bytecode_path(hash), bytecode)?;
        debug!("Saved bytecode cache: {} bytes", bytecode.len());

        let manifest = CacheManifest {
            hash: hash.to_string(),
            entry: entry.display().to_string(),
            modules: modules
                .iter()
                .map(|m| {
                    let mut h = Sha256::new();
                    h.update(m.source.as_bytes());
                    ModuleMeta {
                        path: m.path.display().to_string(),
                        source_hash: hex::encode(h.finalize()),
                    }
                })
                .collect(),
        };

        fs::write(
            self.manifest_path(hash),
            serde_json::to_string_pretty(&manifest)?,
        )?;

        Ok(())
    }

    /// Load cached bytecode from disk.
    pub fn load_bytecode(&self, hash: &str) -> Result<Vec<u8>> {
        let bytes = fs::read(self.bytecode_path(hash))?;
        debug!("Loaded cached bytecode: {} bytes", bytes.len());
        Ok(bytes)
    }

    /// Clear the entire cache directory.
    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            info!("Bytecode cache cleared");
        }
        Ok(())
    }
}

/// Compile JavaScript source to QuickJS bytecode using raw FFI.
///
/// Uses `JS_EVAL_FLAG_COMPILE_ONLY` to compile without executing, then
/// `JS_WriteObject` to serialize the compiled bytecode to bytes.
pub fn compile_to_bytecode<'js>(ctx: &Ctx<'js>, source: &str, filename: &str) -> Result<Vec<u8>> {
    let raw_ctx = ctx.as_raw().as_ptr();
    let c_source = CString::new(source).map_err(|e| {
        JsRaftError::Runtime(format!("Failed to create C string for source: {e}"))
    })?;
    let c_filename = CString::new(filename).map_err(|e| {
        JsRaftError::Runtime(format!("Failed to create C string for filename: {e}"))
    })?;

    // Compile source to bytecode (JS_EVAL_FLAG_COMPILE_ONLY = 32)
    let compiled = unsafe {
        qjs::JS_Eval(
            raw_ctx,
            c_source.as_ptr(),
            source.len() as u64,
            c_filename.as_ptr(),
            qjs::JS_EVAL_FLAG_COMPILE_ONLY as c_int,
        )
    };

    // Check for compilation error
    if unsafe { qjs::JS_IsException(compiled) } {
        return Err(JsRaftError::Runtime(format!(
            "Failed to compile '{}' to bytecode",
            filename
        )));
    }

    // Serialize bytecode to bytes
    let bytecode = unsafe {
        let mut size: u64 = 0;
        let ptr = qjs::JS_WriteObject(
            raw_ctx,
            &mut size,
            compiled,
            qjs::JS_WRITE_OBJ_BYTECODE as c_int,
        );

        if ptr.is_null() {
            qjs::JS_FreeValue(raw_ctx, compiled);
            return Err(JsRaftError::Runtime(format!(
                "Failed to serialize bytecode for '{}'",
                filename
            )));
        }

        // Copy bytes into a Vec and free the QuickJS-allocated buffer
        let bytes = std::slice::from_raw_parts(ptr, size as usize).to_vec();
        qjs::js_free(raw_ctx, ptr as *mut std::os::raw::c_void);
        bytes
    };

    // Free the compiled function object
    unsafe { qjs::JS_FreeValue(raw_ctx, compiled) };

    Ok(bytecode)
}

/// Load and execute pre-compiled bytecode using raw FFI.
///
/// Uses `JS_ReadObject` to deserialize and `JS_EvalFunction` to execute.
pub fn eval_bytecode<'js>(ctx: &Ctx<'js>, bytecode: &[u8], filename: &str) -> Result<()> {
    let raw_ctx = ctx.as_raw().as_ptr();

    // Deserialize bytecode
    let fun_obj = unsafe {
        qjs::JS_ReadObject(
            raw_ctx,
            bytecode.as_ptr(),
            bytecode.len() as u64,
            (qjs::JS_READ_OBJ_BYTECODE | qjs::JS_READ_OBJ_ROM_DATA) as c_int,
        )
    };

    if unsafe { qjs::JS_IsException(fun_obj) } {
        return Err(JsRaftError::Runtime(format!(
            "Failed to load bytecode from '{}'",
            filename
        )));
    }

    // Execute the bytecode
    let result = unsafe { qjs::JS_EvalFunction(raw_ctx, fun_obj) };

    // Check for execution error
    if unsafe { qjs::JS_IsException(result) } {
        unsafe {
            qjs::JS_FreeValue(raw_ctx, result);
            qjs::JS_FreeValue(raw_ctx, fun_obj);
        }
        return Err(JsRaftError::Runtime(format!(
            "Failed to execute cached bytecode from '{}'",
            filename
        )));
    }

    unsafe {
        qjs::JS_FreeValue(raw_ctx, result);
        qjs::JS_FreeValue(raw_ctx, fun_obj);
    }

    Ok(())
}
