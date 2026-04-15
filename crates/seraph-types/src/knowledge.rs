use crate::{ApiId, CodeRef, ExampleId, ModuleId, SymbolId, TraitId, TraitImplId, TypeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Knowledge {
    // Package-level metadata loaded from Cargo.toml / cargo metadata.
    pub crate_meta: CrateMeta,
    // Public module nodes visible in the extracted crate surface.
    pub modules: Vec<ModuleInfo>,
    // Public type nodes such as structs, enums, unions, and type aliases.
    pub types: Vec<TypeInfo>,
    // Public callable surface such as free functions and inherent methods.
    pub apis: Vec<ApiInfo>,
    // Public macro / constant surface that is neither a nominal type nor a callable API.
    #[serde(default)]
    pub symbols: Vec<SymbolInfo>,
    // Public-API-relevant trait nodes, not just local `pub trait` items.
    pub trait_registry: Vec<TraitInfo>,
    // Explicit trait implementation surface for public local nominal types.
    #[serde(default)]
    pub trait_impl_registry: Vec<TraitImplInfo>,
    // Example evidence collected from public item documentation.
    pub examples: Vec<ExampleInfo>,
    // Focused risk-related facts that are hard to recover from normalized schema fields alone.
    pub risk_facts: RiskFacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateMeta {
    // Cargo package name from `[package].name`.
    pub package_name: String,
    // Library target name after Cargo target resolution.
    pub lib_target_name: String,
    // Rust import path used from downstream code, e.g. `serde_json`.
    pub crate_import_name: String,
    // Cargo package version string.
    pub version: String,
    // Rust edition declared by the package.
    pub edition: String,
    // Optional MSRV / rust-version declaration from Cargo metadata.
    pub rust_version: Option<String>,
    // Upstream repository URL when declared by the package.
    pub repository: Option<String>,
    // Manifest path that the extractor was asked to load.
    pub manifest_path: String,
    // Resolved `lib.rs` source path for the active target/profile.
    pub lib_rs_path: String,
    // Names of features enabled by default for the package.
    pub default_features: Vec<String>,
    // Cargo package description, if present.
    pub cargo_description: Option<String>,
    // Top-level crate docs from rustdoc JSON root item.
    pub root_docs: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    // Stable module identifier: `mod::<canonical_path>`.
    pub module_id: ModuleId,
    // Short item name such as `hash_map`.
    pub name: String,
    // Canonical definition path from rustdoc, e.g. `hashbrown::hash_map`.
    pub canonical_path: String,
    // All public access paths that can reach this module.
    pub public_paths: Vec<String>,
    // Parent module in the canonical module tree when one exists.
    pub parent_module_id: Option<ModuleId>,
    // Source location of the module item.
    pub code_ref: CodeRef,
    // Rendered rustdoc text for the module item.
    pub docs: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeInfo {
    // Stable type identifier: `type::<canonical_path>`.
    pub type_id: TypeId,
    // Short item name such as `HashMap`.
    pub name: String,
    // Canonical definition path from rustdoc.
    pub canonical_path: String,
    // All public access paths that can reach this type.
    pub public_paths: Vec<String>,
    // Primary public module anchor for this item. This is not necessarily the
    // canonical definition parent when the type is re-exported through a public
    // facade module.
    pub public_anchor_module_id: ModuleId,
    // Source location of the type definition.
    pub code_ref: CodeRef,
    // Rendered rustdoc text for the type item.
    pub docs: String,
    // Normalized category of the public type node.
    pub kind: TypeKind,
    // Generic parameter names declared directly on the type.
    pub generic_params: Vec<String>,
    // Rust-style where-clause text fragments such as `A: Allocator`.
    pub where_clauses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    Struct,
    Enum,
    Union,
    TypeAlias,
    Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiInfo {
    // Stable API identifier: `api::<canonical_path>`.
    pub api_id: ApiId,
    // Short function/method name such as `contains_key`.
    pub name: String,
    // Canonical definition path from rustdoc.
    pub canonical_path: String,
    // All public access paths that can reach this API.
    pub public_paths: Vec<String>,
    // Primary public module anchor for this item. This is not necessarily the
    // canonical definition parent when the API is surfaced through a facade
    // module.
    pub public_anchor_module_id: ModuleId,
    // Owning nominal type for inherent methods and assoc functions.
    pub owner_type_id: Option<TypeId>,
    // Owning trait for trait methods extracted into the API surface.
    pub owner_trait_id: Option<TraitId>,
    // Source location of the API definition.
    pub code_ref: CodeRef,
    // Rendered rustdoc text for the API item.
    pub docs: String,
    // Normalized callable category.
    pub api_kind: ApiKind,
    // Human-readable signature text reconstructed from rustdoc JSON.
    pub signature_text: String,
    // Receiver text for methods, e.g. `&self` or `self`.
    pub receiver: Option<String>,
    // Generic parameter names declared directly on the API item.
    pub generic_params: Vec<String>,
    // Rust-style where-clause text fragments such as `Q: Hash + Equivalent<K> + ?Sized`.
    pub where_clauses: Vec<String>,
    // Argument type texts excluding `self`.
    pub arg_types: Vec<String>,
    // Return type text when present.
    pub return_type: Option<String>,
    // Whether the API header is `unsafe`.
    pub is_unsafe: bool,
    // Whether the API header is `async`.
    pub is_async: bool,
    // Whether the API header is `const`.
    pub is_const: bool,
    // `has_body` means an item has an implementation body. For trait methods,
    // `true` means the method has a default implementation.
    pub has_body: bool,
    // Whether the function body text contains an internal `unsafe {}` block.
    #[serde(default)]
    pub contains_unsafe_block: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKind {
    FreeFunction,
    AssocFunction,
    InherentMethod,
    TraitMethod,
    Constructor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolInfo {
    // Stable symbol identifier: `symbol::<canonical_path>`.
    pub symbol_id: SymbolId,
    // Short item name such as `json` or `STATIC_MAX_LEVEL`.
    pub name: String,
    // Canonical definition path from rustdoc.
    pub canonical_path: String,
    // All public access paths that can reach this symbol.
    pub public_paths: Vec<String>,
    // Primary public module anchor for this item.
    pub public_anchor_module_id: ModuleId,
    // Source location of the symbol definition.
    pub code_ref: CodeRef,
    // Rendered rustdoc text for the symbol item.
    pub docs: String,
    // Normalized public symbol category.
    pub symbol_kind: SymbolKind,
    // Rendered declaration text for macros when available.
    pub signature_text: Option<String>,
    // Rendered type text for constants when available.
    pub type_text: Option<String>,
    // Rendered value/expr text for constants when rustdoc exposes it.
    pub value_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Macro,
    Constant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitInfo {
    // Stable trait identifier: `trait::<canonical_path>`.
    pub trait_id: TraitId,
    // Short trait name such as `Hash`.
    pub name: String,
    // Canonical definition path of the trait node.
    pub canonical_path: String,
    // Public re-export paths that reach this trait from the current crate.
    pub public_paths: Vec<String>,
    // Primary public module anchor for this trait node when one exists.
    pub public_anchor_module_id: Option<ModuleId>,
    // Source location when the trait definition is local and available.
    pub code_ref: Option<CodeRef>,
    // Rendered rustdoc text for the trait node when available.
    pub docs: String,
    // Relative to the current crate: local definition, public re-export of a
    // non-local trait, or external-only dependency referenced by the public API.
    pub origin: TraitOrigin,
    // Why this trait node exists in Phase 1, e.g. re-export, bound, or supertrait dependency.
    pub exposure_kinds: Vec<TraitExposureKind>,
    // Whether the trait itself is declared `unsafe`.
    pub is_unsafe: bool,
    // Immediate supertrait edges recorded as stable trait ids.
    pub direct_supertrait_ids: Vec<TraitId>,
    // Required trait method names declared without default bodies.
    pub required_methods: Vec<String>,
    // Provided trait method names that have default bodies.
    pub provided_methods: Vec<String>,
    // Structured associated type declarations declared by the trait.
    #[serde(default)]
    pub associated_type_defs: Vec<TraitAssociatedTypeDef>,
    // Explicit reverse edges collected during extraction from public signatures
    // and bounds. These are not derivable from a normalized trait-reference table
    // because Phase 1 still stores clauses as text.
    pub used_by_api_ids: Vec<ApiId>,
    // Reverse edges from public trait surfaces whose associated types or
    // public trait methods mention this trait in bounds.
    // Phase 1 intentionally keeps this at trait granularity only; it does not
    // further split by specific method or associated-type slot.
    #[serde(default)]
    pub used_by_trait_ids: Vec<TraitId>,
    // Reverse edges from public type declarations that mention this trait in bounds.
    pub used_by_type_ids: Vec<TypeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitOrigin {
    Local,
    Reexported,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraitExposureKind {
    Defined,
    Reexported,
    Bound,
    SupertraitDependency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitImplInfo {
    // Stable impl identifier built from the implemented trait reference and the
    // rendered `for` target, e.g.
    // `trait_impl::core::ops::bit::BitAnd<&HashSet<T, S, A>>::for::&HashSet<T, S, A>`.
    pub trait_impl_id: TraitImplId,
    // Public local nominal type that anchors this impl surface entry.
    pub target_type_id: TypeId,
    // Coarse bucket used by downstream ranking/selection logic. Phase 1 keeps
    // the full impl table and adds this hint instead of dropping lower-signal
    // iterator/view impls during extraction.
    #[serde(default)]
    pub surface_bucket: TraitImplSurfaceBucket,
    // Fully rendered trait reference, including trait generic arguments when
    // present, e.g. `core::ops::bit::BitAnd<&HashSet<T, S, A>>`.
    pub trait_ref_text: String,
    // Rendered `for` side type text, e.g. `&HashSet<T, S, A>`.
    pub for_type_text: String,
    // Stable trait node identifier pointing at the implemented trait item.
    pub trait_id: TraitId,
    // Short trait name such as `BitAnd`.
    pub trait_name: String,
    // Canonical definition path of the implemented trait item.
    pub trait_canonical_path: String,
    // Relative to the current crate: local, re-exported, or external trait.
    pub trait_origin: TraitOrigin,
    // Source location of the explicit impl block.
    pub source: CodeRef,
    // Structured associated type assignments declared inside the impl block.
    #[serde(default)]
    pub associated_type_bindings: Vec<TraitAssociatedTypeBinding>,
    // Rust-style where-clause text fragments declared on the impl block.
    pub where_clauses: Vec<String>,
    // Whether the impl header is `unsafe`.
    pub is_unsafe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitAssociatedTypeDef {
    // Short associated type name such as `Item` or `IntoIter`.
    pub name: String,
    // Generic parameter names declared directly on the associated type (for GATs).
    pub generic_params: Vec<String>,
    // Rust-style where-clause text fragments declared on the associated type.
    pub where_clauses: Vec<String>,
    // Declared bounds such as `Clone` or `Debug`.
    pub bounds: Vec<String>,
    // Default assigned type when the trait provides one.
    pub default_type: Option<String>,
    // Source location of the associated type declaration when recoverable.
    pub source: Option<CodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitAssociatedTypeBinding {
    // Short associated type name such as `Item` or `Output`.
    pub name: String,
    // Generic parameter names declared directly on the associated type binding.
    pub generic_params: Vec<String>,
    // Rust-style where-clause text fragments declared on the associated type binding.
    pub where_clauses: Vec<String>,
    // Bounds declared on the associated type binding itself.
    pub bounds: Vec<String>,
    // Concrete type assigned by the impl, e.g. `&'a K` or `IntoIter<K, V, A>`.
    pub assigned_type: Option<String>,
    // Source location of the associated type binding when recoverable.
    pub source: Option<CodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TraitImplSurfaceBucket {
    // Core collection/container types such as `HashMap`, `HashSet`, and `HashTable`.
    PrimaryContainer,
    // Entry-style cursor APIs and raw-entry helper surface.
    EntryOrRawEntry,
    // Iterator, view, set-operation iterator, and similar traversal surface.
    IteratorOrView,
    // Error or hasher support types that are public but not containers.
    ErrorOrHasher,
    // Escape hatch for public impl surface that does not fit the current coarse buckets.
    #[default]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExampleInfo {
    // Stable example identifier. In Phase 1 this is anchored to source file +
    // start line + doc-block ordinal.
    pub example_id: ExampleId,
    // Primary owner of the example snippet.
    pub anchor: ExampleAnchor,
    // Public APIs explicitly associated with this example.
    pub involved_api_ids: Vec<ApiId>,
    // Source location of the owning item, not the inner line range of the code block.
    pub code_ref: CodeRef,
    // Evidence text only; never used as an identity key.
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExampleAnchor {
    Module(ModuleId),
    Type(TypeId),
    Api(ApiId),
    Symbol(SymbolId),
    Trait(TraitId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RiskFacts {
    // FFI / extern ABI callable boundaries surfaced by the crate.
    pub ffi_apis: Vec<FfiApiFact>,
    // Explicit repr/layout facts collected for public types.
    pub repr_types: Vec<TypeLayoutFact>,
    // Public types with explicit Drop implementations.
    pub drop_impl_types: Vec<TypeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiApiFact {
    // Public API that crosses an FFI / extern ABI boundary.
    pub api_id: ApiId,
    // ABI string such as `C`.
    pub abi: String,
    // Source location of the extern declaration when recoverable.
    pub source: Option<CodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeLayoutFact {
    // Public type whose layout attributes were observed.
    pub type_id: TypeId,
    // Normalized repr attributes such as `C` or `Transparent`.
    pub repr_kinds: Vec<ReprKind>,
    // Source location of the `#[repr(...)]` evidence when recoverable.
    pub source: Option<CodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReprKind {
    C,
    Transparent,
    Packed,
    Align(u32),
    Other(String),
}
