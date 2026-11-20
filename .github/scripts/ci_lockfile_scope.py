"""Identify lockfile changes isolated to standalone workspace tools."""

import subprocess
import sys
import tomllib


def git(*args):
    return subprocess.check_output(["git", *args])


def lockfile(ref):
    data = tomllib.loads(git("show", f"{ref}:Cargo.lock").decode())
    tools = {}
    shared = []
    for package in data.pop("package"):
        name = package["name"]
        if name in {"stressgres", "benchmarks"} and "source" not in package:
            if name in tools:
                raise ValueError(f"Duplicate workspace package: {name}")
            tools[name] = package
        else:
            shared.append(package)
    return data, shared, tools


def standalone_only(base, head):
    base = git("merge-base", base, head).decode().strip()
    paths = git("diff", "--no-renames", "--name-only", "-z", base, head).split(b"\0")
    if b"Cargo.lock" not in paths:
        return False
    if any(
        path not in {b"", b"Cargo.lock", b"nix/pg_search.nix"}
        and not path.startswith((b"stressgres/", b"benchmarks/"))
        for path in paths
    ):
        return False

    old_header, old_shared, old_tools = lockfile(base)
    new_header, new_shared, new_tools = lockfile(head)
    return (
        old_header == new_header
        and old_shared == new_shared
        and old_tools.keys() == new_tools.keys()
    )


if __name__ == "__main__":
    try:
        isolated = standalone_only(*sys.argv[1:])
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Could not classify Cargo.lock: {error}", file=sys.stderr)
        isolated = False
    print(f"standalone_only={str(isolated).lower()}")
