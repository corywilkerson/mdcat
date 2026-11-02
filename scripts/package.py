#!/usr/bin/env python3
"""Build verified local source and native binary archives. Never publishes."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent


def cargo(*args):
    subprocess.run(["cargo", *args], cwd=ROOT, check=True)


def main():
    cargo("fmt", "--check")
    cargo("clippy", "--locked", "--offline", "--all-targets", "--", "-D", "warnings")
    cargo("test", "--locked", "--offline")
    subprocess.run([sys.executable, str(ROOT / "scripts" / "test-terminal.py")], cwd=ROOT, check=True)
    cargo("package", "--locked", "--offline", "--allow-dirty")
    metadata = tomllib.loads((ROOT / "Cargo.toml").read_text())
    version = metadata["package"]["version"]
    # Cargo package extracts and builds this directory during verification.
    cargo("test", "--locked", "--offline", "--manifest-path",
          str(ROOT / "target" / "package" / f"mdcat-{version}" / "Cargo.toml"))
    cargo("build", "--release", "--locked", "--offline")
    host = next(line.split(": ", 1)[1] for line in subprocess.check_output(
        ["rustc", "-vV"], text=True).splitlines() if line.startswith("host: "))
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    name = f"mdcat-{version}-{host}"
    archive = dist / f"{name}.tar.gz"
    with tempfile.TemporaryDirectory(prefix="mdcat-package-") as temp:
        bundle = Path(temp) / name
        bundle.mkdir()
        binary_name = "mdcat.exe" if os.name == "nt" else "mdcat"
        shutil.copy2(ROOT / "target" / "release" / binary_name, bundle / binary_name)
        for file in ("README.md", "CHANGELOG.md", "LICENSE"):
            shutil.copy2(ROOT / file, bundle / file)
        shutil.copytree(ROOT / "examples", bundle / "examples")
        shutil.copytree(ROOT / "tests" / "snapshots", bundle / "tests" / "snapshots")
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(bundle, arcname=name)
    source = dist / f"mdcat-{version}.crate"
    shutil.copy2(ROOT / "target" / "package" / source.name, source)
    checksums = []
    for file in (archive, source):
        digest = hashlib.sha256(file.read_bytes()).hexdigest()
        checksums.append(f"{digest}  {file.name}\n")
        print(file.relative_to(ROOT))
    (dist / "SHA256SUMS").write_text("".join(checksums))
    print("dist/SHA256SUMS")


if __name__ == "__main__":
    main()
