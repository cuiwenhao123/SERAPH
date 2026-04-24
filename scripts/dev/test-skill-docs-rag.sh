#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HARNESS_DOC="${ROOT_DIR}/skills/harness-codegen/README.md"
COMPILE_DOC="${ROOT_DIR}/skills/compile-fixer/README.md"

for needle in \
  "RAG Context Input" \
  "rag_target_*.md" \
  "SERAPH_STEP_ENTER" \
  "SERAPH_STEP_OK" \
  "Do not require scenario.json" \
  "2-3 harness variants"; do
  grep -F -- "$needle" "$HARNESS_DOC"
done

grep -F -- "RAG context" "$COMPILE_DOC"
grep -F -- "must not remove the target API" "$COMPILE_DOC"

for legacy in scenario-generator scenario-api-mapper api-planner; do
  grep -F -- "Deprecated in v5.1 active path" "${ROOT_DIR}/skills/${legacy}/README.md"
done
