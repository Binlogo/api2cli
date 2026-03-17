//! OpenAPI Spec Loader

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;

/// Load OpenAPI spec from URL or local file
pub struct SpecLoader;

impl SpecLoader {
    pub fn load(source: &str) -> Result<Value> {
        if source.starts_with("http://") || source.starts_with("https://") {
            Self::load_from_url(source)
        } else {
            Self::load_from_file(source)
        }
    }
    
    fn load_from_url(url: &str) -> Result<Value> {
        let response = reqwest::blocking::get(url)
            .context(format!("Failed to fetch spec from {}", url))?;
        
        let text = response.text().context("Failed to read response body")?;
        
        // Try JSON
        serde_json::from_str(&text)
            .context("Failed to parse OpenAPI spec")
    }
    
    fn load_from_file(path: &str) -> Result<Value> {
        let content = fs::read_to_string(path)
            .context(format!("Failed to read file: {}", path))?;
        
        serde_json::from_str(&content)
            .context("Failed to parse OpenAPI spec")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_spec_loader_placeholder() {
        assert!(true);
    }
}
