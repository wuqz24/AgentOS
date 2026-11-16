#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_root"
source "$HOME/.cargo/env"
mkdir -p artifacts/native
exec > >(tee artifacts/native/checks.log) 2>&1
cargo test --manifest-path agent-abi/Cargo.toml
cargo test --manifest-path easy-fs/Cargo.toml
echo 'Native ABI and filesystem tests passed.'
