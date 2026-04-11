#!/usr/bin/env bash
set -euo pipefail

source "$HOME/.cargo/env"

export ROCM_PATH=/opt/rocm
export HIP_PATH=/opt/rocm
export CMAKE_PREFIX_PATH=/opt/rocm
export PATH=/opt/rocm/bin:$PATH
export LD_LIBRARY_PATH=/opt/rocm/lib:${LD_LIBRARY_PATH:-}

# CMake HIP in this toolchain rejects the hipcc wrapper as compiler path.
unset CMAKE_HIP_COMPILER
unset HIPCXX

# Force PIC for HIP objects to avoid linker relocation errors with PIE binaries.
export HIPFLAGS="${HIPFLAGS:--fPIC}"
export CMAKE_HIP_FLAGS="${CMAKE_HIP_FLAGS:--fPIC}"
export CMAKE_HIP_FLAGS_RELEASE="${CMAKE_HIP_FLAGS_RELEASE:--O3 -DNDEBUG -fPIC}"

# Conservative defaults to avoid VRAM OOM on mid-range AMD cards.
export LEXICON_N_GPU_LAYERS="${LEXICON_N_GPU_LAYERS:-28}"
export LEXICON_MAIN_GPU="${LEXICON_MAIN_GPU:-0}"
export LEXICON_N_CTX="${LEXICON_N_CTX:-2048}"
export LEXICON_N_BATCH="${LEXICON_N_BATCH:-512}"
export LEXICON_N_UBATCH="${LEXICON_N_UBATCH:-128}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ "${1:-}" == "--clean-hip" ]]; then
  (cd "$PROJECT_ROOT/src-tauri" && cargo clean -p llama-cpp-sys-2)
fi

cd "$PROJECT_ROOT"
cargo tauri dev
