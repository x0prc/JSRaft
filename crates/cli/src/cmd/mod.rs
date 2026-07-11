pub mod build;
pub mod fmt;
pub mod install;
pub mod lint;
pub mod repl;
pub mod run;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// JSRaft: A minimal JavaScript runtime + toolchain
#[derive(Parser, Debug)]
#[command(
    name = "jsraft",
    version,
    about = "A minimal JavaScript runtime + compiler toolchain + package ecosystem",
    long_about = None
)]
pub struct Cli {
    /// Run a subcommand
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Path to config file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Execute a JavaScript or TypeScript file
    Run {
        /// File to execute
        file: PathBuf,

        /// Enable bytecode caching (snapshot)
        #[arg(long, default_value = "true")]
        cache: bool,

        /// Custom cache directory
        #[arg(long)]
        cache_dir: Option<PathBuf>,

        /// Pass arguments to the script
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Bundle the project
    Build {
        /// Entry point
        #[arg(short, long)]
        entry: Option<String>,

        /// Output directory
        #[arg(short, long)]
        outdir: Option<String>,

        /// Output format (esm, iife, cjs)
        #[arg(short, long, default_value = "esm")]
        format: String,

        /// Enable minification
        #[arg(short, long)]
        minify: bool,

        /// Enable source maps
        #[arg(long, default_value = "true")]
        sourcemap: bool,
    },

    /// Lint JavaScript or TypeScript files
    Lint {
        /// Files or directories to lint
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Fix auto-fixable issues
        #[arg(short, long)]
        fix: bool,

        /// Maximum number of warnings
        #[arg(long, default_value = "100")]
        max_warnings: usize,
    },

    /// Format JavaScript or TypeScript files
    Fmt {
        /// Files or directories to format
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Check formatting without writing
        #[arg(short, long)]
        check: bool,

        /// Write formatted output to stdout
        #[arg(short, long)]
        stdout: bool,
    },

    /// Install dependencies
    Install {
        /// Package to install
        package: Option<String>,

        /// Version requirement
        version: Option<String>,

        /// Install as dev dependency
        #[arg(short, long)]
        dev: bool,
    },

    /// Remove a package
    Remove {
        /// Package to remove
        package: String,
    },

    /// Search for packages
    Search {
        /// Search query
        query: String,
    },

    /// Initialize a new project
    Init {
        /// Project name
        name: Option<String>,
    },

    /// Start an interactive JavaScript REPL
    Repl,

    /// Run a script from package.json/jsraft.toml
    RunScript {
        /// Script name
        script: String,
    },

    /// Update dependencies
    Update {
        /// Specific package to update
        package: Option<String>,
    },

    /// Check for outdated dependencies
    Outdated,

    /// Generate shell completions
    Completions {
        /// Shell to generate for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}
