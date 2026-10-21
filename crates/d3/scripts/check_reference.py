#!/usr/bin/env python3
"""Check the vendored JavaScript against the manifest's hashes.

The JavaScript under `reference/` is the d3 the port was translated from.
The manifest holds the SHA-256 of every file, so that the copy in the tree
can be checked against the copy that was ported from.

    python3 scripts/check_reference.py
"""

import hashlib
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
REFERENCE = ROOT / "reference"
MANIFEST = REFERENCE / "MANIFEST.json"


def main() -> int:
    manifest = json.loads(MANIFEST.read_text())
    listed = {entry["path"]: entry for entry in manifest["files"]}

    problems = []
    for path, entry in sorted(listed.items()):
        target = REFERENCE / path
        if not target.is_file():
            problems.append(f"missing: {path}")
            continue
        digest = hashlib.sha256(target.read_bytes()).hexdigest()
        if digest != entry["sha256"]:
            problems.append(f"changed: {path}")

    # And nothing that is not listed.
    for target in sorted(REFERENCE.rglob("*")):
        if not target.is_file():
            continue
        if target.name == "MANIFEST.json":
            continue
        relative = str(target.relative_to(REFERENCE))
        if relative not in listed:
            problems.append(f"unlisted: {relative}")

    if problems:
        print("the vendored JavaScript does not match the manifest:")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(
        f"the vendored JavaScript matches the manifest: "
        f"{len(listed)} files, d3 {manifest['d3_version']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
