import json
import os
import shutil
import subprocess
import threading
import time
import uuid
from collections import deque
from pathlib import Path

from .protocol import FrameDecoder, encode_command


def linux_path(path):
    text = str(Path(path).resolve()).replace("\\", "/")
    if os.name == "nt":
        if len(text) < 3 or text[1] != ":":
            raise ValueError("Use a local drive path for WSL")
        return "/mnt/" + text[0].lower() + text[2:]
    return text


class QemuSession:
    def __init__(self, root, output, callback=None, distro="AgentOS-Dev", disk=None):
        self.root, self.output = Path(root).resolve(), Path(output).resolve()
        self.output.mkdir(parents=True, exist_ok=True)
        self.name = "agentos-" + uuid.uuid4().hex
        self.run_id = self.name
        self.distro = distro
        self.disk = Path(disk).resolve() if disk else self.output / "fs.img"
        self.callback = callback
        self.proc = None
        self.ready = threading.Event()
        self.ended = threading.Event()
        self.condition = threading.Condition()
        self.write_lock = threading.Lock()
        self.events = deque(maxlen=2048)
        self.results = {}
        self.pending = set()
        self.event_seq = 0
        self.command_id = 0
        self.decoder = FrameDecoder()
        self.raw = None
        self.records = None

    def _write(self, data):
        # Bound bursts across Windows/WSL pipes and the emulated UART receiver.
        for offset in range(0, len(data), 128):
            view = memoryview(data[offset:offset+128])
            while view:
                count = self.proc.stdin.write(view)
                if not count:
                    raise BrokenPipeError("Guest serial input closed")
                view = view[count:]
            self.proc.stdin.flush()
            if offset + 128 < len(data):
                time.sleep(0.002)

    def start(self, timeout=20):
        build = self.root / "artifacts/build"
        for file in ("kernel.bin", "fs.img", "firmware.bin"):
            if not (build / file).exists():
                raise FileNotFoundError(f"Missing {file}; run scripts/build.sh first")
        if not self.disk.exists():
            self.disk.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(build / "fs.img", self.disk)
        args = ["qemu-system-riscv64", "-name", self.name, "-machine", "virt", "-m", "128M",
                "-bios", linux_path(build / "firmware.bin"), "-display", "none",
                "-monitor", "none", "-serial", "stdio",
                "-device", f"loader,file={linux_path(build / 'kernel.bin')},addr=0x80200000",
                "-drive", f"file={linux_path(self.disk)},if=none,format=raw,id=x0",
                "-device", "virtio-blk-device,drive=x0", "-device", "virtio-gpu-device",
                "-device", "virtio-keyboard-device", "-device", "virtio-mouse-device",
                "-device", "virtio-net-device,netdev=net0", "-netdev", "user,id=net0"]
        command = ["python3", linux_path(self.root / "host/launch_qemu.py"),
                   linux_path(self.output / "qemu.pid"), *args]
        kwargs = {}
        if os.name == "nt":
            command = ["wsl.exe", "-d", self.distro, "--exec", *command]
            kwargs["creationflags"] = subprocess.CREATE_NO_WINDOW
        self.raw = (self.output / "serial.log").open("wb")
        self.records = (self.output / "events.jsonl").open("w", encoding="utf-8")
        self.proc = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.STDOUT, bufsize=0, **kwargs)
        self.reader = threading.Thread(target=self._read, daemon=True, name="agentos-serial")
        self.reader.start()
        if not self.ready.wait(timeout):
            self.stop()
            raise TimeoutError(f"Guest bridge did not become ready; see {self.output / 'serial.log'}")
        return self

    def _publish(self, event):
        with self.condition:
            self.event_seq += 1
            event = {**event, "seq":self.event_seq, "run_id":self.run_id, "host_time":time.time()}
            self.events.append(event)
            if event["event"] == "command_result" and event.get("id") in self.pending:
                self.results[event["id"]] = event
            self.records.write(json.dumps(event, ensure_ascii=False) + "\n")
            self.records.flush()
            self.condition.notify_all()
        if self.callback:
            try:
                self.callback(event)
            except Exception as error:
                # Keep reading the guest if a consumer fails. Do not lose UART data.
                self.records.write(json.dumps({"event":"consumer_error","error":type(error).__name__}) + "\n")
        if event["event"] == "bridge_ready":
            self.ready.set()

    def _read(self):
        tail = b""
        shell_started = False
        try:
            # Windows pipe reads of N bytes can wait for a full buffer. A serial
            # prompt may be shorter and has to be handled before more arrives.
            while data := self.proc.stdout.read(1 if os.name == "nt" else 4096):
                self.raw.write(data)
                if b"\n" in data:
                    self.raw.flush()
                tail = (tail + data)[-4096:]
                if not shell_started and b"Rust user shell" in tail:
                    shell_started = True
                    with self.write_lock:
                        self._write(b"agent_bridge\n")
                for event in self.decoder.feed(data):
                    self._publish(event)
        finally:
            self.ended.set()
            self._publish({"v":1,"event":"qemu_disconnected"})
            with self.condition:
                self.condition.notify_all()

    def command(self, command, timeout=10, **fields):
        with self.write_lock:
            if not self.proc or self.ended.is_set():
                raise RuntimeError("QEMU is disconnected")
            self.command_id += 1
            number = self.command_id
            data = encode_command({"v":1,"id":number,"command":command,**fields})
            with self.condition:
                self.pending.add(number)
            try:
                self._write(data)
            except OSError:
                with self.condition:
                    self.pending.discard(number)
                raise
        deadline = time.monotonic() + timeout
        try:
            with self.condition:
                while number not in self.results:
                    left = deadline - time.monotonic()
                    if left <= 0 or self.ended.is_set():
                        raise TimeoutError(f"Guest did not acknowledge command {number}")
                    self.condition.wait(left)
                return self.results.pop(number)
        finally:
            with self.condition:
                self.pending.discard(number)
                self.results.pop(number, None)

    def wait_event(self, name, after=0, timeout=10, predicate=None):
        deadline = time.monotonic() + timeout
        with self.condition:
            while True:
                for event in self.events:
                    if event["seq"] > after and event["event"] == name and (predicate is None or predicate(event)):
                        return event
                left = deadline - time.monotonic()
                if left <= 0 or self.ended.is_set():
                    raise TimeoutError(f"Waiting for guest event {name}")
                self.condition.wait(left)

    def stop(self):
        if self.proc and self.proc.poll() is None:
            if os.name == "nt":
                pid_file = self.output / "qemu.pid"
                if pid_file.exists():
                    pid = str(int(pid_file.read_text(encoding="ascii")))
                    try:
                        subprocess.run(["wsl.exe","-d",self.distro,"--exec","python3",
                            linux_path(self.root / "host/stop_qemu.py"),pid,self.name],
                            timeout=10, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                            creationflags=subprocess.CREATE_NO_WINDOW, check=False)
                    except subprocess.TimeoutExpired:
                        pass
            else:
                self.proc.terminate()
            try:
                self.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
        if hasattr(self, "reader"):
            self.reader.join(timeout=3)
        for handle in (self.raw, self.records):
            if handle and not handle.closed:
                handle.close()
