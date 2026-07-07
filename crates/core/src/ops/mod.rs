/// Operations (Rust functions exposed to JavaScript).
///
/// This module defines the op system similar to Deno's.
/// Each op is a Rust function that can be called from JS.
pub fn register_ops(_ctx: &rquickjs::Ctx<'_>) -> crate::Result<()> {
    // Register core ops here as we add them
    Ok(())
}
