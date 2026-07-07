use rquickjs::{Ctx, Function};

/// Register timer APIs: setTimeout, setInterval, clearTimeout, clearInterval.
pub fn register(ctx: &Ctx<'_>) -> crate::Result<()> {
    let globals = ctx.globals();

    // For MVP, these are synchronous stubs.
    // A proper implementation would use Tokio's async timers
    // and integrate with the event loop.

    // setTimeout(callback, ms) -> id (stub: runs immediately)
    let set_timeout =
        Function::new(ctx.clone(), |callback: rquickjs::Function<'_>, _ms: Option<i32>| -> i32 {
            // Execute immediately in MVP
            if let Err(_e) = callback.call::<(), ()>(()) {
                // Error in callback
            }
            1 // Return a fake timer ID
        })
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create setTimeout: {e}")))?;

    globals
        .set("setTimeout", set_timeout)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set setTimeout: {e}")))?;

    // setInterval(callback, ms) -> id (stub)
    let set_interval =
        Function::new(ctx.clone(), |_callback: rquickjs::Function<'_>, _ms: Option<i32>| -> i32 {
            // Not implemented in MVP
            0
        })
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create setInterval: {e}")))?;

    globals
        .set("setInterval", set_interval)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to setInterval: {e}")))?;

    // clearTimeout(id) (stub)
    let clear_timeout =
        Function::new(ctx.clone(), |_id: i32| {})
            .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create clearTimeout: {e}")))?;

    globals
        .set("clearTimeout", clear_timeout)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set clearTimeout: {e}")))?;

    // clearInterval(id) (stub)
    let clear_interval =
        Function::new(ctx.clone(), |_id: i32| {})
            .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create clearInterval: {e}")))?;

    globals
        .set("clearInterval", clear_interval)
        .map_err(|e| {
            crate::JsRaftError::Extension(format!("Failed to set clearInterval: {e}"))
        })?;

    Ok(())
}
