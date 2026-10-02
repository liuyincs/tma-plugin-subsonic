#!/usr/bin/env python3
import json
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[1]
package_sha = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else root / "subsonic.tmap.sha256")
sha = package_sha.read_text().split()[0].strip()
if len(sha) != 64 or any(ch not in "0123456789abcdefABCDEF" for ch in sha):
    raise SystemExit(f"invalid SHA-256: {sha!r}")
path = root / "catalog.json"
catalog = json.loads(path.read_text())
catalog["plugins"][0]["sha256"] = sha
path.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
