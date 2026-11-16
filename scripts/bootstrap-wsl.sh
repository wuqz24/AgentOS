#!/usr/bin/env bash
set -euo pipefail
privilege=()
if [[ "$(id -u)" != 0 ]]; then privilege=(sudo); fi
"${privilege[@]}" apt-get update
"${privilege[@]}" apt-get install -y build-essential qemu-system-misc curl git rsync python3
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
fi
source "$HOME/.cargo/env"
project_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_root"
rustup toolchain install 1.99.1 --profile minimal
rustup target add riscv64gc-unknown-none-elf
rustup component add rust-src llvm-tools-preview
cargo install cargo-binutils --version 0.4.0 --locked
echo 'WSL toolchain is ready. Run: bash scripts/verify.sh'
