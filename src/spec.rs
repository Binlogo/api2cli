//! OpenAPI specification parsing.
//!
//! Supports OpenAPI 3.x and Swagger 2.0 in both JSON and YAML formats.
//! Loaded from a local file path or an HTTP(S) URL.

use serde_json::Value;

use crate::error::{Error, Result};

// ── Public data model ────────────────────────────────────────────────────────

/// A fully parsed OpenAPI specification.
#[derive(Debug, Clone)]
pub struct ApiSpec {
    /// Metadata from the `info` object.
    pub info: ApiInfo,
    /// Base server URLs (at least one).
    pub servers: Vec<String>,
    /// All HTTP operations found in `paths`.
    pub operations: Vec<Operation>,
}

/// The `info` block of an OpenAPI spec.
#[derive(Debug, Clone)]
pub struct ApiInfo {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
}

/// A single HTTP operation (one method × one path).
#[derive(Debug, Clone)]
pub struct Operation {
    /// Original `operationId`, or a generated identifier.
    pub operation_id: String,
    /// Kebab-case CLI subcommand name (derived from `operation_id`).
    pub command_name: String,
    pub method: HttpMethod,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<Parameter>,
    /// Present when the operation accepts a request body.
    pub request_body: Option<RequestBody>,
    /// OpenAPI tags (used to group subcommands).
    pub tags: Vec<String>,
}

/// HTTP verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        })
    }
}

/// A single API parameter (path, query, header, or cookie).
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Original name as in the spec.
    pub name: String,
    /// Lowercased, hyphenated name used as a CLI flag (e.g. `pet-id`).
    pub cli_name: String,
    pub location: ParamIn,
    pub required: bool,
    pub schema: Schema,
    pub description: Option<String>,
}

/// Where the parameter is placed in the HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamIn {
    Path,
    Query,
    Header,
    Cookie,
}

/// Minimal schema information needed to build CLI arguments.
#[derive(Debug, Clone, Default)]
pub struct Schema {
    pub type_name: SchemaType,
    /// Allowed values for enum parameters.
    pub enum_values: Option<Vec<String>>,
    /// Default value as a string.
    pub default: Option<String>,
}

/// Simplified JSON Schema type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SchemaType {
    #[default]
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
}

/// Indicates an operation accepts a request body.
#[derive(Debug, Clone)]
pub struct RequestBody {
    pub required: bool,
    pub description: Option<String>,
}

// ── Loader ───────────────────────────────────────────────────────────────────

/// Loads and parses an OpenAPI specification from a URL or file path.
pub struct SpecLoader;

impl SpecLoader {
    /// Load a spec from a URL (`http://` / `https://`) or a local file path.
    /// Both JSON and YAML are supported.
    pub fn load(source: &str) -> Result<ApiSpec> {
        let raw = Self::fetch_raw(source)?;
        Self::parse(&raw, source)
    }

    fn fetch_raw(source: &str) -> Result<Value> {
        let content = if source.starts_with("http://") || source.starts_with("https://") {
            reqwest::blocking::get(source)
                .map_err(|e| Error::SpecLoad {
                    url: source.to_string(),
                    message: e.to_string(),
                })?
                .text()
                .map_err(|e| Error::SpecLoad {
                    url: source.to_string(),
                    message: e.to_string(),
                })?
        } else {
            std::fs::read_to_string(source).map_err(|e| Error::SpecLoad {
                url: source.to_string(),
                message: e.to_string(),
            })?
        };

        // Try JSON first, fall back to YAML.
        serde_json::from_str(&content).or_else(|_| {
            serde_yaml::from_str::<Value>(&content).map_err(|e| Error::SpecLoad {
                url: source.to_string(),
                message: format!("not valid JSON or YAML: {e}"),
            })
        })
    }

    fn parse(value: &Value, source: &str) -> Result<ApiSpec> {
        if value
            .get("swagger")
            .and_then(|v| v.as_str())
            .map(|v| v.starts_with("2."))
            .unwrap_or(false)
        {
            Self::parse_v2(value)
        } else if value
            .get("openapi")
            .and_then(|v| v.as_str())
            .map(|v| v.starts_with("3."))
            .unwrap_or(false)
        {
            Self::parse_v3(value)
        } else {
            Err(Error::UnsupportedVersion(format!(
                "'{source}' does not declare a supported OpenAPI version (swagger: 2.x or openapi: 3.x)"
            )))
        }
    }

    // ── Swagger 2.0 ──────────────────────────────────────────────────────────

    fn parse_v2(spec: &Value) -> Result<ApiSpec> {
        let info = Self::parse_info(spec)?;

        let host = spec
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("localhost");
        let base_path = spec
            .get("basePath")
            .and_then(|v| v.as_str())
            .unwrap_or("/");
        let scheme = spec
            .get("schemes")
            .and_then(|s| s.as_array())
            .and_then(|a| a.first())
            .and_then(|s| s.as_str())
            .unwrap_or("https");

        let server_url = format!(
            "{}://{}{}",
            scheme,
            host,
            base_path.trim_end_matches('/')
        );

        let operations = Self::parse_paths(spec, SpecVersion::V2)?;

        Ok(ApiSpec {
            info,
            servers: vec![server_url],
            operations,
        })
    }

    // ── OpenAPI 3.x ──────────────────────────────────────────────────────────

    fn parse_v3(spec: &Value) -> Result<ApiSpec> {
        let info = Self::parse_info(spec)?;

        let servers: Vec<String> = spec
            .get("servers")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.get("url").and_then(|u| u.as_str()).map(|u| u.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| vec!["/".to_string()]);

        let operations = Self::parse_paths(spec, SpecVersion::V3)?;

        Ok(ApiSpec {
            info,
            servers,
            operations,
        })
    }

    // ── Shared helpers ────────────────────────────────────────────────────────

    fn parse_info(spec: &Value) -> Result<ApiInfo> {
        let info_val = spec
            .get("info")
            .ok_or_else(|| Error::InvalidSpec("missing 'info' field".to_string()))?;

        Ok(ApiInfo {
            title: info_val
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("API")
                .to_string(),
            version: info_val
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.1")
                .to_string(),
            description: info_val
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    fn parse_paths(spec: &Value, version: SpecVersion) -> Result<Vec<Operation>> {
        let mut ops = Vec::new();

        let paths = match spec.get("paths").and_then(|p| p.as_object()) {
            Some(p) => p,
            None => return Ok(ops),
        };

        for (path, path_item) in paths {
            // Parameters defined at the path level apply to all operations in it.
            let path_params: Vec<Value> = path_item
                .get("parameters")
                .and_then(|p| p.as_array())
                .cloned()
                .unwrap_or_default();

            for method_str in &["get", "post", "put", "delete", "patch", "head", "options"] {
                if let Some(op_val) = path_item.get(*method_str) {
                    let method = Self::parse_method(method_str);
                    let op = Self::parse_operation(
                        method,
                        path,
                        op_val,
                        &path_params,
                        spec,
                        version,
                    )?;
                    ops.push(op);
                }
            }
        }

        Ok(ops)
    }

    fn parse_operation(
        method: HttpMethod,
        path: &str,
        op_val: &Value,
        path_params: &[Value],
        spec: &Value,
        version: SpecVersion,
    ) -> Result<Operation> {
        let operation_id = op_val
            .get("operationId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| generate_operation_id(&method.to_string().to_lowercase(), path));

        let command_name = to_kebab_case(&operation_id);

        let summary = op_val
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let description = op_val
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags: Vec<String> = op_val
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Merge path-level params with operation-level params.
        // Operation-level params override path-level params with the same name+in.
        let mut merged_params: Vec<Value> = path_params.to_vec();
        if let Some(arr) = op_val.get("parameters").and_then(|p| p.as_array()) {
            for p in arr {
                // Resolve $ref if present.
                let p = resolve_ref(p, spec).unwrap_or(p.clone());

                let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let loc = p.get("in").and_then(|v| v.as_str()).unwrap_or("");
                merged_params.retain(|ep| {
                    ep.get("name").and_then(|v| v.as_str()) != Some(name)
                        || ep.get("in").and_then(|v| v.as_str()) != Some(loc)
                });
                merged_params.push(p);
            }
        }

        let mut parameters = Vec::new();
        let mut request_body: Option<RequestBody> = None;

        for p in &merged_params {
            let p = resolve_ref(p, spec).unwrap_or(p.clone());
            let loc_str = p.get("in").and_then(|v| v.as_str()).unwrap_or("query");

            // Swagger 2.0 "body" parameter → request body.
            if loc_str == "body" && version == SpecVersion::V2 {
                request_body = Some(RequestBody {
                    required: p
                        .get("required")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    description: p
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            } else {
                parameters.push(Self::parse_parameter(&p, version));
            }
        }

        // OpenAPI 3.x requestBody.
        if version == SpecVersion::V3 {
            if let Some(rb) = op_val.get("requestBody") {
                request_body = Some(RequestBody {
                    required: rb.get("required").and_then(|v| v.as_bool()).unwrap_or(false),
                    description: rb
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }

        Ok(Operation {
            operation_id,
            command_name,
            method,
            path: path.to_string(),
            summary,
            description,
            parameters,
            request_body,
            tags,
        })
    }

    fn parse_parameter(p: &Value, version: SpecVersion) -> Parameter {
        let name = p
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("param")
            .to_string();
        let cli_name = name.replace('_', "-").to_lowercase();

        let location = match p.get("in").and_then(|v| v.as_str()).unwrap_or("query") {
            "path" => ParamIn::Path,
            "header" => ParamIn::Header,
            "cookie" => ParamIn::Cookie,
            _ => ParamIn::Query,
        };

        let required = p
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || location == ParamIn::Path;

        let schema = if version == SpecVersion::V2 {
            // v2: schema may be inline on the parameter itself, or in a `schema` sub-key.
            let schema_val = p.get("schema").unwrap_or(p);
            parse_schema_obj(schema_val)
        } else {
            p.get("schema")
                .map(parse_schema_obj)
                .unwrap_or_default()
        };

        let description = p
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Parameter {
            name,
            cli_name,
            location,
            required,
            schema,
            description,
        }
    }

    fn parse_method(s: &str) -> HttpMethod {
        match s.to_ascii_lowercase().as_str() {
            "get" => HttpMethod::Get,
            "post" => HttpMethod::Post,
            "put" => HttpMethod::Put,
            "delete" => HttpMethod::Delete,
            "patch" => HttpMethod::Patch,
            "head" => HttpMethod::Head,
            "options" => HttpMethod::Options,
            _ => HttpMethod::Get,
        }
    }
}

// ── Free helper functions ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpecVersion {
    V2,
    V3,
}

fn parse_schema_obj(s: &Value) -> Schema {
    let type_name = match s.get("type").and_then(|v| v.as_str()).unwrap_or("string") {
        "integer" => SchemaType::Integer,
        "number" => SchemaType::Number,
        "boolean" => SchemaType::Boolean,
        "array" => SchemaType::Array,
        "object" => SchemaType::Object,
        _ => SchemaType::String,
    };

    let enum_values = s.get("enum").and_then(|e| e.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    });

    let default = s.get("default").map(|d| {
        // Strip surrounding quotes from JSON string values.
        let s = d.to_string();
        s.trim_matches('"').to_string()
    });

    Schema {
        type_name,
        enum_values,
        default,
    }
}

/// Resolve a `{ "$ref": "#/components/..." }` reference within the same document.
/// Returns `None` if the value is not a `$ref` or the target cannot be found.
fn resolve_ref(value: &Value, root: &Value) -> Option<Value> {
    let ref_str = value.get("$ref")?.as_str()?;

    // Only handle local JSON Pointer references starting with "#/".
    let pointer = ref_str.strip_prefix('#')?;
    root.pointer(pointer).cloned()
}

/// Generate an `operationId` from an HTTP method and path when the spec omits one.
///
/// ```text
/// GET  /pet/{petId}  →  "get-pet-by-pet-id"
/// POST /pet          →  "post-pet"
/// ```
fn generate_operation_id(method: &str, path: &str) -> String {
    let mut parts: Vec<String> = vec![method.to_string()];

    for segment in path.trim_start_matches('/').split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment.starts_with('{') && segment.ends_with('}') {
            let param_name = &segment[1..segment.len() - 1];
            parts.push("by".to_string());
            parts.push(to_kebab_case(param_name));
        } else {
            parts.push(segment.to_string());
        }
    }

    parts.join("-")
}

/// Convert a camelCase / PascalCase / snake_case identifier to `kebab-case`.
///
/// ```text
/// "getPetById"        →  "get-pet-by-id"
/// "findPetsByStatus"  →  "find-pets-by-status"
/// "XMLParser"         →  "xml-parser"
/// "some_var_name"     →  "some-var-name"
/// ```
pub fn to_kebab_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(s.len() + 4);

    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() {
            let prev_lower = i > 0 && chars[i - 1].is_lowercase();
            let next_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();

            if i > 0 && (prev_lower || (next_lower && prev_upper)) {
                result.push('-');
            }
            result.extend(ch.to_lowercase());
        } else if ch == '_' || ch == ' ' {
            result.push('-');
        } else {
            result.push(ch);
        }
    }

    // Collapse consecutive hyphens and trim leading/trailing ones.
    let mut out = String::with_capacity(result.len());
    let mut prev_hyphen = false;
    for ch in result.chars() {
        if ch == '-' {
            if !prev_hyphen {
                out.push(ch);
            }
            prev_hyphen = true;
        } else {
            out.push(ch);
            prev_hyphen = false;
        }
    }
    out.trim_matches('-').to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_camel() {
        assert_eq!(to_kebab_case("getPetById"), "get-pet-by-id");
        assert_eq!(to_kebab_case("findPetsByStatus"), "find-pets-by-status");
        assert_eq!(to_kebab_case("addPet"), "add-pet");
        assert_eq!(to_kebab_case("updatePet"), "update-pet");
        assert_eq!(to_kebab_case("deletePet"), "delete-pet");
    }

    #[test]
    fn kebab_pascal_all_caps() {
        assert_eq!(to_kebab_case("XMLParser"), "xml-parser");
        assert_eq!(to_kebab_case("HTTPSConnection"), "https-connection");
    }

    #[test]
    fn kebab_snake() {
        assert_eq!(to_kebab_case("get_pet_by_id"), "get-pet-by-id");
        assert_eq!(to_kebab_case("some_var"), "some-var");
    }

    #[test]
    fn generate_op_id_no_path_param() {
        assert_eq!(generate_operation_id("get", "/pets"), "get-pets");
        assert_eq!(generate_operation_id("post", "/pet"), "post-pet");
    }

    #[test]
    fn generate_op_id_with_path_param() {
        assert_eq!(
            generate_operation_id("get", "/pet/{petId}"),
            "get-pet-by-pet-id"
        );
        assert_eq!(
            generate_operation_id("delete", "/store/order/{orderId}"),
            "delete-store-order-by-order-id"
        );
    }

    #[test]
    fn parse_swagger_2_minimal() {
        let json = serde_json::json!({
            "swagger": "2.0",
            "info": { "title": "Petstore", "version": "1.0.0" },
            "host": "petstore.swagger.io",
            "basePath": "/v2",
            "schemes": ["https"],
            "paths": {
                "/pet": {
                    "get": {
                        "operationId": "getPets",
                        "summary": "List pets",
                        "parameters": [],
                        "responses": {}
                    },
                    "post": {
                        "operationId": "addPet",
                        "summary": "Add a pet",
                        "parameters": [
                            {
                                "in": "body",
                                "name": "body",
                                "required": true
                            }
                        ],
                        "responses": {}
                    }
                },
                "/pet/{petId}": {
                    "get": {
                        "operationId": "getPetById",
                        "summary": "Find pet by ID",
                        "parameters": [
                            {
                                "in": "path",
                                "name": "petId",
                                "required": true,
                                "type": "integer"
                            }
                        ],
                        "responses": {}
                    }
                }
            }
        });

        let spec = SpecLoader::parse(&json, "test").unwrap();

        assert_eq!(spec.info.title, "Petstore");
        assert_eq!(spec.servers, vec!["https://petstore.swagger.io/v2"]);
        assert_eq!(spec.operations.len(), 3);

        let get_pets = spec.operations.iter().find(|o| o.operation_id == "getPets").unwrap();
        assert_eq!(get_pets.command_name, "get-pets");
        assert_eq!(get_pets.method, HttpMethod::Get);
        assert!(get_pets.request_body.is_none());

        let add_pet = spec.operations.iter().find(|o| o.operation_id == "addPet").unwrap();
        assert!(add_pet.request_body.is_some());
        assert!(add_pet.request_body.as_ref().unwrap().required);

        let get_by_id = spec
            .operations
            .iter()
            .find(|o| o.operation_id == "getPetById")
            .unwrap();
        assert_eq!(get_by_id.command_name, "get-pet-by-id");
        assert_eq!(get_by_id.parameters.len(), 1);
        let param = &get_by_id.parameters[0];
        assert_eq!(param.name, "petId");
        assert_eq!(param.cli_name, "petid");
        assert_eq!(param.location, ParamIn::Path);
        assert!(param.required);
    }

    #[test]
    fn parse_openapi_3_minimal() {
        let json = serde_json::json!({
            "openapi": "3.0.0",
            "info": { "title": "Sample API", "version": "0.1.0" },
            "servers": [{ "url": "https://api.example.com/v1" }],
            "paths": {
                "/items": {
                    "get": {
                        "operationId": "listItems",
                        "summary": "List all items",
                        "parameters": [
                            {
                                "name": "limit",
                                "in": "query",
                                "schema": { "type": "integer" }
                            }
                        ],
                        "responses": {}
                    }
                }
            }
        });

        let spec = SpecLoader::parse(&json, "test").unwrap();
        assert_eq!(spec.info.title, "Sample API");
        assert_eq!(spec.servers, vec!["https://api.example.com/v1"]);
        assert_eq!(spec.operations.len(), 1);

        let op = &spec.operations[0];
        assert_eq!(op.operation_id, "listItems");
        assert_eq!(op.command_name, "list-items");
        assert_eq!(op.parameters[0].location, ParamIn::Query);
        assert_eq!(op.parameters[0].schema.type_name, SchemaType::Integer);
    }

    #[test]
    fn unsupported_version_error() {
        let json = serde_json::json!({ "info": { "title": "Bad", "version": "1" } });
        let err = SpecLoader::parse(&json, "test").unwrap_err();
        assert!(matches!(err, crate::error::Error::UnsupportedVersion(_)));
    }
}
