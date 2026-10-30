"""Import release data from an unprivileged job without importing executable code."""

import argparse
import re
import subprocess
from pathlib import Path

RELEASE_FILES = {
    "Cargo.toml",
    "Cargo.lock",
    "nix/pg_search.nix",
    "docs/docs.json",
    "docs/operate/deploy/upgrading.mdx",
    "docs/operate/deploy/self-hosted/extension.mdx",
    "docs/operate/deploy/self-hosted/kubernetes.mdx",
    "docs/operate/deploy/cloud-platforms/digitalocean.mdx",
}


def permitted_change(path, status, version):
    """Only release metadata, generated migrations, and consumed fragments may change."""
    if path in RELEASE_FILES:
        return status == "M"
    if re.fullmatch(
        rf"pg_search/sql/pg_search--\d+\.\d+\.\d+--{re.escape(version)}\.sql",
        path,
    ):
        return status == "A"
    if re.fullmatch(r"docs/(?:project/)?changelog/\d+\.\d+\.\d+\.mdx", path):
        return status in ("A", "M")
    if re.fullmatch(
        r"(?:pg_search/sql/unreleased/[^/]+\.sql|"
        r"docs/(?:project/)?changelog/unreleased/[^/]+\.mdx)",
        path,
    ):
        return status == "D"
    return False


def apply_patch(repo, patch_path, version):
    """Validate staged Git objects before writing them into the trusted checkout."""
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-rc\.\d+)?", version):
        raise ValueError("Invalid release version")

    def git(*args):
        return subprocess.check_output(["git", *args], cwd=repo)

    if git("status", "--porcelain"):
        raise ValueError("Release checkout must be clean")
    patch_path = Path(patch_path).resolve()
    if patch_path.stat().st_size > 20 * 1024 * 1024:
        raise ValueError("Release patch exceeds 20 MiB")
    if patch_path.stat().st_size == 0:
        return
    # --cached never writes artifact-controlled paths into the working tree.
    # Git also rejects unsafe paths; no archive or artifact-supplied script runs.
    git("apply", "--cached", "--whitespace=nowarn", str(patch_path))
    entries = git("diff", "--cached", "--raw", "--no-renames", "-z").split(b"\0")
    deleted = []
    for offset in range(0, len(entries) - 1, 2):
        metadata, path = entries[offset : offset + 2]
        old_mode, new_mode, _, _, status = metadata.decode().split()
        path = path.decode("utf-8")
        if (
            old_mode.lstrip(":") not in ("000000", "100644")
            or new_mode not in ("000000", "100644")
            or not permitted_change(path, status, version)
        ):
            raise ValueError(f"Forbidden release change: {status} {path}")
        if status == "D":
            deleted.append(path)
    # Materialize only the validated tree. Removed fragments must disappear too.
    git("restore", "--worktree", ".")
    for path in deleted:
        (Path(repo) / path).unlink(missing_ok=True)


def main():
    """Apply a same-run artifact to a fresh checkout at the original release SHA."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("patch", type=Path)
    parser.add_argument("version")
    args = parser.parse_args()
    apply_patch(Path.cwd(), args.patch, args.version)


if __name__ == "__main__":
    main()
