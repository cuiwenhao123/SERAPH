# deepSURF Rust Dataset Extraction

This note extracts the Rust evaluation dataset used by the deepSURF paper from the upstream paper and released repository.

## Sources

- Paper: `deepSURF: Dynamic Language Model-Enhanced Fuzz Harness Generation for Rust Libraries`
  - arXiv abstract: <https://arxiv.org/abs/2506.15648>
  - arXiv HTML: <https://arxiv.org/html/2506.15648>
- Upstream repository: <https://github.com/purseclab/deepSURF>
- Dataset root used for the concrete crate list:
  - <https://github.com/purseclab/deepSURF/tree/main/dataset>

## What the paper says

The paper states that the evaluation dataset contains `63` real-world Rust crates.

- The paper text says `44` crates come from `ERASAN` and `RUSTSAN`.
- The paper text says the remaining `19` crates come from `CrabTree` and `RUG`.

## What the released repository contains

The released repository currently exposes these dataset directories under `dataset/`:

- `erasan_crates`: `27`
- `rustsan_crates`: `16`
- `crabtree_crates`: `8`
- `rug_crates`: `12`

That is still `63` total entries, but it sums to `43 + 20` rather than `44 + 19`.

For reproducibility, the CSV extraction in [deepsurf-rust-dataset.csv](/home/cas/Desktop/SERAPH/docs/research/deepsurf-rust-dataset.csv) follows the released repository exactly.

## Notes

- `rug_crates/hashes` is a workspace root, not a version-suffixed single-package directory.
- `rug_crates/time` is also a workspace root; its default member is `time`.
- The list below therefore preserves upstream directory names instead of trying to normalize them heuristically.
