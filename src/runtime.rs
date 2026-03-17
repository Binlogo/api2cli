//! HTTP Runtime - Execute API requests

use anyhow::Result;
use reqwest::blocking::Client;
use std::collections::HashMap;

/// HTTP client for making API requests
pub struct HttpClient {
    client: Client,
    auth_token: Option<String>,
    base_url: Option<String>,
}

impl HttpClient {
    pub fn new(auth_token: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            auth_token,
            base_url: None,
        }
    }
    
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = Some(base_url.to_string());
        self
    }
    
    pub fn request(
        &self,
        method: &str,
        path: &str,
        params: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<String> {
        let url = match &self.base_url {
            Some(base) => format!("{}{}", base, path),
            None => path.to_string(),
        };
        
        let mut request = match method.to_uppercase().as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            _ => anyhow::bail!("Unsupported method: {}", method),
        };
        
        // Add auth
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        
        // Add query params
        if !params.is_empty() {
            request = request.query(params);
        }
        
        // Add body
        if let Some(b) = body {
            request = request.body(b.to_string());
            request = request.header("Content-Type", "application/json");
        }
        
        let response = request.send()?;
        
        Ok(response.text()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_http_client_creation() {
        let client = HttpClient::new(Some("token123".to_string()));
        assert!(client.auth_token.is_some());
    }
}
