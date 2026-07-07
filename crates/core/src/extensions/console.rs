use rquickjs::{Ctx, Function};

/// Register console.log, console.error, console.warn, etc.
pub fn register(ctx: &Ctx<'_>) -> crate::Result<()> {
    let globals = ctx.globals();

    let console = rquickjs::Object::new(ctx.clone())
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create console object: {e}")))?;

    // console.log - accept a single string for MVP
    let log = Function::new(ctx.clone(), |msg: String| {
        println!("{msg}");
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create log: {e}")))?;

    console.set("log", log).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set console.log: {e}"))
    })?;

    // console.error
    let error = Function::new(ctx.clone(), |msg: String| {
        eprintln!("{msg}");
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create error: {e}")))?;

    console.set("error", error).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set console.error: {e}"))
    })?;

    // console.warn
    let warn = Function::new(ctx.clone(), |msg: String| {
        eprintln!("WARN: {msg}");
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create warn: {e}")))?;

    console.set("warn", warn).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set console.warn: {e}"))
    })?;

    // console.info
    let info_fn = Function::new(ctx.clone(), |msg: String| {
        println!("INFO: {msg}");
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create info: {e}")))?;

    console.set("info", info_fn).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set console.info: {e}"))
    })?;

    // console.debug
    let debug_fn = Function::new(ctx.clone(), |msg: String| {
        println!("DEBUG: {msg}");
    })
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create debug: {e}")))?;

    console.set("debug", debug_fn).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set console.debug: {e}"))
    })?;

    globals.set("console", console).map_err(|e| {
        crate::JsRaftError::Extension(format!("Failed to set console global: {e}"))
    })?;

    Ok(())
}
