"""Bounded byte-stream decoder; logs and partial UART reads are expected."""
import json

PREFIX = b"@AGENTOS "
MAX_FRAME = 16384


class FrameDecoder:
    def __init__(self):
        self.buffer = bytearray()
        self.discard = False
        self.errors = 0

    def feed(self, chunk):
        events = []
        for byte in chunk:
            if byte == 10:
                if not self.discard:
                    line = bytes(self.buffer)
                    start = line.find(PREFIX)
                    if start >= 0:
                        try:
                            event = json.loads(line[start + len(PREFIX):].decode("utf-8"))
                            if not isinstance(event, dict) or event.get("v") != 1 or not isinstance(event.get("event"), str):
                                raise ValueError("invalid envelope")
                            events.append(event)
                        except (UnicodeDecodeError, ValueError):
                            self.errors += 1
                self.buffer.clear()
                self.discard = False
            elif not self.discard:
                if len(self.buffer) >= MAX_FRAME:
                    self.discard = True
                    self.errors += 1
                    self.buffer.clear()
                else:
                    self.buffer.append(byte)
        return events


def encode_command(command):
    data = json.dumps(command, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode("utf-8")
    if len(data) > 8192:
        raise ValueError("guest command exceeds 8192 bytes")
    return data + b"\n"
