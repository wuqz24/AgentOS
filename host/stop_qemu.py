import os
import signal
import sys
from pathlib import Path

if __name__ == "__main__":
    pid = int(sys.argv[1])
    expected = sys.argv[2].encode()
    try:
        args = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
        if expected in args and any(b"qemu-system-riscv64" in arg for arg in args):
            os.kill(pid, signal.SIGTERM)
    except (FileNotFoundError, ProcessLookupError):
        pass
