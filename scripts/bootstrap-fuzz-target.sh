#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/bootstrap-fuzz-target.sh --workspace-dir <path> --harness <path> [options] [-- <extra afl-fuzz args>]

Options:
  --workspace-dir <path>   Phase 3 workspace containing `fuzz/` and `_cargo_projects/`
  --harness <path>         Harness source path, absolute or relative to `--workspace-dir`
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
  - This script expects the Phase 3 compile/smoke pipeline to have already created
    `_cargo_projects/<harness_stem>/Cargo.toml`.
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
harness_arg=""
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
    --harness)
      [[ $# -ge 2 ]] || fail "missing value for --harness"
      harness_arg="$2"
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
[[ -n "$harness_arg" ]] || fail "missing required flag: --harness"

mkdir -p "$workspace_dir"
workspace_dir="$(cd "$workspace_dir" && pwd)"

if [[ "$harness_arg" = /* ]]; then
  harness_path="$harness_arg"
else
  harness_path="$workspace_dir/$harness_arg"
fi

[[ -f "$harness_path" ]] || fail "harness not found: $harness_path"

harness_path="$(cd "$(dirname "$harness_path")" && pwd)/$(basename "$harness_path")"
harness_stem="$(basename "${harness_path%.rs}")"

project_dir="$workspace_dir/_cargo_projects/$harness_stem"
manifest_path="$project_dir/Cargo.toml"
project_src="$project_dir/src/main.rs"

[[ -f "$manifest_path" ]] || fail "missing Cargo project for harness: $manifest_path"

campaign_root="$workspace_dir/afl/$harness_stem"
corpus_dir="${corpus_dir:-$campaign_root/corpus}"
findings_dir="${findings_dir:-$campaign_root/findings}"
afl_target_dir="${afl_target_dir:-$workspace_dir/_afl_target}"
binary_path="$afl_target_dir/$build_profile/$harness_stem"

case "$input_mode" in
  file|stdin) ;;
  *)
    fail "invalid --input-mode '$input_mode' (expected 'file' or 'stdin')"
    ;;
esac

read -r -a cargo_afl_cmd <<< "${SERAPH_CARGO_AFL_COMMAND:-cargo afl}"
read -r -a afl_fuzz_cmd <<< "${SERAPH_AFL_FUZZ_COMMAND:-afl-fuzz}"

build_cmd=(env "CARGO_TARGET_DIR=$afl_target_dir" "${cargo_afl_cmd[@]}" build)
if [[ "$build_profile" == "release" ]]; then
  build_cmd+=(--release)
fi
build_cmd+=(--manifest-path "$manifest_path")

target_cmd=("$binary_path")
if [[ "$input_mode" == "file" ]]; then
  target_cmd+=("@@")
fi

fuzz_cmd=("${afl_fuzz_cmd[@]}")
if ((${#extra_afl_args[@]})); then
  fuzz_cmd+=("${extra_afl_args[@]}")
fi
fuzz_cmd+=(-i "$corpus_dir" -o "$findings_dir" -- "${target_cmd[@]}")

echo "workspace=$workspace_dir"
echo "harness=$harness_path"
echo "manifest=$manifest_path"
echo "binary=$binary_path"
echo "build=$(render_cmd "${build_cmd[@]}")"
if ((build_only)); then
  echo "afl_fuzz=skipped (--build-only)"
else
  echo "afl_fuzz=$(render_cmd "${fuzz_cmd[@]}")"
fi

if ((dry_run)); then
  exit 0
fi

ensure_command "${cargo_afl_cmd[0]}"
ensure_command_invocation "'${SERAPH_CARGO_AFL_COMMAND:-cargo afl} --help'" "${cargo_afl_cmd[@]}" --help
if (( ! build_only )); then
  ensure_command "${afl_fuzz_cmd[0]}"
fi

mkdir -p "$(dirname "$project_src")"
cp "$harness_path" "$project_src"

mkdir -p "$corpus_dir"
if ! find "$corpus_dir" -type f -print -quit | grep -q .; then
  head -c 64 /dev/zero > "$corpus_dir/seed.bin"
fi

if [[ -d "$findings_dir" ]] && find "$findings_dir" -mindepth 1 -print -quit | grep -q .; then
  fail "findings directory already contains data: $findings_dir"
fi

"${build_cmd[@]}"

if [[ ! -x "$binary_path" ]]; then
  fail "instrumented binary was not produced: $binary_path"
fi

if ((build_only)); then
  exit 0
fi

"${fuzz_cmd[@]}"
