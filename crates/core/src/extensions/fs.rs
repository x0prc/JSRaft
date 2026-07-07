use rquickjs::{Ctx, Function};
use std::fs;
use std::path::PathBuf;

/// Register filesystem APIs: Deno.readFile, Deno.writeFile, etc.
pub fn register(ctx: &Ctx<'_>) -> crate::Result<()> {
    let globals = ctx.globals();

    // Create a Deno-like namespace
    let deno = rquickjs::Object::new(ctx.clone())
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create Deno object: {e}")))?;

    // Deno.readTextFile(path) -> string
    let read_text_file =
        Function::new(ctx.clone(), |path: String| -> String {
            fs::read_to_string(&path).unwrap_or_else(|e| format!("Error reading '{path}': {e}"))
        })
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create readTextFile: {e}")))?;

    deno.set("readTextFile", read_text_file).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set Deno.readTextFile: {e}"))
    })?;

    // Deno.writeTextFile(path, data) - returns empty string on success
    let write_text_file =
        Function::new(ctx.clone(), |path: String, data: String| -> String {
            match fs::write(&path, &data) {
                Ok(()) => String::new(),
                Err(e) => format!("Error writing '{path}': {e}"),
            }
        })
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create writeTextFile: {e}")))?;

    deno.set("writeTextFile", write_text_file).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set Deno.writeTextFile: {e}"))
    })?;

    // Deno.mkdir(path) - returns empty string on success
    let mkdir = Function::new(ctx.clone(), |path: String| -> String {
        match fs::create_dir_all(&path) {
            Ok(()) => String::new(),
            Err(e) => format!("Error creating directory '{path}': {e}"),
        }
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create mkdir: {e}")))?;

    deno.set("mkdir", mkdir).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set Deno.mkdir: {e}"))
    })?;

    // Deno.remove(path) - returns empty string on success
    let remove = Function::new(ctx.clone(), |path: String| -> String {
        let p = PathBuf::from(&path);
        let result = if p.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match result {
            Ok(()) => String::new(),
            Err(e) => format!("Error removing '{path}': {e}"),
        }
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create remove: {e}")))?;

    deno.set("remove", remove).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set Deno.remove: {e}"))
    })?;

    // Deno.stat(path) -> JSON string with file info
    let stat = Function::new(ctx.clone(), |path: String| -> String {
        match fs::metadata(&path) {
            Ok(metadata) => {
                let mtime = metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                format!(
                    "{{\"isFile\":{},\"isDirectory\":{},\"size\":{},\"mtime\":{}}}",
                    metadata.is_file(),
                    metadata.is_dir(),
                    metadata.len(),
                    mtime
                )
            }
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create stat: {e}")))?;

    deno.set("stat", stat).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set Deno.stat: {e}"))
    })?;

    globals.set("Deno", deno).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set Deno global: {e}"))
    })?;

    Ok(())
}
