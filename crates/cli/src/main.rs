mod cmd;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cmd::Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    fmt().with_env_filter(filter).init();

    match cli.command {
        cmd::Commands::Run {
            file,
            cache,
            cache_dir,
            plugins_dirs,
            watch,
            args,
        } => {
            cmd::run::execute(&file, cache, cache_dir, plugins_dirs, watch, &args).await?;
        }

        cmd::Commands::Build {
            entry,
            outdir,
            format,
            minify,
            sourcemap,
        } => {
            cmd::build::execute(entry.as_deref(), outdir.as_deref(), &format, minify, sourcemap)
                .await?;
        }

        cmd::Commands::Lint {
            paths,
            fix,
            max_warnings,
        } => {
            let path_refs: Vec<_> = paths.iter().map(|p| p.as_path()).collect();
            cmd::lint::execute(&path_refs, fix, max_warnings).await?;
        }

        cmd::Commands::Fmt {
            paths,
            check,
            stdout,
        } => {
            let path_refs: Vec<_> = paths.iter().map(|p| p.as_path()).collect();
            cmd::fmt::execute(&path_refs, check, stdout).await?;
        }

        cmd::Commands::Install {
            package,
            version,
            dev,
        } => {
            cmd::install::execute(package.as_deref(), version.as_deref(), dev).await?;
        }

        cmd::Commands::Remove { package } => {
            println!("Removing {package}...");
            // TODO: implement remove
            todo!("Implement package removal")
        }

        cmd::Commands::Search { query } => {
            println!("Searching for '{query}'...");
            let registry = jsraft_pkg::NpmRegistry::new();
            let results = registry.search(&query).await?;

            for result in &results {
                println!(
                    "{}@{} - {}",
                    result.name,
                    result.version,
                    result.description.chars().take(50).collect::<String>()
                );
            }
        }

        cmd::Commands::Init { name, template } => {
            cmd::init::execute(name, template)?;
        }

        cmd::Commands::Repl => {
            cmd::repl::execute().await?;
        }

        cmd::Commands::RunScript { script } => {
            // Load config and find script
            let root = std::env::current_dir()?;
            let config_path = root.join("jsraft.toml");

            if !config_path.exists() {
                anyhow::bail!("No jsraft.toml found");
            }

            let _content = std::fs::read_to_string(&config_path)?;
            // For MVP, just print the script command
            println!("Running script: {script}");
            // TODO: parse and execute scripts from config
        }

        cmd::Commands::Update { package: _ } => {
            println!("Updating dependencies...");
            // TODO: implement update
            todo!("Implement dependency updates")
        }

        cmd::Commands::Outdated => {
            println!("Checking for outdated packages...");
            // TODO: implement outdated check
            todo!("Implement outdated check")
        }

        cmd::Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = cmd::Cli::command();
            clap_complete::generate(shell, &mut cmd, "jsraft", &mut std::io::stdout());
        }
    }

    Ok(())
}
