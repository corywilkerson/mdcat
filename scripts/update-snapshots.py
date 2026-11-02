#!/usr/bin/env python3
"""Regenerate plain previews; review the diff before accepting layout changes."""
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
subprocess.run(["cargo", "build", "--locked", "--offline"], cwd=ROOT, check=True)
binary = ROOT / "target" / "debug" / "mdcat"
if not binary.exists():
    binary = binary.with_suffix(".exe")
snapshots = ROOT / "tests" / "snapshots"
snapshots.mkdir(parents=True, exist_ok=True)
for name in ("showcase", "nested", "tables", "code", "edge-cases"):
    output = subprocess.check_output(
        [str(binary), "--plain", "--width", "60", str(ROOT / "examples" / f"{name}.md")],
        cwd=ROOT,
    )
    (snapshots / f"{name}.txt").write_bytes(output)
    print(f"Updated tests/snapshots/{name}.txt")
