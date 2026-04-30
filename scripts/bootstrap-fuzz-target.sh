#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/bootstrap-fuzz-target.sh --workspace-dir <path> --merge-report <path> [options] [-- <extra afl-fuzz args>]

Options:
  --workspace-dir <path>   Phase 3 workspace containing `fuzz/` and `_cargo_projects/`
  --merge-report <path>    Merge report JSON path, absolute or relative to `--workspace-dir`
  --input-mode <mode>      `file` (default, pass `@@`) or `stdin`
  --corpus-dir <path>      Override seed corpus directory
  --findings-dir <path>    Override AFL++ findings directory
  --afl-target-dir <path>  Override instrumented Cargo target directory
  --release                Build release binary instead of debug
  --build-only             Build the AFL++ binary but do not launch `afl-fuzz`
  --dry-run                Print the resolved commands without running them
  -h, --help               Show this help

Environment overrides:
  SERAPH_CARGO_AFL_COMMAND  Default: `cargo afl`
  SERAPH_AFL_FUZZ_COMMAND   Default: `afl-fuzz`
  AFL_*                     Passed through to the underlying AFL++ tools

Notes:
  - This script expects the Phase 3 merge step to have already created
    `_cargo_projects/merged_<crate>/Cargo.toml` plus a merge report.
  - If the corpus directory is empty, the script writes a default `seed.bin`.
EOF
}

fail() {
  echo "error: $*" >&2
  exit 2
}

render_cmd() {
  local rendered=""
  local arg
  for arg in "$@"; do
    printf -v rendered '%s%q ' "$rendered" "$arg"
  done
  printf '%s\n' "${rendered% }"
}

ensure_command() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    fail "missing required command: $cmd"
  fi
}

ensure_command_invocation() {
  local description="$1"
  shift
  if ! "$@" >/dev/null 2>&1; then
    fail "unable to execute $description"
  fi
}

workspace_dir=""
merge_report_arg=""
input_mode="file"
corpus_dir=""
findings_dir=""
afl_target_dir=""
build_profile="debug"
dry_run=0
build_only=0
extra_afl_args=()

while (($#)); do
  case "$1" in
    --workspace-dir)
      [[ $# -ge 2 ]] || fail "missing value for --workspace-dir"
      workspace_dir="$2"
      shift 2
      ;;
    --merge-report)
      [[ $# -ge 2 ]] || fail "missing value for --merge-report"
      merge_report_arg="$2"
      shift 2
      ;;
    --input-mode)
      [[ $# -ge 2 ]] || fail "missing value for --input-mode"
      input_mode="$2"
      shift 2
      ;;
    --corpus-dir)
      [[ $# -ge 2 ]] || fail "missing value for --corpus-dir"
      corpus_dir="$2"
      shift 2
      ;;
    --findings-dir)
      [[ $# -ge 2 ]] || fail "missing value for --findings-dir"
      findings_dir="$2"
      shift 2
      ;;
    --afl-target-dir)
      [[ $# -ge 2 ]] || fail "missing value for --afl-target-dir"
      afl_target_dir="$2"
      shift 2
      ;;
    --release)
      build_profile="release"
      shift
      ;;
    --build-only)
      build_only=1
      shift
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      extra_afl_args=("$@")
      break
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

[[ -n "$workspace_dir" ]] || fail "missing required flag: --workspace-dir"
[[ -n "$merge_report_arg" ]] || fail "missing required flag: --merge-report"

mkdir -p "$workspace_dir"
workspace_dir="$(cd "$workspace_dir" && pwd)"

if [[ "$merge_report_arg" = /* ]]; then
  merge_report_path="$merge_report_arg"
else
  merge_report_path="$workspace_dir/$merge_report_arg"
fi

[[ -f "$merge_report_path" ]] || fail "merge report not found: $merge_report_path"
merge_report_path="$(cd "$(dirname "$merge_report_path")" && pwd)/$(basename "$merge_report_path")"

read -r manifest_path target_name default_corpus_dir_from_report < <(
  python3 - "$merge_report_path" <<'PY'
import json
import sys
payload = json.load(open(sys.argv[1], "r", encoding="utf-8"))
print(
    payload["manifest_path"],
    payload["target_name"],
    payload.get("default_corpus_dir", ""),
)
PY
)

[[ -n "$manifest_path" ]] || fail "merge report missing manifest_path"
[[ -n "$target_name" ]] || fail "merge report missing target_name"
[[ -f "$manifest_path" ]] || fail "missing merged Cargo manifest: $manifest_path"

campaign_root="$workspace_dir/afl/$target_name"
if [[ -n "$corpus_dir" ]]; then
  :
elif [[ -n "$default_corpus_dir_from_report" ]]; then
  corpus_dir="$default_corpus_dir_from_report"
else
  corpus_dir="$campaign_root/corpus"
fi
findings_dir="${findings_dir:-$campaign_root/findings}"
afl_target_dir="${afl_target_dir:-$workspace_dir/_afl_target}"
regular_target_dir="$afl_target_dir/regular"
asan_target_dir="$afl_target_dir/asan"
cmplog_target_dir="$afl_target_dir/cmplog"
regular_binary="$regular_target_dir/$build_profile/$target_name"
asan_binary="$asan_target_dir/$build_profile/$target_name"
cmplog_binary="$cmplog_target_dir/$build_profile/$target_name"

case "$input_mode" in
  file|stdin) ;;
  *)
    fail "invalid --input-mode '$input_mode' (expected 'file' or 'stdin')"
    ;;
esac

read -r -a cargo_afl_cmd <<< "${SERAPH_CARGO_AFL_COMMAND:-cargo afl}"
read -r -a afl_fuzz_cmd <<< "${SERAPH_AFL_FUZZ_COMMAND:-afl-fuzz}"

build_regular_cmd=(env "CARGO_TARGET_DIR=$regular_target_dir" "${cargo_afl_cmd[@]}" build)
build_asan_cmd=(env "CARGO_TARGET_DIR=$asan_target_dir" AFL_USE_ASAN=1 "${cargo_afl_cmd[@]}" build)
build_cmplog_cmd=(env "CARGO_TARGET_DIR=$cmplog_target_dir" AFL_LLVM_CMPLOG=1 "${cargo_afl_cmd[@]}" build)
if [[ "$build_profile" == "release" ]]; then
  build_regular_cmd+=(--release)
  build_asan_cmd+=(--release)
  build_cmplog_cmd+=(--release)
fi
build_regular_cmd+=(--manifest-path "$manifest_path" --features seraph_afl)
build_asan_cmd+=(--manifest-path "$manifest_path" --features seraph_afl)
build_cmplog_cmd+=(--manifest-path "$manifest_path" --features seraph_afl)

regular_target_cmd=("$regular_binary")
asan_target_cmd=("$asan_binary")
if [[ "$input_mode" == "file" ]]; then
  regular_target_cmd+=("@@")
  asan_target_cmd+=("@@")
fi

asan_fuzz_cmd=("${afl_fuzz_cmd[@]}" -M asan_main)
cmplog_fuzz_cmd=("${afl_fuzz_cmd[@]}" -S cmplog_aux)
if ((${#extra_afl_args[@]})); then
  asan_fuzz_cmd+=("${extra_afl_args[@]}")
  cmplog_fuzz_cmd+=("${extra_afl_args[@]}")
fi
asan_fuzz_cmd+=(-i "$corpus_dir" -o "$findings_dir" -- "${asan_target_cmd[@]}")
cmplog_fuzz_cmd+=(-i "$corpus_dir" -o "$findings_dir" -c "$cmplog_binary" -- "${regular_target_cmd[@]}")

echo "workspace=$workspace_dir"
echo "merge_report=$merge_report_path"
echo "manifest=$manifest_path"
echo "regular_binary=$regular_binary"
echo "asan_binary=$asan_binary"
echo "cmplog_binary=$cmplog_binary"
echo "build_regular=$(render_cmd "${build_regular_cmd[@]}")"
echo "build_asan=$(render_cmd "${build_asan_cmd[@]}")"
echo "build_cmplog=$(render_cmd "${build_cmplog_cmd[@]}")"
if ((build_only)); then
  echo "asan_fuzz=skipped (--build-only)"
  echo "cmplog_fuzz=skipped (--build-only)"
else
  echo "asan_fuzz=$(render_cmd "${asan_fuzz_cmd[@]}")"
  echo "cmplog_fuzz=$(render_cmd "${cmplog_fuzz_cmd[@]}")"
fi

if ((dry_run)); then
  exit 0
fi

ensure_command "${cargo_afl_cmd[0]}"
ensure_command_invocation "'${SERAPH_CARGO_AFL_COMMAND:-cargo afl} --help'" "${cargo_afl_cmd[@]}" --help
if (( ! build_only )); then
  ensure_command "${afl_fuzz_cmd[0]}"
fi

mkdir -p "$corpus_dir"
if ! find "$corpus_dir" -type f -print -quit | grep -q .; then
  head -c 64 /dev/zero > "$corpus_dir/seed.bin"
fi

if [[ -d "$findings_dir" ]] && find "$findings_dir" -mindepth 1 -print -quit | grep -q .; then
  fail "findings directory already contains data: $findings_dir"
fi

"${build_regular_cmd[@]}"
"${build_asan_cmd[@]}"
"${build_cmplog_cmd[@]}"

if [[ ! -x "$regular_binary" ]]; then
  fail "instrumented binary was not produced: $regular_binary"
fi
if [[ ! -x "$asan_binary" ]]; then
  fail "instrumented binary was not produced: $asan_binary"
fi
if [[ ! -x "$cmplog_binary" ]]; then
  fail "instrumented binary was not produced: $cmplog_binary"
fi

if ((build_only)); then
  exit 0
fi

"${asan_fuzz_cmd[@]}" &
asan_pid=$!
trap 'kill "$asan_pid" 2>/dev/null || true' EXIT
"${cmplog_fuzz_cmd[@]}"
wait "$asan_pid"
