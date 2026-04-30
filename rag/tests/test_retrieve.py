from pathlib import Path

from seraph_rag.graph_builder import build_graph
from seraph_rag.knowledge_loader import load_knowledge
from seraph_rag.retrieve import rank_unsafe_targets, render_context_markdown

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_knowledge.json"


def test_rank_unsafe_targets_returns_only_unsafe_api():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    targets = rank_unsafe_targets(graph)
    assert [target.api_id for target in targets] == ["fn::fixture_crate::Buffer::get_unchecked"]
    assert targets[0].score > 0


def test_render_context_markdown_uses_redesigned_sections():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(
        knowledge,
        graph,
        target,
        idioms=["Handle Result with early return."],
    )
    assert "# SERAPH Rust Harness Context" in markdown
    assert "## Crate Facts" in markdown
    assert "## Target API" in markdown
    assert "## Known Reachable Paths" in markdown
    assert "## Related APIs" in markdown
    assert "## Compile-Time Facts" in markdown
    assert "## Similar API Usage" in markdown
    assert "## Rust Idioms" in markdown
    assert "## Generation Rules" not in markdown


def test_render_context_markdown_surfaces_variant_opportunities():
    knowledge = load_knowledge(FIXTURE)
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    variant_section = markdown.split("## Variant Opportunities", 1)[1].split(
        "## Similar API Usage", 1
    )[0]

    assert "## Variant Opportunities" in markdown
    assert "### Setup Choices" in markdown
    assert "### Input Shaping Choices" in markdown
    assert "### State Progression Choices" in markdown
    assert "### Boundary Choices" in markdown
    assert "- setup API fixture_crate::Buffer::new produces fixture_crate::Buffer" in variant_section
    assert "- target signature includes argument type usize" in variant_section
    assert "- same-owner mutator surfaced in related APIs: fixture_crate::Buffer::push" in variant_section
    assert "- documented safety precondition: The index must be in bounds." in variant_section
    assert "before target" not in variant_section
    assert "call the target immediately" not in variant_section
    assert "prefer documented recoverable boundaries" not in variant_section


def test_render_context_markdown_surfaces_target_usage_hints_from_examples():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [{"module_id": "mod::fixture", "canonical_path": "fixture"}],
        "types": [],
        "trait_registry": [
            {
                "trait_id": "trait::fixture::ByteSlice",
                "name": "ByteSlice",
                "canonical_path": "fixture::ByteSlice",
                "public_anchor_module_id": "mod::fixture",
                "is_unsafe": False,
                "required_methods": [],
                "provided_methods": ["to_str_lossy"],
            }
        ],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::ByteSlice::to_str_lossy",
                "name": "to_str_lossy",
                "canonical_path": "fixture::ByteSlice::to_str_lossy",
                "public_paths": ["fixture::ByteSlice::to_str_lossy"],
                "public_anchor_module_id": "mod::fixture",
                "owner_trait_id": "trait::fixture::ByteSlice",
                "signature_text": "fn to_str_lossy(&Self) -> Cow<'_, str>",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "Cow<'_, str>",
                "api_kind": "trait_method",
                "contains_unsafe_block": True,
                "doc_sections": {
                    "summary": "Convert bytes to lossy UTF-8.",
                    "examples": "\n".join(
                        [
                            "Basic usage:",
                            "",
                            "```",
                            "use fixture::ByteSlice;",
                            "let mut data = <Vec<u8>>::from(\"abc\");",
                            "assert_eq!(\"abc\", data.to_str_lossy());",
                            "```",
                            "",
                            "```",
                            "use fixture::{B, ByteSlice};",
                            "let bs = B(b\"abc\");",
                            "assert_eq!(\"abc\", bs.to_str_lossy());",
                            "```",
                        ]
                    ),
                },
            }
        ],
        "risk_facts": {
            "unsafe_functions": [],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    assert "## Target Usage Hints" in markdown
    assert (
        '- `let mut data = <Vec<u8>>::from("abc"); assert_eq!("abc", data.to_str_lossy());`'
        in markdown
    )
    assert '- `let bs = B(b"abc"); assert_eq!("abc", bs.to_str_lossy());`' in markdown


def test_render_context_markdown_surfaces_owner_type_facts_and_usage_hints():
    knowledge = {
        "crate_meta": {"crate_import_name": "smallvec"},
        "modules": [{"module_id": "mod::smallvec", "canonical_path": "smallvec"}],
        "types": [
            {
                "type_id": "type::smallvec::SmallVec",
                "name": "SmallVec",
                "canonical_path": "smallvec::SmallVec",
                "public_paths": ["smallvec::SmallVec"],
                "public_anchor_module_id": "mod::smallvec",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": True,
                "has_hidden_variants": False,
                "generic_params": ["A"],
                "where_clauses": ["A: Array"],
                "doc_sections": {
                    "examples": "\n".join(
                        [
                            "```rust",
                            "use smallvec::SmallVec;",
                            "let mut v = SmallVec::<[u8; 4]>::new();",
                            "v.push(1);",
                            "```",
                        ]
                    )
                },
            }
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::smallvec::SmallVec::drain",
                "name": "drain",
                "canonical_path": "smallvec::SmallVec::drain",
                "public_paths": ["smallvec::SmallVec::drain"],
                "public_anchor_module_id": "mod::smallvec",
                "owner_type_id": "type::smallvec::SmallVec",
                "signature_text": "fn drain(&mut Self) -> Drain<'_, A::Item>",
                "receiver": "&mut Self",
                "arg_types": [],
                "return_type": "Drain<'_, A::Item>",
                "api_kind": "method",
                "contains_unsafe_block": True,
                "doc_sections": {},
            },
            {
                "api_id": "api::smallvec::SmallVec::new",
                "name": "new",
                "canonical_path": "smallvec::SmallVec::new",
                "public_paths": ["smallvec::SmallVec::new"],
                "public_anchor_module_id": "mod::smallvec",
                "owner_type_id": "type::smallvec::SmallVec",
                "signature_text": "fn new() -> SmallVec<A>",
                "receiver": "",
                "arg_types": [],
                "return_type": "SmallVec<A>",
                "api_kind": "associated_constructor",
                "contains_unsafe_block": False,
                "doc_sections": {},
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    assert "### Owner Type Facts" in markdown
    assert "- smallvec::SmallVec: generic_params=A; where_clauses=A: Array" in markdown
    assert "## Owner Type Usage Hints" in markdown
    assert '- `let mut v = SmallVec::<[u8; 4]>::new();`' in markdown


def test_render_context_markdown_surfaces_compile_critical_import_trait_and_enum_facts():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [
            {"module_id": "mod::fixture", "canonical_path": "fixture"},
            {"module_id": "mod::fixture::io", "canonical_path": "fixture::io"},
            {"module_id": "mod::fixture::avutil", "canonical_path": "fixture::avutil"},
        ],
        "types": [
            {
                "type_id": "type::fixture::Context",
                "name": "Context",
                "canonical_path": "fixture::Context",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::avutil::Dictionary",
                "name": "Dictionary",
                "canonical_path": "fixture::avutil::Dictionary",
                "public_anchor_module_id": "mod::fixture::avutil",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::io::Whence",
                "name": "Whence",
                "canonical_path": "fixture::io::Whence",
                "public_anchor_module_id": "mod::fixture::io",
                "kind": "enum",
                "variants": [
                    {"name": "Size"},
                    {"name": "Set"},
                    {"name": "Cur"},
                    {"name": "End"},
                ],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::fixture::io::IoContext",
                "name": "IoContext",
                "canonical_path": "fixture::io::IoContext",
                "public_anchor_module_id": "mod::fixture::io",
                "is_unsafe": False,
                "required_methods": ["buf_len"],
                "provided_methods": ["read", "seek"],
            }
        ],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::Context::target",
                "name": "target",
                "canonical_path": "fixture::Context::target",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Context",
                "signature": "fn target(&Self)",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "()",
                "api_kind": "method",
                "contains_unsafe_block": True,
            },
            {
                "api_id": "api::fixture::Context::with_io_context",
                "name": "with_io_context",
                "canonical_path": "fixture::Context::with_io_context",
                "public_anchor_module_id": "mod::fixture",
                "signature": "fn with_io_context(&mut dyn IoContext, &mut Dictionary) -> Result<Context, Error>",
                "receiver": "",
                "arg_types": ["&mut dyn IoContext", "&mut Dictionary"],
                "return_type": "Result<Context, Error>",
                "api_kind": "associated_constructor",
            },
            {
                "api_id": "api::fixture::io::IoContext::buf_len",
                "name": "buf_len",
                "canonical_path": "fixture::io::IoContext::buf_len",
                "public_anchor_module_id": "mod::fixture::io",
                "owner_trait_id": "trait::fixture::io::IoContext",
                "signature_text": "fn buf_len(&Self) -> usize",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "usize",
                "api_kind": "trait_method",
                "contains_unsafe_block": False,
            },
            {
                "api_id": "api::fixture::io::IoContext::seek",
                "name": "seek",
                "canonical_path": "fixture::io::IoContext::seek",
                "public_anchor_module_id": "mod::fixture::io",
                "owner_trait_id": "trait::fixture::io::IoContext",
                "signature_text": "fn seek(&mut Self, i64, Whence, bool) -> Result<u64, Error>",
                "receiver": "&mut Self",
                "arg_types": ["i64", "Whence", "bool"],
                "return_type": "Result<u64, Error>",
                "api_kind": "trait_method",
                "contains_unsafe_block": False,
            },
        ],
        "risk_facts": {
            "unsafe_functions": [],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    compile_facts = markdown.split("## Compile-Time Facts", 1)[1].split("## Related APIs", 1)[0]
    compile_fact_lines = compile_facts.splitlines()

    assert "### Exact Import Paths" in compile_facts
    assert "### Required Traits" in compile_facts
    assert "### Trait Method Signatures" in compile_facts
    assert "### Enum Variants" in compile_facts
    assert "## Exact Import Paths" not in compile_fact_lines
    assert "## Required Traits" not in compile_fact_lines
    assert "## Trait Method Signatures" not in compile_fact_lines
    assert "## Enum Variants" not in compile_fact_lines
    assert "fixture::avutil::Dictionary" in compile_facts
    assert "fixture::io::IoContext" in compile_facts
    assert "fixture::io::Whence" in compile_facts
    assert "required_methods=buf_len" in compile_facts
    assert "provided_methods=read, seek" in compile_facts
    assert "fixture::io::IoContext::buf_len: fn buf_len(&self) -> usize [required]" in compile_facts
    assert "fixture::io::IoContext::seek: fn seek(&mut self, i64, Whence, bool) -> Result<u64, Error> [provided]" in compile_facts
    assert "fixture::io::Whence: Size | Set | Cur | End" in compile_facts


def test_render_context_markdown_surfaces_output_initialization_facts_for_mut_output_types(tmp_path):
    ffi_path = tmp_path / "ffi.rs"
    ffi_path.write_text(
        "\n".join(
            [
                "pub type BigArray = [u16; 8192usize];",
                "#[derive(Debug, Copy, Clone)]",
                "pub struct OutputInfo {",
                "    pub count: i32,",
                "    pub tag: [u8; 4usize],",
                "}",
                "",
            ]
        ),
        encoding="utf-8",
    )
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [{"module_id": "mod::fixture", "canonical_path": "fixture"}],
        "types": [
            {
                "type_id": "type::fixture::Context",
                "name": "Context",
                "canonical_path": "fixture::Context",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::BigArray",
                "name": "BigArray",
                "canonical_path": "fixture::BigArray",
                "public_anchor_module_id": "mod::fixture",
                "kind": "type_alias",
                "code_ref": {
                    "file": str(ffi_path),
                    "start_line": 1,
                    "end_line": 1,
                },
                "fields": [],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::OutputInfo",
                "name": "OutputInfo",
                "canonical_path": "fixture::OutputInfo",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "code_ref": {
                    "file": str(ffi_path),
                    "start_line": 3,
                    "end_line": 6,
                },
                "fields": [
                    {
                        "position": 0,
                        "name": "count",
                        "type_text": "i32",
                        "visibility_text": "public",
                        "source": {
                            "file": str(ffi_path),
                            "start_line": 4,
                            "end_line": 4,
                        },
                    },
                    {
                        "position": 1,
                        "name": "tag",
                        "type_text": "[u8; _]",
                        "visibility_text": "public",
                        "source": {
                            "file": str(ffi_path),
                            "start_line": 5,
                            "end_line": 5,
                        },
                    },
                ],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::core::clone::Clone",
                "name": "Clone",
                "canonical_path": "core::clone::Clone",
                "is_unsafe": False,
            },
            {
                "trait_id": "trait::core::marker::Copy",
                "name": "Copy",
                "canonical_path": "core::marker::Copy",
                "is_unsafe": False,
            },
        ],
        "trait_impl_registry": [
            {
                "target_type_id": "type::fixture::BigArray",
                "trait_id": "trait::core::clone::Clone",
                "trait_path": "core::clone::Clone",
            },
            {
                "target_type_id": "type::fixture::BigArray",
                "trait_id": "trait::core::marker::Copy",
                "trait_path": "core::marker::Copy",
            },
            {
                "target_type_id": "type::fixture::OutputInfo",
                "trait_id": "trait::core::clone::Clone",
                "trait_path": "core::clone::Clone",
            },
            {
                "target_type_id": "type::fixture::OutputInfo",
                "trait_id": "trait::core::marker::Copy",
                "trait_path": "core::marker::Copy",
            },
        ],
        "apis": [
            {
                "api_id": "api::fixture::Context::target",
                "name": "target",
                "canonical_path": "fixture::Context::target",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Context",
                "signature": "fn target(&Self, &mut OutputInfo, &mut BigArray) -> Result<()>",
                "receiver": "&Self",
                "arg_types": ["&mut OutputInfo", "&mut BigArray"],
                "return_type": "Result<()>",
                "api_kind": "method",
                "contains_unsafe_block": True,
            },
            {
                "api_id": "api::fixture::Context::new",
                "name": "new",
                "canonical_path": "fixture::Context::new",
                "public_anchor_module_id": "mod::fixture",
                "signature": "fn new() -> Context",
                "receiver": "",
                "arg_types": [],
                "return_type": "Context",
                "api_kind": "associated_constructor",
            },
        ],
        "risk_facts": {
            "unsafe_functions": [],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    compile_facts = markdown.split("## Compile-Time Facts", 1)[1].split("## Related APIs", 1)[0]

    assert "### Output Initialization Facts" in compile_facts
    assert "fixture::BigArray: prefer `let mut value: fixture::BigArray = [0; 8192];`" in compile_facts
    assert (
        "fixture::OutputInfo: prefer `let mut value = fixture::OutputInfo { count: 0, tag: [0; 4] };`"
        in compile_facts
    )


def test_render_context_markdown_surfaces_target_generic_bounds():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [{"module_id": "mod::fixture", "canonical_path": "fixture"}],
        "types": [
            {
                "type_id": "type::fixture::Client",
                "name": "Client",
                "canonical_path": "fixture::Client",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            }
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::Client::set_callback",
                "name": "set_callback",
                "canonical_path": "fixture::Client::set_callback",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Client",
                "signature_text": "fn set_callback(&Self, Option<F>) -> Result<()>",
                "receiver": "&Self",
                "arg_types": ["Option<F>"],
                "return_type": "Result<()>",
                "api_kind": "inherent_method",
                "generic_params": ["F"],
                "where_clauses": ["F: FnMut(*mut c_void, c_int, c_int) + 'static"],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::fixture::Client::create",
                "name": "create",
                "canonical_path": "fixture::Client::create",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Client",
                "signature_text": "fn create() -> Client",
                "arg_types": [],
                "return_type": "Self",
                "api_kind": "constructor",
                "doc_sections": {"safety": ""},
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    target_section = markdown.split("## Target API", 1)[1].split("## Known Reachable Paths", 1)[0]

    assert "- generic_bounds: F: FnMut(*mut c_void, c_int, c_int) + 'static" in target_section


def test_render_context_markdown_surfaces_public_types_mentioned_only_in_target_generic_bounds():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [{"module_id": "mod::fixture", "canonical_path": "fixture"}],
        "types": [
            {
                "type_id": "type::fixture::Client",
                "name": "Client",
                "canonical_path": "fixture::Client",
                "public_paths": ["fixture::Client"],
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::Event",
                "name": "Event",
                "canonical_path": "fixture::internal::Event",
                "public_paths": ["fixture::Event"],
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::Client::set_callback",
                "name": "set_callback",
                "canonical_path": "fixture::Client::set_callback",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Client",
                "signature_text": "fn set_callback(&Self, Option<F>) -> Result<()>",
                "receiver": "&Self",
                "arg_types": ["Option<F>"],
                "return_type": "Result<()>",
                "api_kind": "inherent_method",
                "generic_params": ["F"],
                "where_clauses": ["F: FnMut(*mut Event, c_int)"],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::fixture::Client::create",
                "name": "create",
                "canonical_path": "fixture::Client::create",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Client",
                "signature_text": "fn create() -> Client",
                "arg_types": [],
                "return_type": "Self",
                "api_kind": "constructor",
                "doc_sections": {"safety": ""},
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    compile_facts = markdown.split("## Compile-Time Facts", 1)[1].split("## Related APIs", 1)[0]

    assert "### Exact Import Paths" in compile_facts
    assert "type::fixture::Event => fixture::Event [kind=type]" in compile_facts


def test_render_context_markdown_surfaces_type_trait_facts_for_relevant_argument_types():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [{"module_id": "mod::fixture", "canonical_path": "fixture"}],
        "types": [
            {
                "type_id": "type::fixture::Context",
                "name": "Context",
                "canonical_path": "fixture::Context",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::Selector",
                "name": "Selector",
                "canonical_path": "fixture::Selector",
                "public_anchor_module_id": "mod::fixture",
                "kind": "enum",
                "variants": [{"name": "A"}, {"name": "B"}],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::Payload",
                "name": "Payload",
                "canonical_path": "fixture::Payload",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::core::fmt::Debug",
                "name": "Debug",
                "canonical_path": "core::fmt::Debug",
                "is_unsafe": False,
            },
            {
                "trait_id": "trait::core::clone::Clone",
                "name": "Clone",
                "canonical_path": "core::clone::Clone",
                "is_unsafe": False,
            },
            {
                "trait_id": "trait::core::marker::Copy",
                "name": "Copy",
                "canonical_path": "core::marker::Copy",
                "is_unsafe": False,
            },
        ],
        "trait_impl_registry": [
            {
                "trait_impl_id": "impl::fixture::Selector->Debug",
                "target_type_id": "type::fixture::Selector",
                "trait_ref_text": "core::fmt::Debug",
                "for_type_text": "Selector",
                "trait_id": "trait::core::fmt::Debug",
                "trait_name": "Debug",
                "trait_canonical_path": "core::fmt::Debug",
                "trait_origin": "external",
                "source": {"file": "src/lib.rs", "start_line": 10, "end_line": 10},
                "associated_type_bindings": [],
                "associated_const_bindings": [],
                "where_clauses": [],
                "cfg_attrs": [],
                "is_unsafe": False,
            },
            {
                "trait_impl_id": "impl::fixture::Payload->Clone",
                "target_type_id": "type::fixture::Payload",
                "trait_ref_text": "core::clone::Clone",
                "for_type_text": "Payload",
                "trait_id": "trait::core::clone::Clone",
                "trait_name": "Clone",
                "trait_canonical_path": "core::clone::Clone",
                "trait_origin": "external",
                "source": {"file": "src/lib.rs", "start_line": 12, "end_line": 12},
                "associated_type_bindings": [],
                "associated_const_bindings": [],
                "where_clauses": [],
                "cfg_attrs": [],
                "is_unsafe": False,
            },
            {
                "trait_impl_id": "impl::fixture::Payload->Copy",
                "target_type_id": "type::fixture::Payload",
                "trait_ref_text": "core::marker::Copy",
                "for_type_text": "Payload",
                "trait_id": "trait::core::marker::Copy",
                "trait_name": "Copy",
                "trait_canonical_path": "core::marker::Copy",
                "trait_origin": "external",
                "source": {"file": "src/lib.rs", "start_line": 13, "end_line": 13},
                "associated_type_bindings": [],
                "associated_const_bindings": [],
                "where_clauses": [],
                "cfg_attrs": [],
                "is_unsafe": False,
            },
        ],
        "apis": [
            {
                "api_id": "api::fixture::Context::set_param",
                "name": "set_param",
                "canonical_path": "fixture::Context::set_param",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Context",
                "signature": "fn set_param(&Self, Selector, Payload) -> ()",
                "receiver": "&Self",
                "arg_types": ["Selector", "Payload"],
                "return_type": "()",
                "api_kind": "method",
                "contains_unsafe_block": True,
            },
            {
                "api_id": "api::fixture::Context::new",
                "name": "new",
                "canonical_path": "fixture::Context::new",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Context",
                "signature": "fn new() -> Context",
                "receiver": "",
                "arg_types": [],
                "return_type": "Context",
                "api_kind": "associated_constructor",
            },
        ],
        "risk_facts": {
            "unsafe_functions": [],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    compile_facts = markdown.split("## Compile-Time Facts", 1)[1].split("## Related APIs", 1)[0]
    assert "### Type Trait Facts" in compile_facts
    assert "fixture::Selector [kind=enum]: Copy=no; Clone=no; other_explicit_impls=core::fmt::Debug" in compile_facts
    assert "fixture::Payload [kind=struct]: Copy=yes; Clone=yes; other_explicit_impls=(none)" in compile_facts


def test_render_context_markdown_trait_target_surfaces_implementor_setup():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [
            {"module_id": "mod::fixture", "canonical_path": "fixture"},
            {"module_id": "mod::fixture::buf", "canonical_path": "fixture::buf"},
        ],
        "types": [
            {
                "type_id": "type::fixture::BytesMut",
                "name": "BytesMut",
                "canonical_path": "fixture::buf::BytesMut",
                "public_paths": ["fixture::BytesMut"],
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::UninitSlice",
                "name": "UninitSlice",
                "canonical_path": "fixture::buf::UninitSlice",
                "public_paths": ["fixture::buf::UninitSlice"],
                "public_anchor_module_id": "mod::fixture::buf",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::fixture::BufMut",
                "name": "BufMut",
                "canonical_path": "fixture::buf::BufMut",
                "public_paths": ["fixture::BufMut"],
                "public_anchor_module_id": "mod::fixture",
                "is_unsafe": True,
                "required_methods": ["advance_mut", "chunk_mut"],
                "provided_methods": ["put_slice"],
            }
        ],
        "trait_impl_registry": [
            {
                "trait_impl_id": "trait_impl::fixture::BufMut::for::BytesMut",
                "target_type_id": "type::fixture::BytesMut",
                "trait_id": "trait::fixture::BufMut",
                "trait_name": "BufMut",
                "trait_canonical_path": "fixture::buf::BufMut",
                "for_type_text": "BytesMut",
                "associated_type_bindings": [],
                "is_unsafe": False,
            }
        ],
        "apis": [
            {
                "api_id": "api::fixture::BufMut::advance_mut",
                "name": "advance_mut",
                "canonical_path": "fixture::buf::BufMut::advance_mut",
                "public_paths": ["fixture::BufMut::advance_mut"],
                "public_anchor_module_id": "mod::fixture",
                "owner_trait_id": "trait::fixture::BufMut",
                "signature_text": "unsafe fn advance_mut(&mut Self, usize)",
                "receiver": "&mut Self",
                "arg_types": ["usize"],
                "return_type": "()",
                "api_kind": "trait_method",
                "is_unsafe": True,
                "doc_sections": {"safety": "initialized bytes only"},
            },
            {
                "api_id": "api::fixture::BufMut::chunk_mut",
                "name": "chunk_mut",
                "canonical_path": "fixture::buf::BufMut::chunk_mut",
                "public_paths": ["fixture::BufMut::chunk_mut"],
                "public_anchor_module_id": "mod::fixture",
                "owner_trait_id": "trait::fixture::BufMut",
                "signature_text": "fn chunk_mut(&mut Self) -> &mut UninitSlice",
                "receiver": "&mut Self",
                "arg_types": [],
                "return_type": "&mut UninitSlice",
                "api_kind": "trait_method",
                "contains_unsafe_block": False,
            },
            {
                "api_id": "api::fixture::BytesMut::with_capacity",
                "name": "with_capacity",
                "canonical_path": "fixture::buf::BytesMut::with_capacity",
                "public_paths": ["fixture::BytesMut::with_capacity"],
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::BytesMut",
                "signature_text": "fn with_capacity(usize) -> BytesMut",
                "receiver": "",
                "arg_types": ["usize"],
                "return_type": "BytesMut",
                "api_kind": "associated_constructor",
            },
            {
                "api_id": "api::fixture::UninitSlice::new",
                "name": "new",
                "canonical_path": "fixture::buf::UninitSlice::new",
                "public_paths": ["fixture::buf::UninitSlice::new"],
                "public_anchor_module_id": "mod::fixture::buf",
                "owner_type_id": "type::fixture::UninitSlice",
                "signature_text": "fn new(&mut [u8]) -> &mut UninitSlice",
                "receiver": "",
                "arg_types": ["&mut [u8]"],
                "return_type": "&mut UninitSlice",
                "api_kind": "associated_constructor",
            },
        ],
        "risk_facts": {
            "unsafe_functions": ["api::fixture::BufMut::advance_mut"],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }

    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    assert "## Known Reachable Paths" in markdown
    assert "fixture::BytesMut::with_capacity" in markdown
    assert "## Compile-Time Facts" in markdown
    assert "### Exact Import Paths" in markdown
    assert "type::fixture::BytesMut => fixture::BytesMut [kind=type]" in markdown
    assert "### Required Traits" in markdown
    assert "fixture::BufMut: required_methods=advance_mut, chunk_mut" in markdown
    assert "### Trait Method Signatures" in markdown
    assert "fixture::buf::BufMut::advance_mut: unsafe fn advance_mut(&mut self, usize) [required]" in markdown


def test_render_context_markdown_surfaces_trait_implementor_facts_for_owner_setup_bounds():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [
            {"module_id": "mod::fixture", "canonical_path": "fixture"},
            {"module_id": "mod::fixture::region", "canonical_path": "fixture::region"},
        ],
        "types": [
            {
                "type_id": "type::fixture::SliceVec",
                "name": "SliceVec",
                "canonical_path": "fixture::common::SliceVec",
                "public_paths": ["fixture::SliceVec"],
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
                "generic_params": ["T", "H"],
                "where_clauses": [],
            },
            {
                "type_id": "type::fixture::region::ArenaHandle",
                "name": "ArenaHandle",
                "canonical_path": "fixture::region::ArenaHandle",
                "public_paths": ["fixture::region::ArenaHandle"],
                "public_anchor_module_id": "mod::fixture::region",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::region::ArenaToken",
                "name": "ArenaToken",
                "canonical_path": "fixture::region::ArenaToken",
                "public_paths": ["fixture::region::ArenaToken"],
                "public_anchor_module_id": "mod::fixture::region",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::fixture::AllocHandle",
                "name": "AllocHandle",
                "canonical_path": "fixture::AllocHandle",
                "public_paths": ["fixture::AllocHandle"],
                "public_anchor_module_id": "mod::fixture",
                "is_unsafe": False,
                "required_methods": ["allocate"],
                "provided_methods": [],
            }
        ],
        "trait_impl_registry": [
            {
                "trait_impl_id": "trait_impl::fixture::AllocHandle::for::ArenaHandle",
                "target_type_id": "type::fixture::region::ArenaHandle",
                "trait_id": "trait::fixture::AllocHandle",
                "trait_name": "AllocHandle",
                "trait_canonical_path": "fixture::AllocHandle",
                "for_type_text": "ArenaHandle<'a>",
                "associated_type_bindings": [],
                "is_unsafe": False,
            },
            {
                "trait_impl_id": "trait_impl::fixture::AllocHandle::for::ArenaToken",
                "target_type_id": "type::fixture::region::ArenaToken",
                "trait_id": "trait::fixture::AllocHandle",
                "trait_name": "AllocHandle",
                "trait_canonical_path": "fixture::AllocHandle",
                "for_type_text": "ArenaToken<'a>",
                "associated_type_bindings": [],
                "is_unsafe": False,
            },
        ],
        "apis": [
            {
                "api_id": "api::fixture::SliceVec::split_off",
                "name": "split_off",
                "canonical_path": "fixture::common::SliceVec::split_off",
                "public_paths": ["fixture::SliceVec::split_off"],
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::SliceVec",
                "signature_text": "fn split_off(&mut Self, usize) -> Self",
                "receiver": "&mut Self",
                "arg_types": ["usize"],
                "return_type": "Self",
                "api_kind": "method",
                "contains_unsafe_block": True,
                "where_clauses": ["H: AllocHandle + Clone"],
            },
            {
                "api_id": "api::fixture::SliceVec::with_capacity",
                "name": "with_capacity",
                "canonical_path": "fixture::common::SliceVec::with_capacity",
                "public_paths": ["fixture::SliceVec::with_capacity"],
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::SliceVec",
                "signature_text": "fn with_capacity(H, usize) -> Self",
                "receiver": "",
                "arg_types": ["H", "usize"],
                "return_type": "Self",
                "api_kind": "associated_constructor",
                "contains_unsafe_block": False,
                "where_clauses": ["H: AllocHandle"],
            },
            {
                "api_id": "api::fixture::AllocHandle::allocate",
                "name": "allocate",
                "canonical_path": "fixture::AllocHandle::allocate",
                "public_paths": ["fixture::AllocHandle::allocate"],
                "public_anchor_module_id": "mod::fixture",
                "owner_trait_id": "trait::fixture::AllocHandle",
                "signature_text": "fn allocate(&Self, usize) -> *mut u8",
                "receiver": "&Self",
                "arg_types": ["usize"],
                "return_type": "*mut u8",
                "api_kind": "trait_method",
                "contains_unsafe_block": False,
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    compile_facts = markdown.split("## Compile-Time Facts", 1)[1].split("## Related APIs", 1)[0]
    assert "### Trait Implementor Facts" in compile_facts
    assert (
        "- fixture::AllocHandle: public_implementors=fixture::region::ArenaHandle, fixture::region::ArenaToken"
        in compile_facts
    )


def test_render_context_markdown_discovers_owner_specialization_setup_paths_via_public_aliases(tmp_path):
    alias_path = tmp_path / "region.rs"
    alias_path.write_text(
        "\n".join(
            [
                "pub type SliceVec<'a, T> = fixture::common::SliceVec<T, ArenaHandle<'a>>;",
                "",
            ]
        ),
        encoding="utf-8",
    )
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [
            {"module_id": "mod::fixture", "canonical_path": "fixture"},
            {"module_id": "mod::fixture::common", "canonical_path": "fixture::common"},
            {"module_id": "mod::fixture::region", "canonical_path": "fixture::region"},
        ],
        "types": [
            {
                "type_id": "type::fixture::common::SliceVec",
                "name": "SliceVec",
                "canonical_path": "fixture::common::SliceVec",
                "public_paths": ["fixture::SliceVec"],
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "generic_params": ["T", "H"],
                "where_clauses": [],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::region::SliceVec",
                "name": "SliceVec",
                "canonical_path": "fixture::region::SliceVec",
                "public_paths": ["fixture::region::SliceVec"],
                "public_anchor_module_id": "mod::fixture::region",
                "kind": "type_alias",
                "generic_params": ["'a", "T"],
                "where_clauses": [],
                "code_ref": {
                    "file": str(alias_path),
                    "start_line": 1,
                    "end_line": 1,
                },
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::region::Arena",
                "name": "Arena",
                "canonical_path": "fixture::region::Arena",
                "public_paths": ["fixture::region::Arena"],
                "public_anchor_module_id": "mod::fixture::region",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::region::ArenaToken",
                "name": "ArenaToken",
                "canonical_path": "fixture::region::ArenaToken",
                "public_paths": ["fixture::region::ArenaToken"],
                "public_anchor_module_id": "mod::fixture::region",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::fixture::region::ArenaHandle",
                "name": "ArenaHandle",
                "canonical_path": "fixture::region::ArenaHandle",
                "public_paths": ["fixture::region::ArenaHandle"],
                "public_anchor_module_id": "mod::fixture::region",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::common::SliceVec::split_off",
                "name": "split_off",
                "canonical_path": "fixture::common::SliceVec::split_off",
                "public_paths": ["fixture::SliceVec::split_off"],
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::common::SliceVec",
                "signature_text": "fn split_off(&mut Self, usize) -> Self",
                "receiver": "&mut Self",
                "arg_types": ["usize"],
                "return_type": "Self",
                "api_kind": "method",
                "contains_unsafe_block": True,
                "where_clauses": ["H: Clone"],
            },
            {
                "api_id": "api::fixture::common::SliceVec::new",
                "name": "new",
                "canonical_path": "fixture::common::SliceVec::new",
                "public_paths": ["fixture::SliceVec::new"],
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::common::SliceVec",
                "signature_text": "fn new(H) -> Self",
                "receiver": "",
                "arg_types": ["H"],
                "return_type": "Self",
                "api_kind": "associated_constructor",
                "contains_unsafe_block": False,
                "where_clauses": [],
            },
            {
                "api_id": "api::fixture::region::Arena::new",
                "name": "new",
                "canonical_path": "fixture::region::Arena::new",
                "public_paths": ["fixture::region::Arena::new"],
                "public_anchor_module_id": "mod::fixture::region",
                "owner_type_id": "type::fixture::region::Arena",
                "signature_text": "fn new() -> Self",
                "receiver": "",
                "arg_types": [],
                "return_type": "Self",
                "api_kind": "associated_constructor",
                "contains_unsafe_block": False,
            },
            {
                "api_id": "api::fixture::region::Arena::generation_token",
                "name": "generation_token",
                "canonical_path": "fixture::region::Arena::generation_token",
                "public_paths": ["fixture::region::Arena::generation_token"],
                "public_anchor_module_id": "mod::fixture::region",
                "owner_type_id": "type::fixture::region::Arena",
                "signature_text": "fn generation_token(&'a Self) -> ArenaToken<'a>",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "ArenaToken<'a>",
                "api_kind": "method",
                "contains_unsafe_block": False,
            },
            {
                "api_id": "api::fixture::region::ArenaToken::weak",
                "name": "weak",
                "canonical_path": "fixture::region::ArenaToken::weak",
                "public_paths": ["fixture::region::ArenaToken::weak"],
                "public_anchor_module_id": "mod::fixture::region",
                "owner_type_id": "type::fixture::region::ArenaToken",
                "signature_text": "fn weak(&'a Self) -> ArenaHandle<'a>",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "ArenaHandle<'a>",
                "api_kind": "method",
                "contains_unsafe_block": False,
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    setup_section = markdown.split("## Known Reachable Paths", 1)[1].split("## Compile-Time Facts", 1)[0]

    assert "fixture::region::ArenaToken::weak" in setup_section
    assert "fixture::region::Arena::generation_token" in setup_section
    assert "fixture::region::Arena::new" in setup_section


def test_render_context_markdown_prefers_public_paths_for_compile_facing_context():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [
            {"module_id": "mod::fixture", "canonical_path": "fixture"},
            {"module_id": "mod::fixture::internal", "canonical_path": "fixture::internal"},
        ],
        "types": [
            {
                "type_id": "type::fixture::internal::Widget",
                "name": "Widget",
                "canonical_path": "fixture::internal::Widget",
                "public_paths": ["fixture::Widget"],
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            }
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::internal::Widget::danger",
                "name": "danger",
                "canonical_path": "fixture::internal::Widget::danger",
                "public_paths": ["fixture::Widget::danger"],
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::internal::Widget",
                "signature_text": "unsafe fn danger(&mut Self)",
                "receiver": "&mut Self",
                "arg_types": [],
                "return_type": "()",
                "api_kind": "inherent_method",
                "is_unsafe": True,
                "doc_sections": {"safety": "caller upholds widget invariant"},
            },
            {
                "api_id": "api::fixture::internal::Widget::new",
                "name": "new",
                "canonical_path": "fixture::internal::Widget::new",
                "public_paths": ["fixture::Widget::new"],
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::internal::Widget",
                "signature_text": "fn new() -> Widget",
                "receiver": "",
                "arg_types": [],
                "return_type": "Widget",
                "api_kind": "associated_constructor",
                "is_unsafe": False,
                "contains_unsafe_block": False,
            },
        ],
        "risk_facts": {
            "unsafe_functions": [],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    assert "- path: fixture::Widget::danger" in markdown
    assert "fixture::Widget::new" in markdown
    assert "type::fixture::internal::Widget => fixture::Widget [kind=type]" in markdown
    assert "type::fixture::internal::Widget => fixture::internal::Widget [kind=type]" not in markdown


def test_render_context_markdown_renders_unsafe_signatures_explicitly():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [{"module_id": "mod::fixture", "canonical_path": "fixture"}],
        "types": [
            {
                "type_id": "type::fixture::Buffer",
                "name": "Buffer",
                "canonical_path": "fixture::Buffer",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            }
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::Buffer::danger",
                "name": "danger",
                "canonical_path": "fixture::Buffer::danger",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Buffer",
                "signature_text": "fn danger(&mut Self)",
                "receiver": "&mut Self",
                "arg_types": [],
                "return_type": "()",
                "api_kind": "inherent_method",
                "is_unsafe": True,
                "doc_sections": {"safety": "caller must uphold buffer invariant"},
            },
            {
                "api_id": "api::fixture::Buffer::unsafe_helper",
                "name": "unsafe_helper",
                "canonical_path": "fixture::Buffer::unsafe_helper",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Buffer",
                "signature_text": "fn unsafe_helper(&mut Self)",
                "receiver": "&mut Self",
                "arg_types": [],
                "return_type": "()",
                "api_kind": "inherent_method",
                "is_unsafe": True,
                "doc_sections": {"safety": "must be called in unsafe context"},
            },
            {
                "api_id": "api::fixture::Buffer::new",
                "name": "new",
                "canonical_path": "fixture::Buffer::new",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::Buffer",
                "signature_text": "fn new() -> Buffer",
                "receiver": "",
                "arg_types": [],
                "return_type": "Buffer",
                "api_kind": "associated_constructor",
                "is_unsafe": False,
                "contains_unsafe_block": False,
            },
        ],
        "risk_facts": {
            "unsafe_functions": [],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }
    graph = build_graph(knowledge)
    target = next(
        target
        for target in rank_unsafe_targets(graph)
        if target.api_id == "api::fixture::Buffer::danger"
    )

    markdown = render_context_markdown(knowledge, graph, target)

    assert "- signature: unsafe fn danger(&mut Self)" in markdown
    assert "unsafe fn unsafe_helper(&mut Self)" in markdown


def test_render_context_markdown_keeps_deep_setup_root_constructor():
    knowledge = {
        "crate_meta": {"crate_import_name": "fixture"},
        "modules": [{"module_id": "mod::fixture", "canonical_path": "fixture"}],
        "types": [
            {
                "type_id": f"type::fixture::T{i}",
                "name": f"T{i}",
                "canonical_path": f"fixture::T{i}",
                "public_anchor_module_id": "mod::fixture",
                "kind": "struct",
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            }
            for i in range(5)
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::fixture::T0::target",
                "name": "target",
                "canonical_path": "fixture::T0::target",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::T0",
                "signature": "fn target(&Self)",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "()",
                "api_kind": "method",
                "contains_unsafe_block": True,
            },
            {
                "api_id": "api::fixture::T1::to_t0",
                "name": "to_t0",
                "canonical_path": "fixture::T1::to_t0",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::T1",
                "signature": "fn to_t0(&Self) -> T0",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "T0",
                "api_kind": "method",
            },
            {
                "api_id": "api::fixture::T2::to_t1",
                "name": "to_t1",
                "canonical_path": "fixture::T2::to_t1",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::T2",
                "signature": "fn to_t1(&Self) -> T1",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "T1",
                "api_kind": "method",
            },
            {
                "api_id": "api::fixture::T3::to_t2",
                "name": "to_t2",
                "canonical_path": "fixture::T3::to_t2",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::T3",
                "signature": "fn to_t2(&Self) -> T2",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "T2",
                "api_kind": "method",
            },
            {
                "api_id": "api::fixture::T4::to_t3",
                "name": "to_t3",
                "canonical_path": "fixture::T4::to_t3",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::T4",
                "signature": "fn to_t3(&Self) -> T3",
                "receiver": "&Self",
                "arg_types": [],
                "return_type": "T3",
                "api_kind": "method",
            },
            {
                "api_id": "api::fixture::T4::new",
                "name": "new",
                "canonical_path": "fixture::T4::new",
                "public_anchor_module_id": "mod::fixture",
                "owner_type_id": "type::fixture::T4",
                "signature": "fn new() -> T4",
                "receiver": "",
                "arg_types": [],
                "return_type": "T4",
                "api_kind": "associated_constructor",
            },
        ],
        "risk_facts": {
            "unsafe_functions": [],
            "ffi_functions": [],
            "panic_sites": [],
        },
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    setup_section = markdown.split("## Known Reachable Paths", 1)[1].split("## Compile-Time Facts", 1)[0]
    assert "fixture::T4::new" in setup_section


def test_render_context_markdown_surfaces_owner_documentation_facts_for_default_generic_setup():
    knowledge = {
        "crate_meta": {
            "crate_import_name": "sized_chunks",
            "root_docs": (
                "Their sizing information is encoded in the type. "
                "You can also omit the size, as they all default to a size of 64, "
                "so `SparseChunk<A>` would be a sparse array with a capacity of 64."
            ),
        },
        "modules": [
            {"module_id": "mod::sized_chunks", "canonical_path": "sized_chunks"},
        ],
        "types": [
            {
                "type_id": "type::sized_chunks::SparseChunk",
                "name": "SparseChunk",
                "canonical_path": "sized_chunks::SparseChunk",
                "public_paths": ["sized_chunks::SparseChunk"],
                "public_anchor_module_id": "mod::sized_chunks",
                "kind": "struct",
                "generic_params": ["A", "N"],
                "where_clauses": ["N: Bits + ChunkLength<A>"],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
                "doc_sections": {"summary": "A fixed capacity sparse array."},
            }
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "apis": [
            {
                "api_id": "api::sized_chunks::SparseChunk::get_unchecked",
                "name": "get_unchecked",
                "canonical_path": "sized_chunks::SparseChunk::get_unchecked",
                "public_paths": ["sized_chunks::SparseChunk::get_unchecked"],
                "public_anchor_module_id": "mod::sized_chunks",
                "owner_type_id": "type::sized_chunks::SparseChunk",
                "signature_text": "unsafe fn get_unchecked(&Self, usize) -> &A",
                "receiver": "&Self",
                "arg_types": ["usize"],
                "return_type": "&A",
                "api_kind": "inherent_method",
                "is_unsafe": True,
                "doc_sections": {"safety": "index must be inhabited"},
            },
            {
                "api_id": "api::sized_chunks::SparseChunk::new",
                "name": "new",
                "canonical_path": "sized_chunks::SparseChunk::new",
                "public_paths": ["sized_chunks::SparseChunk::new"],
                "public_anchor_module_id": "mod::sized_chunks",
                "owner_type_id": "type::sized_chunks::SparseChunk",
                "signature_text": "fn new() -> Self",
                "receiver": "",
                "arg_types": [],
                "return_type": "Self",
                "api_kind": "associated_constructor",
                "contains_unsafe_block": False,
                "doc_sections": {},
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    compile_facts = markdown.split("## Compile-Time Facts", 1)[1].split("## Related APIs", 1)[0]
    variant_section = markdown.split("## Variant Opportunities", 1)[1].split(
        "## Similar API Usage", 1
    )[0]

    assert "### Owner Documentation Facts" in compile_facts
    assert "sized_chunks::SparseChunk" in compile_facts
    assert "omit the size" in compile_facts
    assert "defaults to size 64" in compile_facts
    assert "SparseChunk<A>" in compile_facts
    assert "documented owner setup shape" in variant_section
    assert "SparseChunk<A>" in variant_section


def test_render_context_markdown_surfaces_owner_construction_bridges_from_public_trait_impls():
    knowledge = {
        "crate_meta": {"crate_import_name": "stack"},
        "modules": [{"module_id": "mod::stack", "canonical_path": "stack"}],
        "types": [
            {
                "type_id": "type::stack::ArrayVec",
                "name": "ArrayVec",
                "canonical_path": "stack::ArrayVec",
                "public_paths": ["stack::ArrayVec"],
                "public_anchor_module_id": "mod::stack",
                "kind": "struct",
                "generic_params": ["T"],
                "where_clauses": ["T: Array"],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::stack::SmallVec",
                "name": "SmallVec",
                "canonical_path": "stack::SmallVec",
                "public_paths": ["stack::SmallVec"],
                "public_anchor_module_id": "mod::stack",
                "kind": "struct",
                "generic_params": ["T", "S"],
                "where_clauses": ["T: Array"],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::stack::Array",
                "name": "Array",
                "canonical_path": "stack::Array",
                "public_paths": ["stack::Array"],
                "public_anchor_module_id": "mod::stack",
                "is_unsafe": True,
                "required_methods": ["as_mut_ptr", "as_ptr", "len"],
                "provided_methods": ["uninitialized"],
            }
        ],
        "trait_impl_registry": [
            {
                "trait_impl_id": "trait_impl::core::convert::From<T>::for::ArrayVec<T>",
                "target_type_id": "type::stack::ArrayVec",
                "trait_id": "trait::core::convert::From",
                "trait_name": "From",
                "trait_canonical_path": "core::convert::From",
                "trait_ref_text": "core::convert::From<T>",
                "for_type_text": "ArrayVec<T>",
                "where_clauses": ["T: Array"],
                "associated_type_bindings": [],
                "associated_const_bindings": [],
                "is_unsafe": False,
            },
            {
                "trait_impl_id": "trait_impl::core::convert::From<ArrayVec<T>>::for::SmallVec<T, S>",
                "target_type_id": "type::stack::SmallVec",
                "trait_id": "trait::core::convert::From",
                "trait_name": "From",
                "trait_canonical_path": "core::convert::From",
                "trait_ref_text": "core::convert::From<ArrayVec<T>>",
                "for_type_text": "SmallVec<T, S>",
                "where_clauses": ["T: Array"],
                "associated_type_bindings": [],
                "associated_const_bindings": [],
                "is_unsafe": False,
            },
        ],
        "apis": [
            {
                "api_id": "api::stack::ArrayVec::into_inner",
                "name": "into_inner",
                "canonical_path": "stack::ArrayVec::into_inner",
                "public_paths": ["stack::ArrayVec::into_inner"],
                "public_anchor_module_id": "mod::stack",
                "owner_type_id": "type::stack::ArrayVec",
                "signature_text": "fn into_inner(Self) -> Result<T, Self>",
                "receiver": "Self",
                "arg_types": [],
                "return_type": "Result<T, Self>",
                "api_kind": "inherent_method",
                "contains_unsafe_block": True,
                "doc_sections": {},
            },
            {
                "api_id": "api::stack::SmallVec::into_inner",
                "name": "into_inner",
                "canonical_path": "stack::SmallVec::into_inner",
                "public_paths": ["stack::SmallVec::into_inner"],
                "public_anchor_module_id": "mod::stack",
                "owner_type_id": "type::stack::SmallVec",
                "signature_text": "fn into_inner(Self) -> ArrayVec<T>",
                "receiver": "Self",
                "arg_types": [],
                "return_type": "ArrayVec<T>",
                "api_kind": "inherent_method",
                "contains_unsafe_block": False,
                "doc_sections": {},
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    compile_facts = markdown.split("## Compile-Time Facts", 1)[1].split("## Related APIs", 1)[0]
    variant_section = markdown.split("## Variant Opportunities", 1)[1].split(
        "## Similar API Usage", 1
    )[0]

    assert "### Owner Construction Bridges" in compile_facts
    assert "stack::ArrayVec::from(...)" in compile_facts
    assert "impl core::convert::From<T> for stack::ArrayVec<T>" in compile_facts
    assert "where=T: Array" in compile_facts
    assert "public conversion constructor available" in variant_section
    assert "stack::ArrayVec::from(...)" in variant_section


def test_render_context_markdown_deprioritizes_owner_bridge_setup_when_direct_owner_conversion_exists():
    knowledge = {
        "crate_meta": {"crate_import_name": "stack"},
        "modules": [{"module_id": "mod::stack", "canonical_path": "stack"}],
        "types": [
            {
                "type_id": "type::stack::ArrayVec",
                "name": "ArrayVec",
                "canonical_path": "stack::ArrayVec",
                "public_paths": ["stack::ArrayVec"],
                "public_anchor_module_id": "mod::stack",
                "kind": "struct",
                "generic_params": ["T"],
                "where_clauses": ["T: Array"],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
            {
                "type_id": "type::stack::SmallVec",
                "name": "SmallVec",
                "canonical_path": "stack::SmallVec",
                "public_paths": ["stack::SmallVec"],
                "public_anchor_module_id": "mod::stack",
                "kind": "struct",
                "generic_params": ["T", "S"],
                "where_clauses": ["T: Array"],
                "variants": [],
                "has_hidden_fields": False,
                "has_hidden_variants": False,
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::stack::Array",
                "name": "Array",
                "canonical_path": "stack::Array",
                "public_paths": ["stack::Array"],
                "public_anchor_module_id": "mod::stack",
                "is_unsafe": True,
                "required_methods": ["as_mut_ptr", "as_ptr", "len"],
                "provided_methods": ["uninitialized"],
            }
        ],
        "trait_impl_registry": [
            {
                "trait_impl_id": "trait_impl::core::convert::From<T>::for::ArrayVec<T>",
                "target_type_id": "type::stack::ArrayVec",
                "trait_id": "trait::core::convert::From",
                "trait_name": "From",
                "trait_canonical_path": "core::convert::From",
                "trait_ref_text": "core::convert::From<T>",
                "for_type_text": "ArrayVec<T>",
                "where_clauses": ["T: Array"],
                "associated_type_bindings": [],
                "associated_const_bindings": [],
                "is_unsafe": False,
            }
        ],
        "apis": [
            {
                "api_id": "api::stack::ArrayVec::into_inner",
                "name": "into_inner",
                "canonical_path": "stack::ArrayVec::into_inner",
                "public_paths": ["stack::ArrayVec::into_inner"],
                "public_anchor_module_id": "mod::stack",
                "owner_type_id": "type::stack::ArrayVec",
                "signature_text": "fn into_inner(Self) -> Result<T, Self>",
                "receiver": "Self",
                "arg_types": [],
                "return_type": "Result<T, Self>",
                "api_kind": "inherent_method",
                "contains_unsafe_block": True,
                "doc_sections": {},
            },
            {
                "api_id": "api::stack::SmallVec::into_inner",
                "name": "into_inner",
                "canonical_path": "stack::SmallVec::into_inner",
                "public_paths": ["stack::SmallVec::into_inner"],
                "public_anchor_module_id": "mod::stack",
                "owner_type_id": "type::stack::SmallVec",
                "signature_text": "fn into_inner(Self) -> Coalesce2<ArrayVec<T>, S>",
                "receiver": "Self",
                "arg_types": [],
                "return_type": "Coalesce2<ArrayVec<T>, S>",
                "api_kind": "inherent_method",
                "contains_unsafe_block": False,
                "doc_sections": {},
            },
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)
    reachability_section = markdown.split("## Known Reachable Paths", 1)[1].split(
        "## Compile-Time Facts", 1
    )[0]
    variant_section = markdown.split("## Variant Opportunities", 1)[1].split(
        "## Similar API Usage", 1
    )[0]

    assert "stack::SmallVec::into_inner" not in reachability_section
    assert "stack::ArrayVec::from(...)" in variant_section
