# Research Notes

This directory will hold experiment notes, benchmark plans, evaluation records, and research-oriented observations for SERAPH.

## 2026-04-29 Cleaned deepSURF Round-1 Replay

- Baseline round-1 snapshot remains in:
  - [docs/research/deepsurf-round1-results.json](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-results.json)
  - [docs/research/deepsurf-round1-results.csv](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-results.csv)
  - [docs/research/deepsurf-round1-results.md](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-results.md)
- Cleaned latest-code replay snapshot is recorded in:
  - [docs/research/deepsurf-round1-latest-20260429-cleaned.json](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-cleaned.json)
  - [docs/research/deepsurf-round1-latest-20260429-cleaned.csv](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-cleaned.csv)
  - [docs/research/deepsurf-round1-latest-20260429-cleaned.md](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-cleaned.md)
- Follow-up targeted-rerun aggregate snapshot is recorded in:
  - [docs/research/deepsurf-round1-latest-20260429-targeted-rerun.json](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-targeted-rerun.json)
  - [docs/research/deepsurf-round1-latest-20260429-targeted-rerun.csv](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-targeted-rerun.csv)
  - [docs/research/deepsurf-round1-latest-20260429-targeted-rerun.md](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-targeted-rerun.md)
- Brand-new whole-batch full-rerun snapshot from the latest code state is recorded in:
  - [docs/research/deepsurf-round1-latest-20260429-fullrerun.json](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-fullrerun.json)
  - [docs/research/deepsurf-round1-latest-20260429-fullrerun.csv](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-fullrerun.csv)
  - [docs/research/deepsurf-round1-latest-20260429-fullrerun.md](/home/cas/Desktop/SERAPH/docs/research/deepsurf-round1-latest-20260429-fullrerun.md)
- Cleaned latest-code aggregate counts:
  - `validated`: `32`
  - `bug`: `1`
  - `runtime_error`: `2`
  - `attempted`: `2`
  - `crate_build_failed`: `6`
  - `no_targets`: `15`
  - `extract_failed`: `6`
- Targeted-rerun aggregate counts after replacing the two remaining `attempted` crates with latest-code reruns:
  - `validated`: `34`
  - `bug`: `1`
  - `runtime_error`: `2`
  - `crate_build_failed`: `6`
  - `no_targets`: `15`
  - `extract_failed`: `6`
- Fresh full-rerun aggregate counts from scratch on the latest code state:
  - `validated`: `35`
  - `bug`: `1`
  - `runtime_error`: `1`
  - `crate_build_failed`: `6`
  - `no_targets`: `15`
  - `extract_failed`: `6`
- The two replaced rerun workspaces are:
  - `stack-0.3.0`: `/tmp/seraph-stack-rerun-20260429-contextfix2`
  - `sized-chunks-0.6.2`: `/tmp/seraph-sized-chunks-rerun-20260429-contextfix`
- Note: this targeted-rerun snapshot is more up to date for the two former residuals, but it is still not a brand-new whole-batch replay from scratch.
- The fresh full rerun removes the last residual `attempted` states and improves over the targeted-rerun aggregate by:
  - `validated`: `34 -> 35`
  - `runtime_error`: `2 -> 1`
