//! # api2cli
//!
//! **Turn any OpenAPI/Swagger specification into a fully-functional CLI with
//! one command.**
//!
//! ```text
//! api2cli generate https://petstore.swagger.io/v2/swagger.json --name pets
//! cd pets && cargo build --release
//! ./target/release/pets pet get-pet-by-id --pet-id 1
//! ```
//!
//! ## Library usage
//!
//! Use [`DynamicCli`] to embed a live API CLI inside your own binary:
//!
//! ```no_run
//! use api2cli::{DynamicCli, DynamicCliConfig, OutputFormat};
//!
//! fn main() -> anyhow::Result<()> {
//!     DynamicCli::new(DynamicCliConfig {
//!         spec_source: "https://petstore.swagger.io/v2/swagger.json".to_string(),
//!         app_name: "pets".to_string(),
//!         base_url_override: None,
//!         auth_token: None,
//!         output_format: OutputFormat::Pretty,
//!     })?
//!     .run()?;
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod generator;
pub mod runtime;
pub mod spec;

pub use error::{Error, Result};
pub use generator::ProjectGenerator;
pub use runtime::{DynamicCli, DynamicCliConfig, OutputFormat};
pub use spec::{ApiSpec, Operation, SpecLoader};
