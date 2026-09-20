#!/usr/bin/env python3
from __future__ import annotations

import os
import re
from pathlib import Path


home_root = b"/" + b"Users/"
home_prefix = re.compile(home_root + rb"[^/\s]+/")

for root, dirs, files in os.walk(".", followlinks=False):
    dirs[:] = [name for name in dirs if name != ".git"]
    for name in files:
        path = Path(root) / name
        if path.is_symlink():
            continue
        data = path.read_bytes()
        if b"\x00" in data[:8192] or not home_prefix.search(data):
            continue
        path.write_bytes(home_prefix.sub(b"~/", data))
