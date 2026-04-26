from seraph_rag.graph_builder import build_graph
from seraph_rag.retrieve import (
    rank_unsafe_targets,
    render_context_markdown,
    select_unsafe_target,
    target_missing_seed_type_ids,
)


def ranking_knowledge():
    return {
        "crate_meta": {"crate_import_name": "rank_fixture"},
        "modules": [
            {"module_id": "mod::rank_fixture", "canonical_path": "rank_fixture"}
        ],
        "types": [
            {
                "type_id": "type::rank_fixture::Bag",
                "name": "Bag",
                "canonical_path": "rank_fixture::Bag",
                "public_anchor_module_id": "mod::rank_fixture",
            }
        ],
        "apis": [
            {
                "api_id": "api::rank_fixture::Bag::new",
                "name": "new",
                "canonical_path": "rank_fixture::Bag::new",
                "public_anchor_module_id": "mod::rank_fixture",
                "owner_type_id": "type::rank_fixture::Bag",
                "api_kind": "constructor",
                "signature_text": "fn new() -> Bag",
                "return_type": "Self",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::rank_fixture::Bag::iter",
                "name": "iter",
                "canonical_path": "rank_fixture::Bag::iter",
                "public_anchor_module_id": "mod::rank_fixture",
                "owner_type_id": "type::rank_fixture::Bag",
                "signature_text": "fn iter(&Self)",
                "receiver": "&Self",
                "return_type": "Iter",
                "arg_types": [],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::rank_fixture::Bag::get_unchecked",
                "name": "get_unchecked",
                "canonical_path": "rank_fixture::Bag::get_unchecked",
                "public_anchor_module_id": "mod::rank_fixture",
                "owner_type_id": "type::rank_fixture::Bag",
                "signature_text": "unsafe fn get_unchecked(&Self, usize) -> u8",
                "receiver": "&Self",
                "return_type": "u8",
                "arg_types": ["usize"],
                "is_unsafe": True,
                "doc_sections": {"safety": "index must be in bounds"},
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }


def test_target_ranking_prefers_explicit_unchecked_safety_api():
    graph = build_graph(ranking_knowledge())
    targets = rank_unsafe_targets(graph)
    assert targets[0].api_id == "api::rank_fixture::Bag::get_unchecked"


def test_related_api_rendering_includes_roles_and_prioritizes_constructor():
    knowledge = ranking_knowledge()
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]
    markdown = render_context_markdown(knowledge, graph, target)
    assert "Bag::new" in markdown
    assert markdown.index("Bag::new") < markdown.index("Bag::iter")


def target_companion_knowledge():
    return {
        "crate_meta": {"crate_import_name": "companion_fixture"},
        "modules": [
            {"module_id": "mod::companion_fixture", "canonical_path": "companion_fixture"},
            {"module_id": "mod::companion_fixture::util", "canonical_path": "companion_fixture::util"},
        ],
        "types": [
            {
                "type_id": "type::companion_fixture::Session",
                "name": "Session",
                "canonical_path": "companion_fixture::Session",
                "public_anchor_module_id": "mod::companion_fixture",
            },
            {
                "type_id": "type::companion_fixture::Ffmpeg",
                "name": "Ffmpeg",
                "canonical_path": "companion_fixture::Ffmpeg",
                "public_anchor_module_id": "mod::companion_fixture",
            },
            {
                "type_id": "type::companion_fixture::Error",
                "name": "Error",
                "canonical_path": "companion_fixture::util::Error",
                "public_anchor_module_id": "mod::companion_fixture::util",
            },
        ],
        "apis": [
            {
                "api_id": "api::companion_fixture::Session::danger",
                "name": "danger",
                "canonical_path": "companion_fixture::Session::danger",
                "public_anchor_module_id": "mod::companion_fixture",
                "owner_type_id": "type::companion_fixture::Session",
                "signature_text": "fn danger(&Self)",
                "receiver": "&Self",
                "return_type": "()",
                "arg_types": [],
                "api_kind": "method",
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::companion_fixture::Session::helper",
                "name": "helper",
                "canonical_path": "companion_fixture::Session::helper",
                "public_anchor_module_id": "mod::companion_fixture",
                "owner_type_id": "type::companion_fixture::Session",
                "signature_text": "fn helper(&Self)",
                "receiver": "&Self",
                "return_type": "()",
                "arg_types": [],
                "api_kind": "method",
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::companion_fixture::Ffmpeg::new",
                "name": "new",
                "canonical_path": "companion_fixture::Ffmpeg::new",
                "public_anchor_module_id": "mod::companion_fixture",
                "owner_type_id": "type::companion_fixture::Ffmpeg",
                "signature_text": "fn new() -> Ffmpeg",
                "return_type": "Self",
                "arg_types": [],
                "api_kind": "constructor",
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::companion_fixture::util::Error::eof",
                "name": "eof",
                "canonical_path": "companion_fixture::util::Error::eof",
                "public_anchor_module_id": "mod::companion_fixture::util",
                "owner_type_id": "type::companion_fixture::Error",
                "signature_text": "fn eof() -> Error",
                "return_type": "Self",
                "arg_types": [],
                "api_kind": "constructor",
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }


def test_render_context_markdown_includes_other_target_apis_as_related_companions():
    knowledge = target_companion_knowledge()
    graph = build_graph(knowledge)
    target = next(
        item
        for item in rank_unsafe_targets(graph)
        if item.api_id == "api::companion_fixture::Session::danger"
    )

    markdown = render_context_markdown(knowledge, graph, target, max_related_apis=6)

    related = markdown.split("## Related APIs", 1)[1].split("## Semantically Similar API Docs", 1)[0]
    assert "companion_fixture::Session::helper" in related
    assert "companion_fixture::Ffmpeg::new" in related
    assert "companion_fixture::util::Error::eof" in related


def setup_chain_knowledge():
    return {
        "crate_meta": {"crate_import_name": "chain_fixture"},
        "modules": [
            {"module_id": "mod::chain_fixture::avcodec", "canonical_path": "chain_fixture::avcodec"},
            {"module_id": "mod::chain_fixture::avformat", "canonical_path": "chain_fixture::avformat"},
            {"module_id": "mod::chain_fixture::avutil", "canonical_path": "chain_fixture::avutil"},
        ],
        "types": [
            {
                "type_id": "type::chain_fixture::DecodeContext",
                "name": "DecodeContext",
                "canonical_path": "chain_fixture::DecodeContext",
                "public_anchor_module_id": "mod::chain_fixture::avcodec",
            },
            {
                "type_id": "type::chain_fixture::Packet",
                "name": "Packet",
                "canonical_path": "chain_fixture::Packet",
                "public_anchor_module_id": "mod::chain_fixture::avcodec",
            },
            {
                "type_id": "type::chain_fixture::VideoFrame",
                "name": "VideoFrame",
                "canonical_path": "chain_fixture::VideoFrame",
                "public_anchor_module_id": "mod::chain_fixture::avutil",
            },
            {
                "type_id": "type::chain_fixture::InputCodecParameters",
                "name": "InputCodecParameters",
                "canonical_path": "chain_fixture::InputCodecParameters",
                "public_anchor_module_id": "mod::chain_fixture::avcodec",
            },
            {
                "type_id": "type::chain_fixture::InputStream",
                "name": "InputStream",
                "canonical_path": "chain_fixture::InputStream",
                "public_anchor_module_id": "mod::chain_fixture::avformat",
            },
            {
                "type_id": "type::chain_fixture::InputFormatContext",
                "name": "InputFormatContext",
                "canonical_path": "chain_fixture::InputFormatContext",
                "public_anchor_module_id": "mod::chain_fixture::avformat",
            },
            {
                "type_id": "type::chain_fixture::Dictionary",
                "name": "Dictionary",
                "canonical_path": "chain_fixture::Dictionary",
                "public_anchor_module_id": "mod::chain_fixture::avutil",
            },
            {
                "type_id": "type::chain_fixture::Error",
                "name": "Error",
                "canonical_path": "chain_fixture::Error",
                "public_anchor_module_id": "mod::chain_fixture::avutil",
            },
        ],
        "apis": [
            {
                "api_id": "api::chain_fixture::DecodeContext::decode_video",
                "name": "decode_video",
                "canonical_path": "chain_fixture::DecodeContext::decode_video",
                "public_anchor_module_id": "mod::chain_fixture::avcodec",
                "owner_type_id": "type::chain_fixture::DecodeContext",
                "api_kind": "inherent_method",
                "signature_text": "fn decode_video(&Self, &Packet, &mut VideoFrame) -> Result<bool, Error>",
                "receiver": "&Self",
                "return_type": "Result<bool, Error>",
                "arg_types": ["&Packet", "&mut VideoFrame"],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::chain_fixture::InputCodecParameters::new_decoder",
                "name": "new_decoder",
                "canonical_path": "chain_fixture::InputCodecParameters::new_decoder",
                "public_anchor_module_id": "mod::chain_fixture::avcodec",
                "owner_type_id": "type::chain_fixture::InputCodecParameters",
                "api_kind": "inherent_method",
                "signature_text": "fn new_decoder(&Self, &mut Dictionary) -> Result<DecodeContext, Error>",
                "receiver": "&Self",
                "return_type": "Result<DecodeContext, Error>",
                "arg_types": ["&mut Dictionary"],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": "decoder state must outlive packet references"},
            },
            {
                "api_id": "api::chain_fixture::InputStream::codecpar",
                "name": "codecpar",
                "canonical_path": "chain_fixture::InputStream::codecpar",
                "public_anchor_module_id": "mod::chain_fixture::avformat",
                "owner_type_id": "type::chain_fixture::InputStream",
                "api_kind": "inherent_method",
                "signature_text": "fn codecpar(&Self) -> InputCodecParameters",
                "receiver": "&Self",
                "return_type": "InputCodecParameters",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::chain_fixture::InputFormatContext::read_frame",
                "name": "read_frame",
                "canonical_path": "chain_fixture::InputFormatContext::read_frame",
                "public_anchor_module_id": "mod::chain_fixture::avformat",
                "owner_type_id": "type::chain_fixture::InputFormatContext",
                "api_kind": "inherent_method",
                "signature_text": "fn read_frame(&Self) -> Result<Packet, Error>",
                "receiver": "&Self",
                "return_type": "Result<Packet, Error>",
                "arg_types": [],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::chain_fixture::InputFormatContext::stream",
                "name": "stream",
                "canonical_path": "chain_fixture::InputFormatContext::stream",
                "public_anchor_module_id": "mod::chain_fixture::avformat",
                "owner_type_id": "type::chain_fixture::InputFormatContext",
                "api_kind": "inherent_method",
                "signature_text": "fn stream(&Self) -> InputStream",
                "receiver": "&Self",
                "return_type": "InputStream",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::chain_fixture::VideoFrame::empty",
                "name": "empty",
                "canonical_path": "chain_fixture::VideoFrame::empty",
                "public_anchor_module_id": "mod::chain_fixture::avutil",
                "owner_type_id": "type::chain_fixture::VideoFrame",
                "api_kind": "constructor",
                "signature_text": "fn empty() -> Result<VideoFrame, Error>",
                "return_type": "Result<VideoFrame, Error>",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::chain_fixture::Dictionary::new",
                "name": "new",
                "canonical_path": "chain_fixture::Dictionary::new",
                "public_anchor_module_id": "mod::chain_fixture::avutil",
                "owner_type_id": "type::chain_fixture::Dictionary",
                "api_kind": "constructor",
                "signature_text": "fn new() -> Dictionary",
                "return_type": "Self",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::chain_fixture::Error::eof",
                "name": "eof",
                "canonical_path": "chain_fixture::Error::eof",
                "public_anchor_module_id": "mod::chain_fixture::avutil",
                "owner_type_id": "type::chain_fixture::Error",
                "api_kind": "constructor",
                "signature_text": "fn eof() -> Error",
                "return_type": "Self",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::chain_fixture::Error::invalid_data",
                "name": "invalid_data",
                "canonical_path": "chain_fixture::Error::invalid_data",
                "public_anchor_module_id": "mod::chain_fixture::avutil",
                "owner_type_id": "type::chain_fixture::Error",
                "api_kind": "constructor",
                "signature_text": "fn invalid_data() -> Error",
                "return_type": "Self",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }


def test_render_context_markdown_surfaces_required_setup_chain_by_default():
    knowledge = setup_chain_knowledge()
    graph = build_graph(knowledge)
    target = next(
        target
        for target in rank_unsafe_targets(graph)
        if target.api_id == "api::chain_fixture::DecodeContext::decode_video"
    )

    markdown = render_context_markdown(knowledge, graph, target)

    required = markdown.split("## Required Setup APIs", 1)[1].split("## Related APIs", 1)[0]
    assert "InputCodecParameters::new_decoder" in required
    assert "InputStream::codecpar" in required
    assert "InputFormatContext::read_frame" in required
    assert required.index("InputStream::codecpar") < required.index("InputCodecParameters::new_decoder")


def deref_setup_knowledge():
    return {
        "crate_meta": {"crate_import_name": "deref_chain_fixture"},
        "modules": [
            {"module_id": "mod::deref_chain_fixture", "canonical_path": "deref_chain_fixture"}
        ],
        "types": [
            {
                "type_id": "type::deref_chain_fixture::Wrapper",
                "name": "Wrapper",
                "canonical_path": "deref_chain_fixture::Wrapper",
                "public_anchor_module_id": "mod::deref_chain_fixture",
            },
            {
                "type_id": "type::deref_chain_fixture::OpaqueTarget",
                "name": "OpaqueTarget",
                "canonical_path": "deref_chain_fixture::OpaqueTarget",
                "public_anchor_module_id": "mod::deref_chain_fixture",
            },
        ],
        "apis": [
            {
                "api_id": "api::deref_chain_fixture::Source::make_wrapper",
                "name": "make_wrapper",
                "canonical_path": "deref_chain_fixture::Source::make_wrapper",
                "public_anchor_module_id": "mod::deref_chain_fixture",
                "api_kind": "constructor",
                "signature_text": "fn make_wrapper() -> Wrapper",
                "return_type": "Wrapper",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::deref_chain_fixture::OpaqueTarget::dims",
                "name": "dims",
                "canonical_path": "deref_chain_fixture::OpaqueTarget::dims",
                "public_anchor_module_id": "mod::deref_chain_fixture",
                "owner_type_id": "type::deref_chain_fixture::OpaqueTarget",
                "api_kind": "inherent_method",
                "signature_text": "fn dims(&Self) -> usize",
                "receiver": "&Self",
                "return_type": "usize",
                "arg_types": [],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
        ],
        "trait_registry": [
            {
                "trait_id": "trait::core::ops::deref::Deref",
                "name": "Deref",
                "canonical_path": "core::ops::deref::Deref",
                "is_unsafe": False,
            }
        ],
        "trait_impl_registry": [
            {
                "trait_impl_id": "impl::Wrapper->Deref",
                "target_type_id": "type::deref_chain_fixture::Wrapper",
                "trait_id": "trait::core::ops::deref::Deref",
                "trait_name": "Deref",
                "associated_type_bindings": [
                    {
                        "name": "Target",
                        "assigned_type": "OpaqueTarget",
                    }
                ],
            }
        ],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }


def test_render_context_uses_deref_wrapper_producer_for_target_owner_type():
    knowledge = deref_setup_knowledge()
    graph = build_graph(knowledge)
    target = rank_unsafe_targets(graph)[0]

    markdown = render_context_markdown(knowledge, graph, target)

    required = markdown.split("## Required Setup APIs", 1)[1].split("## Related APIs", 1)[0]
    assert "Source::make_wrapper" in required


def unconstructible_target_knowledge():
    return {
        "crate_meta": {"crate_import_name": "construct_fixture"},
        "modules": [{"module_id": "mod::construct_fixture", "canonical_path": "construct_fixture"}],
        "types": [
            {
                "type_id": "type::construct_fixture::DecodeContext",
                "name": "DecodeContext",
                "canonical_path": "construct_fixture::DecodeContext",
                "public_anchor_module_id": "mod::construct_fixture",
            },
            {
                "type_id": "type::construct_fixture::EncodeContext",
                "name": "EncodeContext",
                "canonical_path": "construct_fixture::EncodeContext",
                "public_anchor_module_id": "mod::construct_fixture",
            },
            {
                "type_id": "type::construct_fixture::Packet",
                "name": "Packet",
                "canonical_path": "construct_fixture::Packet",
                "public_anchor_module_id": "mod::construct_fixture",
            },
        ],
        "apis": [
            {
                "api_id": "api::construct_fixture::Seed::new_decoder",
                "name": "new_decoder",
                "canonical_path": "construct_fixture::Seed::new_decoder",
                "public_anchor_module_id": "mod::construct_fixture",
                "api_kind": "constructor",
                "signature_text": "fn new_decoder() -> DecodeContext",
                "return_type": "DecodeContext",
                "arg_types": [],
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::construct_fixture::DecodeContext::decode",
                "name": "decode",
                "canonical_path": "construct_fixture::DecodeContext::decode",
                "public_anchor_module_id": "mod::construct_fixture",
                "owner_type_id": "type::construct_fixture::DecodeContext",
                "api_kind": "inherent_method",
                "signature_text": "fn decode(&Self, &Packet)",
                "receiver": "&Self",
                "return_type": "()",
                "arg_types": ["&Packet"],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": ""},
            },
            {
                "api_id": "api::construct_fixture::EncodeContext::open",
                "name": "open",
                "canonical_path": "construct_fixture::EncodeContext::open",
                "public_anchor_module_id": "mod::construct_fixture",
                "owner_type_id": "type::construct_fixture::EncodeContext",
                "api_kind": "inherent_method",
                "signature_text": "fn open(&mut Self)",
                "receiver": "&mut Self",
                "return_type": "()",
                "arg_types": [],
                "contains_unsafe_block": True,
                "doc_sections": {"safety": "requires initialized encoder context"},
            },
        ],
        "trait_registry": [],
        "trait_impl_registry": [],
        "risk_facts": {"unsafe_functions": [], "ffi_functions": [], "panic_sites": []},
    }


def test_select_unsafe_target_skips_unconstructible_targets_by_default():
    knowledge = unconstructible_target_knowledge()
    graph = build_graph(knowledge)

    missing = target_missing_seed_type_ids(graph, "api::construct_fixture::EncodeContext::open")
    assert missing == ["type::construct_fixture::EncodeContext"]

    target = select_unsafe_target(graph, round_no=1)
    assert target.api_id == "api::construct_fixture::DecodeContext::decode"
