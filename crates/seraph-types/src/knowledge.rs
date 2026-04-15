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
    // Public macro / constant / associated-constant surface that is neither a
    // nominal type node nor a callable API.
    #[serde(default)]
    pub symbols: Vec<SymbolInfo>,
    // Public-API-relevant trait nodes, not just local `pub trait` items.
    pub trait_registry: Vec<TraitInfo>,
    // Explicit trait implementation surface for public local nominal types.
    #[serde(default)]
    pub trait_impl_registry: Vec<TraitImplInfo>,
    // Example evidence collected from public item documentation.
    #[serde(default)]
    pub examples: Vec<ExampleInfo>,
    // Focused risk-related facts that are hard to recover from normalized schema fields alone.
    pub risk_facts: RiskFacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DocSections {
    // First paragraph / overview.
    pub summary: String,
    // Raw `# Panics` section body when present.
    pub panics: String,
    // Raw `# Errors` section body when present.
    pub errors: String,
    // Raw `# Safety` section body when present.
    pub safety: String,
    // Raw `# Example` / `# Examples` section body when present.
    pub examples: String,
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
    // Structured slices of root docs. Empty strings mean the section was absent.
    #[serde(default)]
    pub root_doc_sections: DocSections,
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
    // Structured slices derived from `docs`.
    #[serde(default)]
    pub doc_sections: DocSections,
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
    // Structured slices derived from `docs`.
    #[serde(default)]
    pub doc_sections: DocSections,
    // Normalized category of the public type node.
    pub kind: TypeKind,
    // Generic parameter names declared directly on the type.
    pub generic_params: Vec<String>,
    // Rust-style where-clause text fragments such as `A: Allocator`.
    pub where_clauses: Vec<String>,
    // Whether the item is annotated with `#[non_exhaustive]`.
    #[serde(default)]
    pub is_non_exhaustive: bool,
    // Recoverable fields for structs/unions and tuple variants.
    #[serde(default)]
    pub fields: Vec<TypeFieldInfo>,
    // Recoverable enum variant surface facts.
    #[serde(default)]
    pub variants: Vec<EnumVariantInfo>,
    // rustdoc JSON may strip private fields while still indicating they exist.
    #[serde(default)]
    pub has_hidden_fields: bool,
    // rustdoc JSON may strip non-public variants while still indicating they exist.
    #[serde(default)]
    pub has_hidden_variants: bool,
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
pub struct TypeFieldInfo {
    // 0-based declaration order.
    pub position: u32,
    // Named fields use `Some(name)`; tuple fields use `None`.
    pub name: Option<String>,
    // Rendered Rust type text.
    pub type_text: String,
    // Raw visibility syntax such as `pub`, `pub(crate)`, or `default`.
    pub visibility_text: String,
    // Source location when recoverable.
    pub source: Option<CodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumVariantInfo {
    // Variant short name such as `Ok` or `Null`.
    pub name: String,
    // Structural shape of the variant.
    pub kind: VariantKind,
    // Whether the variant itself is annotated with `#[non_exhaustive]`.
    #[serde(default)]
    pub is_non_exhaustive: bool,
    // Explicit discriminant text when recoverable.
    pub discriminant_text: Option<String>,
    // Variant field surface when present.
    #[serde(default)]
    pub fields: Vec<TypeFieldInfo>,
    // Source location when recoverable.
    pub source: Option<CodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantKind {
    Unit,
    Tuple,
    Struct,
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
    // Structured slices derived from `docs`.
    #[serde(default)]
    pub doc_sections: DocSections,
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
    // Deterministic structural summary of the return type.
    pub return_shape: Option<ReturnShape>,
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
    // This can still be `true` even when `is_unsafe` is `false`.
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
pub struct ReturnShape {
    // Top-level structural kind of the rendered return type.
    pub kind: ReturnShapeKind,
    // Top-level contained types rendered as Rust text, when recoverable.
    #[serde(default)]
    pub inner_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReturnShapeKind {
    Unit,
    Never,
    Primitive,
    Nominal,
    Tuple,
    Array,
    Slice,
    Ref,
    RefMut,
    RawPtr,
    Result,
    Option,
    ImplTrait,
    DynTrait,
    Other,
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
    // Owning nominal type for inherent associated constants. `None` for
    // top-level constants and macros.
    pub owner_type_id: Option<TypeId>,
    // Source location of the symbol definition.
    pub code_ref: CodeRef,
    // Rendered rustdoc text for the symbol item.
    pub docs: String,
    // Structured slices derived from `docs`.
    #[serde(default)]
    pub doc_sections: DocSections,
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
    AssociatedConstant,
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
    // Structured slices derived from `docs`.
    #[serde(default)]
    pub doc_sections: DocSections,
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
    // Structured associated const declarations declared by the trait,
    // including their declared types and any default values.
    #[serde(default)]
    pub associated_const_defs: Vec<TraitAssociatedConstDef>,
    // Explicit reverse edges collected during extraction from public signatures
    // and bounds. These are not derivable from a normalized trait-reference table
    // because Phase 1 still stores clauses as text.
    #[serde(default)]
    pub used_by_api_ids: Vec<ApiId>,
    // Reverse edges from public trait surfaces whose associated types or
    // public trait methods mention this trait in bounds.
    #[serde(default)]
    pub used_by_trait_ids: Vec<TraitId>,
    // Reverse edges from public type declarations that mention this trait in bounds.
    #[serde(default)]
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
    // Structured associated const assignments declared inside the impl block.
    // These are the concrete values chosen by this impl, not the trait-level declaration.
    #[serde(default)]
    pub associated_const_bindings: Vec<TraitAssociatedConstBinding>,
    // Rust-style where-clause text fragments declared on the impl block.
    pub where_clauses: Vec<String>,
    // Raw cfg-bearing attrs attached to the impl block, e.g.
    // `#[cfg(feature = "postgres")]`. Phase 1 keeps the raw text only.
    #[serde(default)]
    pub cfg_attrs: Vec<String>,
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
pub struct TraitAssociatedConstDef {
    // Short associated const name such as `NAME` or `URL_SCHEMES`.
    pub name: String,
    // Rendered declared const type.
    pub type_text: String,
    // Default value text when the trait provides one.
    pub default_value_text: Option<String>,
    // Source location when recoverable.
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitAssociatedConstBinding {
    // Short associated const name such as `NAME` or `PARAM_CHECKING`.
    pub name: String,
    // Concrete value assigned by the impl, if rustdoc exposes it.
    pub value_text: Option<String>,
    // Source location when recoverable.
    pub source: Option<CodeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExampleInfo {
    // Stable example identifier. In Phase 1 this is anchored to source file +
    // start line + doc-block ordinal.
    pub example_id: ExampleId,
    // Primary owner of the example snippet.
    pub anchor: ExampleAnchor,
    // Public APIs explicitly associated with this example.
    // This is a best-effort link set, not a guarantee that only these APIs appear in the snippet.
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
    // Preferred v3 name for public non-Rust-ABI API boundaries.
    // This is about exposed ABI surface, not internal FFI calls hidden inside implementation code.
    #[serde(default)]
    pub extern_abi_apis: Vec<ExternAbiApiFact>,
    // Explicit repr/layout facts collected for public types.
    #[serde(default)]
    pub repr_types: Vec<TypeLayoutFact>,
    // Public types with explicit Drop implementations.
    // This records the presence of a Drop impl only; later stages can interpret risk.
    #[serde(default)]
    pub drop_impl_types: Vec<TypeId>,
    // Explicit panic-like evidence found by source-span scan.
    // Only syntactic panic-like sites are recorded here; inferred panic possibility is out of scope.
    #[serde(default)]
    pub explicit_panic_sites: Vec<ExplicitPanicSiteFact>,
    // Signature-level borrowed return facts with no risk interpretation.
    // This stays as raw evidence so later stages can decide whether it matters semantically.
    #[serde(default)]
    pub borrowed_return_apis: Vec<BorrowedReturnFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternAbiApiFact {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitPanicSiteFact {
    // Owning API or impl that contains the explicit panic-like site.
    pub owner: RiskOwner,
    // Macro family label such as `panic!` or `assert!`.
    pub panic_kind: String,
    // Source location of the evidence.
    pub source: CodeRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskOwner {
    Api(ApiId),
    TraitImpl(TraitImplId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorrowedReturnFact {
    // API whose return type syntactically borrows from `self` or input arguments.
    pub api_id: ApiId,
    // Rendered return type text as seen by downstream consumers.
    pub return_type_text: String,
    // True when the returned borrow is tied to the receiver.
    pub from_self: bool,
    // 0-based argument positions whose borrow flows into the return type.
    #[serde(default)]
    pub from_arg_positions: Vec<u32>,
    // Explicit lifetime names matched across inputs and output. Can be empty
    // when the fact comes solely from Rust lifetime elision rules such as `&self -> &T`.
    #[serde(default)]
    pub lifetime_names: Vec<String>,
    // Source location when recoverable.
    pub source: Option<CodeRef>,
}
