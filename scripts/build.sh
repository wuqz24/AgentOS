#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "$0")/.." && pwd)"
source "$HOME/.cargo/env"
cache_base="${XDG_CACHE_HOME:-$HOME/.cache}/agentos"
mkdir -p "$cache_base/build" "$project_root/artifacts/build"
build_root="$(realpath "$cache_base/build")"
expected="$(realpath "$cache_base")/build"
[[ "$build_root" == "$expected" && "$build_root" != "$project_root" ]] || exit 2
# The destination is this project's dedicated cache; keep compiled targets there.
rsync -a --delete --exclude .git --exclude target --exclude artifacts --exclude __pycache__ \
    --exclude local.env --exclude .venv "$project_root/" "$build_root/"
cd "$build_root"
features=""
if [[ "${1:-}" == test ]]; then features="agent-test"; fi
make -C os build FEATURES="$features" > "$project_root/artifacts/build/build.log" 2>&1 || {
    tail -100 "$project_root/artifacts/build/build.log"
    exit 1
}
target=riscv64gc-unknown-none-elf/release
cp "os/target/$target/os.bin" "$project_root/artifacts/build/kernel.bin"
cp "user/target/$target/fs.img" "$project_root/artifacts/build/fs.img"
cp bootloader/rustsbi-qemu.bin "$project_root/artifacts/build/firmware.bin"
for manifest in os user easy-fs easy-fs-fuse agent-abi; do
    if [[ -f "$manifest/Cargo.lock" ]]; then cp "$manifest/Cargo.lock" "$project_root/$manifest/Cargo.lock"; fi
done
rustc --version > "$project_root/artifacts/build/versions.txt"
printf 'Kernel features: %s\n' "${features:-production}" >> "$project_root/artifacts/build/versions.txt"
qemu-system-riscv64 --version >> "$project_root/artifacts/build/versions.txt"
printf '%s\n' "$build_root" > "$project_root/artifacts/build/build-root.txt"
python3 "$project_root/scripts/build_manifest.py" --source "$build_root" --output "$project_root/artifacts/build"
echo "AgentOS build passed."
if [[ "${1:-}" == test ]]; then
    python3 scripts/qemu_test.py --root "$build_root" --output "$project_root/artifacts/verification"
fi
