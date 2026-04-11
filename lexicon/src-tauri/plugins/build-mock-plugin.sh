#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/dist"
TARGET="wasm32-wasip2"

mkdir -p "$OUTPUT_DIR"

mapfile -t PLUGINS < <(
  find "$SCRIPT_DIR" -mindepth 1 -maxdepth 1 -type d \
    ! -name "dist" \
    ! -name "target" \
    -printf "%f\n" \
    | sort
)

for plugin in "${PLUGINS[@]}"; do
  plugin_dir="$SCRIPT_DIR/$plugin"
  if [[ ! -f "$plugin_dir/Cargo.toml" ]]; then
    continue
  fi

  artifact_name="${plugin//-/_}"

  cargo build \
    --manifest-path "$plugin_dir/Cargo.toml" \
    --target "$TARGET" \
    --release

  cp "$plugin_dir/target/$TARGET/release/${artifact_name}.wasm" "$OUTPUT_DIR/$plugin.wasm"
  echo "Plugin generated at: $OUTPUT_DIR/$plugin.wasm"
done

echo "All plugins compiled successfully."
