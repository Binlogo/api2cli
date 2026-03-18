# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `api2cli generate` subcommand: scaffold a standalone Cargo project from any
  OpenAPI spec URL or local file.
- `api2cli run` subcommand: execute a spec as a live CLI without code
  generation — ideal for quick exploration and scripting.
- **Dynamic CLI runtime** (`DynamicCli`): loads an `ApiSpec` at runtime,
  builds a `clap` command tree grouped by OpenAPI tag, and dispatches HTTP
  requests with path substitution, query parameters, and request bodies.
- **OpenAPI spec parser** (`SpecLoader`): supports Swagger 2.0 and OpenAPI
  3.x in both JSON and YAML formats.
- Local `$ref` resolution for parameter references inside `components/`.
- Auto-derived `operationId` when the spec omits one (e.g.
  `GET /pet/{petId}` → `get-pet-by-petid`).
- camelCase → kebab-case conversion for `operationId` and parameter names.
- Global CLI flags: `--token` / `API_TOKEN`, `--base-url`, `--output`.
- `--output raw` flag for pipe-friendly output.
- `--body @file.json` syntax to read request body from a file.
- Coloured stderr status indicators (method, URL, HTTP status code).
- Integration tests with local fixture files (offline, no network required).
- `ProjectGenerator`: generates `Cargo.toml`, `src/main.rs`, and `.gitignore`
  for the scaffolded project.

[Unreleased]: https://github.com/Binlogo/api2cli/compare/HEAD...HEAD
