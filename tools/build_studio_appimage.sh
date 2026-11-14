#!/usr/bin/env bash
# Compatibility entry point for earlier development instructions.
set -euo pipefail
exec "$(dirname "$0")/build_appimage.sh" "$@"
