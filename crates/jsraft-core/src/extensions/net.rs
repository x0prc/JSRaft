use rquickjs::{Ctx, Function};

/// Register networking APIs: fetch, etc.
pub fn register(ctx: &Ctx<'_>) -> crate::Result<()> {
    let globals = ctx.globals();

    // Global fetch (basic implementation)
    let fetch = Function::new(
        ctx.clone(),
        move |url: String| -> String {
            // Synchronous fetch using reqwest for MVP
            let client = reqwest::blocking::Client::new();

            match client.get(&url).send() {
                Ok(response) => {
                    let status = response.status().as_u16();
                    match response.text() {
                        Ok(body) => {
                            if status >= 400 {
                                format!("{{\"error\":\"HTTP {status}\",\"body\":\"{body}\"}}")
                            } else {
                                body
                            }
                        }
                        Err(e) => format!("{{\"error\":\"Response read error: {e}\"}}"),
                    }
                }
                Err(e) => format!("{{\"error\":\"Fetch error: {e}\"}}"),
            }
        },
    )
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create fetch: {e}")))?;

    globals
        .set("fetch", fetch)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set fetch: {e}")))?;

    Ok(())
}
