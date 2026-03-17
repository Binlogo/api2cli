//! CLI Generator - Convert OpenAPI to CLI commands

use anyhow::Result;
use serde_json::Value;

/// Generates CLI commands from OpenAPI spec
pub struct CliGenerator {
    commands: Vec<CliCommand>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParamLocation {
    Path,
    Query,
    Header,
    Body,
}

#[derive(Debug, Clone)]
pub struct CliCommand {
    pub name: String,
    pub path: String,
    pub method: String,
    pub description: String,
    pub params: Vec<CliParam>,
    pub has_body: bool,
}

#[derive(Debug, Clone)]
pub struct CliParam {
    pub name: String,
    pub location: ParamLocation,
    pub required: bool,
    pub param_type: String,
    pub description: String,
}

impl CliGenerator {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }
    
    /// Generate CLI commands from OpenAPI spec JSON value
    pub fn generate_from_json(&mut self, spec: &Value) -> Result<()> {
        let paths = spec.get("paths")
            .and_then(|p| p.as_object())
            .ok_or_else(|| anyhow::anyhow!("No paths found in spec"))?;
        
        for (path, path_item) in paths {
            if let Some(obj) = path_item.as_object() {
                for method in ["get", "post", "put", "delete", "patch"] {
                    if let Some(op) = obj.get(method) {
                        self.add_command(method, path, op)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn add_command(&mut self, verb: &str, path: &str, op: &Value) -> Result<()> {
        let name = self.make_command_name(verb, path);
        
        let description = op.get("summary")
            .and_then(|v| v.as_str())
            .or_else(|| op.get("description").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        
        let mut params = Vec::new();
        
        // Check for requestBody
        let has_body = op.get("requestBody").is_some();
        
        if let Some(parameters) = op.get("parameters").and_then(|p| p.as_array()) {
            for p in parameters {
                let name = p.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                let location_str = p.get("in")
                    .and_then(|v| v.as_str())
                    .unwrap_or("query");
                
                let location = match location_str {
                    "path" => ParamLocation::Path,
                    "header" => ParamLocation::Header,
                    "body" => ParamLocation::Body,
                    _ => ParamLocation::Query,
                };
                
                let required = p.get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                let param_type = p.get("schema")
                    .and_then(|s| s.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("string")
                    .to_string();
                
                let description = p.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                
                params.push(CliParam {
                    name,
                    location,
                    required,
                    param_type,
                    description,
                });
            }
        }
        
        self.commands.push(CliCommand {
            name,
            path: path.to_string(),
            method: verb.to_uppercase(),
            description,
            params,
            has_body,
        });
        
        Ok(())
    }
    
    /// Convert path to command name: /users/{id} -> get-users-id
    pub fn make_command_name(&self, verb: &str, path: &str) -> String {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        let name = parts.iter()
            .map(|p| {
                if p.starts_with('{') && p.ends_with('}') {
                    format!("-{}", &p[1..p.len()-1])
                } else {
                    format!("-{}", p)
                }
            })
            .collect::<Vec<_>>()
            .join("");
        
        format!("{}{}", verb, name)
    }
    
    /// Generate complete CLI application code
    pub fn generate_cli_app(&self, name: &str, base_url: Option<&str>, token: Option<&str>) -> String {
        let mut code = String::new();
        
        // Header
        code.push_str(&format!("//! {} - Auto-generated CLI from OpenAPI spec\n\n", name));
        code.push_str("use anyhow::Result;\n");
        code.push_str("use clap::{Parser, Subcommand};\n");
        code.push_str("use reqwest::blocking::Client;\n\n");
        
        // Args struct
        code.push_str("#[derive(Parser, Debug)]\n");
        code.push_str("#[command(name = \"cli\")]\n");
        code.push_str("#[command(about = \"Auto-generated CLI from OpenAPI\")]\n");
        code.push_str("struct Args {\n");
        code.push_str("    #[command(subcommand)]\n");
        code.push_str("    command: Commands,\n");
        code.push_str("}\n\n");
        
        // Commands enum
        code.push_str("#[derive(Subcommand, Debug)]\n");
        code.push_str("enum Commands {\n");
        
        for cmd in &self.commands {
            let variant_name = cmd.name.replace('-', "_");
            let enum_variant = variant_name.split('_')
                .map(|s| {
                    let mut chars = s.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<String>();
            
            let struct_name = enum_variant.clone();
            
            code.push_str(&format!("    /// {}\n", cmd.description));
            code.push_str(&format!("    {}({}),\n", enum_variant, struct_name));
        }
        code.push_str("}\n\n");
        
        // Args for each command
        for cmd in &self.commands {
            let variant_name = cmd.name.replace('-', "_");
            
            // Convert to UpperCamelCase
            let struct_name = variant_name.split('_')
                .map(|s| {
                    let mut chars = s.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<String>();
            
            code.push_str(&format!("#[derive(Parser, Debug)]\n"));
            code.push_str(&format!("struct {} {{\n", struct_name));
            
            // Add path params first (required positional)
            for param in &cmd.params {
                if param.location == ParamLocation::Path {
                    let arg_name = param.name.replace('-', "_");
                    code.push_str(&format!("    /// {}\n", param.description));
                    code.push_str(&format!("    {}: String,\n", arg_name));
                }
            }
            
            // Add query params
            for param in &cmd.params {
                if param.location == ParamLocation::Query {
                    let arg_name = param.name.replace('-', "_");
                    code.push_str(&format!("    /// {}\n", param.description));
                    code.push_str("    #[arg(long)]\n");
                    code.push_str(&format!("    {}: Option<String>,\n", arg_name));
                }
            }
            
            // Add body if present
            if cmd.has_body {
                code.push_str("    /// JSON request body\n");
                code.push_str("    #[arg(long)]\n");
                code.push_str("    body: Option<String>,\n");
            }
            
            code.push_str("}\n\n");
        }
        
        // Main function
        code.push_str("fn main() -> Result<()> {\n");
        code.push_str("    let args = Args::parse();\n");
        code.push_str("    let client = Client::new();\n");
        code.push_str(&format!("    let base_url = \"{}\";\n\n", base_url.unwrap_or("https://api.example.com")));
        
        if let Some(t) = token {
            code.push_str(&format!("    let token = \"{}\";\n", t));
        } else {
            code.push_str("    let token = std::env::var(\"API_TOKEN\").ok();\n");
        }
        
        code.push_str("    match args.command {\n");
        
        for cmd in &self.commands {
            let variant_name = cmd.name.replace('-', "_");
            let method_lower = cmd.method.to_lowercase();
            
            let enum_variant = variant_name.split('_')
                .map(|s| {
                    let mut chars = s.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<String>();
            
            code.push_str(&format!("        Commands::{}(args) => {{\n", enum_variant));
            
            // Build URL with path params
            let mut url_path = cmd.path.clone();
            for param in &cmd.params {
                if param.location == ParamLocation::Path {
                    let arg_name = param.name.replace('-', "_");
                    url_path = url_path.replace(
                        &format!("{{{}}}", param.name),
                        &format!("${{{}}}", arg_name)
                    );
                }
            }
            
            let url_replacements = if cmd.params.iter().any(|p| p.location == ParamLocation::Path) { 
                let mut replaces = String::new();
                for param in &cmd.params {
                    if param.location == ParamLocation::Path {
                        let arg_name = param.name.replace('-', "_");
                        replaces.push_str(&format!(
                            ".replace(\"${{{}}}\", &args.{})",
                            arg_name, arg_name
                        ));
                    }
                }
                replaces
            } else {
                String::new()
            };
            
            let base_url_str = base_url.unwrap_or("https://api.example.com").trim_end_matches('/');
            let url_template = format!("{}/{}", base_url_str, url_path.trim_start_matches('/'));
            let url_code = if !url_replacements.is_empty() {
                format!("let mut url = \"{}\"{};", url_template, url_replacements)
            } else {
                format!("let url = \"{}\";", url_template)
            };
            code.push_str(&format!("            {}\n", url_code));
            
            // Build request
            code.push_str(&format!("            let mut request = client.{}(url);\n", method_lower));
            
            // Add auth
            code.push_str("            if let Some(t) = &token {\n");
            code.push_str("                request = request.bearer_auth(t);\n");
            code.push_str("            }\n");
            
            // Add query params
            for param in &cmd.params {
                if param.location == ParamLocation::Query {
                    let arg_name = param.name.replace('-', "_");
                    code.push_str(&format!("            if let Some(v) = &args.{} {{\n", arg_name));
                    code.push_str(&format!("                request = request.query(&[(\"{}\", v)]);\n", param.name));
                    code.push_str("            }\n");
                }
            }
            
            // Add body only if the command has body
            if cmd.has_body {
                code.push_str("            if let Some(b) = &args.body {\n");
                code.push_str("                request = request.body(b.clone());\n");
                code.push_str("                request = request.header(\"Content-Type\", \"application/json\");\n");
                code.push_str("            }\n");
            }
            
            // Execute and print
            code.push_str("            let response = request.send()?;\n");
            code.push_str("            println!(\"{}\", response.text()?);\n");
            code.push_str("            Ok(())\n");
            code.push_str("        }\n");
        }
        
        code.push_str("    }\n");
        code.push_str("}\n");
        
        code
    }
}

impl Default for CliGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_command_name() {
        let gen = CliGenerator::new();
        assert_eq!(gen.make_command_name("get", "/users"), "get-users");
        assert_eq!(gen.make_command_name("get", "/users/{id}"), "get-users-id");
    }
}
