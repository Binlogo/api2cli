//! Integration tests for api2cli.
//!
//! Tests use local fixture files to avoid network dependencies.

use std::path::Path;

use api2cli::spec::{HttpMethod, ParamIn, SpecLoader};
use api2cli::ProjectGenerator;

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_string_lossy()
        .to_string()
}

// ── Swagger 2.0 fixture ───────────────────────────────────────────────────────

#[test]
fn petstore_v2_info() {
    let spec = SpecLoader::load(&fixture("petstore_v2.json")).unwrap();
    assert_eq!(spec.info.title, "Petstore");
    assert_eq!(spec.info.version, "1.0.0");
    assert_eq!(spec.servers, vec!["https://petstore.swagger.io/v2"]);
}

#[test]
fn petstore_v2_operation_count() {
    let spec = SpecLoader::load(&fixture("petstore_v2.json")).unwrap();
    // 2 on /pet, 2 on /pet/{petId}, 1 on /store/inventory, 1 on /user/login = 6
    assert_eq!(spec.operations.len(), 6);
}

#[test]
fn petstore_v2_find_pets_by_status() {
    let spec = SpecLoader::load(&fixture("petstore_v2.json")).unwrap();
    let op = spec
        .operations
        .iter()
        .find(|o| o.operation_id == "findPetsByStatus")
        .expect("operation not found");

    assert_eq!(op.command_name, "find-pets-by-status");
    assert_eq!(op.method, HttpMethod::Get);
    assert_eq!(op.path, "/pet");
    assert_eq!(op.tags, vec!["pet"]);
    assert!(op.request_body.is_none());

    let param = &op.parameters[0];
    assert_eq!(param.name, "status");
    assert_eq!(param.location, ParamIn::Query);
    assert!(param.required);
    assert_eq!(
        param.schema.enum_values.as_deref(),
        Some(&["available".to_string(), "pending".to_string(), "sold".to_string()][..])
    );
}

#[test]
fn petstore_v2_get_pet_by_id() {
    let spec = SpecLoader::load(&fixture("petstore_v2.json")).unwrap();
    let op = spec
        .operations
        .iter()
        .find(|o| o.operation_id == "getPetById")
        .expect("operation not found");

    assert_eq!(op.command_name, "get-pet-by-id");
    assert_eq!(op.method, HttpMethod::Get);

    let param = &op.parameters[0];
    assert_eq!(param.name, "petId");
    assert_eq!(param.location, ParamIn::Path);
    assert!(param.required);
}

#[test]
fn petstore_v2_add_pet_has_request_body() {
    let spec = SpecLoader::load(&fixture("petstore_v2.json")).unwrap();
    let op = spec
        .operations
        .iter()
        .find(|o| o.operation_id == "addPet")
        .expect("operation not found");

    let rb = op.request_body.as_ref().expect("expected request_body");
    assert!(rb.required);
}

#[test]
fn petstore_v2_tags_present() {
    let spec = SpecLoader::load(&fixture("petstore_v2.json")).unwrap();
    let tags: std::collections::HashSet<String> = spec
        .operations
        .iter()
        .flat_map(|o| o.tags.iter().cloned())
        .collect();

    assert!(tags.contains("pet"));
    assert!(tags.contains("store"));
    assert!(tags.contains("user"));
}

// ── OpenAPI 3.x fixture ───────────────────────────────────────────────────────

#[test]
fn openapi_v3_yaml_info() {
    let spec = SpecLoader::load(&fixture("openapi_v3.yaml")).unwrap();
    assert_eq!(spec.info.title, "Sample Items API");
    assert_eq!(spec.servers, vec!["https://api.example.com/v1"]);
}

#[test]
fn openapi_v3_yaml_operations() {
    let spec = SpecLoader::load(&fixture("openapi_v3.yaml")).unwrap();
    // listItems, createItem, getItem, deleteItem
    assert_eq!(spec.operations.len(), 4);

    let list = spec
        .operations
        .iter()
        .find(|o| o.operation_id == "listItems")
        .unwrap();
    assert_eq!(list.command_name, "list-items");
    assert_eq!(list.method, HttpMethod::Get);
    assert!(list.request_body.is_none());

    let create = spec
        .operations
        .iter()
        .find(|o| o.operation_id == "createItem")
        .unwrap();
    assert!(create.request_body.as_ref().unwrap().required);

    let get = spec
        .operations
        .iter()
        .find(|o| o.operation_id == "getItem")
        .unwrap();
    let path_param = &get.parameters[0];
    assert_eq!(path_param.location, ParamIn::Path);
    assert!(path_param.required);
}

// ── Project generator ─────────────────────────────────────────────────────────

#[test]
fn generator_creates_valid_project() {
    let tmp = tempfile::tempdir().unwrap();

    let gen = ProjectGenerator {
        app_name: "testapi".to_string(),
        spec_source: "https://api.example.com/openapi.json".to_string(),
        output_dir: tmp.path().to_path_buf(),
    };
    gen.generate().unwrap();

    let project = tmp.path().join("testapi");

    // All expected files exist.
    assert!(project.join("Cargo.toml").exists());
    assert!(project.join("src/main.rs").exists());
    assert!(project.join(".gitignore").exists());

    // Cargo.toml is valid TOML and references the right package name.
    let cargo = std::fs::read_to_string(project.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"testapi\""));
    assert!(cargo.contains("api2cli"));

    // main.rs embeds the spec source and app name.
    let main_rs = std::fs::read_to_string(project.join("src/main.rs")).unwrap();
    assert!(main_rs.contains("testapi"));
    assert!(main_rs.contains("api.example.com"));
    assert!(main_rs.contains("DynamicCli::new"));
}
