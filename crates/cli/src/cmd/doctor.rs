use anyhow::Result;

/// Print project and toolchain diagnostics.
pub async fn execute() -> Result<()> {
    let cwd = std::env::current_dir()?;

    println!("JSRaft Doctor");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("CWD: {}", cwd.display());
    println!();

    check("jsraft.toml", cwd.join("jsraft.toml").exists());
    check("src/", cwd.join("src").is_dir());
    check("node_modules/", cwd.join("node_modules").is_dir());
    check("plugins/", cwd.join("plugins").is_dir());
    check(".jsraft/cache/", cwd.join(".jsraft").join("cache").is_dir());

    println!();
    println!("Runtime: QuickJS");
    println!("TypeScript transform: enabled");
    println!("Bytecode cache: enabled by default");
    println!("Permissions: allow-by-default; use --secure for deny-by-default");

    Ok(())
}

fn check(label: &str, ok: bool) {
    let status = if ok { "ok" } else { "missing" };
    println!("{label}: {status}");
}
