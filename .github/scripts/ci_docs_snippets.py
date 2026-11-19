"""Skip the snippet smoke test for docs edits with identical executable input."""

import io
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

from extract_code_snippets import inventory


def git(*args):
    return subprocess.check_output(["git", *args])


def unchanged(base, head):
    base = git("merge-base", base, head).decode().strip()
    paths = git("diff", "--no-renames", "--name-only", "-z", base, head).split(b"\0")
    paths = [path for path in paths if path]
    if not paths or any(not path.startswith(b"docs/") for path in paths):
        return False
    # The docs version is also an input to snippet classification and extraction.
    if b"docs/docs.json" in paths:
        return False
    with tempfile.TemporaryDirectory() as temp:
        for name, ref in (("base", base), ("head", head)):
            with tarfile.open(fileobj=io.BytesIO(git("archive", ref, "docs"))) as archive:
                archive.extractall(Path(temp) / name, filter="data")
        return inventory(Path(temp) / "base/docs") == inventory(Path(temp) / "head/docs")


if __name__ == "__main__":
    try:
        if unchanged(*sys.argv[1:]):
            print("snippets=unchanged")
    except (OSError, ValueError, tarfile.TarError, subprocess.CalledProcessError) as error:
        print(f"Could not compare docs snippets: {error}", file=sys.stderr)
