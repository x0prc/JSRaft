use rquickjs::{Ctx, Function, Object};
use std::path::PathBuf;

/// Register path utilities: path.join, path.resolve, etc.
pub fn register(ctx: &Ctx<'_>) -> crate::Result<()> {
    let globals = ctx.globals();

    let path_obj = Object::new(ctx.clone())
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create path object: {e}")))?;

    // path.join(segments...) -> string
    // Use a JSON array approach for variadic args
    let join = Function::new(
        ctx.clone(),
        |args_str: String| -> String {
            // Parse JSON array of segments
            let segments: Vec<String> = serde_json::from_str(&args_str)
                .unwrap_or_default();
            let mut result = PathBuf::new();
            for seg in &segments {
                result.push(seg);
            }
            result.to_string_lossy().to_string()
        },
    )
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create join: {e}")))?;

    path_obj
        .set("join", join)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set path.join: {e}")))?;

    // path.resolve(segments...) -> string
    let resolve = Function::new(
        ctx.clone(),
        |args_str: String| -> String {
            let segments: Vec<String> = serde_json::from_str(&args_str)
                .unwrap_or_default();
            let mut result = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."));

            for seg in &segments {
                let p = PathBuf::from(seg);
                if p.is_absolute() {
                    result = p;
                } else {
                    result.push(seg);
                }
            }

            result.canonicalize()
                .unwrap_or(result)
                .to_string_lossy()
                .to_string()
        },
    )
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create resolve: {e}")))?;

    path_obj
        .set("resolve", resolve)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set path.resolve: {e}")))?;

    // path.basename(path) -> string
    let basename = Function::new(ctx.clone(), |path: String| -> String {
        std::path::Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create basename: {e}")))?;

    path_obj
        .set("basename", basename)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set path.basename: {e}")))?;

    // path.dirname(path) -> string
    let dirname = Function::new(ctx.clone(), |path: String| -> String {
        std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".into())
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create dirname: {e}")))?;

    path_obj
        .set("dirname", dirname)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set path.dirname: {e}")))?;

    // path.extname(path) -> string
    let extname = Function::new(ctx.clone(), |path: String| -> String {
        std::path::Path::new(&path)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default()
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create extname: {e}")))?;

    path_obj
        .set("extname", extname)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set path.extname: {e}")))?;

    // path.relative(from, to) -> string
    let relative = Function::new(
        ctx.clone(),
        |from: String, to: String| -> String {
            let from_path = PathBuf::from(&from);
            let to_path = PathBuf::from(&to);

            let from_parts: Vec<String> = from_path
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect();
            let to_parts: Vec<String> = to_path
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect();

            let mut common_len = 0;
            for i in 0..from_parts.len().min(to_parts.len()) {
                if from_parts[i] == to_parts[i] {
                    common_len = i + 1;
                } else {
                    break;
                }
            }

            let mut result = PathBuf::new();
            for _ in common_len..from_parts.len() {
                result.push("..");
            }
            for part in &to_parts[common_len..] {
                result.push(part);
            }

            result.to_string_lossy().to_string()
        },
    )
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create relative: {e}")))?;

    path_obj
        .set("relative", relative)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set path.relative: {e}")))?;

    globals
        .set("path", path_obj)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set path global: {e}")))?;

    Ok(())
}
