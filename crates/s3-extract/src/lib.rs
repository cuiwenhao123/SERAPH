#![forbid(unsafe_code)]

//! Minimal Phase 1 extraction bootstrap for SERAPH.

use seraph_types::{
    ApiId, ApiInfo, ApiKind, BorrowedReturnFact, CodeRef, CrateMeta, DocSections, EnumVariantInfo,
    ExampleAnchor, ExampleId, ExampleInfo, ExplicitPanicSiteFact, ExternAbiApiFact, Knowledge,
    ModuleId, ModuleInfo, ReprKind, ReturnShape, ReturnShapeKind, RiskFacts, RiskOwner, SymbolId,
    SymbolInfo, SymbolKind, TraitAssociatedConstBinding, TraitAssociatedConstDef,
    TraitAssociatedTypeBinding, TraitAssociatedTypeDef, TraitExposureKind, TraitId, TraitImplId,
    TraitImplInfo, TraitInfo, TraitOrigin, TypeFieldInfo, TypeId, TypeInfo, TypeKind,
    TypeLayoutFact, VariantKind,
};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_RUSTDOC_TOOLCHAIN: &str = "nightly-2025-01-26";

#[derive(Debug)]
pub enum ExtractError {
    CargoMetadataLaunch(std::io::Error),
    CargoMetadataFailed(String),
    CargoMetadataJson(serde_json::Error),
    MissingPackage,
    MissingLibTarget,
    WorkspaceFallbackIo(std::io::Error),
    RustdocLaunch(std::io::Error),
    RustdocFailed(String),
    RustdocOutputRead(std::io::Error),
    RustdocJson(serde_json::Error),
    MissingRustdocRoot,
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CargoMetadataLaunch(err) => write!(f, "failed to launch cargo metadata: {err}"),
            Self::CargoMetadataFailed(stderr) => {
                write!(f, "cargo metadata exited with error: {stderr}")
            }
            Self::CargoMetadataJson(err) => {
                write!(f, "failed to parse cargo metadata JSON: {err}")
            }
            Self::MissingPackage => f.write_str("cargo metadata did not return a package"),
            Self::MissingLibTarget => f.write_str("cargo metadata did not return a library target"),
            Self::WorkspaceFallbackIo(err) => {
                write!(f, "failed to prepare workspace fallback copy: {err}")
            }
            Self::RustdocLaunch(err) => write!(f, "failed to launch cargo rustdoc: {err}"),
            Self::RustdocFailed(stderr) => write!(f, "cargo rustdoc exited with error: {stderr}"),
            Self::RustdocOutputRead(err) => write!(f, "failed to read rustdoc JSON output: {err}"),
            Self::RustdocJson(err) => write!(f, "failed to parse rustdoc JSON: {err}"),
            Self::MissingRustdocRoot => {
                f.write_str("rustdoc JSON did not contain a crate root item")
            }
        }
    }
}

impl std::error::Error for ExtractError {}

#[derive(Debug, Deserialize)]
struct MetadataResponse {
    workspace_root: String,
    packages: Vec<MetadataPackage>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    name: String,
    version: String,
    edition: String,
    rust_version: Option<String>,
    repository: Option<String>,
    description: Option<String>,
    manifest_path: String,
    features: BTreeMap<String, Vec<String>>,
    targets: Vec<MetadataTarget>,
}

#[derive(Debug, Deserialize)]
struct MetadataTarget {
    name: String,
    kind: Vec<String>,
    src_path: String,
}

#[derive(Debug, Deserialize)]
struct RustdocResponse {
    root: u64,
    index: BTreeMap<String, RustdocItem>,
    paths: BTreeMap<String, RustdocPathEntry>,
}

#[derive(Debug, Deserialize)]
struct RustdocItem {
    name: Option<String>,
    span: Option<RustdocSpan>,
    visibility: String,
    docs: Option<String>,
    #[serde(default, deserialize_with = "deserialize_rustdoc_attrs")]
    attrs: Vec<String>,
    inner: Value,
}

#[derive(Debug, Deserialize)]
struct RustdocPathEntry {
    crate_id: u32,
    path: Vec<String>,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct RustdocSpan {
    filename: String,
    begin: [u32; 2],
    end: [u32; 2],
}

fn deserialize_rustdoc_attrs<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Vec::<Value>::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .filter_map(|value| rustdoc_attr_to_string(&value))
        .collect())
}

fn rustdoc_attr_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(object) => object
            .get("raw")
            .and_then(Value::as_str)
            .or_else(|| object.get("other").and_then(Value::as_str))
            .map(ToOwned::to_owned),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct PublicPathState {
    public_paths: BTreeMap<String, BTreeSet<String>>,
    reexport_paths: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug)]
struct RustdocFacts {
    root_docs: String,
    modules: Vec<ModuleInfo>,
    types: Vec<TypeInfo>,
    apis: Vec<ApiInfo>,
    symbols: Vec<SymbolInfo>,
    trait_registry: Vec<TraitInfo>,
    trait_impl_registry: Vec<TraitImplInfo>,
    examples: Vec<ExampleInfo>,
    risk_facts: RiskFacts,
}

#[derive(Debug, Clone)]
struct TypeContext {
    item_id: String,
    type_id: TypeId,
    canonical_path: String,
    public_paths: Vec<String>,
    public_anchor_module_id: ModuleId,
}

#[derive(Debug, Clone)]
struct TraitRefInfo {
    item_id: Option<String>,
    canonical_path: String,
    name: String,
}

#[derive(Debug)]
struct ManifestExecution {
    input_crate_dir: PathBuf,
    exec_crate_dir: PathBuf,
    exec_manifest_path: PathBuf,
    _temp_copy: Option<TempManifestCopy>,
}

#[derive(Debug)]
struct TempManifestCopy {
    root: PathBuf,
}

impl Drop for TempManifestCopy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl ManifestExecution {
    fn direct(manifest_path: &Path) -> Self {
        let crate_dir = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Self {
            input_crate_dir: crate_dir.clone(),
            exec_crate_dir: crate_dir,
            exec_manifest_path: manifest_path.to_path_buf(),
            _temp_copy: None,
        }
    }

    fn detached_copy(manifest_path: &Path) -> Result<Self, ExtractError> {
        let input_crate_dir = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let temp_root = create_workspace_fallback_dir()?;
        copy_dir_all_for_workspace_fallback(&input_crate_dir, &temp_root)?;

        Ok(Self {
            input_crate_dir,
            exec_crate_dir: temp_root.clone(),
            exec_manifest_path: temp_root.join(
                manifest_path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("Cargo.toml")),
            ),
            _temp_copy: Some(TempManifestCopy { root: temp_root }),
        })
    }

    fn rewrite_path(&self, path: &str) -> String {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            if let Ok(relative) = candidate.strip_prefix(&self.exec_crate_dir) {
                return self
                    .input_crate_dir
                    .join(relative)
                    .to_string_lossy()
                    .into_owned();
            }
        }
        path.to_owned()
    }

    fn uses_detached_copy(&self) -> bool {
        self._temp_copy.is_some()
    }
}

pub fn normalize_import_name(crate_name: &str) -> String {
    crate_name.replace('-', "_")
}

pub fn build_minimal_knowledge(crate_name: &str) -> Knowledge {
    let import_name = normalize_import_name(crate_name);

    Knowledge {
        crate_meta: CrateMeta {
            package_name: crate_name.to_owned(),
            lib_target_name: import_name.clone(),
            crate_import_name: import_name,
            version: String::new(),
            edition: String::new(),
            rust_version: None,
            repository: None,
            manifest_path: String::new(),
            lib_rs_path: String::new(),
            default_features: Vec::new(),
            cargo_description: None,
            root_docs: String::new(),
            root_doc_sections: DocSections::default(),
        },
        modules: Vec::new(),
        types: Vec::new(),
        apis: Vec::new(),
        symbols: Vec::new(),
        trait_registry: Vec::new(),
        trait_impl_registry: Vec::new(),
        examples: Vec::new(),
        risk_facts: RiskFacts::default(),
    }
}

pub fn extract_knowledge_from_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<Knowledge, ExtractError> {
    let manifest_path = manifest_path.as_ref();
    let manifest_path_text = manifest_path.to_string_lossy().into_owned();

    let (metadata, manifest_execution) = load_metadata_with_fallback(manifest_path)?;
    let workspace_root = PathBuf::from(&metadata.workspace_root);

    let mut packages = metadata.packages;
    if packages.is_empty() {
        return Err(ExtractError::MissingPackage);
    }
    // Workspace metadata can list the root package first, so pick the package
    // whose manifest exactly matches the crate we were asked to extract.
    let package_index = packages
        .iter()
        .position(|package| {
            Path::new(&package.manifest_path) == manifest_execution.exec_manifest_path
        })
        .unwrap_or(0);
    let package = packages.swap_remove(package_index);
    let lib_target = package
        .targets
        .iter()
        .find(|target| target.kind.iter().any(|kind| kind == "lib"))
        .ok_or(ExtractError::MissingLibTarget)?;

    let mut knowledge = Knowledge {
        crate_meta: CrateMeta {
            package_name: package.name,
            lib_target_name: lib_target.name.clone(),
            crate_import_name: lib_target.name.clone(),
            version: package.version,
            edition: package.edition,
            rust_version: package.rust_version,
            repository: package.repository,
            manifest_path: manifest_path_text,
            lib_rs_path: manifest_execution.rewrite_path(&lib_target.src_path),
            default_features: package.features.get("default").cloned().unwrap_or_default(),
            cargo_description: package.description,
            root_docs: String::new(),
            root_doc_sections: DocSections::default(),
        },
        modules: Vec::new(),
        types: Vec::new(),
        apis: Vec::new(),
        symbols: Vec::new(),
        trait_registry: Vec::new(),
        trait_impl_registry: Vec::new(),
        examples: Vec::new(),
        risk_facts: RiskFacts::default(),
    };

    let rustdoc = load_rustdoc_json(
        &manifest_execution.exec_manifest_path,
        &knowledge.crate_meta.lib_target_name,
        &workspace_root,
        manifest_execution.uses_detached_copy(),
    )?;
    let facts = extract_rustdoc_facts(&rustdoc, &knowledge.crate_meta.crate_import_name)?;

    knowledge.crate_meta.root_docs = facts.root_docs;
    knowledge.crate_meta.root_doc_sections =
        doc_sections_from_docs(&knowledge.crate_meta.root_docs);
    knowledge.modules = facts.modules;
    knowledge.types = facts.types;
    knowledge.apis = facts.apis;
    knowledge.symbols = facts.symbols;
    knowledge.trait_registry = facts.trait_registry;
    knowledge.trait_impl_registry = facts.trait_impl_registry;
    knowledge.examples = facts.examples;
    knowledge.risk_facts = facts.risk_facts;
    rewrite_knowledge_paths(&mut knowledge, &manifest_execution);
    backfill_api_unsafe_block_presence(&mut knowledge, &manifest_execution.input_crate_dir);
    backfill_trait_impl_unsafety_from_source(&mut knowledge, &manifest_execution.input_crate_dir);
    backfill_explicit_panic_sites(&mut knowledge, &manifest_execution.input_crate_dir);

    Ok(knowledge)
}

pub fn write_knowledge_json(
    path: impl AsRef<Path>,
    knowledge: &Knowledge,
) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(knowledge)
        .expect("serializing Knowledge to JSON should not fail");
    fs::write(path, json)?;
    Ok(())
}

fn load_metadata_with_fallback(
    manifest_path: &Path,
) -> Result<(MetadataResponse, ManifestExecution), ExtractError> {
    match run_cargo_metadata(manifest_path, false) {
        Ok(metadata) => Ok((metadata, ManifestExecution::direct(manifest_path))),
        Err(ExtractError::CargoMetadataFailed(stderr)) if is_workspace_mismatch_error(&stderr) => {
            let manifest_execution = ManifestExecution::detached_copy(manifest_path)?;
            let metadata = run_cargo_metadata(&manifest_execution.exec_manifest_path, true)?;
            Ok((metadata, manifest_execution))
        }
        Err(err) => Err(err),
    }
}

fn run_cargo_metadata(
    manifest_path: &Path,
    offline: bool,
) -> Result<MetadataResponse, ExtractError> {
    let manifest_path_text = manifest_path.to_string_lossy().into_owned();
    let mut command = Command::new("cargo");
    command.args([
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--manifest-path",
        &manifest_path_text,
    ]);
    if offline {
        command.arg("--offline");
    }
    let output = command
        .output()
        .map_err(ExtractError::CargoMetadataLaunch)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stderr = if stderr.is_empty() {
            format!("exit status {}", output.status)
        } else {
            stderr
        };
        return Err(ExtractError::CargoMetadataFailed(stderr));
    }

    serde_json::from_slice(&output.stdout).map_err(ExtractError::CargoMetadataJson)
}

fn is_workspace_mismatch_error(stderr: &str) -> bool {
    stderr.contains("current package believes it's in a workspace when it's not")
}

fn create_workspace_fallback_dir() -> Result<PathBuf, ExtractError> {
    let base = std::env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    for attempt in 0..32_u32 {
        let candidate = base.join(format!(
            "seraph-workspace-fallback-{}-{timestamp}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(ExtractError::WorkspaceFallbackIo(err)),
        }
    }

    Err(ExtractError::WorkspaceFallbackIo(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "unable to allocate unique workspace fallback directory",
    )))
}

fn copy_dir_all_for_workspace_fallback(src: &Path, dst: &Path) -> Result<(), ExtractError> {
    for entry in fs::read_dir(src).map_err(ExtractError::WorkspaceFallbackIo)? {
        let entry = entry.map_err(ExtractError::WorkspaceFallbackIo)?;
        let entry_type = entry
            .file_type()
            .map_err(ExtractError::WorkspaceFallbackIo)?;
        let entry_name = entry.file_name();
        if matches!(entry_name.to_str(), Some("target" | ".git")) {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&entry_name);
        if entry_type.is_dir() {
            fs::create_dir(&dst_path).map_err(ExtractError::WorkspaceFallbackIo)?;
            copy_dir_all_for_workspace_fallback(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(ExtractError::WorkspaceFallbackIo)?;
        }
    }

    Ok(())
}

fn load_rustdoc_json(
    manifest_path: &Path,
    lib_target_name: &str,
    workspace_root: &Path,
    offline: bool,
) -> Result<RustdocResponse, ExtractError> {
    let manifest_path_text = manifest_path.to_string_lossy().into_owned();
    let toolchain = std::env::var("SERAPH_RUSTDOC_TOOLCHAIN")
        .unwrap_or_else(|_| DEFAULT_RUSTDOC_TOOLCHAIN.to_owned());

    let mut command = Command::new("cargo");
    command
        .arg(format!("+{toolchain}"))
        .arg("rustdoc")
        .arg("--lib")
        .arg("--manifest-path")
        .arg(&manifest_path_text);
    if offline {
        command.arg("--offline");
    }
    let output = command
        .arg("--")
        .arg("-Z")
        .arg("unstable-options")
        .arg("--output-format")
        .arg("json")
        .output()
        .map_err(ExtractError::RustdocLaunch)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stderr = if stderr.is_empty() {
            format!("exit status {}", output.status)
        } else {
            stderr
        };
        return Err(ExtractError::RustdocFailed(stderr));
    }

    // For standalone crates rustdoc usually lands in `<crate>/target/doc`, but
    // workspace members may emit into the workspace root target directory.
    let rustdoc_paths = rustdoc_output_candidates(manifest_path, lib_target_name, workspace_root);
    let raw = read_first_existing_rustdoc_output(&rustdoc_paths)?;
    serde_json::from_str(&raw).map_err(ExtractError::RustdocJson)
}

fn rustdoc_output_candidates(
    manifest_path: &Path,
    lib_target_name: &str,
    workspace_root: &Path,
) -> Vec<PathBuf> {
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let filename = format!("{lib_target_name}.json");
    let mut candidates = vec![manifest_dir.join("target").join("doc").join(&filename)];
    let workspace_candidate = workspace_root.join("target").join("doc").join(filename);
    if workspace_candidate != candidates[0] {
        candidates.push(workspace_candidate);
    }
    candidates
}

fn read_first_existing_rustdoc_output(paths: &[PathBuf]) -> Result<String, ExtractError> {
    let mut first_error = None;

    for path in paths {
        match fs::read_to_string(path) {
            Ok(raw) => return Ok(raw),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                // Keep trying the fallback locations before surfacing a missing-file error.
                if first_error.is_none() {
                    first_error = Some(err);
                }
            }
            Err(err) => return Err(ExtractError::RustdocOutputRead(err)),
        }
    }

    Err(ExtractError::RustdocOutputRead(first_error.unwrap_or_else(
        || {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "rustdoc JSON output was not found in any expected target directory",
            )
        },
    )))
}

fn extract_rustdoc_facts(
    rustdoc: &RustdocResponse,
    crate_import_name: &str,
) -> Result<RustdocFacts, ExtractError> {
    let root_id = rustdoc.root.to_string();
    let root_item = rustdoc
        .index
        .get(&root_id)
        .ok_or(ExtractError::MissingRustdocRoot)?;
    let root_docs = root_item.docs.clone().unwrap_or_default();

    let public_paths = collect_public_paths(rustdoc, &root_id, crate_import_name);
    let (modules, module_ids) = build_modules(rustdoc, &root_id, crate_import_name, &public_paths)?;
    let (types, type_contexts) = build_types(rustdoc, &module_ids, &public_paths);
    let (apis, api_item_ids) = build_apis(rustdoc, &module_ids, &public_paths, &type_contexts);
    let (symbols, symbol_item_ids) =
        build_symbols(rustdoc, &module_ids, &public_paths, &type_contexts);
    let trait_registry = build_trait_registry(
        rustdoc,
        &module_ids,
        &public_paths,
        &type_contexts,
        &api_item_ids,
    );
    let trait_impl_registry =
        build_trait_impl_registry(rustdoc, &module_ids, &public_paths, &type_contexts);
    let examples = build_examples(
        rustdoc,
        &module_ids,
        &type_contexts,
        &apis,
        &api_item_ids,
        &symbols,
        &symbol_item_ids,
    );
    let risk_facts = build_risk_facts(
        rustdoc,
        &type_contexts,
        &api_item_ids,
        &trait_impl_registry,
        &apis,
    );

    Ok(RustdocFacts {
        root_docs,
        modules,
        types,
        apis,
        symbols,
        trait_registry,
        trait_impl_registry,
        examples,
        risk_facts,
    })
}

fn collect_public_paths(
    rustdoc: &RustdocResponse,
    root_id: &str,
    crate_import_name: &str,
) -> PublicPathState {
    let mut state = PublicPathState::default();
    let mut visited = BTreeSet::new();
    walk_public_module(
        rustdoc,
        root_id,
        crate_import_name,
        &mut state,
        &mut visited,
    );
    state
}

fn walk_public_module(
    rustdoc: &RustdocResponse,
    module_item_id: &str,
    current_public_module_path: &str,
    state: &mut PublicPathState,
    visited: &mut BTreeSet<(String, String)>,
) {
    let visit_key = (
        module_item_id.to_owned(),
        current_public_module_path.to_owned(),
    );
    if !visited.insert(visit_key) {
        return;
    }

    let Some(module_item) = rustdoc.index.get(module_item_id) else {
        return;
    };
    let Some(items) = module_item_ids(module_item) else {
        return;
    };

    for child_id in items {
        let Some(child_item) = rustdoc.index.get(&child_id) else {
            continue;
        };
        if !is_public(child_item) {
            continue;
        }

        if is_module_item(child_item) {
            if let Some(name) = child_item.name.as_deref() {
                let path = join_public_path(current_public_module_path, name);
                state
                    .public_paths
                    .entry(child_id.clone())
                    .or_default()
                    .insert(path.clone());
                walk_public_module(rustdoc, &child_id, &path, state, visited);
            }
            continue;
        }

        if let Some(use_item) = use_value(child_item) {
            let Some(target_id) = use_item.get("id").and_then(value_id_to_string) else {
                continue;
            };
            let is_glob = use_item
                .get("is_glob")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            if is_glob {
                if let Some(target_item) = rustdoc.index.get(&target_id) {
                    if is_module_item(target_item) {
                        walk_public_module(
                            rustdoc,
                            &target_id,
                            current_public_module_path,
                            state,
                            visited,
                        );
                    }
                }
                continue;
            }

            let Some(name) = use_item.get("name").and_then(Value::as_str) else {
                continue;
            };
            let path = join_public_path(current_public_module_path, name);
            state
                .public_paths
                .entry(target_id.clone())
                .or_default()
                .insert(path.clone());
            state
                .reexport_paths
                .entry(target_id)
                .or_default()
                .insert(path);
            continue;
        }

        if let Some(name) = child_item.name.as_deref() {
            let path = join_public_path(current_public_module_path, name);
            state.public_paths.entry(child_id).or_default().insert(path);
        }
    }
}

fn build_modules(
    rustdoc: &RustdocResponse,
    root_id: &str,
    crate_import_name: &str,
    public_paths: &PublicPathState,
) -> Result<(Vec<ModuleInfo>, BTreeMap<String, ModuleId>), ExtractError> {
    let root_item = rustdoc
        .index
        .get(root_id)
        .ok_or(ExtractError::MissingRustdocRoot)?;
    let mut module_rows = Vec::new();
    module_rows.push(ModuleRow {
        canonical_path: crate_import_name.to_owned(),
        name: crate_import_name.to_owned(),
        public_paths: vec![crate_import_name.to_owned()],
        code_ref: code_ref_from_span(root_item.span.as_ref()),
        docs: root_item.docs.clone().unwrap_or_default(),
    });

    for (item_id, path_entry) in &rustdoc.paths {
        if path_entry.crate_id != 0 || path_entry.kind != "module" {
            continue;
        }

        let canonical_path = join_path_segments(&path_entry.path);
        if canonical_path == crate_import_name {
            continue;
        }

        let Some(item) = rustdoc.index.get(item_id) else {
            continue;
        };

        module_rows.push(ModuleRow {
            canonical_path,
            name: item
                .name
                .clone()
                .unwrap_or_else(|| path_entry.path.last().cloned().unwrap_or_default()),
            public_paths: sorted_paths(public_paths.public_paths.get(item_id)),
            code_ref: code_ref_from_span(item.span.as_ref()),
            docs: item.docs.clone().unwrap_or_default(),
        });
    }

    module_rows.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));

    let mut module_ids = BTreeMap::new();
    for row in &module_rows {
        module_ids.insert(
            row.canonical_path.clone(),
            module_id_for_path(&row.canonical_path),
        );
    }

    let mut modules = Vec::new();
    for row in module_rows {
        let parent_module_id = parent_path(&row.canonical_path)
            .and_then(|parent_path| module_ids.get(&parent_path).cloned());
        let doc_sections = doc_sections_from_docs(&row.docs);

        modules.push(ModuleInfo {
            module_id: module_ids
                .get(&row.canonical_path)
                .cloned()
                .expect("module id should exist"),
            name: row.name,
            canonical_path: row.canonical_path,
            public_paths: row.public_paths,
            parent_module_id,
            code_ref: row.code_ref,
            docs: row.docs,
            doc_sections,
        });
    }

    Ok((modules, module_ids))
}

fn build_types(
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
) -> (Vec<TypeInfo>, BTreeMap<String, TypeContext>) {
    let mut rows = Vec::new();

    for (item_id, path_entry) in &rustdoc.paths {
        if path_entry.crate_id != 0 {
            continue;
        }

        let Some(type_kind) = type_kind_from_path_kind(&path_entry.kind) else {
            continue;
        };
        let Some(item) = rustdoc.index.get(item_id) else {
            continue;
        };
        if !is_public(item) {
            continue;
        }

        let public_paths = sorted_paths(public_paths.public_paths.get(item_id));
        if public_paths.is_empty() {
            continue;
        }

        let canonical_path = join_path_segments(&path_entry.path);
        let Some(public_anchor_module_id) =
            public_anchor_module_id(module_ids, &canonical_path, &public_paths)
        else {
            continue;
        };

        let generics_value = type_generics_value(item);
        let (fields, variants, has_hidden_fields, has_hidden_variants) =
            type_surface_from_item(item, rustdoc);
        rows.push(TypeRow {
            item_id: item_id.clone(),
            type_id: type_id_for_path(&canonical_path),
            name: item
                .name
                .clone()
                .unwrap_or_else(|| path_entry.path.last().cloned().unwrap_or_default()),
            canonical_path,
            public_paths,
            public_anchor_module_id,
            code_ref: code_ref_from_span(item.span.as_ref()),
            docs: item.docs.clone().unwrap_or_default(),
            kind: type_kind,
            generic_params: generic_param_names(generics_value),
            where_clauses: where_clause_strings(generics_value),
            is_non_exhaustive: has_attr(&item.attrs, "non_exhaustive"),
            fields,
            variants,
            has_hidden_fields,
            has_hidden_variants,
        });
    }

    rows.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));

    let mut types = Vec::new();
    let mut type_contexts = BTreeMap::new();
    for row in rows {
        let type_context = TypeContext {
            item_id: row.item_id.clone(),
            type_id: row.type_id.clone(),
            canonical_path: row.canonical_path.clone(),
            public_paths: row.public_paths.clone(),
            public_anchor_module_id: row.public_anchor_module_id.clone(),
        };
        type_contexts.insert(row.item_id.clone(), type_context);
        let doc_sections = doc_sections_from_docs(&row.docs);

        types.push(TypeInfo {
            type_id: row.type_id,
            name: row.name,
            canonical_path: row.canonical_path,
            public_paths: row.public_paths,
            public_anchor_module_id: row.public_anchor_module_id,
            code_ref: row.code_ref,
            docs: row.docs,
            doc_sections,
            kind: row.kind,
            generic_params: row.generic_params,
            where_clauses: row.where_clauses,
            is_non_exhaustive: row.is_non_exhaustive,
            fields: row.fields,
            variants: row.variants,
            has_hidden_fields: row.has_hidden_fields,
            has_hidden_variants: row.has_hidden_variants,
        });
    }

    (types, type_contexts)
}

fn build_apis(
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    type_contexts: &BTreeMap<String, TypeContext>,
) -> (Vec<ApiInfo>, BTreeMap<String, ApiId>) {
    let mut apis = Vec::new();
    let mut api_item_ids = BTreeMap::new();
    let mut seen_api_paths = BTreeSet::new();
    let mut method_item_ids = BTreeSet::new();

    for type_context in type_contexts.values() {
        let Some(type_item) = rustdoc.index.get(&type_context.item_id) else {
            continue;
        };

        for impl_id in type_impl_ids(type_item) {
            let Some(impl_item) = rustdoc.index.get(&impl_id) else {
                continue;
            };
            let Some(impl_value) = impl_value(impl_item) else {
                continue;
            };
            if impl_value
                .get("trait")
                .is_some_and(|value| !value.is_null())
            {
                continue;
            }

            let Some(items) = impl_value.get("items").and_then(Value::as_array) else {
                continue;
            };

            for item_id_value in items {
                let Some(item_id) = value_id_to_string(item_id_value) else {
                    continue;
                };
                let Some(api_item) = rustdoc.index.get(&item_id) else {
                    continue;
                };
                if !is_public(api_item) || !is_function_item(api_item) {
                    continue;
                }
                method_item_ids.insert(item_id.clone());

                let Some(name) = api_item.name.clone() else {
                    continue;
                };
                let canonical_path = format!("{}::{name}", type_context.canonical_path);
                if !seen_api_paths.insert(canonical_path.clone()) {
                    continue;
                }

                let api_kind = classify_inherent_api_kind(api_item, type_context);
                let public_paths = type_context
                    .public_paths
                    .iter()
                    .map(|path| format!("{path}::{name}"))
                    .collect::<Vec<_>>();
                let Some(api) = build_api_info(
                    api_item,
                    &canonical_path,
                    public_paths,
                    type_context.public_anchor_module_id.clone(),
                    Some(type_context.type_id.clone()),
                    None,
                    api_kind,
                ) else {
                    continue;
                };
                api_item_ids.insert(item_id.clone(), api.api_id.clone());
                apis.push(api);
            }
        }
    }

    for (item_id, path_entry) in &rustdoc.paths {
        if path_entry.crate_id != 0 || path_entry.kind != "trait" {
            continue;
        }
        let Some(trait_item) = rustdoc.index.get(item_id) else {
            continue;
        };
        if !is_public(trait_item) || !is_trait_item(trait_item) {
            continue;
        }
        let owner_public_paths = sorted_paths(public_paths.public_paths.get(item_id));
        if owner_public_paths.is_empty() {
            continue;
        }

        let canonical_trait_path = join_path_segments(&path_entry.path);
        let Some(public_anchor_module_id) =
            public_anchor_module_id(module_ids, &canonical_trait_path, &owner_public_paths)
        else {
            continue;
        };
        let owner_trait_id = trait_id_for_path(&canonical_trait_path);
        let Some(trait_items) = trait_value(trait_item)
            .and_then(|value| value.get("items"))
            .and_then(Value::as_array)
        else {
            continue;
        };

        for trait_item_id_value in trait_items {
            let Some(trait_item_id) = value_id_to_string(trait_item_id_value) else {
                continue;
            };
            let Some(api_item) = rustdoc.index.get(&trait_item_id) else {
                continue;
            };
            if !is_function_item(api_item) {
                continue;
            }

            let Some(name) = api_item.name.clone() else {
                continue;
            };
            let canonical_path = format!("{canonical_trait_path}::{name}");
            if !seen_api_paths.insert(canonical_path.clone()) {
                continue;
            }

            let public_paths = owner_public_paths
                .iter()
                .map(|path| format!("{path}::{name}"))
                .collect::<Vec<_>>();
            let Some(api) = build_api_info(
                api_item,
                &canonical_path,
                public_paths,
                public_anchor_module_id.clone(),
                None,
                Some(owner_trait_id.clone()),
                ApiKind::TraitMethod,
            ) else {
                continue;
            };
            api_item_ids.insert(trait_item_id, api.api_id.clone());
            apis.push(api);
        }
    }

    for (item_id, item_public_paths) in &public_paths.public_paths {
        if method_item_ids.contains(item_id) {
            continue;
        }
        let Some(api_item) = rustdoc.index.get(item_id) else {
            continue;
        };
        if !is_public(api_item) || !is_function_item(api_item) {
            continue;
        }

        let public_paths = sorted_paths(Some(item_public_paths));
        if public_paths.is_empty() {
            continue;
        }

        let canonical_path = if let Some(path_entry) = rustdoc.paths.get(item_id) {
            join_path_segments(&path_entry.path)
        } else {
            public_paths[0].clone()
        };
        if !seen_api_paths.insert(canonical_path.clone()) {
            continue;
        }

        let Some(public_anchor_module_id) =
            public_anchor_module_id(module_ids, &canonical_path, &public_paths)
        else {
            continue;
        };
        let api_kind = if function_value(api_item).and_then(receiver_string).is_some() {
            ApiKind::InherentMethod
        } else {
            ApiKind::FreeFunction
        };
        let Some(api) = build_api_info(
            api_item,
            &canonical_path,
            public_paths,
            public_anchor_module_id,
            None,
            None,
            api_kind,
        ) else {
            continue;
        };
        api_item_ids.insert(item_id.clone(), api.api_id.clone());
        apis.push(api);
    }

    apis.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    (apis, api_item_ids)
}

fn classify_inherent_api_kind(api_item: &RustdocItem, type_context: &TypeContext) -> ApiKind {
    let Some(function_value) = function_value(api_item) else {
        return ApiKind::AssocFunction;
    };

    if receiver_string(function_value).is_some() {
        return ApiKind::InherentMethod;
    }

    if returns_owned_self(function_value, type_context) {
        ApiKind::Constructor
    } else {
        ApiKind::AssocFunction
    }
}

fn returns_owned_self(function_value: &Value, type_context: &TypeContext) -> bool {
    let Some(output) = function_value
        .get("sig")
        .and_then(|sig| sig.get("output"))
        .filter(|value| !value.is_null())
    else {
        return false;
    };

    if matches_owner_return_type(output, type_context) {
        return true;
    }

    // Keep constructor detection intentionally narrow: direct owned returns and the
    // common `Result<Self, E>` / `Result<Type, E>` wrapper are enough for Phase 1.
    output
        .get("resolved_path")
        .and_then(Value::as_object)
        .filter(|resolved_path| {
            matches!(
                resolved_path.get("path").and_then(Value::as_str),
                Some("Result" | "core::result::Result" | "std::result::Result")
            )
        })
        .and_then(|resolved_path| first_type_generic_arg(resolved_path.get("args")?))
        .is_some_and(|inner| matches_owner_return_type(inner, type_context))
}

fn matches_owner_return_type(value: &Value, type_context: &TypeContext) -> bool {
    let rendered = render_type(value);
    if rendered == "Self" {
        return true;
    }

    let short_name = type_context
        .canonical_path
        .rsplit("::")
        .next()
        .unwrap_or(type_context.canonical_path.as_str());
    if rendered == short_name || rendered == type_context.canonical_path {
        return true;
    }

    type_context
        .public_paths
        .iter()
        .any(|path| rendered == *path)
}

fn first_type_generic_arg<'a>(args_value: &'a Value) -> Option<&'a Value> {
    args_value
        .get("angle_bracketed")
        .and_then(|angle| angle.get("args"))
        .and_then(Value::as_array)
        .and_then(|args| args.iter().find_map(|arg| arg.get("type")))
}

fn build_symbols(
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    type_contexts: &BTreeMap<String, TypeContext>,
) -> (Vec<SymbolInfo>, BTreeMap<String, SymbolId>) {
    let mut symbols = Vec::new();
    let mut symbol_item_ids = BTreeMap::new();
    let mut seen_symbol_paths = BTreeSet::new();

    for (item_id, path_entry) in &rustdoc.paths {
        if path_entry.crate_id != 0 {
            continue;
        }
        let Some(symbol_kind) = symbol_kind_from_path_kind(&path_entry.kind) else {
            continue;
        };
        let Some(item) = rustdoc.index.get(item_id) else {
            continue;
        };
        if !is_public(item) {
            continue;
        }

        let public_paths = sorted_paths(public_paths.public_paths.get(item_id));
        if public_paths.is_empty() {
            continue;
        }

        let canonical_path = join_path_segments(&path_entry.path);
        let Some(public_anchor_module_id) =
            public_anchor_module_id(module_ids, &canonical_path, &public_paths)
        else {
            continue;
        };

        let symbol = SymbolInfo {
            symbol_id: symbol_id_for_path(&canonical_path),
            name: item
                .name
                .clone()
                .unwrap_or_else(|| path_entry.path.last().cloned().unwrap_or_default()),
            canonical_path,
            public_paths,
            public_anchor_module_id,
            owner_type_id: None,
            code_ref: code_ref_from_span(item.span.as_ref()),
            docs: item.docs.clone().unwrap_or_default(),
            doc_sections: doc_sections_from_docs(item.docs.as_deref().unwrap_or_default()),
            symbol_kind,
            signature_text: macro_signature_text(item),
            type_text: constant_type_text(item),
            value_text: constant_value_text(item),
        };
        seen_symbol_paths.insert(symbol.canonical_path.clone());
        symbol_item_ids.insert(item_id.clone(), symbol.symbol_id.clone());
        symbols.push(symbol);
    }

    for type_context in type_contexts.values() {
        let Some(type_item) = rustdoc.index.get(&type_context.item_id) else {
            continue;
        };

        for impl_id in type_impl_ids(type_item) {
            let Some(impl_item) = rustdoc.index.get(&impl_id) else {
                continue;
            };
            let Some(impl_value) = impl_value(impl_item) else {
                continue;
            };
            if impl_value
                .get("trait")
                .is_some_and(|value| !value.is_null())
            {
                continue;
            }
            if impl_value
                .get("is_synthetic")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                continue;
            }

            let Some(items) = impl_value.get("items").and_then(Value::as_array) else {
                continue;
            };

            for item_id_value in items {
                let Some(item_id) = value_id_to_string(item_id_value) else {
                    continue;
                };
                let Some(symbol_item) = rustdoc.index.get(&item_id) else {
                    continue;
                };
                if !is_public(symbol_item) || assoc_const_value(symbol_item).is_none() {
                    continue;
                }

                let Some(name) = symbol_item.name.clone() else {
                    continue;
                };
                let canonical_path = format!("{}::{name}", type_context.canonical_path);
                if !seen_symbol_paths.insert(canonical_path.clone()) {
                    continue;
                }

                let public_paths = type_context
                    .public_paths
                    .iter()
                    .map(|path| format!("{path}::{name}"))
                    .collect::<Vec<_>>();
                let assoc_const = assoc_const_value(symbol_item)
                    .expect("associated const presence already checked");
                let symbol = SymbolInfo {
                    symbol_id: symbol_id_for_path(&canonical_path),
                    name,
                    canonical_path,
                    public_paths,
                    public_anchor_module_id: type_context.public_anchor_module_id.clone(),
                    owner_type_id: Some(type_context.type_id.clone()),
                    code_ref: code_ref_from_span(symbol_item.span.as_ref()),
                    docs: symbol_item.docs.clone().unwrap_or_default(),
                    doc_sections: doc_sections_from_docs(
                        symbol_item.docs.as_deref().unwrap_or_default(),
                    ),
                    symbol_kind: SymbolKind::AssociatedConstant,
                    signature_text: None,
                    type_text: assoc_const.get("type").map(render_type),
                    value_text: assoc_const_value_text(assoc_const),
                };
                symbol_item_ids.insert(item_id, symbol.symbol_id.clone());
                symbols.push(symbol);
            }
        }
    }

    symbols.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    (symbols, symbol_item_ids)
}

fn build_api_info(
    api_item: &RustdocItem,
    canonical_path: &str,
    public_paths: Vec<String>,
    public_anchor_module_id: ModuleId,
    owner_type_id: Option<TypeId>,
    owner_trait_id: Option<TraitId>,
    api_kind: ApiKind,
) -> Option<ApiInfo> {
    let name = api_item.name.clone()?;
    let function_value = function_value(api_item)?;
    let receiver = receiver_string(function_value);
    let arg_types = argument_types(function_value);
    let return_type = return_type_string(function_value);

    Some(ApiInfo {
        api_id: api_id_for_path(canonical_path),
        name: name.clone(),
        canonical_path: canonical_path.to_owned(),
        public_paths,
        public_anchor_module_id,
        owner_type_id,
        owner_trait_id,
        code_ref: code_ref_from_span(api_item.span.as_ref()),
        docs: api_item.docs.clone().unwrap_or_default(),
        doc_sections: doc_sections_from_docs(api_item.docs.as_deref().unwrap_or_default()),
        api_kind,
        signature_text: signature_text(
            &name,
            receiver.as_deref(),
            &arg_types,
            return_type.as_deref(),
        ),
        receiver,
        generic_params: generic_param_names(function_value.get("generics")),
        where_clauses: where_clause_strings(function_value.get("generics")),
        arg_types,
        return_type,
        return_shape: return_shape_from_function(function_value),
        is_unsafe: function_header_flag(function_value, "is_unsafe"),
        is_async: function_header_flag(function_value, "is_async"),
        is_const: function_header_flag(function_value, "is_const"),
        has_body: function_value
            .get("has_body")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        contains_unsafe_block: false,
    })
}

fn build_trait_registry(
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    type_contexts: &BTreeMap<String, TypeContext>,
    api_item_ids: &BTreeMap<String, ApiId>,
) -> Vec<TraitInfo> {
    let mut traits = BTreeMap::<String, TraitInfo>::new();

    for (item_id, path_entry) in &rustdoc.paths {
        if path_entry.crate_id != 0 || path_entry.kind != "trait" {
            continue;
        }
        let Some(item) = rustdoc.index.get(item_id) else {
            continue;
        };
        if !is_public(item) || !is_trait_item(item) {
            continue;
        }

        let canonical_path = join_path_segments(&path_entry.path);
        let trait_id = trait_id_for_path(&canonical_path);
        let public_anchor_module_id = public_anchor_module_id(
            module_ids,
            &canonical_path,
            &sorted_paths(public_paths.public_paths.get(item_id)),
        );

        traits.insert(
            trait_id.as_str().to_owned(),
            TraitInfo {
                trait_id,
                name: item
                    .name
                    .clone()
                    .unwrap_or_else(|| path_entry.path.last().cloned().unwrap_or_default()),
                canonical_path,
                public_paths: sorted_paths(public_paths.public_paths.get(item_id)),
                public_anchor_module_id,
                code_ref: item
                    .span
                    .as_ref()
                    .map(|span| code_ref_from_span(Some(span))),
                docs: item.docs.clone().unwrap_or_default(),
                doc_sections: doc_sections_from_docs(item.docs.as_deref().unwrap_or_default()),
                origin: TraitOrigin::Local,
                exposure_kinds: vec![TraitExposureKind::Defined],
                is_unsafe: trait_value(item)
                    .and_then(|value| value.get("is_unsafe"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                direct_supertrait_ids: direct_supertrait_ids(item, rustdoc),
                required_methods: trait_method_names(item, rustdoc, "required"),
                provided_methods: trait_method_names(item, rustdoc, "provided"),
                associated_type_defs: trait_associated_type_defs(item, rustdoc),
                associated_const_defs: trait_associated_const_defs(item, rustdoc),
                used_by_api_ids: Vec::new(),
                used_by_trait_ids: Vec::new(),
                used_by_type_ids: Vec::new(),
            },
        );
    }

    for (target_id, paths) in &public_paths.reexport_paths {
        let Some(path_entry) = rustdoc.paths.get(target_id) else {
            continue;
        };
        if path_entry.kind != "trait" {
            continue;
        }

        let canonical_path = join_path_segments(&path_entry.path);
        let key = trait_id_for_path(&canonical_path).as_str().to_owned();
        if let Some(existing) = traits.get_mut(&key) {
            merge_sorted_paths(&mut existing.public_paths, paths);
            push_unique_exposure_kind(&mut existing.exposure_kinds, TraitExposureKind::Reexported);
            continue;
        }

        traits.insert(
            key,
            TraitInfo {
                trait_id: trait_id_for_path(&canonical_path),
                name: path_entry.path.last().cloned().unwrap_or_default(),
                canonical_path,
                public_paths: sorted_paths(Some(paths)),
                public_anchor_module_id: public_anchor_module_id(
                    module_ids,
                    &join_path_segments(&path_entry.path),
                    &sorted_paths(Some(paths)),
                ),
                code_ref: None,
                docs: String::new(),
                doc_sections: DocSections::default(),
                origin: TraitOrigin::Reexported,
                exposure_kinds: vec![TraitExposureKind::Reexported],
                is_unsafe: false,
                direct_supertrait_ids: Vec::new(),
                required_methods: Vec::new(),
                provided_methods: Vec::new(),
                associated_type_defs: Vec::new(),
                associated_const_defs: Vec::new(),
                used_by_api_ids: Vec::new(),
                used_by_trait_ids: Vec::new(),
                used_by_type_ids: Vec::new(),
            },
        );
    }

    for (item_id, path_entry) in &rustdoc.paths {
        if path_entry.crate_id != 0 || path_entry.kind != "trait" {
            continue;
        }
        let Some(item) = rustdoc.index.get(item_id) else {
            continue;
        };
        if !is_public(item) || !is_trait_item(item) {
            continue;
        }
        // Reuse the owning public trait id as the reverse-edge anchor for
        // any trait references discovered inside this trait surface.
        let owner_trait_id = trait_id_for_path(&join_path_segments(&path_entry.path));

        record_trait_usages_from_trait_associated_types(
            &mut traits,
            rustdoc,
            module_ids,
            public_paths,
            item,
            &owner_trait_id,
        );
        record_trait_usages_from_public_trait_methods(
            &mut traits,
            rustdoc,
            module_ids,
            public_paths,
            item,
            &owner_trait_id,
        );
    }

    for type_context in type_contexts.values() {
        let Some(type_item) = rustdoc.index.get(&type_context.item_id) else {
            continue;
        };
        record_trait_usages_from_generics(
            &mut traits,
            rustdoc,
            module_ids,
            public_paths,
            type_generics_value(type_item),
            None,
            None,
            Some(&type_context.type_id),
        );

        for impl_id in type_impl_ids(type_item) {
            let Some(impl_item) = rustdoc.index.get(&impl_id) else {
                continue;
            };
            let Some(impl_value) = impl_value(impl_item) else {
                continue;
            };
            if impl_value
                .get("trait")
                .is_some_and(|value| !value.is_null())
            {
                continue;
            }

            let public_api_ids = impl_value
                .get("items")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(value_id_to_string)
                        .filter_map(|item_id| api_item_ids.get(&item_id))
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            if public_api_ids.is_empty() {
                continue;
            }

            let impl_generics = impl_value.get("generics");
            for api_id in &public_api_ids {
                record_trait_usages_from_generics(
                    &mut traits,
                    rustdoc,
                    module_ids,
                    public_paths,
                    impl_generics,
                    Some(api_id),
                    None,
                    None,
                );
            }
        }
    }

    for (item_id, api_id) in api_item_ids {
        let Some(api_item) = rustdoc.index.get(item_id) else {
            continue;
        };
        record_trait_usages_from_value(
            &mut traits,
            rustdoc,
            module_ids,
            public_paths,
            function_value(api_item),
            Some(api_id),
            None,
            None,
        );
    }

    ensure_supertrait_nodes(&mut traits, rustdoc, module_ids, public_paths);
    traits.into_values().collect()
}

fn build_trait_impl_registry(
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    type_contexts: &BTreeMap<String, TypeContext>,
) -> Vec<TraitImplInfo> {
    let mut trait_impls = BTreeMap::<String, TraitImplInfo>::new();

    for type_context in type_contexts.values() {
        let Some(type_item) = rustdoc.index.get(&type_context.item_id) else {
            continue;
        };

        for impl_id in type_impl_ids(type_item) {
            let Some(impl_item) = rustdoc.index.get(&impl_id) else {
                continue;
            };
            let Some(impl_value) = impl_value(impl_item) else {
                continue;
            };
            if impl_value
                .get("is_synthetic")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                continue;
            }
            if impl_value
                .get("blanket_impl")
                .is_some_and(|value| !value.is_null())
            {
                continue;
            }

            let Some(trait_value) = impl_value.get("trait").filter(|value| !value.is_null()) else {
                continue;
            };
            let Some(for_value) = impl_value.get("for") else {
                continue;
            };
            let Some(target_type_context) =
                public_type_context_from_impl_target(for_value, type_contexts)
            else {
                continue;
            };
            let Some(trait_ref) = trait_ref_from_value(trait_value, rustdoc) else {
                continue;
            };
            if !is_public_trait_impl_surface(&trait_ref, rustdoc) {
                continue;
            }

            let trait_info = trait_info_from_ref(
                &trait_ref,
                TraitExposureKind::Bound,
                rustdoc,
                module_ids,
                public_paths,
            );
            let trait_ref_text = render_canonical_trait_ref(trait_value, &trait_ref.canonical_path);
            let for_type_text = render_type(for_value);
            let trait_impl_id = trait_impl_id_for_surface(&trait_ref_text, &for_type_text);

            trait_impls.insert(
                trait_impl_id.as_str().to_owned(),
                TraitImplInfo {
                    trait_impl_id,
                    target_type_id: target_type_context.type_id.clone(),
                    trait_ref_text,
                    for_type_text,
                    trait_id: trait_info.trait_id,
                    trait_name: trait_info.name,
                    trait_canonical_path: trait_info.canonical_path,
                    trait_origin: trait_info.origin,
                    source: code_ref_from_span(impl_item.span.as_ref()),
                    associated_type_bindings: trait_impl_associated_type_bindings(
                        impl_item, rustdoc,
                    ),
                    associated_const_bindings: trait_impl_associated_const_bindings(
                        impl_item, rustdoc,
                    ),
                    where_clauses: where_clause_strings(impl_value.get("generics")),
                    cfg_attrs: cfg_attrs_from_attrs(&impl_item.attrs),
                    is_unsafe: impl_value
                        .get("is_unsafe")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                },
            );
        }
    }

    trait_impls.into_values().collect()
}

fn record_trait_usages_from_generics(
    traits: &mut BTreeMap<String, TraitInfo>,
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    generics_value: Option<&Value>,
    api_id: Option<&ApiId>,
    // When a bound comes from a public trait surface, keep only the owning
    // trait-level reverse edge. We intentionally avoid finer attribution here.
    owner_trait_id: Option<&TraitId>,
    type_id: Option<&TypeId>,
) {
    record_trait_usages_from_value(
        traits,
        rustdoc,
        module_ids,
        public_paths,
        generics_value,
        api_id,
        owner_trait_id,
        type_id,
    );
}

fn record_trait_usages_from_value(
    traits: &mut BTreeMap<String, TraitInfo>,
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    value: Option<&Value>,
    api_id: Option<&ApiId>,
    owner_trait_id: Option<&TraitId>,
    type_id: Option<&TypeId>,
) {
    let mut trait_refs = BTreeMap::<String, TraitRefInfo>::new();
    if let Some(value) = value {
        collect_trait_refs_from_value(value, rustdoc, &mut trait_refs);
    }

    for trait_ref in trait_refs.into_values() {
        let trait_info = trait_info_from_ref(
            &trait_ref,
            TraitExposureKind::Bound,
            rustdoc,
            module_ids,
            public_paths,
        );
        let key = trait_info.trait_id.as_str().to_owned();
        let entry = traits.entry(key).or_insert(trait_info.clone());
        if entry.trait_id != trait_info.trait_id {
            continue;
        }
        merge_trait_info(entry, trait_info);
        push_unique_exposure_kind(&mut entry.exposure_kinds, TraitExposureKind::Bound);
        if let Some(api_id) = api_id {
            push_unique_api_id(&mut entry.used_by_api_ids, api_id);
        }
        if let Some(owner_trait_id) = owner_trait_id {
            push_unique_trait_id(&mut entry.used_by_trait_ids, owner_trait_id);
        }
        if let Some(type_id) = type_id {
            push_unique_type_id(&mut entry.used_by_type_ids, type_id);
        }
    }
}

fn record_trait_usages_from_trait_associated_types(
    traits: &mut BTreeMap<String, TraitInfo>,
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    trait_item: &RustdocItem,
    // Public trait that owns the associated type definition we are scanning.
    owner_trait_id: &TraitId,
) {
    let mut trait_refs = BTreeMap::<String, TraitRefInfo>::new();

    let Some(items) = trait_item
        .inner
        .get("trait")
        .and_then(|trait_value| trait_value.get("items"))
        .and_then(Value::as_array)
    else {
        return;
    };

    for item_id in items.iter().filter_map(value_id_to_string) {
        let Some(assoc_item) = rustdoc.index.get(&item_id) else {
            continue;
        };
        let Some(assoc_type) = assoc_type_value(assoc_item) else {
            continue;
        };
        collect_trait_refs_from_value(assoc_type, rustdoc, &mut trait_refs);
    }

    for trait_ref in trait_refs.into_values() {
        let trait_info = trait_info_from_ref(
            &trait_ref,
            TraitExposureKind::Bound,
            rustdoc,
            module_ids,
            public_paths,
        );
        let key = trait_info.trait_id.as_str().to_owned();
        let entry = traits.entry(key).or_insert(trait_info.clone());
        if entry.trait_id != trait_info.trait_id {
            continue;
        }
        merge_trait_info(entry, trait_info);
        push_unique_exposure_kind(&mut entry.exposure_kinds, TraitExposureKind::Bound);
        push_unique_trait_id(&mut entry.used_by_trait_ids, owner_trait_id);
    }
}

fn record_trait_usages_from_public_trait_methods(
    traits: &mut BTreeMap<String, TraitInfo>,
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
    trait_item: &RustdocItem,
    // Public trait that owns the method surface we are scanning.
    owner_trait_id: &TraitId,
) {
    // Keep reverse edges from trait method signatures at trait granularity
    // instead of branching into per-method attribution in Phase 1.
    let Some(items) = trait_item
        .inner
        .get("trait")
        .and_then(|trait_value| trait_value.get("items"))
        .and_then(Value::as_array)
    else {
        return;
    };

    for item_id in items.iter().filter_map(value_id_to_string) {
        let Some(method_item) = rustdoc.index.get(&item_id) else {
            continue;
        };
        record_trait_usages_from_value(
            traits,
            rustdoc,
            module_ids,
            public_paths,
            function_value(method_item),
            None,
            Some(owner_trait_id),
            None,
        );
    }
}

fn ensure_supertrait_nodes(
    traits: &mut BTreeMap<String, TraitInfo>,
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
) {
    let supertraits = traits
        .values()
        .flat_map(|trait_info| trait_info.direct_supertrait_ids.iter().cloned())
        .collect::<BTreeSet<TraitId>>();

    for supertrait_id in supertraits {
        let canonical_path = supertrait_id
            .as_str()
            .strip_prefix("trait::")
            .unwrap_or_default()
            .to_owned();
        let trait_ref =
            trait_ref_from_canonical_path(rustdoc, &canonical_path).unwrap_or_else(|| {
                TraitRefInfo {
                    item_id: None,
                    canonical_path: canonical_path.clone(),
                    name: canonical_path
                        .rsplit("::")
                        .next()
                        .unwrap_or_default()
                        .to_owned(),
                }
            });
        let trait_info = trait_info_from_ref(
            &trait_ref,
            TraitExposureKind::SupertraitDependency,
            rustdoc,
            module_ids,
            public_paths,
        );
        let entry = traits
            .entry(supertrait_id.as_str().to_owned())
            .or_insert(trait_info.clone());
        merge_trait_info(entry, trait_info);
        push_unique_exposure_kind(
            &mut entry.exposure_kinds,
            TraitExposureKind::SupertraitDependency,
        );
    }
}

fn build_examples(
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    type_contexts: &BTreeMap<String, TypeContext>,
    apis: &[ApiInfo],
    api_item_ids: &BTreeMap<String, ApiId>,
    symbols: &[SymbolInfo],
    symbol_item_ids: &BTreeMap<String, SymbolId>,
) -> Vec<ExampleInfo> {
    let api_by_id = apis
        .iter()
        .map(|api| (api.api_id.as_str().to_owned(), api))
        .collect::<BTreeMap<_, _>>();
    let symbol_by_id = symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.as_str().to_owned(), symbol))
        .collect::<BTreeMap<_, _>>();
    let api_reference_labels = api_reference_labels(apis);

    let mut examples = Vec::new();
    for (item_id, api_id) in api_item_ids {
        let Some(api_item) = rustdoc.index.get(item_id) else {
            continue;
        };
        let Some(api) = api_by_id.get(api_id.as_str()) else {
            continue;
        };
        for (index, snippet) in rust_doc_code_blocks(api_item.docs.as_deref().unwrap_or_default())
            .into_iter()
            .enumerate()
        {
            examples.push(ExampleInfo {
                example_id: example_id_for_code_ref(&api.code_ref, index),
                anchor: ExampleAnchor::Api(api.api_id.clone()),
                involved_api_ids: involved_api_ids_for_snippet(
                    &snippet,
                    &api_reference_labels,
                    Some(&api.api_id),
                ),
                code_ref: api.code_ref.clone(),
                snippet: Some(snippet),
            });
        }
    }

    for type_context in type_contexts.values() {
        let Some(type_item) = rustdoc.index.get(&type_context.item_id) else {
            continue;
        };
        let code_ref = code_ref_from_span(type_item.span.as_ref());
        for (index, snippet) in rust_doc_code_blocks(type_item.docs.as_deref().unwrap_or_default())
            .into_iter()
            .enumerate()
        {
            examples.push(ExampleInfo {
                example_id: example_id_for_code_ref(&code_ref, index),
                anchor: ExampleAnchor::Type(type_context.type_id.clone()),
                involved_api_ids: involved_api_ids_for_snippet(
                    &snippet,
                    &api_reference_labels,
                    None,
                ),
                code_ref: code_ref.clone(),
                snippet: Some(snippet),
            });
        }
    }

    for (item_id, symbol_id) in symbol_item_ids {
        let Some(symbol_item) = rustdoc.index.get(item_id) else {
            continue;
        };
        let Some(symbol) = symbol_by_id.get(symbol_id.as_str()) else {
            continue;
        };
        for (index, snippet) in
            rust_doc_code_blocks(symbol_item.docs.as_deref().unwrap_or_default())
                .into_iter()
                .enumerate()
        {
            examples.push(ExampleInfo {
                example_id: example_id_for_code_ref(&symbol.code_ref, index),
                anchor: ExampleAnchor::Symbol(symbol.symbol_id.clone()),
                involved_api_ids: involved_api_ids_for_snippet(
                    &snippet,
                    &api_reference_labels,
                    None,
                ),
                code_ref: symbol.code_ref.clone(),
                snippet: Some(snippet),
            });
        }
    }

    for (item_id, path_entry) in &rustdoc.paths {
        if path_entry.crate_id != 0 {
            continue;
        }
        let Some(item) = rustdoc.index.get(item_id) else {
            continue;
        };
        if !is_public(item) {
            continue;
        }

        let snippets = rust_doc_code_blocks(item.docs.as_deref().unwrap_or_default());
        if snippets.is_empty() {
            continue;
        }
        let canonical_path = join_path_segments(&path_entry.path);
        let code_ref = code_ref_from_span(item.span.as_ref());

        let anchor = match path_entry.kind.as_str() {
            "module" => module_ids
                .get(&canonical_path)
                .cloned()
                .map(ExampleAnchor::Module),
            "trait" => Some(ExampleAnchor::Trait(trait_id_for_path(&canonical_path))),
            _ => None,
        };

        let Some(anchor) = anchor else {
            continue;
        };

        for (index, snippet) in snippets.into_iter().enumerate() {
            examples.push(ExampleInfo {
                example_id: example_id_for_code_ref(&code_ref, index),
                anchor: anchor.clone(),
                involved_api_ids: involved_api_ids_for_snippet(
                    &snippet,
                    &api_reference_labels,
                    None,
                ),
                code_ref: code_ref.clone(),
                snippet: Some(snippet),
            });
        }
    }

    examples.sort_by(|left, right| left.example_id.as_str().cmp(right.example_id.as_str()));
    examples
}

fn rust_doc_code_blocks(docs: &str) -> Vec<String> {
    let mut in_block = false;
    let mut is_rust_block = false;
    let mut snippet_lines = Vec::new();
    let mut snippets = Vec::new();

    for line in docs.lines() {
        let trimmed = line.trim();
        if let Some(info) = trimmed.strip_prefix("```") {
            if !in_block {
                in_block = true;
                is_rust_block = is_rust_doc_fence(info);
                snippet_lines.clear();
                continue;
            }

            if is_rust_block && !snippet_lines.is_empty() {
                let snippet = snippet_lines.join("\n").trim().to_owned();
                if !snippet.is_empty() {
                    snippets.push(snippet);
                }
            }

            in_block = false;
            is_rust_block = false;
            snippet_lines.clear();
            continue;
        }

        if in_block && is_rust_block {
            snippet_lines.push(line.to_owned());
        }
    }

    snippets
}

fn is_rust_doc_fence(info: &str) -> bool {
    let info = info.trim();
    if info.is_empty() {
        return true;
    }

    let lowered = info.to_ascii_lowercase();
    !lowered.split(',').any(|part| {
        let part = part.trim();
        part == "text" || part == "txt"
    })
}

fn doc_sections_from_docs(docs: &str) -> DocSections {
    DocSections {
        summary: markdown_summary(docs),
        panics: markdown_heading_body(docs, "Panics"),
        errors: markdown_heading_body(docs, "Errors"),
        safety: markdown_heading_body(docs, "Safety"),
        // Real crates mix `# Example` and `# Examples`, so treat both as the
        // same raw-doc section in Phase 1.
        examples: markdown_heading_body_any(docs, &["Examples", "Example"]),
    }
}

fn markdown_summary(docs: &str) -> String {
    let mut paragraph = Vec::new();
    let mut in_fence = false;

    for line in docs.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if trimmed.is_empty() {
            if let Some(summary) = markdown_summary_paragraph(&paragraph) {
                return summary;
            }
            paragraph.clear();
            continue;
        }
        if trimmed.starts_with('#') {
            if let Some(summary) = markdown_summary_paragraph(&paragraph) {
                return summary;
            }
            paragraph.clear();
            continue;
        }

        paragraph.push(trimmed.to_owned());
    }

    markdown_summary_paragraph(&paragraph).unwrap_or_default()
}

fn markdown_heading_body(docs: &str, heading: &str) -> String {
    markdown_heading_body_any(docs, &[heading])
}

fn markdown_heading_body_any(docs: &str, headings: &[&str]) -> String {
    let mut lines = Vec::new();
    let mut in_section = false;
    let mut in_fence = false;

    for line in docs.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            if in_section {
                lines.push(line.to_owned());
            }
            continue;
        }
        if !in_fence {
            if let Some(current_heading) = markdown_heading_text(trimmed) {
                if headings.iter().any(|heading| current_heading == *heading) {
                    in_section = true;
                    lines.clear();
                    continue;
                }
                if in_section {
                    break;
                }
            }
        }

        if in_section {
            lines.push(line.to_owned());
        }
    }

    lines.join("\n").trim().to_owned()
}

fn markdown_summary_paragraph(lines: &[String]) -> Option<String> {
    if lines.is_empty() || markdown_paragraph_is_decorative(lines) {
        return None;
    }
    Some(lines.join("\n"))
}

fn markdown_paragraph_is_decorative(lines: &[String]) -> bool {
    lines.iter().all(|line| markdown_line_is_decorative(line))
}

fn markdown_line_is_decorative(line: &str) -> bool {
    let trimmed = line.trim();
    // Skip badge/link-definition paragraphs so summary lands on the first real
    // prose paragraph instead of docs.rs/GitHub chrome.
    trimmed.starts_with("[![")
        || trimmed.starts_with("<br")
        || (trimmed.starts_with('[') && trimmed.contains("]:"))
}

fn markdown_heading_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let heading = trimmed.strip_prefix('#')?;
    Some(heading.trim_start_matches('#').trim())
}

fn example_id_for_code_ref(code_ref: &CodeRef, block_index: usize) -> ExampleId {
    ExampleId::from(format!(
        "ex::{}::{}::{}",
        code_ref.file,
        code_ref.start_line,
        block_index + 1
    ))
}

fn api_reference_labels(apis: &[ApiInfo]) -> Vec<(String, ApiId)> {
    let mut labels = BTreeMap::<String, ApiId>::new();

    for api in apis {
        if let Some((owner, method)) = api.canonical_path.rsplit_once("::") {
            if let Some((_, type_name)) = owner.rsplit_once("::") {
                labels
                    .entry(format!("{type_name}::{method}"))
                    .or_insert_with(|| api.api_id.clone());
            }
        }

        for public_path in &api.public_paths {
            labels
                .entry(public_path.clone())
                .or_insert_with(|| api.api_id.clone());
            if let Some((owner, method)) = public_path.rsplit_once("::") {
                if let Some((_, type_name)) = owner.rsplit_once("::") {
                    labels
                        .entry(format!("{type_name}::{method}"))
                        .or_insert_with(|| api.api_id.clone());
                }
            }
        }
    }

    labels.into_iter().collect()
}

fn involved_api_ids_for_snippet(
    snippet: &str,
    labels: &[(String, ApiId)],
    anchor_api_id: Option<&ApiId>,
) -> Vec<ApiId> {
    let mut api_ids = Vec::new();
    if let Some(anchor_api_id) = anchor_api_id {
        push_unique_api_id(&mut api_ids, anchor_api_id);
    }

    for (label, api_id) in labels {
        if snippet.contains(label) {
            push_unique_api_id(&mut api_ids, api_id);
        }
    }

    api_ids
}

fn build_risk_facts(
    rustdoc: &RustdocResponse,
    type_contexts: &BTreeMap<String, TypeContext>,
    api_item_ids: &BTreeMap<String, ApiId>,
    trait_impl_registry: &[TraitImplInfo],
    apis: &[ApiInfo],
) -> RiskFacts {
    let extern_abi_apis = build_extern_abi_api_facts(rustdoc, api_item_ids);
    RiskFacts {
        extern_abi_apis,
        repr_types: build_repr_type_facts(rustdoc, type_contexts),
        drop_impl_types: build_drop_impl_type_ids(trait_impl_registry),
        explicit_panic_sites: Vec::new(),
        borrowed_return_apis: build_borrowed_return_facts(rustdoc, api_item_ids, apis),
    }
}

fn build_extern_abi_api_facts(
    rustdoc: &RustdocResponse,
    api_item_ids: &BTreeMap<String, ApiId>,
) -> Vec<ExternAbiApiFact> {
    let mut facts = api_item_ids
        .iter()
        .filter_map(|(item_id, api_id)| {
            let api_item = rustdoc.index.get(item_id)?;
            let function_value = function_value(api_item)?;
            let abi = function_abi_string(function_value)?;
            if abi == "Rust" {
                return None;
            }

            Some(ExternAbiApiFact {
                api_id: api_id.clone(),
                abi: abi.to_owned(),
                source: api_item
                    .span
                    .as_ref()
                    .map(|span| code_ref_from_span(Some(span))),
            })
        })
        .collect::<Vec<_>>();
    facts.sort_by(|left, right| left.api_id.cmp(&right.api_id));
    facts
}

fn function_abi_string(function_value: &Value) -> Option<String> {
    let abi = function_value
        .get("header")
        .and_then(|header| header.get("abi"))?;
    if let Some(abi) = abi.as_str() {
        return Some(abi.to_owned());
    }

    let (name, details) = abi.as_object()?.iter().next()?;
    let unwind = details
        .get("unwind")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if unwind {
        Some(format!("{name}-unwind"))
    } else {
        Some(name.clone())
    }
}

fn build_repr_type_facts(
    rustdoc: &RustdocResponse,
    type_contexts: &BTreeMap<String, TypeContext>,
) -> Vec<TypeLayoutFact> {
    let mut facts = type_contexts
        .values()
        .filter_map(|type_context| {
            let item = rustdoc.index.get(&type_context.item_id)?;
            let repr_kinds = repr_kinds_from_attrs(&item.attrs);
            if repr_kinds.is_empty() {
                return None;
            }

            Some(TypeLayoutFact {
                type_id: type_context.type_id.clone(),
                repr_kinds,
                source: item
                    .span
                    .as_ref()
                    .map(|span| code_ref_from_span(Some(span))),
            })
        })
        .collect::<Vec<_>>();
    facts.sort_by(|left, right| left.type_id.cmp(&right.type_id));
    facts
}

fn build_drop_impl_type_ids(trait_impl_registry: &[TraitImplInfo]) -> Vec<TypeId> {
    trait_impl_registry
        .iter()
        .filter(|trait_impl| trait_impl.trait_canonical_path == "core::ops::drop::Drop")
        .map(|trait_impl| trait_impl.target_type_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn build_borrowed_return_facts(
    rustdoc: &RustdocResponse,
    api_item_ids: &BTreeMap<String, ApiId>,
    apis: &[ApiInfo],
) -> Vec<BorrowedReturnFact> {
    let api_by_id = apis
        .iter()
        .map(|api| (api.api_id.as_str().to_owned(), api))
        .collect::<BTreeMap<_, _>>();
    let mut facts = Vec::new();

    for (item_id, api_id) in api_item_ids {
        let Some(api_item) = rustdoc.index.get(item_id) else {
            continue;
        };
        let Some(function_value) = function_value(api_item) else {
            continue;
        };
        let Some(api) = api_by_id.get(api_id.as_str()) else {
            continue;
        };
        if let Some(fact) = borrowed_return_fact_from_function(api, function_value) {
            facts.push(fact);
        }
    }

    facts.sort_by(|left, right| left.api_id.cmp(&right.api_id));
    facts
}

fn borrowed_return_fact_from_function(
    api: &ApiInfo,
    function_value: &Value,
) -> Option<BorrowedReturnFact> {
    let inputs = function_value
        .get("sig")
        .and_then(|sig| sig.get("inputs"))
        .and_then(Value::as_array)?;
    let output = function_value
        .get("sig")
        .and_then(|sig| sig.get("output"))
        .filter(|output| !output.is_null())?;

    let mut output_lifetimes = Vec::new();
    let mut output_has_elided_borrow = false;
    collect_borrow_lifetimes(output, &mut output_lifetimes, &mut output_has_elided_borrow);
    if output_lifetimes.is_empty() && !output_has_elided_borrow {
        return None;
    }

    let mut receiver_lifetimes = BTreeSet::new();
    let mut receiver_is_borrowed = false;
    let mut arg_lifetime_positions = BTreeMap::<String, BTreeSet<u32>>::new();
    let mut borrowed_arg_positions = BTreeSet::<u32>::new();

    let mut arg_position = 0_u32;
    for input in inputs {
        let Some(input) = input.as_array() else {
            continue;
        };
        let Some(name) = input.first().and_then(Value::as_str) else {
            continue;
        };
        let Some(ty) = input.get(1) else {
            continue;
        };

        let mut input_lifetimes = Vec::new();
        let mut input_has_elided_borrow = false;
        collect_borrow_lifetimes(ty, &mut input_lifetimes, &mut input_has_elided_borrow);

        if name == "self" {
            receiver_is_borrowed = !input_lifetimes.is_empty() || input_has_elided_borrow;
            receiver_lifetimes.extend(input_lifetimes.into_iter());
            continue;
        }

        if !input_lifetimes.is_empty() || input_has_elided_borrow {
            borrowed_arg_positions.insert(arg_position);
        }
        for lifetime in input_lifetimes {
            arg_lifetime_positions
                .entry(lifetime)
                .or_default()
                .insert(arg_position);
        }
        arg_position += 1;
    }

    let mut matched_lifetimes = BTreeSet::new();
    let mut from_arg_positions = BTreeSet::new();
    let mut from_self = false;

    for lifetime in &output_lifetimes {
        if receiver_lifetimes.contains(lifetime) {
            from_self = true;
            matched_lifetimes.insert(lifetime.clone());
        }
        if let Some(positions) = arg_lifetime_positions.get(lifetime) {
            matched_lifetimes.insert(lifetime.clone());
            from_arg_positions.extend(positions.iter().copied());
        }
    }

    if output_has_elided_borrow {
        if receiver_is_borrowed {
            from_self = true;
        } else if borrowed_arg_positions.len() == 1 {
            from_arg_positions.extend(borrowed_arg_positions.iter().copied());
        }
    }

    if !from_self && from_arg_positions.is_empty() {
        return None;
    }

    Some(BorrowedReturnFact {
        api_id: api.api_id.clone(),
        return_type_text: api
            .return_type
            .clone()
            .unwrap_or_else(|| render_type(output)),
        from_self,
        from_arg_positions: from_arg_positions.into_iter().collect(),
        lifetime_names: matched_lifetimes.into_iter().collect(),
        source: Some(api.code_ref.clone()),
    })
}

fn collect_borrow_lifetimes(
    value: &Value,
    explicit_lifetimes: &mut Vec<String>,
    has_elided_borrow: &mut bool,
) {
    match value {
        Value::Object(object) => {
            if let Some(borrowed_ref) = object.get("borrowed_ref").and_then(Value::as_object) {
                if let Some(lifetime) = borrowed_ref.get("lifetime").and_then(Value::as_str) {
                    explicit_lifetimes.push(lifetime.to_owned());
                } else {
                    *has_elided_borrow = true;
                }
                if let Some(inner) = borrowed_ref.get("type") {
                    collect_borrow_lifetimes(inner, explicit_lifetimes, has_elided_borrow);
                }
                return;
            }

            for nested in object.values() {
                collect_borrow_lifetimes(nested, explicit_lifetimes, has_elided_borrow);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_borrow_lifetimes(item, explicit_lifetimes, has_elided_borrow);
            }
        }
        _ => {}
    }
}

fn repr_kinds_from_attrs(attrs: &[String]) -> Vec<ReprKind> {
    let mut kinds = Vec::new();
    for attr in attrs {
        let Some(inner) = attr
            .strip_prefix("#[repr(")
            .and_then(|value| value.strip_suffix(")]"))
        else {
            continue;
        };

        for raw_kind in inner
            .split(',')
            .map(str::trim)
            .filter(|kind| !kind.is_empty())
        {
            let kind = if raw_kind == "C" {
                ReprKind::C
            } else if raw_kind == "transparent" {
                ReprKind::Transparent
            } else if raw_kind.starts_with("packed") {
                ReprKind::Packed
            } else if let Some(value) = raw_kind
                .strip_prefix("align(")
                .and_then(|value| value.strip_suffix(')'))
                .and_then(|value| value.parse::<u32>().ok())
            {
                ReprKind::Align(value)
            } else {
                ReprKind::Other(raw_kind.to_owned())
            };

            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
    }
    kinds
}

fn rewrite_knowledge_paths(knowledge: &mut Knowledge, manifest_execution: &ManifestExecution) {
    knowledge.crate_meta.lib_rs_path =
        manifest_execution.rewrite_path(&knowledge.crate_meta.lib_rs_path);

    for module in &mut knowledge.modules {
        rewrite_code_ref_path(&mut module.code_ref, manifest_execution);
    }
    for ty in &mut knowledge.types {
        rewrite_code_ref_path(&mut ty.code_ref, manifest_execution);
    }
    for api in &mut knowledge.apis {
        rewrite_code_ref_path(&mut api.code_ref, manifest_execution);
    }
    for symbol in &mut knowledge.symbols {
        rewrite_code_ref_path(&mut symbol.code_ref, manifest_execution);
    }
    for trait_info in &mut knowledge.trait_registry {
        if let Some(code_ref) = &mut trait_info.code_ref {
            rewrite_code_ref_path(code_ref, manifest_execution);
        }
    }
    for trait_impl in &mut knowledge.trait_impl_registry {
        rewrite_code_ref_path(&mut trait_impl.source, manifest_execution);
    }
    for example in &mut knowledge.examples {
        rewrite_code_ref_path(&mut example.code_ref, manifest_execution);
    }
    for extern_abi_api in &mut knowledge.risk_facts.extern_abi_apis {
        if let Some(source) = &mut extern_abi_api.source {
            rewrite_code_ref_path(source, manifest_execution);
        }
    }
    for repr_type in &mut knowledge.risk_facts.repr_types {
        if let Some(source) = &mut repr_type.source {
            rewrite_code_ref_path(source, manifest_execution);
        }
    }
}

fn backfill_trait_impl_unsafety_from_source(knowledge: &mut Knowledge, crate_dir: &Path) {
    let mut source_cache = BTreeMap::<String, Option<String>>::new();

    for trait_impl in &mut knowledge.trait_impl_registry {
        if trait_impl.is_unsafe {
            continue;
        }
        if source_span_contains_unsafe_impl(&trait_impl.source, crate_dir, &mut source_cache) {
            trait_impl.is_unsafe = true;
        }
    }
}

fn backfill_api_unsafe_block_presence(knowledge: &mut Knowledge, crate_dir: &Path) {
    let mut source_cache = BTreeMap::<String, Option<String>>::new();

    for api in &mut knowledge.apis {
        api.contains_unsafe_block =
            source_span_contains_unsafe_block(&api.code_ref, crate_dir, &mut source_cache);
    }
}

fn backfill_explicit_panic_sites(knowledge: &mut Knowledge, crate_dir: &Path) {
    let mut source_cache = BTreeMap::<String, Option<String>>::new();
    let mut panic_sites = Vec::new();

    for api in &knowledge.apis {
        for panic_kind in source_span_panic_kinds(&api.code_ref, crate_dir, &mut source_cache) {
            panic_sites.push(ExplicitPanicSiteFact {
                owner: RiskOwner::Api(api.api_id.clone()),
                panic_kind,
                source: api.code_ref.clone(),
            });
        }
    }

    for trait_impl in &knowledge.trait_impl_registry {
        for panic_kind in source_span_panic_kinds(&trait_impl.source, crate_dir, &mut source_cache)
        {
            panic_sites.push(ExplicitPanicSiteFact {
                owner: RiskOwner::TraitImpl(trait_impl.trait_impl_id.clone()),
                panic_kind,
                source: trait_impl.source.clone(),
            });
        }
    }

    panic_sites.sort_by(|left, right| {
        format!("{:?}:{}", left.owner, left.panic_kind)
            .cmp(&format!("{:?}:{}", right.owner, right.panic_kind))
    });
    knowledge.risk_facts.explicit_panic_sites = panic_sites;
}

fn source_span_contains_unsafe_impl(
    source: &CodeRef,
    crate_dir: &Path,
    source_cache: &mut BTreeMap<String, Option<String>>,
) -> bool {
    let Some(snippet) = source_span_text(source, crate_dir, source_cache) else {
        return false;
    };

    snippet
        .lines()
        .map(str::trim_start)
        .find(|line| !line.is_empty())
        .is_some_and(|line| line.contains("unsafe impl"))
}

fn source_span_contains_unsafe_block(
    source: &CodeRef,
    crate_dir: &Path,
    source_cache: &mut BTreeMap<String, Option<String>>,
) -> bool {
    let Some(snippet) = source_span_text(source, crate_dir, source_cache) else {
        return false;
    };

    snippet
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains("unsafe {")
}

fn source_span_panic_kinds(
    source: &CodeRef,
    crate_dir: &Path,
    source_cache: &mut BTreeMap<String, Option<String>>,
) -> Vec<String> {
    let Some(snippet) = source_span_text(source, crate_dir, source_cache) else {
        return Vec::new();
    };

    let mut kinds = BTreeSet::new();
    for panic_kind in [
        "debug_assert_eq!",
        "debug_assert_ne!",
        "debug_assert!",
        "assert_eq!",
        "assert_ne!",
        "assert!",
        "panic!",
        "todo!",
        "unimplemented!",
        "unreachable!",
    ] {
        if snippet.contains(panic_kind) {
            kinds.insert(panic_kind.to_owned());
        }
    }

    kinds.into_iter().collect()
}

fn source_span_text(
    source: &CodeRef,
    crate_dir: &Path,
    source_cache: &mut BTreeMap<String, Option<String>>,
) -> Option<String> {
    let source_path = if Path::new(&source.file).is_absolute() {
        PathBuf::from(&source.file)
    } else {
        crate_dir.join(&source.file)
    };
    let cache_key = source_path.to_string_lossy().into_owned();
    let raw = source_cache
        .entry(cache_key)
        .or_insert_with(|| fs::read_to_string(&source_path).ok());
    let raw = raw.as_deref()?;

    Some(
        raw.lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let line_no = index + 1;
                (line_no >= source.start_line as usize && line_no <= source.end_line as usize)
                    .then_some(line)
            })
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn rewrite_code_ref_path(code_ref: &mut CodeRef, manifest_execution: &ManifestExecution) {
    code_ref.file = manifest_execution.rewrite_path(&code_ref.file);
}

fn module_item_ids(item: &RustdocItem) -> Option<Vec<String>> {
    item.inner
        .get("module")
        .and_then(|module| module.get("items"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(value_id_to_string)
                .collect::<Vec<String>>()
        })
}

fn type_impl_ids(item: &RustdocItem) -> Vec<String> {
    for key in ["struct", "enum", "union", "type_alias"] {
        if let Some(ids) = item
            .inner
            .get(key)
            .and_then(|value| value.get("impls"))
            .and_then(Value::as_array)
        {
            return ids
                .iter()
                .filter_map(value_id_to_string)
                .collect::<Vec<String>>();
        }
    }
    Vec::new()
}

fn type_generics_value(item: &RustdocItem) -> Option<&Value> {
    for key in ["struct", "enum", "union", "type_alias"] {
        if let Some(generics) = item.inner.get(key).and_then(|value| value.get("generics")) {
            return Some(generics);
        }
    }
    None
}

fn trait_value(item: &RustdocItem) -> Option<&Value> {
    item.inner.get("trait")
}

fn impl_value(item: &RustdocItem) -> Option<&Value> {
    item.inner.get("impl")
}

fn function_value(item: &RustdocItem) -> Option<&Value> {
    item.inner.get("function")
}

fn assoc_type_value(item: &RustdocItem) -> Option<&Value> {
    item.inner.get("assoc_type")
}

fn use_value(item: &RustdocItem) -> Option<&Value> {
    item.inner.get("use")
}

fn struct_field_value(item: &RustdocItem) -> Option<&Value> {
    item.inner.get("struct_field")
}

fn assoc_const_value(item: &RustdocItem) -> Option<&Value> {
    item.inner.get("assoc_const")
}

fn cfg_attrs_from_attrs(attrs: &[String]) -> Vec<String> {
    let mut cfg_attrs = Vec::new();
    for attr in attrs {
        if attr.contains("cfg(") || attr.contains("cfg_attr(") {
            if !cfg_attrs.contains(attr) {
                cfg_attrs.push(attr.clone());
            }
        }
    }
    cfg_attrs
}

fn has_attr(attrs: &[String], attr_name: &str) -> bool {
    attrs.iter().any(|attr| {
        let trimmed = attr.trim();
        trimmed == attr_name
            || trimmed == format!("#[{attr_name}]")
            || trimmed.starts_with(&format!("#[{attr_name}("))
    })
}

fn type_surface_from_item(
    item: &RustdocItem,
    rustdoc: &RustdocResponse,
) -> (Vec<TypeFieldInfo>, Vec<EnumVariantInfo>, bool, bool) {
    if let Some(struct_value) = item.inner.get("struct").and_then(Value::as_object) {
        let (fields, has_hidden_fields) =
            struct_like_fields_from_kind(struct_value.get("kind"), rustdoc);
        return (fields, Vec::new(), has_hidden_fields, false);
    }
    if let Some(union_value) = item.inner.get("union").and_then(Value::as_object) {
        let fields = union_value
            .get("fields")
            .and_then(Value::as_array)
            .map(|fields| fields_from_ids(fields, rustdoc))
            .unwrap_or_default();
        return (fields, Vec::new(), false, false);
    }
    if let Some(enum_value) = item.inner.get("enum").and_then(Value::as_object) {
        let variants = enum_value
            .get("variants")
            .and_then(Value::as_array)
            .map(|variants| {
                variants
                    .iter()
                    .filter_map(value_id_to_string)
                    .filter_map(|variant_id| {
                        let variant_item = rustdoc.index.get(&variant_id)?;
                        enum_variant_info_from_item(variant_item, rustdoc)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let has_hidden_variants = enum_value
            .get("has_stripped_variants")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        return (Vec::new(), variants, false, has_hidden_variants);
    }

    (Vec::new(), Vec::new(), false, false)
}

fn struct_like_fields_from_kind(
    kind_value: Option<&Value>,
    rustdoc: &RustdocResponse,
) -> (Vec<TypeFieldInfo>, bool) {
    let Some(kind) = kind_value.and_then(Value::as_object) else {
        return (Vec::new(), false);
    };

    if let Some(plain) = kind.get("plain").and_then(Value::as_object) {
        let fields = plain
            .get("fields")
            .and_then(Value::as_array)
            .map(|fields| fields_from_ids(fields, rustdoc))
            .unwrap_or_default();
        let has_hidden = plain
            .get("has_stripped_fields")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        return (fields, has_hidden);
    }

    if let Some(tuple) = kind.get("tuple").and_then(Value::as_array) {
        return (fields_from_ids(tuple, rustdoc), false);
    }

    (Vec::new(), false)
}

fn fields_from_ids(field_ids: &[Value], rustdoc: &RustdocResponse) -> Vec<TypeFieldInfo> {
    field_ids
        .iter()
        .enumerate()
        .filter_map(|(position, field_id_value)| {
            let field_id = value_id_to_string(field_id_value)?;
            let field_item = rustdoc.index.get(&field_id)?;
            type_field_info_from_item(field_item, position as u32)
        })
        .collect()
}

fn type_field_info_from_item(item: &RustdocItem, position: u32) -> Option<TypeFieldInfo> {
    let field_type = struct_field_value(item)?;
    let name = item.name.as_deref().and_then(|name| {
        if name.chars().all(|ch| ch.is_ascii_digit()) {
            None
        } else {
            Some(name.to_owned())
        }
    });

    Some(TypeFieldInfo {
        position,
        name,
        type_text: render_type(field_type),
        visibility_text: item.visibility.clone(),
        source: item
            .span
            .as_ref()
            .map(|span| code_ref_from_span(Some(span))),
    })
}

fn enum_variant_info_from_item(
    item: &RustdocItem,
    rustdoc: &RustdocResponse,
) -> Option<EnumVariantInfo> {
    let variant = item.inner.get("variant")?.as_object()?;
    let (kind, fields) = enum_variant_kind_and_fields(variant.get("kind"), rustdoc);

    Some(EnumVariantInfo {
        name: item.name.clone()?,
        kind,
        is_non_exhaustive: has_attr(&item.attrs, "non_exhaustive"),
        discriminant_text: variant_discriminant_text(variant.get("discriminant")),
        fields,
        source: item
            .span
            .as_ref()
            .map(|span| code_ref_from_span(Some(span))),
    })
}

fn enum_variant_kind_and_fields(
    kind_value: Option<&Value>,
    rustdoc: &RustdocResponse,
) -> (VariantKind, Vec<TypeFieldInfo>) {
    let Some(kind_value) = kind_value else {
        return (VariantKind::Unit, Vec::new());
    };

    if let Some(kind_text) = kind_value.as_str() {
        if kind_text == "plain" {
            return (VariantKind::Unit, Vec::new());
        }
    }

    let Some(kind) = kind_value.as_object() else {
        return (VariantKind::Unit, Vec::new());
    };

    if let Some(tuple) = kind.get("tuple").and_then(Value::as_array) {
        return (VariantKind::Tuple, fields_from_ids(tuple, rustdoc));
    }
    if let Some(struct_fields) = kind.get("struct").and_then(Value::as_object) {
        let fields = struct_fields
            .get("fields")
            .and_then(Value::as_array)
            .map(|fields| fields_from_ids(fields, rustdoc))
            .unwrap_or_default();
        return (VariantKind::Struct, fields);
    }

    (VariantKind::Unit, Vec::new())
}

fn variant_discriminant_text(discriminant_value: Option<&Value>) -> Option<String> {
    let discriminant = discriminant_value?.as_object()?;
    discriminant
        .get("value")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            discriminant
                .get("expr")
                .and_then(Value::as_str)
                .filter(|expr| !expr.is_empty())
                .map(ToOwned::to_owned)
        })
}

fn is_public(item: &RustdocItem) -> bool {
    item.visibility == "public"
}

fn is_module_item(item: &RustdocItem) -> bool {
    item.inner.get("module").is_some()
}

fn is_function_item(item: &RustdocItem) -> bool {
    item.inner.get("function").is_some()
}

fn is_trait_item(item: &RustdocItem) -> bool {
    item.inner.get("trait").is_some()
}

fn code_ref_from_span(span: Option<&RustdocSpan>) -> CodeRef {
    match span {
        Some(span) => CodeRef {
            file: span.filename.clone(),
            start_line: span.begin[0],
            end_line: span.end[0],
        },
        None => CodeRef {
            file: String::new(),
            start_line: 0,
            end_line: 0,
        },
    }
}

fn module_id_for_path(path: &str) -> ModuleId {
    ModuleId::from(format!("mod::{path}"))
}

fn type_id_for_path(path: &str) -> TypeId {
    TypeId::from(format!("type::{path}"))
}

fn api_id_for_path(path: &str) -> ApiId {
    ApiId::from(format!("api::{path}"))
}

fn symbol_id_for_path(path: &str) -> SymbolId {
    SymbolId::from(format!("symbol::{path}"))
}

fn trait_id_for_path(path: &str) -> TraitId {
    TraitId::from(format!("trait::{path}"))
}

fn join_path_segments(path: &[String]) -> String {
    path.join("::")
}

fn join_public_path(module_path: &str, name: &str) -> String {
    format!("{module_path}::{name}")
}

fn parent_path(path: &str) -> Option<String> {
    path.rsplit_once("::").map(|(parent, _)| parent.to_owned())
}

fn public_anchor_module_id(
    module_ids: &BTreeMap<String, ModuleId>,
    canonical_path: &str,
    public_paths: &[String],
) -> Option<ModuleId> {
    if let Some(module_id) =
        parent_path(canonical_path).and_then(|path| module_ids.get(&path).cloned())
    {
        return Some(module_id);
    }

    public_paths
        .iter()
        .filter_map(|path| parent_path(path))
        .filter_map(|path| module_ids.get(&path).cloned())
        .max_by_key(|module_id| module_id.as_str().len())
}

fn sorted_paths(paths: Option<&BTreeSet<String>>) -> Vec<String> {
    paths
        .map(|values| values.iter().cloned().collect())
        .unwrap_or_default()
}

fn merge_sorted_paths(existing: &mut Vec<String>, extra: &BTreeSet<String>) {
    let mut merged = existing.iter().cloned().collect::<BTreeSet<String>>();
    merged.extend(extra.iter().cloned());
    *existing = merged.into_iter().collect();
}

fn push_unique_exposure_kind(
    exposure_kinds: &mut Vec<TraitExposureKind>,
    new_kind: TraitExposureKind,
) {
    if !exposure_kinds.contains(&new_kind) {
        exposure_kinds.push(new_kind);
    }
}

fn push_unique_api_id(api_ids: &mut Vec<ApiId>, api_id: &ApiId) {
    if !api_ids.contains(api_id) {
        api_ids.push(api_id.clone());
    }
}

fn push_unique_trait_id(trait_ids: &mut Vec<TraitId>, trait_id: &TraitId) {
    if !trait_ids.contains(trait_id) {
        trait_ids.push(trait_id.clone());
    }
}

fn push_unique_type_id(type_ids: &mut Vec<TypeId>, type_id: &TypeId) {
    if !type_ids.contains(type_id) {
        type_ids.push(type_id.clone());
    }
}

fn merge_trait_info(existing: &mut TraitInfo, incoming: TraitInfo) {
    merge_sorted_paths(
        &mut existing.public_paths,
        &incoming.public_paths.iter().cloned().collect(),
    );
    if existing.public_anchor_module_id.is_none() {
        existing.public_anchor_module_id = incoming.public_anchor_module_id;
    }
    if existing.code_ref.is_none() {
        existing.code_ref = incoming.code_ref;
    }
    if existing.docs.is_empty() && !incoming.docs.is_empty() {
        existing.docs = incoming.docs;
    }
    if existing.doc_sections == DocSections::default()
        && incoming.doc_sections != DocSections::default()
    {
        existing.doc_sections = incoming.doc_sections;
    }
    if origin_rank(incoming.origin.clone()) > origin_rank(existing.origin.clone()) {
        existing.origin = incoming.origin;
    }
    existing.is_unsafe |= incoming.is_unsafe;
    merge_trait_ids(
        &mut existing.direct_supertrait_ids,
        incoming.direct_supertrait_ids,
    );
    merge_strings(&mut existing.required_methods, incoming.required_methods);
    merge_strings(&mut existing.provided_methods, incoming.provided_methods);
    merge_associated_type_defs(
        &mut existing.associated_type_defs,
        incoming.associated_type_defs,
    );
    merge_associated_const_defs(
        &mut existing.associated_const_defs,
        incoming.associated_const_defs,
    );
    // Reverse edges can be discovered from multiple surfaces; merge them just
    // like supertrait ids instead of letting later passes overwrite earlier ones.
    merge_trait_ids(&mut existing.used_by_trait_ids, incoming.used_by_trait_ids);
    for exposure_kind in incoming.exposure_kinds {
        push_unique_exposure_kind(&mut existing.exposure_kinds, exposure_kind);
    }
}

fn origin_rank(origin: TraitOrigin) -> u8 {
    match origin {
        TraitOrigin::External => 0,
        TraitOrigin::Reexported => 1,
        TraitOrigin::Local => 2,
    }
}

fn merge_trait_ids(existing: &mut Vec<TraitId>, incoming: Vec<TraitId>) {
    let mut merged = existing.iter().cloned().collect::<BTreeSet<_>>();
    merged.extend(incoming);
    *existing = merged.into_iter().collect();
}

fn merge_strings(existing: &mut Vec<String>, incoming: Vec<String>) {
    let mut merged = existing.iter().cloned().collect::<BTreeSet<_>>();
    merged.extend(incoming);
    *existing = merged.into_iter().collect();
}

fn merge_associated_type_defs(
    existing: &mut Vec<TraitAssociatedTypeDef>,
    incoming: Vec<TraitAssociatedTypeDef>,
) {
    let mut merged = existing
        .iter()
        .cloned()
        .map(|assoc_type| (assoc_type.name.clone(), assoc_type))
        .collect::<BTreeMap<_, _>>();

    for assoc_type in incoming {
        merged.entry(assoc_type.name.clone()).or_insert(assoc_type);
    }

    *existing = merged.into_values().collect();
}

fn merge_associated_const_defs(
    existing: &mut Vec<TraitAssociatedConstDef>,
    incoming: Vec<TraitAssociatedConstDef>,
) {
    let mut merged = existing
        .iter()
        .cloned()
        .map(|assoc_const| (assoc_const.name.clone(), assoc_const))
        .collect::<BTreeMap<_, _>>();

    for assoc_const in incoming {
        merged
            .entry(assoc_const.name.clone())
            .or_insert(assoc_const);
    }

    *existing = merged.into_values().collect();
}

fn trait_impl_id_for_surface(trait_ref_text: &str, for_type_text: &str) -> TraitImplId {
    TraitImplId::from(format!(
        "trait_impl::{trait_ref_text}::for::{for_type_text}"
    ))
}

fn is_public_trait_impl_surface(trait_ref: &TraitRefInfo, rustdoc: &RustdocResponse) -> bool {
    let Some(item_id) = &trait_ref.item_id else {
        return true;
    };
    let Some(path_entry) = rustdoc.paths.get(item_id) else {
        return true;
    };
    if path_entry.crate_id != 0 {
        return true;
    }

    rustdoc
        .index
        .get(item_id)
        .is_some_and(|item| is_public(item) && is_trait_item(item))
}

fn public_type_context_from_impl_target<'a>(
    value: &'a Value,
    type_contexts: &'a BTreeMap<String, TypeContext>,
) -> Option<&'a TypeContext> {
    let object = value.as_object()?;

    if let Some(resolved_path) = object.get("resolved_path").and_then(Value::as_object) {
        let item_id = resolved_path.get("id").and_then(value_id_to_string)?;
        return type_contexts.get(&item_id);
    }
    if let Some(borrowed_ref) = object.get("borrowed_ref").and_then(Value::as_object) {
        return borrowed_ref
            .get("type")
            .and_then(|inner| public_type_context_from_impl_target(inner, type_contexts));
    }
    if let Some(raw_pointer) = object.get("raw_pointer").and_then(Value::as_object) {
        return raw_pointer
            .get("type")
            .and_then(|inner| public_type_context_from_impl_target(inner, type_contexts));
    }
    if let Some(array) = object.get("array").and_then(Value::as_object) {
        return array
            .get("type")
            .and_then(|inner| public_type_context_from_impl_target(inner, type_contexts));
    }
    if let Some(slice) = object.get("slice") {
        return public_type_context_from_impl_target(slice, type_contexts);
    }
    if let Some(qualified_path) = object.get("qualified_path").and_then(Value::as_object) {
        return qualified_path
            .get("self_type")
            .and_then(|inner| public_type_context_from_impl_target(inner, type_contexts));
    }

    None
}

fn collect_trait_refs_from_value(
    value: &Value,
    rustdoc: &RustdocResponse,
    refs: &mut BTreeMap<String, TraitRefInfo>,
) {
    match value {
        Value::Object(object) => {
            if let Some(trait_ref) = object
                .get("trait_bound")
                .and_then(|trait_bound| trait_bound.get("trait"))
                .and_then(|trait_value| trait_ref_from_value(trait_value, rustdoc))
            {
                refs.insert(trait_ref.canonical_path.clone(), trait_ref);
            }
            if let Some(trait_ref) = object
                .get("trait")
                .and_then(|trait_value| trait_ref_from_value(trait_value, rustdoc))
            {
                refs.insert(trait_ref.canonical_path.clone(), trait_ref);
            }

            for nested in object.values() {
                collect_trait_refs_from_value(nested, rustdoc, refs);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_trait_refs_from_value(item, rustdoc, refs);
            }
        }
        _ => {}
    }
}

fn trait_ref_from_canonical_path(
    rustdoc: &RustdocResponse,
    canonical_path: &str,
) -> Option<TraitRefInfo> {
    rustdoc
        .paths
        .iter()
        .find(|(_, path_entry)| {
            path_entry.kind == "trait" && join_path_segments(&path_entry.path) == canonical_path
        })
        .map(|(item_id, path_entry)| TraitRefInfo {
            item_id: Some(item_id.clone()),
            canonical_path: join_path_segments(&path_entry.path),
            name: path_entry.path.last().cloned().unwrap_or_default(),
        })
}

fn trait_ref_from_value(value: &Value, rustdoc: &RustdocResponse) -> Option<TraitRefInfo> {
    let item_id = value.get("id").and_then(value_id_to_string);
    let canonical_path = item_id
        .as_ref()
        .and_then(|id| rustdoc.paths.get(id))
        .map(|path_entry| join_path_segments(&path_entry.path))
        .or_else(|| {
            value
                .get("path")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })?;
    let name = canonical_path
        .rsplit("::")
        .next()
        .unwrap_or_default()
        .to_owned();
    Some(TraitRefInfo {
        item_id,
        canonical_path,
        name,
    })
}

fn trait_info_from_ref(
    trait_ref: &TraitRefInfo,
    exposure_kind: TraitExposureKind,
    rustdoc: &RustdocResponse,
    module_ids: &BTreeMap<String, ModuleId>,
    public_paths: &PublicPathState,
) -> TraitInfo {
    let public_paths_vec = trait_ref
        .item_id
        .as_ref()
        .map(|item_id| sorted_paths(public_paths.public_paths.get(item_id)))
        .unwrap_or_default();
    let public_anchor_module_id = if public_paths_vec.is_empty() {
        None
    } else {
        public_anchor_module_id(module_ids, &trait_ref.canonical_path, &public_paths_vec)
    };
    let path_entry = trait_ref
        .item_id
        .as_ref()
        .and_then(|item_id| rustdoc.paths.get(item_id));
    let item = trait_ref
        .item_id
        .as_ref()
        .and_then(|item_id| rustdoc.index.get(item_id))
        .filter(|item| is_trait_item(item));

    let origin = match path_entry.map(|path_entry| path_entry.crate_id) {
        Some(0) => TraitOrigin::Local,
        Some(_) if !public_paths_vec.is_empty() => TraitOrigin::Reexported,
        _ => TraitOrigin::External,
    };

    TraitInfo {
        trait_id: trait_id_for_path(&trait_ref.canonical_path),
        name: item
            .and_then(|item| item.name.clone())
            .unwrap_or_else(|| trait_ref.name.clone()),
        canonical_path: trait_ref.canonical_path.clone(),
        public_paths: public_paths_vec,
        public_anchor_module_id,
        code_ref: item
            .and_then(|item| item.span.as_ref())
            .map(|span| code_ref_from_span(Some(span))),
        docs: item.and_then(|item| item.docs.clone()).unwrap_or_default(),
        doc_sections: doc_sections_from_docs(
            item.and_then(|trait_item| trait_item.docs.as_deref())
                .unwrap_or_default(),
        ),
        origin,
        exposure_kinds: vec![exposure_kind],
        is_unsafe: item
            .and_then(|trait_item| trait_value(trait_item))
            .and_then(|trait_value| trait_value.get("is_unsafe"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        direct_supertrait_ids: item
            .map(|item| direct_supertrait_ids(item, rustdoc))
            .unwrap_or_default(),
        required_methods: item
            .map(|item| trait_method_names(item, rustdoc, "required"))
            .unwrap_or_default(),
        provided_methods: item
            .map(|item| trait_method_names(item, rustdoc, "provided"))
            .unwrap_or_default(),
        associated_type_defs: item
            .map(|item| trait_associated_type_defs(item, rustdoc))
            .unwrap_or_default(),
        associated_const_defs: item
            .map(|item| trait_associated_const_defs(item, rustdoc))
            .unwrap_or_default(),
        used_by_api_ids: Vec::new(),
        used_by_trait_ids: Vec::new(),
        used_by_type_ids: Vec::new(),
    }
}

fn value_id_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn type_kind_from_path_kind(path_kind: &str) -> Option<TypeKind> {
    match path_kind {
        "struct" => Some(TypeKind::Struct),
        "enum" => Some(TypeKind::Enum),
        "union" => Some(TypeKind::Union),
        "type_alias" => Some(TypeKind::TypeAlias),
        _ => None,
    }
}

fn symbol_kind_from_path_kind(path_kind: &str) -> Option<SymbolKind> {
    match path_kind {
        "macro" => Some(SymbolKind::Macro),
        "constant" => Some(SymbolKind::Constant),
        _ => None,
    }
}

fn generic_param_names(generics_value: Option<&Value>) -> Vec<String> {
    generics_value
        .and_then(|generics| generics.get("params"))
        .and_then(Value::as_array)
        .map(|params| {
            params
                .iter()
                .filter_map(generic_param_name)
                .collect()
        })
        .unwrap_or_default()
}

fn generic_param_name(param: &Value) -> Option<String> {
    let name = param.get("name").and_then(Value::as_str)?;
    let kind = param.get("kind").and_then(Value::as_object)?;

    if kind.get("lifetime").is_some() || kind.get("const").is_some() {
        return Some(name.to_owned());
    }

    if let Some(type_param) = kind.get("type").and_then(Value::as_object) {
        if type_param
            .get("is_synthetic")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return None;
        }
        return Some(name.to_owned());
    }

    Some(name.to_owned())
}

fn macro_signature_text(item: &RustdocItem) -> Option<String> {
    item.inner
        .get("macro")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn constant_type_text(item: &RustdocItem) -> Option<String> {
    item.inner
        .get("constant")
        .and_then(|constant| constant.get("type"))
        .map(render_type)
}

fn constant_value_text(item: &RustdocItem) -> Option<String> {
    let const_value = item.inner.get("constant")?.get("const")?;
    if let Some(value) = const_value.get("value") {
        if !value.is_null() {
            return value
                .as_str()
                .map(ToOwned::to_owned)
                .or_else(|| serde_json::to_string(value).ok());
        }
    }

    const_value
        .get("expr")
        .and_then(Value::as_str)
        .filter(|expr| *expr != "_")
        .map(ToOwned::to_owned)
}

fn where_clause_strings(generics_value: Option<&Value>) -> Vec<String> {
    let mut predicates = inline_generic_param_bound_strings(generics_value);

    if let Some(where_predicates) = generics_value
        .and_then(|generics| generics.get("where_predicates"))
        .and_then(Value::as_array)
    {
        for predicate in where_predicates
            .iter()
            .map(render_where_predicate)
            .filter(|predicate| !predicate.is_empty())
        {
            if !predicates.contains(&predicate) {
                predicates.push(predicate);
            }
        }
    }

    predicates
}

fn inline_generic_param_bound_strings(generics_value: Option<&Value>) -> Vec<String> {
    generics_value
        .and_then(|generics| generics.get("params"))
        .and_then(Value::as_array)
        .map(|params| {
            params
                .iter()
                .filter_map(render_inline_generic_param_bound)
                .collect()
        })
        .unwrap_or_default()
}

fn render_inline_generic_param_bound(param: &Value) -> Option<String> {
    let name = param.get("name").and_then(Value::as_str)?;
    let kind = param.get("kind").and_then(Value::as_object)?;

    if let Some(type_param) = kind.get("type").and_then(Value::as_object) {
        if type_param
            .get("is_synthetic")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return None;
        }

        let bounds = type_param
            .get("bounds")
            .and_then(Value::as_array)
            .map(|bounds| {
                bounds
                    .iter()
                    .map(render_generic_bound)
                    .filter(|bound| !bound.is_empty())
                    .collect::<Vec<_>>()
                    .join(" + ")
            })
            .unwrap_or_default();

        return if bounds.is_empty() {
            None
        } else {
            Some(format!("{name}: {bounds}"))
        };
    }

    if let Some(lifetime_param) = kind.get("lifetime").and_then(Value::as_object) {
        let bounds = lifetime_param
            .get("outlives")
            .and_then(Value::as_array)
            .map(|bounds| {
                bounds
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" + ")
            })
            .unwrap_or_default();

        return if bounds.is_empty() {
            None
        } else {
            Some(format!("{name}: {bounds}"))
        };
    }

    None
}

fn render_where_predicate(predicate: &Value) -> String {
    if let Some(bound_predicate) = predicate.get("bound_predicate").and_then(Value::as_object) {
        return render_bound_predicate(bound_predicate);
    }
    if let Some(region_predicate) = predicate.get("region_predicate").and_then(Value::as_object) {
        return render_region_predicate(region_predicate);
    }
    if let Some(eq_predicate) = predicate
        .get("eq_predicate")
        .and_then(Value::as_object)
        .or_else(|| {
            predicate
                .get("equality_predicate")
                .and_then(Value::as_object)
        })
    {
        return render_eq_predicate(eq_predicate);
    }
    serde_json::to_string(predicate).unwrap_or_default()
}

fn render_bound_predicate(bound_predicate: &Map<String, Value>) -> String {
    let binder = render_binder(bound_predicate.get("generic_params"));
    let lhs = bound_predicate
        .get("type")
        .map(render_type)
        .unwrap_or_else(|| "_".to_owned());
    let bounds = bound_predicate
        .get("bounds")
        .and_then(Value::as_array)
        .map(|bounds| {
            bounds
                .iter()
                .map(render_generic_bound)
                .filter(|bound| !bound.is_empty())
                .collect::<Vec<_>>()
                .join(" + ")
        })
        .unwrap_or_default();

    if bounds.is_empty() {
        lhs
    } else if binder.is_empty() {
        format!("{lhs}: {bounds}")
    } else {
        format!("{binder}{lhs}: {bounds}")
    }
}

fn render_region_predicate(region_predicate: &Map<String, Value>) -> String {
    let lifetime = region_predicate
        .get("lifetime")
        .and_then(Value::as_str)
        .unwrap_or("'_");
    let bounds = region_predicate
        .get("bounds")
        .and_then(Value::as_array)
        .map(|bounds| {
            bounds
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" + ")
        })
        .unwrap_or_default();

    if bounds.is_empty() {
        lifetime.to_owned()
    } else {
        format!("{lifetime}: {bounds}")
    }
}

fn render_eq_predicate(eq_predicate: &Map<String, Value>) -> String {
    let lhs = eq_predicate
        .get("lhs")
        .or_else(|| eq_predicate.get("type"))
        .map(render_term)
        .unwrap_or_else(|| "_".to_owned());
    let rhs = eq_predicate
        .get("rhs")
        .or_else(|| eq_predicate.get("term"))
        .map(render_term)
        .unwrap_or_else(|| "_".to_owned());
    format!("{lhs} = {rhs}")
}

fn render_term(value: &Value) -> String {
    if let Some(ty) = value.get("type") {
        return render_type(ty);
    }
    if let Some(constant) = value.get("constant") {
        return constant
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| serde_json::to_string(constant).unwrap_or_default());
    }
    if let Some(generic) = value.get("generic").and_then(Value::as_str) {
        return generic.to_owned();
    }
    render_type(value)
}

fn render_binder(params_value: Option<&Value>) -> String {
    let names = params_value
        .and_then(Value::as_array)
        .map(|params| {
            params
                .iter()
                .filter_map(|param| param.get("name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if names.is_empty() {
        String::new()
    } else {
        format!("for<{}> ", names.join(", "))
    }
}

fn render_generic_bound(bound: &Value) -> String {
    if let Some(trait_bound) = bound.get("trait_bound").and_then(Value::as_object) {
        return render_trait_bound(trait_bound);
    }
    if let Some(outlives) = bound.get("outlives").and_then(Value::as_str) {
        return outlives.to_owned();
    }
    serde_json::to_string(bound).unwrap_or_default()
}

fn render_trait_bound(trait_bound: &Map<String, Value>) -> String {
    let binder = render_binder(trait_bound.get("generic_params"));
    let trait_text = trait_bound
        .get("trait")
        .map(render_trait_ref)
        .unwrap_or_else(|| "_".to_owned());
    let text = if binder.is_empty() {
        trait_text
    } else {
        format!("{binder}{trait_text}")
    };

    match trait_bound
        .get("modifier")
        .and_then(Value::as_str)
        .unwrap_or("none")
    {
        "maybe" => format!("?{text}"),
        "maybe_const" => format!("~const {text}"),
        "none" => text,
        other => format!("{other} {text}"),
    }
}

fn render_trait_ref(value: &Value) -> String {
    let Some(object) = value.as_object() else {
        return serde_json::to_string(value).unwrap_or_default();
    };

    let base = object
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let rendered_args = object
        .get("args")
        .and_then(render_path_args_suffix)
        .unwrap_or_default();

    format!("{base}{rendered_args}")
}

fn render_canonical_trait_ref(value: &Value, canonical_path: &str) -> String {
    let Some(object) = value.as_object() else {
        return canonical_path.to_owned();
    };
    let rendered_args = object
        .get("args")
        .and_then(render_path_args_suffix)
        .unwrap_or_default();

    if rendered_args.is_empty() {
        canonical_path.to_owned()
    } else {
        format!("{canonical_path}{rendered_args}")
    }
}

fn receiver_string(function_value: &Value) -> Option<String> {
    let inputs = function_value
        .get("sig")
        .and_then(|sig| sig.get("inputs"))
        .and_then(Value::as_array)?;
    let first = inputs.first()?.as_array()?;
    let name = first.first()?.as_str()?;
    if name != "self" {
        return None;
    }
    first.get(1).map(render_type)
}

fn argument_types(function_value: &Value) -> Vec<String> {
    function_value
        .get("sig")
        .and_then(|sig| sig.get("inputs"))
        .and_then(Value::as_array)
        .map(|inputs| {
            inputs
                .iter()
                .filter_map(Value::as_array)
                .filter_map(|input| {
                    let is_self = input
                        .first()
                        .and_then(Value::as_str)
                        .is_some_and(|name| name == "self");
                    if is_self {
                        None
                    } else {
                        input.get(1).map(render_type)
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn return_type_string(function_value: &Value) -> Option<String> {
    function_value
        .get("sig")
        .and_then(|sig| sig.get("output"))
        .filter(|output| !output.is_null())
        .map(render_type)
}

fn return_shape_from_function(function_value: &Value) -> Option<ReturnShape> {
    let output = function_value
        .get("sig")
        .and_then(|sig| sig.get("output"))?;
    Some(return_shape_from_type(output))
}

fn return_shape_from_type(value: &Value) -> ReturnShape {
    if value.is_null() {
        return ReturnShape {
            kind: ReturnShapeKind::Unit,
            inner_types: Vec::new(),
        };
    }

    let Some(object) = value.as_object() else {
        return ReturnShape {
            kind: ReturnShapeKind::Other,
            inner_types: Vec::new(),
        };
    };

    if let Some(primitive) = object.get("primitive").and_then(Value::as_str) {
        let kind = if matches!(primitive, "never" | "!") {
            ReturnShapeKind::Never
        } else {
            ReturnShapeKind::Primitive
        };
        return ReturnShape {
            kind,
            inner_types: Vec::new(),
        };
    }

    if let Some(borrowed_ref) = object.get("borrowed_ref").and_then(Value::as_object) {
        let kind = if borrowed_ref
            .get("is_mutable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            ReturnShapeKind::RefMut
        } else {
            ReturnShapeKind::Ref
        };
        return ReturnShape {
            kind,
            inner_types: borrowed_ref
                .get("type")
                .map(render_type)
                .into_iter()
                .collect(),
        };
    }

    if let Some(raw_pointer) = object.get("raw_pointer").and_then(Value::as_object) {
        return ReturnShape {
            kind: ReturnShapeKind::RawPtr,
            inner_types: raw_pointer
                .get("type")
                .map(render_type)
                .into_iter()
                .collect(),
        };
    }

    if let Some(tuple) = object.get("tuple").and_then(Value::as_array) {
        return ReturnShape {
            kind: ReturnShapeKind::Tuple,
            inner_types: tuple.iter().map(render_type).collect(),
        };
    }

    if let Some(array) = object.get("array").and_then(Value::as_object) {
        return ReturnShape {
            kind: ReturnShapeKind::Array,
            inner_types: array.get("type").map(render_type).into_iter().collect(),
        };
    }

    if let Some(slice) = object.get("slice") {
        return ReturnShape {
            kind: ReturnShapeKind::Slice,
            inner_types: vec![render_type(slice)],
        };
    }

    if let Some(dyn_trait) = object.get("dyn_trait").and_then(Value::as_object) {
        return ReturnShape {
            kind: ReturnShapeKind::DynTrait,
            inner_types: dyn_trait_bounds_texts(dyn_trait),
        };
    }

    if let Some(impl_trait) = object.get("impl_trait").and_then(Value::as_array) {
        return ReturnShape {
            kind: ReturnShapeKind::ImplTrait,
            inner_types: impl_trait.iter().map(render_generic_bound).collect(),
        };
    }

    if let Some(resolved_path) = object.get("resolved_path").and_then(Value::as_object) {
        let path = resolved_path
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let kind = match path {
            "Result" | "core::result::Result" | "std::result::Result" => ReturnShapeKind::Result,
            "Option" | "core::option::Option" | "std::option::Option" => ReturnShapeKind::Option,
            _ => ReturnShapeKind::Nominal,
        };
        return ReturnShape {
            kind,
            inner_types: top_level_type_args(resolved_path.get("args")),
        };
    }

    if object.get("qualified_path").is_some() {
        return ReturnShape {
            kind: ReturnShapeKind::Nominal,
            inner_types: Vec::new(),
        };
    }

    ReturnShape {
        kind: ReturnShapeKind::Other,
        inner_types: Vec::new(),
    }
}

fn top_level_type_args(args_value: Option<&Value>) -> Vec<String> {
    args_value
        .and_then(|args| args.get("angle_bracketed"))
        .and_then(|angle| angle.get("args"))
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .filter_map(|arg| arg.get("type"))
                .map(render_type)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn dyn_trait_bounds_texts(dyn_trait: &Map<String, Value>) -> Vec<String> {
    dyn_trait
        .get("traits")
        .and_then(Value::as_array)
        .map(|traits| traits.iter().filter_map(render_dyn_trait_entry).collect())
        .unwrap_or_default()
}

fn function_header_flag(function_value: &Value, field: &str) -> bool {
    function_value
        .get("header")
        .and_then(|header| header.get(field))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn signature_text(
    name: &str,
    receiver: Option<&str>,
    arg_types: &[String],
    return_type: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    if let Some(receiver) = receiver {
        parts.push(receiver.to_owned());
    }
    parts.extend(arg_types.iter().cloned());

    let args = parts.join(", ");
    match return_type {
        Some(output) => format!("fn {name}({args}) -> {output}"),
        None => format!("fn {name}({args})"),
    }
}

fn render_type(value: &Value) -> String {
    match value {
        Value::Object(object) => {
            if let Some(primitive) = object.get("primitive").and_then(Value::as_str) {
                return primitive.to_owned();
            }
            if let Some(impl_trait) = object.get("impl_trait").and_then(Value::as_array) {
                let bounds = impl_trait
                    .iter()
                    .map(render_generic_bound)
                    .filter(|bound| !bound.is_empty())
                    .collect::<Vec<_>>()
                    .join(" + ");
                return if bounds.is_empty() {
                    "impl _".to_owned()
                } else {
                    format!("impl {bounds}")
                };
            }
            if let Some(generic) = object.get("generic").and_then(Value::as_str) {
                return generic.to_owned();
            }
            if let Some(qualified_path) = object.get("qualified_path").and_then(Value::as_object) {
                let name = qualified_path
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("_");
                let args_suffix = qualified_path
                    .get("args")
                    .and_then(render_path_args_suffix)
                    .unwrap_or_default();
                let self_type = qualified_path
                    .get("self_type")
                    .map(render_type)
                    .unwrap_or_else(|| "_".to_owned());
                let trait_text = qualified_path
                    .get("trait")
                    .map(render_trait_ref)
                    .unwrap_or_default();

                if trait_text.is_empty() {
                    return format!("{self_type}::{name}{args_suffix}");
                }

                return format!("<{self_type} as {trait_text}>::{name}{args_suffix}");
            }
            if let Some(resolved_path) = object.get("resolved_path").and_then(Value::as_object) {
                let base = resolved_path
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let rendered_args = resolved_path
                    .get("args")
                    .and_then(render_path_args_suffix)
                    .unwrap_or_default();
                return if rendered_args.is_empty() {
                    base
                } else {
                    format!("{base}{rendered_args}")
                };
            }
            if let Some(dyn_trait) = object.get("dyn_trait").and_then(Value::as_object) {
                return render_dyn_trait_object(dyn_trait);
            }
            if let Some(borrowed_ref) = object.get("borrowed_ref").and_then(Value::as_object) {
                let mut prefix = "&".to_owned();
                if let Some(lifetime) = borrowed_ref.get("lifetime").and_then(Value::as_str) {
                    prefix.push_str(lifetime);
                    prefix.push(' ');
                }
                if borrowed_ref
                    .get("is_mutable")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    prefix.push_str("mut ");
                }
                let inner = borrowed_ref
                    .get("type")
                    .map(render_type)
                    .unwrap_or_else(|| "_".to_owned());
                return format!("{prefix}{inner}");
            }
            if let Some(tuple) = object.get("tuple").and_then(Value::as_array) {
                let inner = tuple.iter().map(render_type).collect::<Vec<_>>().join(", ");
                return format!("({inner})");
            }
            if let Some(slice) = object.get("slice") {
                return format!("[{}]", render_type(slice));
            }
            if let Some(array) = object.get("array").and_then(Value::as_object) {
                let item = array
                    .get("type")
                    .map(render_type)
                    .unwrap_or_else(|| "_".to_owned());
                return format!("[{item}; _]");
            }
            if let Some(raw_pointer) = object.get("raw_pointer").and_then(Value::as_object) {
                let prefix = if raw_pointer
                    .get("is_mutable")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "*mut "
                } else {
                    "*const "
                };
                let inner = raw_pointer
                    .get("type")
                    .map(render_type)
                    .unwrap_or_else(|| "_".to_owned());
                return format!("{prefix}{inner}");
            }
            serde_json::to_string(value).unwrap_or_default()
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn render_dyn_trait_object(dyn_trait: &Map<String, Value>) -> String {
    let mut parts = dyn_trait
        .get("traits")
        .and_then(Value::as_array)
        .map(|traits| {
            traits
                .iter()
                .filter_map(render_dyn_trait_entry)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(lifetime) = dyn_trait.get("lifetime").and_then(Value::as_str) {
        parts.push(lifetime.to_owned());
    }

    if parts.is_empty() {
        "dyn _".to_owned()
    } else {
        format!("dyn {}", parts.join(" + "))
    }
}

fn render_dyn_trait_entry(entry: &Value) -> Option<String> {
    let object = entry.as_object()?;
    let trait_text = object.get("trait").map(render_trait_ref)?;
    let binder = render_binder(object.get("generic_params"));
    if binder.is_empty() {
        Some(trait_text)
    } else {
        Some(format!("{binder}{trait_text}"))
    }
}

fn render_path_args_suffix(args_value: &Value) -> Option<String> {
    if args_value.get("angle_bracketed").is_some() {
        return render_angle_bracketed_args_suffix(args_value);
    }
    if args_value.get("parenthesized").is_some() {
        return render_parenthesized_args_suffix(args_value);
    }
    None
}

fn render_angle_bracketed_args_suffix(args_value: &Value) -> Option<String> {
    let angle = args_value.get("angle_bracketed")?;
    let mut rendered = Vec::new();

    if let Some(args) = angle.get("args").and_then(Value::as_array) {
        rendered.extend(args.iter().filter_map(render_generic_arg));
    }
    if let Some(constraints) = angle.get("constraints").and_then(Value::as_array) {
        rendered.extend(constraints.iter().map(render_assoc_constraint));
    }

    if rendered.is_empty() {
        None
    } else {
        Some(format!("<{}>", rendered.join(", ")))
    }
}

fn render_parenthesized_args_suffix(args_value: &Value) -> Option<String> {
    let parenthesized = args_value.get("parenthesized")?;
    let inputs = parenthesized
        .get("inputs")
        .and_then(Value::as_array)
        .map(|inputs| {
            inputs
                .iter()
                .map(render_type)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let output = parenthesized.get("output");

    let mut rendered = format!("({inputs})");
    if let Some(output) = output.filter(|value| !value.is_null()) {
        rendered.push_str(" -> ");
        rendered.push_str(&render_type(output));
    }
    Some(rendered)
}

fn render_generic_arg(arg: &Value) -> Option<String> {
    if let Some(ty) = arg.get("type") {
        return Some(render_type(ty));
    }
    if let Some(lifetime) = arg.get("lifetime").and_then(Value::as_str) {
        return Some(lifetime.to_owned());
    }
    arg.get("const").map(render_type)
}

fn render_assoc_constraint(constraint: &Value) -> String {
    let name = constraint
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("_");
    let args_suffix = constraint
        .get("args")
        .and_then(render_path_args_suffix)
        .unwrap_or_default();

    if let Some(equality_type) = constraint
        .get("binding")
        .and_then(|binding| binding.get("equality"))
        .and_then(|equality| equality.get("type"))
    {
        return format!("{name}{args_suffix} = {}", render_type(equality_type));
    }

    if let Some(constraints) = constraint
        .get("binding")
        .and_then(|binding| {
            binding
                .get("constraints")
                .or_else(|| binding.get("constraint"))
        })
        .and_then(Value::as_array)
    {
        let rendered = constraints
            .iter()
            .map(render_generic_bound)
            .filter(|bound| !bound.is_empty())
            .collect::<Vec<_>>()
            .join(" + ");
        if !rendered.is_empty() {
            return format!("{name}{args_suffix}: {rendered}");
        }
    }

    format!("{name}{args_suffix}")
}

fn direct_supertrait_ids(item: &RustdocItem, rustdoc: &RustdocResponse) -> Vec<TraitId> {
    let Some(bounds) = item
        .inner
        .get("trait")
        .and_then(|trait_value| trait_value.get("bounds"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut trait_ids = BTreeSet::new();
    for bound in bounds {
        if let Some(canonical_path) = trait_canonical_path_from_bound(bound, rustdoc) {
            trait_ids.insert(trait_id_for_path(&canonical_path));
        }
    }
    trait_ids.into_iter().collect()
}

fn trait_canonical_path_from_bound(bound: &Value, rustdoc: &RustdocResponse) -> Option<String> {
    let trait_value = bound
        .get("trait_bound")
        .and_then(|trait_bound| trait_bound.get("trait"))?;
    if let Some(id) = trait_value.get("id").and_then(value_id_to_string) {
        if let Some(path_entry) = rustdoc.paths.get(&id) {
            return Some(join_path_segments(&path_entry.path));
        }
    }
    trait_value
        .get("path")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn trait_method_names(item: &RustdocItem, rustdoc: &RustdocResponse, mode: &str) -> Vec<String> {
    let Some(items) = item
        .inner
        .get("trait")
        .and_then(|trait_value| trait_value.get("items"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let provided = item
        .inner
        .get("trait")
        .and_then(|trait_value| trait_value.get("provided_trait_methods"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(value_id_to_string)
                .collect::<BTreeSet<String>>()
        })
        .unwrap_or_default();

    items
        .iter()
        .filter_map(value_id_to_string)
        .filter_map(|item_id| {
            let trait_item = rustdoc.index.get(&item_id)?;
            if !is_function_item(trait_item) {
                return None;
            }

            // Prefer per-method `has_body` when rustdoc emits it; some crates omit the
            // trait-level `provided_trait_methods` summary even though method bodies exist.
            let has_body = function_value(trait_item)
                .and_then(|function| function.get("has_body"))
                .and_then(Value::as_bool);
            let keep = match mode {
                "required" => has_body
                    .map(|has_body| !has_body)
                    .unwrap_or(!provided.contains(&item_id)),
                "provided" => has_body.unwrap_or(provided.contains(&item_id)),
                _ => false,
            };
            if keep {
                trait_item.name.clone()
            } else {
                None
            }
        })
        .collect()
}

fn trait_associated_type_defs(
    item: &RustdocItem,
    rustdoc: &RustdocResponse,
) -> Vec<TraitAssociatedTypeDef> {
    item.inner
        .get("trait")
        .and_then(|trait_value| trait_value.get("items"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(value_id_to_string)
                .filter_map(|item_id| {
                    let trait_item = rustdoc.index.get(&item_id)?;
                    trait_associated_type_def_from_item(trait_item)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn trait_associated_const_defs(
    item: &RustdocItem,
    rustdoc: &RustdocResponse,
) -> Vec<TraitAssociatedConstDef> {
    item.inner
        .get("trait")
        .and_then(|trait_value| trait_value.get("items"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(value_id_to_string)
                .filter_map(|item_id| {
                    let trait_item = rustdoc.index.get(&item_id)?;
                    trait_associated_const_def_from_item(trait_item)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn trait_associated_const_def_from_item(item: &RustdocItem) -> Option<TraitAssociatedConstDef> {
    let assoc_const = assoc_const_value(item)?;
    Some(TraitAssociatedConstDef {
        name: item.name.clone()?,
        type_text: assoc_const
            .get("type")
            .map(render_type)
            .unwrap_or_else(|| "_".to_owned()),
        default_value_text: assoc_const_value_text(assoc_const),
        source: item
            .span
            .as_ref()
            .map(|span| code_ref_from_span(Some(span))),
    })
}

fn trait_associated_type_def_from_item(item: &RustdocItem) -> Option<TraitAssociatedTypeDef> {
    let assoc_type = assoc_type_value(item)?;
    Some(TraitAssociatedTypeDef {
        name: item.name.clone()?,
        generic_params: generic_param_names(assoc_type.get("generics")),
        where_clauses: where_clause_strings(assoc_type.get("generics")),
        bounds: associated_type_bounds(assoc_type),
        default_type: assoc_type
            .get("type")
            .filter(|value| !value.is_null())
            .map(render_type),
        source: item
            .span
            .as_ref()
            .map(|span| code_ref_from_span(Some(span))),
    })
}

fn trait_impl_associated_type_bindings(
    impl_item: &RustdocItem,
    rustdoc: &RustdocResponse,
) -> Vec<TraitAssociatedTypeBinding> {
    impl_value(impl_item)
        .and_then(|impl_value| impl_value.get("items"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(value_id_to_string)
                .filter_map(|item_id| {
                    let impl_member_item = rustdoc.index.get(&item_id)?;
                    trait_associated_type_binding_from_item(impl_member_item)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn trait_impl_associated_const_bindings(
    impl_item: &RustdocItem,
    rustdoc: &RustdocResponse,
) -> Vec<TraitAssociatedConstBinding> {
    impl_value(impl_item)
        .and_then(|impl_value| impl_value.get("items"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(value_id_to_string)
                .filter_map(|item_id| {
                    let impl_member_item = rustdoc.index.get(&item_id)?;
                    trait_associated_const_binding_from_item(impl_member_item)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn trait_associated_const_binding_from_item(
    item: &RustdocItem,
) -> Option<TraitAssociatedConstBinding> {
    let assoc_const = assoc_const_value(item)?;
    Some(TraitAssociatedConstBinding {
        name: item.name.clone()?,
        value_text: assoc_const_value_text(assoc_const),
        source: item
            .span
            .as_ref()
            .map(|span| code_ref_from_span(Some(span))),
    })
}

fn assoc_const_value_text(assoc_const: &Value) -> Option<String> {
    let value = assoc_const.get("value")?;
    if value.is_null() {
        return None;
    }
    if let Some(text) = value.as_str() {
        if text == "_" {
            return None;
        }
        return Some(text.to_owned());
    }
    serde_json::to_string(value).ok()
}

fn trait_associated_type_binding_from_item(
    item: &RustdocItem,
) -> Option<TraitAssociatedTypeBinding> {
    let assoc_type = assoc_type_value(item)?;
    Some(TraitAssociatedTypeBinding {
        name: item.name.clone()?,
        generic_params: generic_param_names(assoc_type.get("generics")),
        where_clauses: where_clause_strings(assoc_type.get("generics")),
        bounds: associated_type_bounds(assoc_type),
        assigned_type: assoc_type
            .get("type")
            .filter(|value| !value.is_null())
            .map(render_type),
        source: item
            .span
            .as_ref()
            .map(|span| code_ref_from_span(Some(span))),
    })
}

fn associated_type_bounds(assoc_type: &Value) -> Vec<String> {
    assoc_type
        .get("bounds")
        .and_then(Value::as_array)
        .map(|bounds| {
            bounds
                .iter()
                .map(render_generic_bound)
                .filter(|bound| !bound.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[derive(Debug)]
struct ModuleRow {
    canonical_path: String,
    name: String,
    public_paths: Vec<String>,
    code_ref: CodeRef,
    docs: String,
}

#[derive(Debug)]
struct TypeRow {
    item_id: String,
    type_id: TypeId,
    name: String,
    canonical_path: String,
    public_paths: Vec<String>,
    public_anchor_module_id: ModuleId,
    code_ref: CodeRef,
    docs: String,
    kind: TypeKind,
    generic_params: Vec<String>,
    where_clauses: Vec<String>,
    is_non_exhaustive: bool,
    fields: Vec<TypeFieldInfo>,
    variants: Vec<EnumVariantInfo>,
    has_hidden_fields: bool,
    has_hidden_variants: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collect_trait_refs_from_generics_recurses_into_associated_type_constraints() {
        let generics = json!({
            "params": [
                {
                    "name": "I",
                    "kind": {
                        "type": {
                            "bounds": [
                                {
                                    "trait_bound": {
                                        "trait": {
                                            "path": "Iterator",
                                            "id": 1,
                                            "args": {
                                                "angle_bracketed": {
                                                    "args": [],
                                                    "constraints": [
                                                        {
                                                            "name": "Item",
                                                            "args": {
                                                                "angle_bracketed": {
                                                                    "args": [],
                                                                    "constraints": []
                                                                }
                                                            },
                                                            "binding": {
                                                                "constraints": [
                                                                    {
                                                                        "trait_bound": {
                                                                            "trait": {
                                                                                "path": "Clone",
                                                                                "id": 2,
                                                                                "args": {
                                                                                    "angle_bracketed": {
                                                                                        "args": [],
                                                                                        "constraints": []
                                                                                    }
                                                                                }
                                                                            },
                                                                            "generic_params": [],
                                                                            "modifier": "none"
                                                                        }
                                                                    }
                                                                ]
                                                            }
                                                        }
                                                    ]
                                                }
                                            }
                                        },
                                        "generic_params": [],
                                        "modifier": "none"
                                    }
                                }
                            ],
                            "default": null,
                            "is_synthetic": false
                        }
                    }
                }
            ],
            "where_predicates": []
        });
        let rustdoc = RustdocResponse {
            root: 0,
            index: BTreeMap::new(),
            paths: BTreeMap::from([
                (
                    "1".to_owned(),
                    RustdocPathEntry {
                        crate_id: 1,
                        path: vec![
                            "core".to_owned(),
                            "iter".to_owned(),
                            "traits".to_owned(),
                            "iterator".to_owned(),
                            "Iterator".to_owned(),
                        ],
                        kind: "trait".to_owned(),
                    },
                ),
                (
                    "2".to_owned(),
                    RustdocPathEntry {
                        crate_id: 1,
                        path: vec!["core".to_owned(), "clone".to_owned(), "Clone".to_owned()],
                        kind: "trait".to_owned(),
                    },
                ),
            ]),
        };

        let mut refs = BTreeMap::<String, TraitRefInfo>::new();
        collect_trait_refs_from_value(&generics, &rustdoc, &mut refs);
        let paths = refs
            .into_values()
            .map(|trait_ref| trait_ref.canonical_path)
            .collect::<BTreeSet<_>>();

        assert!(paths.contains("core::iter::traits::iterator::Iterator"));
        assert!(paths.contains("core::clone::Clone"));
    }

    #[test]
    fn build_trait_registry_includes_traits_from_trait_associated_type_bounds() {
        let associated_type_bound = json!({
            "trait_bound": {
                "trait": {
                    "path": "Iterator",
                    "id": 1,
                    "args": {
                        "angle_bracketed": {
                            "args": [],
                            "constraints": [
                                {
                                    "name": "Item",
                                    "args": {
                                        "angle_bracketed": {
                                            "args": [],
                                            "constraints": []
                                        }
                                    },
                                    "binding": {
                                        "constraints": [
                                            {
                                                "trait_bound": {
                                                    "trait": {
                                                        "path": "Clone",
                                                        "id": 2,
                                                        "args": {
                                                            "angle_bracketed": {
                                                                "args": [],
                                                                "constraints": []
                                                            }
                                                        }
                                                    },
                                                    "generic_params": [],
                                                    "modifier": "none"
                                                }
                                            }
                                        ]
                                    }
                                }
                            ]
                        }
                    }
                },
                "generic_params": [],
                "modifier": "none"
            }
        });
        let rustdoc = RustdocResponse {
            root: 0,
            index: BTreeMap::from([
                (
                    "10".to_owned(),
                    RustdocItem {
                        name: Some("MyTrait".to_owned()),
                        span: None,
                        visibility: "public".to_owned(),
                        docs: None,
                        attrs: vec![],
                        inner: json!({
                            "trait": {
                                "is_unsafe": false,
                                "bounds": [],
                                "items": ["11"]
                            }
                        }),
                    },
                ),
                (
                    "11".to_owned(),
                    RustdocItem {
                        name: Some("Iter".to_owned()),
                        span: None,
                        visibility: "public".to_owned(),
                        docs: None,
                        attrs: vec![],
                        inner: json!({
                            "assoc_type": {
                                "generics": {
                                    "params": [],
                                    "where_predicates": []
                                },
                                "bounds": [associated_type_bound],
                                "type": null
                            }
                        }),
                    },
                ),
            ]),
            paths: BTreeMap::from([
                (
                    "1".to_owned(),
                    RustdocPathEntry {
                        crate_id: 1,
                        path: vec![
                            "core".to_owned(),
                            "iter".to_owned(),
                            "traits".to_owned(),
                            "iterator".to_owned(),
                            "Iterator".to_owned(),
                        ],
                        kind: "trait".to_owned(),
                    },
                ),
                (
                    "2".to_owned(),
                    RustdocPathEntry {
                        crate_id: 1,
                        path: vec!["core".to_owned(), "clone".to_owned(), "Clone".to_owned()],
                        kind: "trait".to_owned(),
                    },
                ),
                (
                    "10".to_owned(),
                    RustdocPathEntry {
                        crate_id: 0,
                        path: vec!["demo".to_owned(), "MyTrait".to_owned()],
                        kind: "trait".to_owned(),
                    },
                ),
            ]),
        };
        let module_ids = BTreeMap::from([("demo".to_owned(), module_id_for_path("demo"))]);

        let traits = build_trait_registry(
            &rustdoc,
            &module_ids,
            &PublicPathState::default(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        let paths = traits
            .iter()
            .map(|trait_info| trait_info.canonical_path.clone())
            .collect::<BTreeSet<_>>();
        let iterator_trait = traits
            .iter()
            .find(|trait_info| {
                trait_info.canonical_path == "core::iter::traits::iterator::Iterator"
            })
            .expect("Iterator trait should be present");
        let clone_trait = traits
            .iter()
            .find(|trait_info| trait_info.canonical_path == "core::clone::Clone")
            .expect("Clone trait should be present");

        assert!(paths.contains("demo::MyTrait"));
        assert!(paths.contains("core::iter::traits::iterator::Iterator"));
        assert!(paths.contains("core::clone::Clone"));
        assert!(iterator_trait
            .used_by_trait_ids
            .contains(&TraitId::from("trait::demo::MyTrait")));
        assert!(clone_trait
            .used_by_trait_ids
            .contains(&TraitId::from("trait::demo::MyTrait")));
    }

    #[test]
    fn where_clause_strings_render_nested_associated_trait_bounds_from_constraint_key() {
        let generics: Value = serde_json::from_str(
            r#"{
                "params": [],
                "where_predicates": [
                    {
                        "bound_predicate": {
                            "type": { "generic": "I" },
                            "bounds": [
                                {
                                    "trait_bound": {
                                        "trait": {
                                            "path": "ParallelIterator",
                                            "id": 20,
                                            "args": {
                                                "angle_bracketed": {
                                                    "args": [],
                                                    "constraints": [
                                                        {
                                                            "name": "Item",
                                                            "args": {
                                                                "angle_bracketed": {
                                                                    "args": [],
                                                                    "constraints": []
                                                                }
                                                            },
                                                            "binding": {
                                                                "constraint": [
                                                                    {
                                                                        "trait_bound": {
                                                                            "trait": {
                                                                                "path": "IntoIterator",
                                                                                "id": 737,
                                                                                "args": {
                                                                                    "angle_bracketed": {
                                                                                        "args": [],
                                                                                        "constraints": [
                                                                                            {
                                                                                                "name": "Item",
                                                                                                "args": {
                                                                                                    "angle_bracketed": {
                                                                                                        "args": [],
                                                                                                        "constraints": []
                                                                                                    }
                                                                                                },
                                                                                                "binding": {
                                                                                                    "constraint": [
                                                                                                        {
                                                                                                            "trait_bound": {
                                                                                                                "trait": {
                                                                                                                    "path": "Send",
                                                                                                                    "id": 7,
                                                                                                                    "args": {
                                                                                                                        "angle_bracketed": {
                                                                                                                            "args": [],
                                                                                                                            "constraints": []
                                                                                                                        }
                                                                                                                    }
                                                                                                                },
                                                                                                                "generic_params": [],
                                                                                                                "modifier": "none"
                                                                                                            }
                                                                                                        }
                                                                                                    ]
                                                                                                }
                                                                                            }
                                                                                        ]
                                                                                    }
                                                                                }
                                                                            },
                                                                            "generic_params": [],
                                                                            "modifier": "none"
                                                                        }
                                                                    }
                                                                ]
                                                            }
                                                        }
                                                    ]
                                                }
                                            }
                                        },
                                        "generic_params": [],
                                        "modifier": "none"
                                    }
                                }
                            ],
                            "generic_params": []
                        }
                    }
                ]
            }"#,
        )
        .expect("nested associated trait-bound test JSON should parse");

        assert_eq!(
            where_clause_strings(Some(&generics)),
            vec!["I: ParallelIterator<Item: IntoIterator<Item: Send>>".to_owned()]
        );
    }

    #[test]
    fn build_trait_registry_includes_traits_from_public_trait_method_bounds() {
        let rustdoc = RustdocResponse {
            root: 0,
            index: BTreeMap::from([
                (
                    "10".to_owned(),
                    RustdocItem {
                        name: Some("ParallelLike".to_owned()),
                        span: None,
                        visibility: "public".to_owned(),
                        docs: None,
                        attrs: vec![],
                        inner: json!({
                            "trait": {
                                "is_unsafe": false,
                                "bounds": [],
                                "items": ["11"]
                            }
                        }),
                    },
                ),
                (
                    "11".to_owned(),
                    RustdocItem {
                        name: Some("flatten_iter".to_owned()),
                        span: None,
                        visibility: "default".to_owned(),
                        docs: None,
                        attrs: vec![],
                        inner: json!({
                            "function": {
                                "sig": {
                                    "inputs": [["self", {"generic": "Self"}]],
                                    "output": {"primitive": "usize"},
                                    "is_c_variadic": false
                                },
                                "generics": {
                                    "params": [],
                                    "where_predicates": [
                                        {
                                            "bound_predicate": {
                                                "type": {
                                                    "qualified_path": {
                                                        "name": "Item",
                                                        "args": {
                                                            "angle_bracketed": {
                                                                "args": [],
                                                                "constraints": []
                                                            }
                                                        },
                                                        "self_type": {
                                                            "generic": "Self"
                                                        },
                                                        "trait": {
                                                            "path": "",
                                                            "id": 10,
                                                            "args": null
                                                        }
                                                    }
                                                },
                                                "bounds": [
                                                    {
                                                        "trait_bound": {
                                                            "trait": {
                                                                "path": "IntoIterator",
                                                                "id": 737,
                                                                "args": {
                                                                    "angle_bracketed": {
                                                                        "args": [],
                                                                        "constraints": [
                                                                            {
                                                                                "name": "Item",
                                                                                "args": {
                                                                                    "angle_bracketed": {
                                                                                        "args": [],
                                                                                        "constraints": []
                                                                                    }
                                                                                },
                                                                                "binding": {
                                                                                    "constraint": [
                                                                                        {
                                                                                            "trait_bound": {
                                                                                                "trait": {
                                                                                                    "path": "Send",
                                                                                                    "id": 7,
                                                                                                    "args": {
                                                                                                        "angle_bracketed": {
                                                                                                            "args": [],
                                                                                                            "constraints": []
                                                                                                        }
                                                                                                    }
                                                                                                },
                                                                                                "generic_params": [],
                                                                                                "modifier": "none"
                                                                                            }
                                                                                        }
                                                                                    ]
                                                                                }
                                                                            }
                                                                        ]
                                                                    }
                                                                }
                                                            },
                                                            "generic_params": [],
                                                            "modifier": "none"
                                                        }
                                                    }
                                                ],
                                                "generic_params": []
                                            }
                                        }
                                    ]
                                },
                                "header": {
                                    "is_const": false,
                                    "is_unsafe": false,
                                    "is_async": false,
                                    "abi": "Rust"
                                },
                                "has_body": false
                            }
                        }),
                    },
                ),
            ]),
            paths: BTreeMap::from([
                (
                    "10".to_owned(),
                    RustdocPathEntry {
                        crate_id: 0,
                        path: vec!["demo".to_owned(), "ParallelLike".to_owned()],
                        kind: "trait".to_owned(),
                    },
                ),
                (
                    "7".to_owned(),
                    RustdocPathEntry {
                        crate_id: 1,
                        path: vec!["core".to_owned(), "marker".to_owned(), "Send".to_owned()],
                        kind: "trait".to_owned(),
                    },
                ),
                (
                    "737".to_owned(),
                    RustdocPathEntry {
                        crate_id: 1,
                        path: vec![
                            "core".to_owned(),
                            "iter".to_owned(),
                            "traits".to_owned(),
                            "collect".to_owned(),
                            "IntoIterator".to_owned(),
                        ],
                        kind: "trait".to_owned(),
                    },
                ),
            ]),
        };
        let module_ids = BTreeMap::from([("demo".to_owned(), module_id_for_path("demo"))]);

        let traits = build_trait_registry(
            &rustdoc,
            &module_ids,
            &PublicPathState::default(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        );
        let paths = traits
            .iter()
            .map(|trait_info| trait_info.canonical_path.clone())
            .collect::<BTreeSet<_>>();
        let into_iterator_trait = traits
            .iter()
            .find(|trait_info| {
                trait_info.canonical_path == "core::iter::traits::collect::IntoIterator"
            })
            .expect("IntoIterator trait should be present");
        let send_trait = traits
            .iter()
            .find(|trait_info| trait_info.canonical_path == "core::marker::Send")
            .expect("Send trait should be present");

        assert!(paths.contains("demo::ParallelLike"));
        assert!(paths.contains("core::iter::traits::collect::IntoIterator"));
        assert!(paths.contains("core::marker::Send"));
        assert!(into_iterator_trait
            .used_by_trait_ids
            .contains(&TraitId::from("trait::demo::ParallelLike")));
        assert!(send_trait
            .used_by_trait_ids
            .contains(&TraitId::from("trait::demo::ParallelLike")));
    }

    #[test]
    fn trait_method_names_falls_back_to_method_has_body_when_trait_summary_is_missing() {
        let rustdoc = RustdocResponse {
            root: 0,
            index: BTreeMap::from([
                (
                    "10".to_owned(),
                    RustdocItem {
                        name: Some("ExampleTrait".to_owned()),
                        span: None,
                        visibility: "public".to_owned(),
                        docs: None,
                        attrs: vec![],
                        inner: json!({
                            "trait": {
                                "is_unsafe": false,
                                "bounds": [],
                                "items": ["11", "12"]
                            }
                        }),
                    },
                ),
                (
                    "11".to_owned(),
                    RustdocItem {
                        name: Some("required".to_owned()),
                        span: None,
                        visibility: "default".to_owned(),
                        docs: None,
                        attrs: vec![],
                        inner: json!({
                            "function": {
                                "sig": {
                                    "inputs": [["self", {"generic": "Self"}]],
                                    "output": null,
                                    "is_c_variadic": false
                                },
                                "generics": {
                                    "params": [],
                                    "where_predicates": []
                                },
                                "header": {
                                    "is_const": false,
                                    "is_unsafe": false,
                                    "is_async": false,
                                    "abi": "Rust"
                                },
                                "has_body": false
                            }
                        }),
                    },
                ),
                (
                    "12".to_owned(),
                    RustdocItem {
                        name: Some("provided".to_owned()),
                        span: None,
                        visibility: "default".to_owned(),
                        docs: None,
                        attrs: vec![],
                        inner: json!({
                            "function": {
                                "sig": {
                                    "inputs": [["self", {"generic": "Self"}]],
                                    "output": {"primitive": "usize"},
                                    "is_c_variadic": false
                                },
                                "generics": {
                                    "params": [],
                                    "where_predicates": []
                                },
                                "header": {
                                    "is_const": false,
                                    "is_unsafe": false,
                                    "is_async": false,
                                    "abi": "Rust"
                                },
                                "has_body": true
                            }
                        }),
                    },
                ),
            ]),
            paths: BTreeMap::from([(
                "10".to_owned(),
                RustdocPathEntry {
                    crate_id: 0,
                    path: vec!["demo".to_owned(), "ExampleTrait".to_owned()],
                    kind: "trait".to_owned(),
                },
            )]),
        };
        let trait_item = rustdoc
            .index
            .get("10")
            .expect("trait item should be present");

        assert_eq!(
            trait_method_names(trait_item, &rustdoc, "required"),
            vec!["required".to_owned()]
        );
        assert_eq!(
            trait_method_names(trait_item, &rustdoc, "provided"),
            vec!["provided".to_owned()]
        );
    }

    #[test]
    fn render_type_renders_simple_qualified_path_as_assoc_projection() {
        let qualified = json!({
            "qualified_path": {
                "name": "Item",
                "args": {
                    "angle_bracketed": {
                        "args": [],
                        "constraints": []
                    }
                },
                "self_type": {
                    "generic": "I"
                },
                "trait": {
                    "path": "",
                    "id": 20,
                    "args": null
                }
            }
        });

        assert_eq!(render_type(&qualified), "I::Item");
    }

    #[test]
    fn render_type_renders_nested_qualified_path_with_trait_context() {
        let qualified = json!({
            "qualified_path": {
                "name": "Item",
                "args": {
                    "angle_bracketed": {
                        "args": [],
                        "constraints": []
                    }
                },
                "self_type": {
                    "qualified_path": {
                        "name": "Item",
                        "args": {
                            "angle_bracketed": {
                                "args": [],
                                "constraints": []
                            }
                        },
                        "self_type": {
                            "generic": "I"
                        },
                        "trait": {
                            "path": "",
                            "id": 20,
                            "args": null
                        }
                    }
                },
                "trait": {
                    "path": "IntoIterator",
                    "id": 737,
                    "args": {
                        "angle_bracketed": {
                            "args": [],
                            "constraints": []
                        }
                    }
                }
            }
        });

        assert_eq!(render_type(&qualified), "<I::Item as IntoIterator>::Item");
    }

    #[test]
    fn render_type_renders_borrowed_dyn_trait_objects() {
        let ty = json!({
            "borrowed_ref": {
                "lifetime": "'a",
                "is_mutable": true,
                "type": {
                    "dyn_trait": {
                        "traits": [
                            {
                                "trait": {
                                    "path": "IoContext",
                                    "id": 286,
                                    "args": {
                                        "angle_bracketed": {
                                            "args": [],
                                            "constraints": []
                                        }
                                    }
                                },
                                "generic_params": []
                            }
                        ],
                        "lifetime": null
                    }
                }
            }
        });

        assert_eq!(render_type(&ty), "&'a mut dyn IoContext");
    }

    #[test]
    fn generic_param_names_skips_synthetic_impl_trait_params() {
        let generics = json!({
            "params": [
                {
                    "name": "'a",
                    "kind": {
                        "lifetime": {
                            "outlives": []
                        }
                    }
                },
                {
                    "name": "T",
                    "kind": {
                        "type": {
                            "bounds": [],
                            "default": null,
                            "is_synthetic": false
                        }
                    }
                },
                {
                    "name": "impl AsRef<str> + ?Sized",
                    "kind": {
                        "type": {
                            "bounds": [],
                            "default": null,
                            "is_synthetic": true
                        }
                    }
                }
            ],
            "where_predicates": []
        });

        assert_eq!(
            generic_param_names(Some(&generics)),
            vec!["'a".to_owned(), "T".to_owned()]
        );
    }

    #[test]
    fn where_clause_strings_include_inline_type_param_bounds() {
        let generics = json!({
            "params": [
                {
                    "name": "A",
                    "kind": {
                        "type": {
                            "bounds": [
                                {
                                    "trait_bound": {
                                        "trait": {
                                            "path": "NoUninit",
                                            "id": 1,
                                            "args": {
                                                "angle_bracketed": {
                                                    "args": [],
                                                    "constraints": []
                                                }
                                            }
                                        },
                                        "generic_params": [],
                                        "modifier": "none"
                                    }
                                }
                            ],
                            "default": null,
                            "is_synthetic": false
                        }
                    }
                },
                {
                    "name": "B",
                    "kind": {
                        "type": {
                            "bounds": [
                                {
                                    "trait_bound": {
                                        "trait": {
                                            "path": "AnyBitPattern",
                                            "id": 2,
                                            "args": {
                                                "angle_bracketed": {
                                                    "args": [],
                                                    "constraints": []
                                                }
                                            }
                                        },
                                        "generic_params": [],
                                        "modifier": "none"
                                    }
                                }
                            ],
                            "default": null,
                            "is_synthetic": false
                        }
                    }
                }
            ],
            "where_predicates": []
        });

        assert_eq!(
            where_clause_strings(Some(&generics)),
            vec![
                "A: NoUninit".to_owned(),
                "B: AnyBitPattern".to_owned()
            ]
        );
    }
}
