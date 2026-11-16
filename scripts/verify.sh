#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
mkdir -p artifacts/verification
exec > >(tee artifacts/verification/checks.log) 2>&1
source "$HOME/.cargo/env" 2>/dev/null || true
command -v cargo >/dev/null || { echo 'cargo is missing; run bootstrap-wsl.sh'; exit 1; }
command -v qemu-system-riscv64 >/dev/null || { echo 'qemu-system-riscv64 is missing; run bootstrap-wsl.sh'; exit 1; }
bash scripts/build.sh test
build_root="$(cat artifacts/build/build-root.txt)"
bash scripts/check_native.sh
python3 -m unittest discover -s tests -v
python3 scripts/test_persistence.py --root "$build_root" --output artifacts/persistence
python3 scripts/test_bridge.py
python3 scripts/test_service.py
echo 'AgentOS verification passed. External model acceptance requires host/local.env.'
