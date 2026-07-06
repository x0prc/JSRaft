pub mod console;
pub mod fs;
pub mod net;
pub mod path;
pub mod process;
pub mod timers;

/// Get all default extensions (just a list of module names for reference).
pub fn default_extension_names() -> Vec<&'static str> {
    vec![
        "console",
        "fs",
        "net",
        "path",
        "process",
        "timers",
    ]
}
