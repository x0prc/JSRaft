use crate::RuntimePermissions;
use rquickjs::{Ctx, Function, Object, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Register networking APIs: fetch, etc.
pub fn register(ctx: &Ctx<'_>, permissions: &RuntimePermissions) -> crate::Result<()> {
    let globals = ctx.globals();

    // Global fetch (basic implementation)
    let allow_net = permissions.net;
    let fetch = Function::new(
        ctx.clone(),
        move |url: String| -> String {
            if !allow_net {
                return "{\"error\":\"Permission denied: net access requires --allow-net\"}".into();
            }
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

    let serve = Function::new(ctx.clone(), serve_native)
    .map_err(|e| crate::JsRaftError::Extension(format!("Failed to create serve: {e}")))?;

    globals
        .set("__jsraft_serve", serve)
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to set __jsraft_serve: {e}")))?;

    let bootstrap = if permissions.net {
        r#"
        globalThis.JSRaft = globalThis.JSRaft || {};
        JSRaft.serve = function serve(handler, options) {
            const port = Number((options && options.port) || 3000);
            return __jsraft_serve(port, handler);
        };
        globalThis.Deno = globalThis.Deno || {};
        Deno.serve = function serve(options, handler) {
            if (typeof options === "function") {
                handler = options;
                options = {};
            }
            const port = Number((options && options.port) || 3000);
            return __jsraft_serve(port, handler);
        };
    "#
    } else {
        r#"
        globalThis.JSRaft = globalThis.JSRaft || {};
        JSRaft.serve = function serve() {
            throw new Error("Permission denied: net access requires --allow-net");
        };
        globalThis.Deno = globalThis.Deno || {};
        Deno.serve = function serve() {
            throw new Error("Permission denied: net access requires --allow-net");
        };
    "#
    };
    let _: Value = ctx
        .eval(bootstrap.as_bytes())
        .map_err(|e| crate::JsRaftError::Extension(format!("Failed to register serve APIs: {e}")))?;

    Ok(())
}

fn serve_native<'js>(ctx: Ctx<'js>, port: u16, handler: Function<'js>) -> rquickjs::Result<()> {
    serve_blocking(ctx, port, handler)
}

fn serve_blocking<'js>(ctx: Ctx<'js>, port: u16, handler: Function<'js>) -> rquickjs::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| rquickjs::Error::new_from_js_message("io", "server", e.to_string()))?;
    println!("Listening on http://127.0.0.1:{port}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_connection(&ctx, &handler, &mut stream) {
                    let _ = write_response(
                        &mut stream,
                        500,
                        &BTreeMap::new(),
                        &format!("Internal Server Error: {error}"),
                    );
                }
            }
            Err(error) => eprintln!("Server connection error: {error}"),
        }
    }

    Ok(())
}

fn handle_connection<'js>(
    ctx: &Ctx<'js>,
    handler: &Function<'js>,
    stream: &mut TcpStream,
) -> rquickjs::Result<()> {
    let request = read_request(stream)
        .map_err(|e| rquickjs::Error::new_from_js_message("io", "request", e.to_string()))?;
    let js_request = request_to_js(ctx, &request)?;
    let response: Value = handler.call((js_request,))?;
    let response = response_from_js(response)?;

    write_response(stream, response.status, &response.headers, &response.body)
        .map_err(|e| rquickjs::Error::new_from_js_message("io", "response", e.to_string()))?;
    Ok(())
}

struct HttpRequest {
    method: String,
    path: String,
    url: String,
    headers: BTreeMap<String, String>,
    body: String,
}

struct HttpResponse {
    status: u16,
    headers: BTreeMap<String, String>,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<HttpRequest> {
    let mut buffer = [0_u8; 16 * 1024];
    let size = stream.read(&mut buffer)?;
    let raw = String::from_utf8_lossy(&buffer[..size]);
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    Ok(HttpRequest {
        method,
        url: format!("http://127.0.0.1{path}"),
        path,
        headers,
        body: body.to_string(),
    })
}

fn request_to_js<'js>(ctx: &Ctx<'js>, request: &HttpRequest) -> rquickjs::Result<Object<'js>> {
    let object = Object::new(ctx.clone())?;
    object.set("method", request.method.clone())?;
    object.set("url", request.url.clone())?;
    object.set("path", request.path.clone())?;
    object.set("body", request.body.clone())?;

    let headers = Object::new(ctx.clone())?;
    for (name, value) in &request.headers {
        headers.set(name.as_str(), value.clone())?;
    }
    object.set("headers", headers)?;

    Ok(object)
}

fn response_from_js(value: Value<'_>) -> rquickjs::Result<HttpResponse> {
    if let Some(body) = value.as_string().and_then(|s| s.to_string().ok()) {
        return Ok(HttpResponse {
            status: 200,
            headers: BTreeMap::new(),
            body,
        });
    }

    let object = value
        .into_object()
        .ok_or_else(|| {
            rquickjs::Error::new_from_js_message(
                "value",
                "response",
                "expected string or object",
            )
        })?;

    let status = if object.contains_key("status")? {
        object.get::<_, u16>("status")?
    } else {
        200
    };
    let body = if object.contains_key("body")? {
        object.get::<_, String>("body")?
    } else {
        String::new()
    };

    let mut headers = BTreeMap::new();
    if object.contains_key("contentType")? {
        headers.insert("content-type".to_string(), object.get::<_, String>("contentType")?);
    }

    Ok(HttpResponse { status, headers, body })
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    headers: &BTreeMap<String, String>,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    if !headers.contains_key("content-type") {
        response.push_str("content-type: text/plain; charset=utf-8\r\n");
    }
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    response.push_str(body);
    stream.write_all(response.as_bytes())
}
