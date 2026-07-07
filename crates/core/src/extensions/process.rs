use rquickjs::{Ctx, Function};

/// Register process APIs: env, exit, argv, etc.
pub fn register(ctx: &Ctx<'_>) -> crate::Result<()> {
    let globals = ctx.globals();

    let process = rquickjs::Object::new(ctx.clone())
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create process object: {e}")))?;

    // process.env -> object
    let env_obj = rquickjs::Object::new(ctx.clone())
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create env object: {e}")))?;

    for (key, value) in std::env::vars() {
        env_obj
            .set(&key, value)
            .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set env var: {e}")))?;
    }

    process
        .set("env", env_obj)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set process.env: {e}")))?;

    // process.argv -> array
    let argv: Vec<String> = std::env::args().collect();
    process
        .set("argv", argv)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set process.argv: {e}")))?;

    // process.exit(code) - returns never (but we need a return type)
    let exit = Function::new(ctx.clone(), |code: Option<i32>| -> String {
        let code = code.unwrap_or(0);
        std::process::exit(code);
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create exit: {e}")))?;

    process
        .set("exit", exit)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set process.exit: {e}")))?;

    // process.cwd() -> string
    let cwd = Function::new(ctx.clone(), || -> String {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".into())
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create cwd: {e}")))?;

    process
        .set("cwd", cwd)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set process.cwd: {e}")))?;

    // process.chdir(path) - returns empty string on success
    let chdir = Function::new(ctx.clone(), |path: String| -> String {
        match std::env::set_current_dir(&path) {
            Ok(()) => String::new(),
            Err(e) => format!("Error changing directory to '{path}': {e}"),
        }
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create chdir: {e}")))?;

    process
        .set("chdir", chdir)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set process.chdir: {e}")))?;

    // process.pid
    process
        .set("pid", std::process::id())
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set process.pid: {e}")))?;

    globals
        .set("process", process)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set process global: {e}")))?;

    Ok(())
}
