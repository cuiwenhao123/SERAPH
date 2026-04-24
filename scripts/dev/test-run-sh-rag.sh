#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT="$(bash "${ROOT_DIR}/scripts/run.sh" \
  --knowledge "${ROOT_DIR}/rag/tests/fixtures/minimal_knowledge.json" \
  --workspace-dir /tmp/seraph-run-sh-test \
  --round 7 \
  --dry-run)"

printf '%s\n' "$OUTPUT" | grep -F -- "python3 -m seraph_rag.cli index"
printf '%s\n' "$OUTPUT" | grep -F -- "--vectordb /tmp/seraph-run-sh-test/vectordb"
printf '%s\n' "$OUTPUT" | grep -F -- "python3 -m seraph_rag.cli graph"
printf '%s\n' "$OUTPUT" | grep -F -- "--graph /tmp/seraph-run-sh-test/graph.pkl"
printf '%s\n' "$OUTPUT" | grep -F -- "python3 -m seraph_rag.cli retrieve"
printf '%s\n' "$OUTPUT" | grep -F -- "--round 7"
printf '%s\n' "$OUTPUT" | grep -F -- "rag_target_007.md"
