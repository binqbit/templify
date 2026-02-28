#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_PATH="$SCRIPT_DIR/bin/templify"

if [ ! -x "$BIN_PATH" ]; then
  echo "Templify binary is missing. Run ./build.sh first." >&2
  exit 1
fi

cd "$SCRIPT_DIR"
exec "$BIN_PATH" "$@"
