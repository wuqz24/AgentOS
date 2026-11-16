#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_root"
# Preserve the fault-injection build fingerprint before replacing the images.
if [[ -f artifacts/build/versions.txt ]] && grep -q 'agent-test' artifacts/build/versions.txt; then
    cp artifacts/build/{manifest.json,versions.txt} artifacts/verification/
fi
bash scripts/build.sh
build_root="$(cat artifacts/build/build-root.txt)"
mkdir -p artifacts/production
cp artifacts/build/{manifest.json,versions.txt} artifacts/production/
python3 scripts/qemu_test.py --root "$build_root" --output artifacts/production
python3 scripts/run_benchmarks.py --root "$build_root" --output artifacts/benchmarks
echo 'Production regression and benchmark measurements passed.'
