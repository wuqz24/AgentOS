"""Run inside Linux/WSL so the host can stop exactly its own QEMU process."""
import os
import sys
from pathlib import Path

if __name__ == "__main__":
    Path(sys.argv[1]).write_text(str(os.getpid()), encoding="ascii")
    os.execvp(sys.argv[2], sys.argv[2:])
