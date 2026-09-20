#!/usr/bin/env python3
from __future__ import annotations

import os
from pathlib import Path


private_prefix = os.environ.get("ORDINCONN_PRIVATE_HOME_PREFIX", "").encode("utf-8")
if not private_prefix:
    raise SystemExit("ORDINCONN_PRIVATE_HOME_PREFIX is required")

for root, dirs, files in os.walk(".", followlinks=False):
    dirs[:] = [name for name in dirs if name != ".git"]
    for name in files:
        path = Path(root) / name
        if path.is_symlink():
            continue
        data = path.read_bytes()
        if b"\x00" in data[:8192] or private_prefix not in data:
            continue
        path.write_bytes(data.replace(private_prefix, b"~/"))
