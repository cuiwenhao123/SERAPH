from seraph_rag.idioms import bundled_idioms


def test_bundled_idioms_include_unsafe_and_result_guidance():
    idioms = bundled_idioms()
    categories = {idiom.metadata["category"] for idiom in idioms}
    text = "\n".join(idiom.text for idiom in idioms)
    assert "unsafe_semantics" in categories
    assert "error_handling" in categories
    assert "Result" in text
