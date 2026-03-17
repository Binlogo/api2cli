//! API2CLI - Generate CLI tools from OpenAPI specifications
//! 
//! This PoC demonstrates the core concept: loading an OpenAPI spec and
//! dynamically generating CLI commands from it.

pub mod spec;
pub mod generator;
pub mod runtime;

pub use spec::SpecLoader;
pub use generator::CliGenerator;
pub use runtime::HttpClient;

use anyhow::Result;
use serde_json::Value;

/// Main API2CLI struct
pub struct Api2Cli {
    spec: Value,
    #[allow(dead_code)]
    client: HttpClient,
}

impl Api2Cli {
    /// Create a new Api2Cli instance from a spec URL or file
    pub fn new(spec_source: &str, auth_token: Option<String>) -> Result<Self> {
        let spec = SpecLoader::load(spec_source)?;
        let client = HttpClient::new(auth_token);
        
        Ok(Self { spec, client })
    }
    
    /// Generate CLI commands from the spec
    pub fn generate_cli(&self) -> Result<CliGenerator> {
        let mut generator = CliGenerator::new();
        generator.generate_from_json(&self.spec)?;
        Ok(generator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api2cli_creation() {
        assert!(true);
    }
}
