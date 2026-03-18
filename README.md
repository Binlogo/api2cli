# api2cli

> **Turn any OpenAPI / Swagger specification into a fully-functional CLI with one command.**

[![Crates.io](https://img.shields.io/crates/v/api2cli.svg)](https://crates.io/crates/api2cli)
[![docs.rs](https://img.shields.io/docsrs/api2cli)](https://docs.rs/api2cli)
[![CI](https://github.com/Binlogo/api2cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Binlogo/api2cli/actions)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

```
api2cli generate https://petstore.swagger.io/v2/swagger.json --name pets
cd pets && cargo build --release

./target/release/pets --help
./target/release/pets pet find-pets-by-status --status available
./target/release/pets pet get-pet-by-id --pet-id 1
./target/release/pets store get-inventory
```

Inspired by the [Google Workspace CLI](https://github.com/googleworkspace/cli).

---

## How it works

```
OpenAPI spec (URL or file)
        │
        ▼
  ┌─────────────┐       api2cli generate        ┌──────────────────────┐
  │  SpecLoader │ ─────────────────────────────▶ │  Cargo project       │
  │  (v2 + v3)  │                                │  src/main.rs embeds  │
  └──────┬──────┘                                │  spec URL at compile │
         │                                       │  time; uses api2cli  │
         │  parse                                │  lib at runtime      │
         ▼                                       └──────────────────────┘
  ┌─────────────┐       api2cli run
  │   ApiSpec   │ ──────────────────────────────▶ live interactive CLI
  └──────┬──────┘
         │
         │  build_command()
         ▼
  ┌──────────────────┐
  │  clap Command    │  tag ▸ operation ▸ flags
  │  tree (dynamic)  │
  └──────┬───────────┘
         │  dispatch()
         ▼
  ┌──────────────┐
  │  HTTP client │ ──── path params, query params, body ──▶ API
  └──────────────┘
```

At runtime, the generated binary (or `api2cli run`) reads the OpenAPI spec,
assembles a `clap` command tree grouped by **tag**, and dispatches the matched
operation as an HTTP request — all without any static code generation.

---

## Installation

### From source (requires Rust ≥ 1.75)

```bash
cargo install --git https://github.com/Binlogo/api2cli
```

### From crates.io (once published)

```bash
cargo install api2cli
```

---

## Usage

### `api2cli generate` — scaffold a project

```
api2cli generate <SPEC> --name <NAME> [--output <DIR>]
```

| Argument | Description |
|----------|-------------|
| `SPEC`   | OpenAPI spec URL or local file path (JSON or YAML) |
| `--name` | Name of the generated binary / Cargo package |
| `--output` | Parent directory for the new project (default: `.`) |

**Example — Petstore:**

```bash
api2cli generate https://petstore.swagger.io/v2/swagger.json --name pets
cd pets
cargo build --release

# Use the generated CLI
./target/release/pets --help
./target/release/pets pet --help
./target/release/pets pet find-pets-by-status --status available
./target/release/pets pet add-pet --body '{"name":"Buddy","photoUrls":["https://example.com/buddy.jpg"],"status":"available"}'
./target/release/pets pet get-pet-by-id --pet-id 1
./target/release/pets store get-inventory
./target/release/pets user login-user --username user1 --password 12345
```

### `api2cli run` — execute a spec directly

For quick exploration without building a binary:

```bash
api2cli run https://petstore.swagger.io/v2/swagger.json -- pet get-pet-by-id --pet-id 1
```

Everything after `--` is forwarded to the dynamic CLI.

---

## Generated CLI features

| Feature | Details |
|---------|---------|
| **Tag grouping** | Operations are organised under tag-based subcommands |
| **Path parameters** | Auto-detected, required flags |
| **Query parameters** | Optional `--flag value` flags with enum validation |
| **Request body** | `--body '<json>'` or `--body @file.json` |
| **Auth** | `--token <bearer>` or `API_TOKEN` env var |
| **Base URL override** | `--base-url https://staging.api.example.com` |
| **Pretty output** | JSON responses are pretty-printed by default |
| **Raw output** | `--output raw` for piping to `jq` or other tools |

---

## Library usage

Add `api2cli` as a dependency to embed a live API CLI in your own binary:

```toml
[dependencies]
api2cli = { git = "https://github.com/Binlogo/api2cli" }
anyhow = "1.0"
```

```rust
use api2cli::{DynamicCli, DynamicCliConfig, OutputFormat};

fn main() -> anyhow::Result<()> {
    DynamicCli::new(DynamicCliConfig {
        spec_source: "https://api.example.com/openapi.json".to_string(),
        app_name: "myapi".to_string(),
        base_url_override: None,
        auth_token: None,
        output_format: OutputFormat::Pretty,
    })?
    .run()?;
    Ok(())
}
```

Or drive it programmatically with custom args:

```rust
cli.run_with_args(["myapi", "items", "list-items", "--limit", "5"])?;
```

---

## Spec support

| Feature | Status |
|---------|--------|
| Swagger 2.0 (JSON) | ✅ |
| Swagger 2.0 (YAML) | ✅ |
| OpenAPI 3.0 (JSON) | ✅ |
| OpenAPI 3.0 (YAML) | ✅ |
| OpenAPI 3.1 | ✅ |
| Local `$ref` resolution | ✅ |
| External `$ref` (URL) | 🚧 planned |
| Multipart / file upload | 🚧 planned |
| OAuth2 / OIDC flows | 🚧 planned |

---

## Project structure

```
api2cli/
├── src/
│   ├── lib.rs          # Public API surface
│   ├── main.rs         # `api2cli` binary (generate + run)
│   ├── error.rs        # Error types
│   ├── spec.rs         # OpenAPI v2/v3 parser
│   ├── runtime.rs      # Dynamic CLI builder & HTTP executor
│   └── generator.rs    # Cargo project scaffolder
├── tests/
│   ├── fixtures/       # Local spec files for offline tests
│   └── integration_test.rs
├── Cargo.toml
└── README.md
```

---

## Contributing

Contributions are welcome! Please open an issue or pull request.

1. Fork the repo and create a feature branch.
2. Run `cargo test` — all tests must pass.
3. Run `cargo clippy -- -D warnings` and `cargo fmt`.
4. Open a PR with a clear description.

---

## Related projects

| Project | Language | Notes |
|---------|----------|-------|
| [restish](https://github.com/danielgtaylor/restish) | Go | Interactive REST CLI |
| [swagger2cli](https://github.com/safe-waters/swagger2cli) | Go | Shell script generation |
| [openapi-generator](https://openapi-generator.tech) | Java | Full client/server codegen |
| [Google Workspace CLI](https://github.com/googleworkspace/cli) | Go | Inspiration for this project |

---

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
