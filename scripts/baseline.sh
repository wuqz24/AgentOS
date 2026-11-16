#!/usr/bin/env bash
set -euo pipefail
source "$HOME/.cargo/env"
project_root="$(cd "$(dirname "$0")/.." && pwd)"
baseline_root="$HOME/.cache/agentos/baseline"
mkdir -p "$baseline_root"
cd "$project_root"
python3 - "$baseline_root" <<'PY'
import sys, zipfile
with zipfile.ZipFile('artifacts/baseline/source-before-implementation.zip') as z:
    z.extractall(sys.argv[1])
PY
cd "$baseline_root"
make -C os build > "$project_root/artifacts/baseline/build.log" 2>&1 || {
    tail -80 "$project_root/artifacts/baseline/build.log"
    exit 1
}
tail -8 "$project_root/artifacts/baseline/build.log"
python3 "$project_root/scripts/qemu_test.py" --root "$baseline_root" \
    --output "$project_root/artifacts/baseline/runtime" --baseline
