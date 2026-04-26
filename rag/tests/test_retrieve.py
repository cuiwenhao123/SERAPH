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
