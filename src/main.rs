//! API2CLI - Turn any RESTful API into an executable CLI with one command

use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::env;

mod spec;
mod generator;
mod runtime;

use spec::SpecLoader;
use generator::CliGenerator;
use runtime::HttpClient;

#[derive(Parser, Debug)]
#[command(name = "api2cli")]
#[command(about = "Turn any RESTful API into an executable CLI with one command", long_about = None)]
struct Args {
    /// OpenAPI spec URL or local file path
    #[arg(default_value = "https://petstore.swagger.io/v2/swagger.json")]
    spec: String,
    
    /// Output CLI application name
    #[arg(short, long, default_value = "myapi")]
    name: String,
    
    /// Base URL for the API (if not specified in spec)
    #[arg(short, long)]
    base_url: Option<String>,
    
    /// Auth token (Bearer or API Key)
    #[arg(short, long)]
    token: Option<String>,
    
    /// Output directory
    #[arg(short, long, default_value = ".")]
    output: String,
    
    /// Generate shell completions
    #[arg(long)]
    completions: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("🔧 Generating CLI: {}", args.name);
    println!("📡 Loading spec: {}\n", args.spec);
    
    // Load spec
    let spec_json = SpecLoader::load(&args.spec)?;
    
    // Generate CLI
    let mut generator = CliGenerator::new();
    generator.generate_from_json(&spec_json)?;
    
    // Get base URL from spec or args
    let base_url = args.base_url.or_else(|| {
        spec_json.get("host")
            .and_then(|h| h.as_str())
            .map(|h| {
                let scheme = spec_json.get("schemes")
                    .and_then(|s| s.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|s| s.as_str())
                    .unwrap_or("https");
                let base_path = spec_json.get("basePath")
                    .and_then(|b| b.as_str())
                    .unwrap_or("");
                format!("{}://{}{}", scheme, h, base_path)
            })
    });
    
    // Generate the CLI application
    let cli_code = generator.generate_cli_app(&args.name, base_url.as_deref(), args.token.as_deref());
    
    // Write to output
    let output_dir = if args.output == "." {
        env::current_dir()?
    } else {
        std::path::PathBuf::from(&args.output)
    };
    
    let src_dir = output_dir.join(&args.name).join("src");
    std::fs::create_dir_all(&src_dir)?;
    
    // Write Cargo.toml
    std::fs::write(
        output_dir.join(&args.name).join("Cargo.toml"),
        generate_cargo_toml(&args.name),
    )?;
    
    // Write main.rs
    std::fs::write(src_dir.join("main.rs"), cli_code)?;
    
    println!("✅ Generated CLI app: {}/", args.name);
    println!("📦 To build and run:");
    println!("   cd {} && cargo build --release", args.name);
    println!("   ./target/release/{}", args.name);
    
    Ok(())
}

fn generate_cargo_toml(name: &str) -> String {
    format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
description = "Auto-generated CLI from OpenAPI spec"

[dependencies]
clap = {{ version = "4.5", features = ["derive"] }}
reqwest = {{ version = "0.12", features = ["json", "blocking"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
anyhow = "1.0"

[[bin]]
name = "{}"
path = "src/main.rs"
"#, name, name)
}
