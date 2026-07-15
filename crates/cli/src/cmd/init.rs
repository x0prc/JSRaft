use anyhow::Result;
use jsraft_pkg::PkgConfig;
use std::fs;
use std::path::Path;
use tracing::info;

/// Available project templates.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum Template {
    /// Minimal JavaScript project
    #[default]
    Default,
    /// TypeScript project with tsconfig.json
    TypeScript,
    /// Web server project with HTTP handler
    Server,
}

impl std::fmt::Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Template::Default => write!(f, "default"),
            Template::TypeScript => write!(f, "typescript"),
            Template::Server => write!(f, "server"),
        }
    }
}

/// Scaffold a new JSRaft project.
pub fn execute(name: Option<String>, template: Template) -> Result<()> {
    let project_name = name.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    });

    let root = std::env::current_dir()?;

    // Check if directory is not empty
    let has_files = root.read_dir()?.next().is_some();
    if has_files && !root.join("jsraft.toml").exists() {
        anyhow::bail!(
            "Directory is not empty. Use --name to create in a subdirectory, or run in an empty directory."
        );
    }

    info!("Initializing project: {project_name} (template: {template})");

    // Create project structure based on template
    match template {
        Template::Default => scaffold_default(&root, &project_name)?,
        Template::TypeScript => scaffold_typescript(&root, &project_name)?,
        Template::Server => scaffold_server(&root, &project_name)?,
    }

    // Always create these files
    create_gitignore(&root)?;
    create_readme(&root, &project_name)?;

    println!("Created project: {project_name}");
    println!("\nProject structure:");
    print_tree(&root)?;

    println!("\nGet started:");
    match template {
        Template::Default | Template::TypeScript => {
            println!("  jsraft run src/index.js");
            println!("  jsraft run src/index.js --watch");
        }
        Template::Server => {
            println!("  jsraft run src/server.js");
            println!("  curl http://localhost:3000");
        }
    }

    Ok(())
}

/// Scaffold the default JavaScript template.
fn scaffold_default(root: &Path, name: &str) -> Result<()> {
    // jsraft.toml
    let config = PkgConfig {
        name: Some(name.to_string()),
        version: Some("0.1.0".into()),
        ..Default::default()
    };
    fs::write(root.join("jsraft.toml"), config.to_toml()?)?;

    // src/index.js
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("src/index.js"),
        r#"// Welcome to JSRaft!
console.log("Hello from JSRaft!");

// Try editing this file and running:
//   jsraft run src/index.js --watch
"#,
    )?;

    Ok(())
}

/// Scaffold the TypeScript template.
fn scaffold_typescript(root: &Path, name: &str) -> Result<()> {
    // jsraft.toml
    let config = PkgConfig {
        name: Some(name.to_string()),
        version: Some("0.1.0".into()),
        ..Default::default()
    };
    fs::write(root.join("jsraft.toml"), config.to_toml()?)?;

    // tsconfig.json
    fs::write(
        root.join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src"]
}
"#,
    )?;

    // src/index.ts
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("src/index.ts"),
        r#"// Welcome to JSRaft with TypeScript!
const greeting: string = "Hello from JSRaft!";
console.log(greeting);

// Try editing this file and running:
//   jsraft run src/index.ts --watch
"#,
    )?;

    Ok(())
}

/// Scaffold the server template.
fn scaffold_server(root: &Path, name: &str) -> Result<()> {
    // jsraft.toml
    let config = PkgConfig {
        name: Some(name.to_string()),
        version: Some("0.1.0".into()),
        ..Default::default()
    };
    fs::write(root.join("jsraft.toml"), config.to_toml()?)?;

    // src/server.js
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("src/server.js"),
        r#"// JSRaft Web Server
// A minimal HTTP server example

const PORT = 3000;

// Simple request handler
function handleRequest(method, path) {
    if (path === "/") {
        return {
            status: 200,
            body: JSON.stringify({ message: "Hello from JSRaft Server!" }),
        };
    }

    if (path === "/health") {
        return {
            status: 200,
            body: JSON.stringify({ status: "ok" }),
        };
    }

    return {
        status: 404,
        body: JSON.stringify({ error: "Not found" }),
    };
}

console.log(`Server starting on http://localhost:${PORT}`);
console.log("Routes:");
console.log("  GET /        - Hello message");
console.log("  GET /health  - Health check");
console.log("");
console.log("Note: HTTP server support is coming soon!");
console.log("For now, this demonstrates the server scaffolding.");
"#,
    )?;

    // src/routes.js
    fs::write(
        root.join("src/routes.js"),
        r#"// Route definitions
// This file defines the API routes for the server.

export const routes = {
    "GET /": {
        handler: () => ({ message: "Hello from JSRaft Server!" }),
    },
    "GET /health": {
        handler: () => ({ status: "ok" }),
    },
};
"#,
    )?;

    Ok(())
}

/// Create .gitignore
fn create_gitignore(root: &Path) -> Result<()> {
    fs::write(
        root.join(".gitignore"),
        r#"# Dependencies
node_modules/

# Build output
dist/
build/

# JSRaft cache
.jsraft/

# OS files
.DS_Store
Thumbs.db

# Editor files
*.swp
*.swo
*~
.vscode/
.idea/

# Logs
*.log
"#,
    )?;
    Ok(())
}

/// Create README.md
fn create_readme(root: &Path, name: &str) -> Result<()> {
    fs::write(
        root.join("README.md"),
        format!(
            r#"# {name}

A JSRaft project.

## Getting Started

```bash
# Run the project
jsraft run src/index.js

# Run with watch mode
jsraft run src/index.js --watch

# Run the REPL
jsraft repl
```

## Plugins

Place JavaScript plugins in `plugins/` or pass a custom directory:

```bash
jsraft run src/index.js --plugins-dir ./plugins
```

Plugins run before your entry file and can register with:

```js
JSRaft.registerPlugin("my-plugin");
globalThis.myFeature = "available to the app";
```

## Project Structure

```
{name}/
  jsraft.toml    # Project configuration
  src/           # Source files
  plugins/       # Optional runtime plugins
  .gitignore     # Git ignore rules
```

## Learn More

- [JSRaft Documentation](https://github.com/c0ldheat/jsraft)
- [JSRaft Examples](https://github.com/c0ldheat/jsraft/examples)
"#
        ),
    )?;
    Ok(())
}

/// Print a simple directory tree.
fn print_tree(root: &Path) -> Result<()> {
    let entries = fs::read_dir(root)?;
    let mut files: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !name.starts_with('.') && name != "target"
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                format!("{name}/")
            } else {
                name
            }
        })
        .collect();

    files.sort();
    for file in &files {
        println!("  {file}");
    }

    Ok(())
}
