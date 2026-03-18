//! `api2cli` — the command-line tool.
//!
//! Two primary subcommands:
//!
//! * **`generate`** — scaffold a standalone Cargo project whose binary uses
//!   `api2cli` as a library to run the spec as a CLI.
//! * **`run`** — execute the spec directly as a CLI without any code
//!   generation (great for exploration and one-off calls).

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use api2cli::{DynamicCli, DynamicCliConfig, OutputFormat, ProjectGenerator};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "api2cli",
    about = "Turn any OpenAPI/Swagger spec into a fully-functional CLI",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a standalone CLI binary project from an OpenAPI spec.
    ///
    /// The generated project contains a single `main.rs` that embeds the spec
    /// URL and uses `api2cli` as a library. Build it with `cargo build` and
    /// ship the resulting binary as a native CLI for your API.
    ///
    /// Example:
    ///   api2cli generate https://petstore.swagger.io/v2/swagger.json --name pets
    Generate {
        /// OpenAPI spec URL or local file path (JSON or YAML).
        spec: String,

        /// Name for the generated application (also used as the binary name).
        #[arg(short, long)]
        name: String,

        /// Directory in which to create the project folder.
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },

    /// Execute an OpenAPI spec directly as a live CLI — no code generation.
    ///
    /// Everything after the spec URL/path is forwarded to the dynamic CLI.
    ///
    /// Example:
    ///   api2cli run https://petstore.swagger.io/v2/swagger.json -- pet get-pet-by-id --pet-id 1
    Run {
        /// OpenAPI spec URL or local file path (JSON or YAML).
        spec: String,

        /// Arguments forwarded to the dynamic CLI (use `--` to separate them).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { spec, name, output } => {
            let gen = ProjectGenerator {
                app_name: name,
                spec_source: spec,
                output_dir: output,
            };
            gen.generate()?;
        }

        Commands::Run { spec, args } => {
            let app_name = derive_app_name(&spec);

            let config = DynamicCliConfig {
                spec_source: spec,
                app_name: app_name.clone(),
                base_url_override: None,
                auth_token: None,
                output_format: OutputFormat::Pretty,
            };

            let dynamic = DynamicCli::new(config)?;

            // Prepend the binary name so the arg list mirrors what clap expects
            // from std::env::args.
            let full_args: Vec<String> = std::iter::once(app_name).chain(args).collect();
            dynamic.run_with_args(full_args)?;
        }
    }

    Ok(())
}

/// Derive a short, readable app name from a spec URL or file path.
///
/// ```text
/// "https://petstore.swagger.io/v2/swagger.json" → "swagger"
/// "/path/to/openapi.yaml"                        → "openapi"
/// "myapi.json"                                   → "myapi"
/// ```
fn derive_app_name(spec: &str) -> String {
    // Take the last path segment, strip query string and fragment, then
    // remove the file extension.
    let last = spec
        .split(&['/', '\\'][..])
        .next_back()
        .unwrap_or(spec)
        .split('?')
        .next()
        .unwrap_or(spec)
        .split('#')
        .next()
        .unwrap_or(spec);

    let stem = last
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(last);

    if stem.is_empty() {
        "api".to_string()
    } else {
        stem.to_string()
    }
}

use colored::Colorize as _;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_name_from_url() {
        assert_eq!(
            derive_app_name("https://petstore.swagger.io/v2/swagger.json"),
            "swagger"
        );
        assert_eq!(
            derive_app_name("https://api.example.com/openapi.yaml"),
            "openapi"
        );
    }

    #[test]
    fn derive_name_from_file() {
        assert_eq!(derive_app_name("/path/to/myapi.json"), "myapi");
        assert_eq!(derive_app_name("spec.yaml"), "spec");
    }

    #[test]
    fn derive_name_fallback() {
        assert_eq!(derive_app_name("https://api.example.com/"), "api");
    }
}
