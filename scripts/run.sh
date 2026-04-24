#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_DIR="${ROOT_DIR}/workspace"
KNOWLEDGE_PATH=""
MANIFEST_PATH=""
ROUND="1"
DRY_RUN="false"
PYTHON_BIN="${PYTHON_BIN:-python3}"

usage() {
  cat <<'USAGE'
Usage: scripts/run.sh [options]

Build SERAPH Phase 2 RAG artifacts and retrieve one unsafe-target context.

Options:
  --manifest-path <path>   Target crate Cargo.toml. Runs s3-extract first.
  --knowledge <path>       Existing Phase 1 knowledge.json to consume.
  --workspace-dir <path>   Runtime workspace directory. Default: ./workspace.
  --round <n>              Synthesis round number. Default: 1.
  --dry-run                Print commands without executing them.
  -h, --help               Show this help.

Outputs:
  <workspace>/knowledge.json       when --manifest-path is used
  <workspace>/vectordb/            ChromaDB persistent vector store
  <workspace>/graph.pkl            NetworkX semantic graph pickle
  <workspace>/contexts/rag_target_RRR.md
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest-path)
      MANIFEST_PATH="${2:-}"
      shift 2
      ;;
    --knowledge)
      KNOWLEDGE_PATH="${2:-}"
      shift 2
      ;;
    --workspace-dir)
      WORKSPACE_DIR="${2:-}"
      shift 2
      ;;
    --round)
      ROUND="${2:-}"
      shift 2
      ;;
    --dry-run)
      DRY_RUN="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "${KNOWLEDGE_PATH}" && -z "${MANIFEST_PATH}" ]]; then
  echo "missing required argument: --knowledge <path> or --manifest-path <path>" >&2
  usage >&2
  exit 2
fi

if [[ -n "${KNOWLEDGE_PATH}" && -n "${MANIFEST_PATH}" ]]; then
  echo "choose only one input: --knowledge or --manifest-path" >&2
  exit 2
fi

printf -v ROUND_PADDED "%03d" "${ROUND}"
KNOWLEDGE_OUT="${WORKSPACE_DIR}/knowledge.json"
VECTORDB_PATH="${WORKSPACE_DIR}/vectordb"
GRAPH_PATH="${WORKSPACE_DIR}/graph.pkl"
CONTEXT_DIR="${WORKSPACE_DIR}/contexts"
CONTEXT_FILE="${CONTEXT_DIR}/rag_target_${ROUND_PADDED}.md"

run_cmd() {
  if [[ "${DRY_RUN}" == "true" ]]; then
    printf '%q ' "$@"
    printf '\n'
  else
    "$@"
  fi
}

if [[ "${DRY_RUN}" != "true" ]]; then
  mkdir -p "${WORKSPACE_DIR}" "${CONTEXT_DIR}"
fi

if [[ -n "${MANIFEST_PATH}" ]]; then
  KNOWLEDGE_PATH="${KNOWLEDGE_OUT}"
  run_cmd cargo run -p s3-extract -- \
    --manifest-path "${MANIFEST_PATH}" \
    --output "${KNOWLEDGE_PATH}"
fi

run_cmd "${PYTHON_BIN}" -m seraph_rag.cli index \
  --knowledge "${KNOWLEDGE_PATH}" \
  --vectordb "${VECTORDB_PATH}"

run_cmd "${PYTHON_BIN}" -m seraph_rag.cli graph \
  --knowledge "${KNOWLEDGE_PATH}" \
  --graph "${GRAPH_PATH}"

run_cmd "${PYTHON_BIN}" -m seraph_rag.cli targets \
  --graph "${GRAPH_PATH}"

run_cmd "${PYTHON_BIN}" -m seraph_rag.cli retrieve \
  --knowledge "${KNOWLEDGE_PATH}" \
  --graph "${GRAPH_PATH}" \
  --vectordb "${VECTORDB_PATH}" \
  --round "${ROUND}" \
  --output "${CONTEXT_FILE}"

if [[ "${DRY_RUN}" != "true" ]]; then
  printf 'RAG context written to %s\n' "${CONTEXT_FILE}"
fi
