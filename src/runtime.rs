//! Dynamic CLI runtime.
//!
//! [`DynamicCli`] converts an [`ApiSpec`] into a live, fully-interactive `clap`
//! command tree *at runtime* — no code generation required.
//!
//! # Command hierarchy
//!
//! Operations are grouped by their first OpenAPI tag:
//!
//! ```text
//! pets pet get-pet-by-id --pet-id 1234
//! pets pet find-pets-by-status --status available
//! pets store get-inventory
//! pets user login-user --username john --password secret
//! ```
//!
//! If the spec has only one tag (or no tags at all), the tag level is omitted
//! and operations appear as top-level subcommands.

use std::collections::HashMap;

use clap::{Arg, ArgMatches, Command};
use colored::Colorize;
use indexmap::IndexMap;
use reqwest::blocking::Client;

use crate::error::{Error, Result};
use crate::spec::{ApiSpec, HttpMethod, Operation, ParamIn, Parameter};

// ── Configuration ─────────────────────────────────────────────────────────────

/// Output format for API responses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Pretty-print JSON (default).
    #[default]
    Pretty,
    /// Print the raw response body without any formatting.
    Raw,
}

/// Configuration passed to [`DynamicCli::new`].
#[derive(Debug, Clone)]
pub struct DynamicCliConfig {
    /// URL or file path of the OpenAPI spec.
    pub spec_source: String,
    /// Name used as the root `clap::Command` name (typically the binary name).
    pub app_name: String,
    /// Override the base URL from the spec's `servers` list.
    pub base_url_override: Option<String>,
    /// Bearer token used for `Authorization: Bearer <token>` header.
    pub auth_token: Option<String>,
    /// Default output format; can be overridden with `--output` at runtime.
    pub output_format: OutputFormat,
}

// ── DynamicCli ────────────────────────────────────────────────────────────────

/// Loads an OpenAPI spec and runs it as an interactive CLI.
pub struct DynamicCli {
    config: DynamicCliConfig,
    spec: ApiSpec,
    client: Client,
}

impl DynamicCli {
    /// Load the spec and initialise the HTTP client.
    ///
    /// Prints a one-line status message to stderr.
    pub fn new(config: DynamicCliConfig) -> Result<Self> {
        eprintln!(
            "{} Loading spec from {} …",
            "→".cyan().bold(),
            config.spec_source.cyan()
        );
        let spec = crate::spec::SpecLoader::load(&config.spec_source)?;
        eprintln!(
            "{} {} ({})",
            "✓".green().bold(),
            spec.info.title.bold(),
            spec.info.version.dimmed()
        );

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(Error::Http)?;

        Ok(Self {
            config,
            spec,
            client,
        })
    }

    /// Build the command tree and run against `std::env::args`.
    pub fn run(self) -> Result<()> {
        let cmd = self.build_command();
        let matches = cmd.get_matches();
        self.dispatch(&matches)
    }

    /// Build the command tree and run against a custom argument list.
    ///
    /// The first element of `args` should be the binary / app name (like
    /// `std::env::args` does).
    pub fn run_with_args<I, T>(self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let cmd = self.build_command();
        match cmd.try_get_matches_from(args) {
            Ok(matches) => self.dispatch(&matches),
            Err(e) => {
                e.exit();
            }
        }
    }

    // ── Command tree builder ──────────────────────────────────────────────────

    fn build_command(&self) -> Command {
        let about = self
            .spec
            .info
            .description
            .as_deref()
            .unwrap_or(self.spec.info.title.as_str());

        let mut app = Command::new(self.config.app_name.clone())
            .about(about.to_string())
            .version(self.spec.info.version.clone())
            .arg_required_else_help(true)
            .subcommand_required(true);

        // Group operations by their first tag.
        let mut tagged: IndexMap<String, Vec<&Operation>> = IndexMap::new();
        for op in &self.spec.operations {
            let tag = op
                .tags
                .first()
                .cloned()
                .unwrap_or_else(|| "default".to_string());
            tagged.entry(tag).or_default().push(op);
        }

        let use_tag_groups = tagged.len() > 1
            || tagged
                .keys()
                .next()
                .map(|t| t != "default")
                .unwrap_or(false);

        if use_tag_groups {
            for (tag, ops) in &tagged {
                let mut tag_cmd = Command::new(tag.to_lowercase())
                    .about(format!("Operations tagged '{tag}'"))
                    .arg_required_else_help(true)
                    .subcommand_required(true);

                for op in ops {
                    tag_cmd = tag_cmd.subcommand(Self::build_op_command(op));
                }
                app = app.subcommand(tag_cmd);
            }
        } else {
            for op in self.spec.operations.iter() {
                app = app.subcommand(Self::build_op_command(op));
            }
        }

        // Global flags available on every subcommand.
        app.arg(
            Arg::new("token")
                .long("token")
                .short('t')
                .global(true)
                .env("API_TOKEN")
                .help("Bearer token for authentication"),
        )
        .arg(
            Arg::new("base-url")
                .long("base-url")
                .global(true)
                .help("Override the base URL from the spec"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .global(true)
                .value_parser(["pretty", "raw"])
                .default_value("pretty")
                .help("Response output format"),
        )
    }

    fn build_op_command(op: &Operation) -> Command {
        let about = op
            .summary
            .as_deref()
            .or(op.description.as_deref())
            .unwrap_or("");

        let mut cmd = Command::new(op.command_name.clone())
            .about(format!("[{}] {}", op.method, about));

        for param in &op.parameters {
            cmd = cmd.arg(Self::build_arg(param));
        }

        if let Some(rb) = &op.request_body {
            let desc = rb
                .description
                .as_deref()
                .unwrap_or("Request body as a JSON string or @path/to/file");
            cmd = cmd.arg(
                Arg::new("body")
                    .long("body")
                    .short('d')
                    .help(desc.to_string())
                    .required(rb.required),
            );
        }

        cmd
    }

    fn build_arg(param: &Parameter) -> Arg {
        let help = param
            .description
            .as_deref()
            .unwrap_or(param.name.as_str())
            .to_string();

        let mut arg = Arg::new(param.cli_name.clone())
            .long(param.cli_name.clone())
            .help(help)
            .required(param.required);

        // Annotate enum parameters with their allowed values.
        if let Some(ref values) = param.schema.enum_values {
            let possible: Vec<clap::builder::PossibleValue> = values
                .iter()
                .map(|v| clap::builder::PossibleValue::new(v.clone()))
                .collect();
            arg = arg.value_parser(clap::builder::PossibleValuesParser::new(possible));
        }

        // Show the default value in the help text.
        if let Some(ref default) = param.schema.default {
            arg = arg.default_value(default.clone());
        }

        arg
    }

    // ── Dispatcher ────────────────────────────────────────────────────────────

    fn dispatch(&self, matches: &ArgMatches) -> Result<()> {
        let token = matches
            .get_one::<String>("token")
            .cloned()
            .or_else(|| self.config.auth_token.clone());

        let base_url = matches
            .get_one::<String>("base-url")
            .cloned()
            .or_else(|| self.config.base_url_override.clone())
            .or_else(|| self.spec.servers.first().cloned())
            .unwrap_or_else(|| "https://localhost".to_string());

        let output_fmt = matches
            .get_one::<String>("output")
            .map(|s| {
                if s == "raw" {
                    OutputFormat::Raw
                } else {
                    OutputFormat::Pretty
                }
            })
            .unwrap_or_else(|| self.config.output_format.clone());

        // Determine if tag-grouping is in use.
        let use_tag_groups = self.spec.operations.iter().any(|op| !op.tags.is_empty())
            && {
                let tags: std::collections::HashSet<_> = self
                    .spec
                    .operations
                    .iter()
                    .filter_map(|o| o.tags.first())
                    .collect();
                tags.len() > 1 || tags.iter().next().map(|t| t.as_str() != "default").unwrap_or(false)
            };

        if use_tag_groups {
            for op in &self.spec.operations {
                let tag = op
                    .tags
                    .first()
                    .map(|t| t.to_lowercase())
                    .unwrap_or_else(|| "default".to_string());

                if let Some(tag_m) = matches.subcommand_matches(&tag) {
                    if let Some(op_m) = tag_m.subcommand_matches(&op.command_name) {
                        return self.execute(op, op_m, &base_url, token.as_deref(), &output_fmt);
                    }
                }
            }
        } else {
            for op in &self.spec.operations {
                if let Some(op_m) = matches.subcommand_matches(&op.command_name) {
                    return self.execute(op, op_m, &base_url, token.as_deref(), &output_fmt);
                }
            }
        }

        Ok(())
    }

    // ── HTTP executor ─────────────────────────────────────────────────────────

    fn execute(
        &self,
        op: &Operation,
        matches: &ArgMatches,
        base_url: &str,
        token: Option<&str>,
        fmt: &OutputFormat,
    ) -> Result<()> {
        // Substitute path parameters and collect query parameters.
        let mut path = op.path.clone();
        let mut query: Vec<(String, String)> = Vec::new();
        let mut header_overrides: HashMap<String, String> = HashMap::new();

        for param in &op.parameters {
            if let Some(val) = matches.get_one::<String>(&param.cli_name) {
                match param.location {
                    ParamIn::Path => {
                        path = path.replace(&format!("{{{}}}", param.name), val.as_str());
                    }
                    ParamIn::Query => {
                        query.push((param.name.clone(), val.clone()));
                    }
                    ParamIn::Header => {
                        header_overrides.insert(param.name.clone(), val.clone());
                    }
                    ParamIn::Cookie => {
                        // TODO: cookie support
                    }
                }
            }
        }

        let url = format!(
            "{}/{}",
            base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );

        // Build request.
        let mut request = match op.method {
            HttpMethod::Get => self.client.get(&url),
            HttpMethod::Post => self.client.post(&url),
            HttpMethod::Put => self.client.put(&url),
            HttpMethod::Delete => self.client.delete(&url),
            HttpMethod::Patch => self.client.patch(&url),
            HttpMethod::Head => self.client.head(&url),
            HttpMethod::Options => self.client.request(reqwest::Method::OPTIONS, &url),
        };

        if let Some(t) = token {
            request = request.bearer_auth(t);
        }

        for (k, v) in &header_overrides {
            request = request.header(k.as_str(), v.as_str());
        }

        if !query.is_empty() {
            request = request.query(&query);
        }

        // Request body: raw JSON string or @file path.
        if let Some(body_arg) = matches.get_one::<String>("body") {
            let body = if let Some(file_path) = body_arg.strip_prefix('@') {
                std::fs::read_to_string(file_path)?
            } else {
                body_arg.clone()
            };
            request = request
                .body(body)
                .header("Content-Type", "application/json");
        }

        eprintln!(
            "{} {} {}",
            "→".cyan().bold(),
            op.method.to_string().yellow().bold(),
            url.underline()
        );

        let response = request.send()?;
        let status = response.status();
        let body = response.text()?;

        let status_str = if status.is_success() {
            status.to_string().green().bold()
        } else {
            status.to_string().red().bold()
        };
        eprintln!("{} {}", "←".cyan().bold(), status_str);

        match fmt {
            OutputFormat::Pretty => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    println!("{}", serde_json::to_string_pretty(&json)?);
                } else {
                    println!("{body}");
                }
            }
            OutputFormat::Raw => println!("{body}"),
        }

        if !status.is_success() {
            std::process::exit(1);
        }

        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::ApiInfo;

    fn make_spec(ops: Vec<Operation>) -> ApiSpec {
        ApiSpec {
            info: ApiInfo {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            servers: vec!["https://api.example.com".to_string()],
            operations: ops,
        }
    }

    #[test]
    fn build_command_flat_no_tags() {
        let spec = make_spec(vec![
            Operation {
                operation_id: "listItems".to_string(),
                command_name: "list-items".to_string(),
                method: HttpMethod::Get,
                path: "/items".to_string(),
                summary: Some("List items".to_string()),
                description: None,
                parameters: vec![],
                request_body: None,
                tags: vec![],
            },
        ]);

        let config = DynamicCliConfig {
            spec_source: "test".to_string(),
            app_name: "myapi".to_string(),
            base_url_override: None,
            auth_token: None,
            output_format: OutputFormat::Pretty,
        };

        // Manually build to test structure (DynamicCli::new would try to load the spec).
        let cli = DynamicCliBuilder { config, spec };
        let cmd = cli.build_command();

        // Should have a "list-items" subcommand directly.
        assert!(cmd
            .get_subcommands()
            .any(|s| s.get_name() == "list-items"));
    }

    // Internal helper struct for testing the builder without loading a real spec.
    struct DynamicCliBuilder {
        config: DynamicCliConfig,
        spec: ApiSpec,
    }

    impl DynamicCliBuilder {
        fn build_command(&self) -> Command {
            let cli = DynamicCli {
                config: self.config.clone(),
                spec: self.spec.clone(),
                client: Client::new(),
            };
            cli.build_command()
        }
    }
}
