#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

cargo build --release
mkdir -p "$SCRIPT_DIR/bin"
cp "$SCRIPT_DIR/target/release/templify" "$SCRIPT_DIR/bin/templify"

echo "Build completed: $SCRIPT_DIR/bin/templify"
