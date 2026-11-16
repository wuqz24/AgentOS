"""Local configuration. Never serialize API credentials into browser state."""
from dataclasses import dataclass
import os
from pathlib import Path
from urllib.parse import urlsplit


@dataclass(frozen=True)
class Config:
    base_url: str = ""
    model: str = ""
    api_key: str = ""
    timeout: int = 60
    distro: str = "AgentOS-Dev"

    @property
    def configured(self):
        return bool(self.base_url and self.model)

    @classmethod
    def load(cls, root):
        values = {}
        path = Path(root) / "host/local.env"
        if path.exists():
            for line in path.read_text(encoding="utf-8-sig").splitlines():
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                key, sep, value = line.partition("=")
                if not sep or not key.startswith("AGENTOS_"):
                    raise ValueError("Invalid local.env entry; expected AGENTOS_NAME=value")
                values[key.strip()] = value.strip().strip('"').strip("'")
        values.update({k:v for k,v in os.environ.items() if k.startswith("AGENTOS_")})
        url = values.get("AGENTOS_BASE_URL", "").rstrip("/")
        if url:
            parsed = urlsplit(url)
            if parsed.scheme not in ("http", "https") or not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment:
                raise ValueError("Base URL must be an HTTP(S) API endpoint without credentials/query")
        timeout = int(values.get("AGENTOS_MODEL_TIMEOUT", "60"))
        if not 1 <= timeout <= 90:
            raise ValueError("Model timeout must be 1..90 seconds")
        return cls(url, values.get("AGENTOS_MODEL", ""), values.get("AGENTOS_API_KEY", ""),
                   timeout, values.get("AGENTOS_WSL_DISTRO", "AgentOS-Dev"))

