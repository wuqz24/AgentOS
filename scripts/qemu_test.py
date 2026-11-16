#!/usr/bin/env python3
"""Run real guest programs, retain raw serial evidence, and fail on timeout/panic."""
import argparse
import json
import queue
import subprocess
import threading
import time
from pathlib import Path


def qemu_command(root, disk=None):
    target = "riscv64gc-unknown-none-elf/release"
    return [
        "qemu-system-riscv64", "-machine", "virt", "-m", "128M",
        "-bios", str(root / "bootloader/rustsbi-qemu.bin"),
        "-display", "none", "-monitor", "none", "-serial", "stdio",
        "-device", f"loader,file={root / 'os/target' / target / 'os.bin'},addr=0x80200000",
        "-drive", f"file={disk or root / 'user/target' / target / 'fs.img'},if=none,format=raw,id=x0",
        "-device", "virtio-blk-device,drive=x0", "-device", "virtio-gpu-device",
        "-device", "virtio-keyboard-device", "-device", "virtio-mouse-device",
        "-device", "virtio-net-device,netdev=net0", "-netdev", "user,id=net0",
    ]


class Guest:
    def __init__(self, root, log, disk=None):
        self.log = log.open("wb")
        self.proc = subprocess.Popen(qemu_command(root,disk), stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT, bufsize=0)
        self.queue = queue.Queue()
        self.text = ""
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        while data := self.proc.stdout.read(4096):
            self.log.write(data)
            self.log.flush()
            self.queue.put(data)
        self.queue.put(None)

    def wait(self, marker, timeout=90, start=0):
        deadline = time.monotonic() + timeout
        while marker not in self.text[start:]:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"waiting for {marker!r}; tail={self.text[-1200:]}")
            try:
                data = self.queue.get(timeout=min(remaining, 1))
            except queue.Empty:
                continue
            if data is None:
                raise RuntimeError(f"QEMU exited {self.proc.poll()}: {self.text[-1200:]}")
            self.text += data.decode("utf-8", errors="replace")
            if "Panicked at" in self.text[start:] or "panicked at" in self.text[start:]:
                raise RuntimeError(f"guest panic: {self.text[-1800:]}")
        return self.text[start:]

    def run(self, program, marker, timeout=90):
        start = len(self.text)
        self.proc.stdin.write((program + "\n").encode())
        self.proc.stdin.flush()
        return self.wait(marker, timeout, start)

    def close(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()
        # Reader has received EOF before the stream is closed below.
        time.sleep(0.05)
        self.log.close()


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    p.add_argument("--output", type=Path)
    p.add_argument("--baseline", action="store_true")
    p.add_argument("--program", action="append", default=[])
    a = p.parse_args()
    out = a.output or a.root / "artifacts" / "verification"
    out.mkdir(parents=True, exist_ok=True)
    guest = Guest(a.root, out / "serial.log")
    report = {"mode": "baseline" if a.baseline else "verification", "tests": [], "passed": False}
    try:
        guest.wait("Rust user shell")
        programs = [("agent_admin", "AgentOS demo passed."),
                    ("agent_probe", "unknown-tool error path passed")] if a.baseline else [
                        (name, "[PASS] " + name) for name in (a.program or ["agent_tests", "agent_loop_tests", "agent_file_tests"])]
        for program, marker in programs:
            guest.run(program, marker, timeout=180)
            report["tests"].append({"program": program, "passed": True})
            print(f"PASS {program}", flush=True)
        report["passed"] = True
    finally:
        guest.close()
        (out / "results.json").write_text(json.dumps(report, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
