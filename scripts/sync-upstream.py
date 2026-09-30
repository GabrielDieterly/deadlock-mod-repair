#!/usr/bin/env python3
"""Refresh the recorded engine snapshot from a clean local manager checkout."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys


def main():
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python scripts/sync-upstream.py /path/to/manager")
    source = Path(sys.argv[1]).resolve()
    root = Path(__file__).resolve().parents[1]
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=source):
        raise SystemExit("Commit the upstream changes first so the snapshot has a reproducible revision.")
    manifest_path = root / "UPSTREAM.json"
    manifest = json.loads(manifest_path.read_text())
    for entry in manifest["files"]:
        data = (source / entry["source"]).read_bytes()
        (root / entry["destination"]).write_bytes(data)
        entry["sha256"] = hashlib.sha256(data).hexdigest()
    manifest["revision"] = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=source, text=True
    ).strip()
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Updated {len(manifest['files'])} files from {manifest['revision']}.")


if __name__ == "__main__":
    main()
