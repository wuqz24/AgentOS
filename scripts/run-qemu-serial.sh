#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
bash scripts/build.sh
build_root="$(cat artifacts/build/build-root.txt)"
cd "$build_root"
make -C os run
