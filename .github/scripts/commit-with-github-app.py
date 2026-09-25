#!/usr/bin/env python3
"""Commit the staged tree with a verified GitHub App signature, without pushing."""

import argparse
import base64
import json
import os
import subprocess


def git(*args):
    return subprocess.check_output(["git", *args])


def api(endpoint, payload):
    return json.loads(
        subprocess.check_output(
            ["gh", "api", "--method", "POST", endpoint, "--input", "-"],
            input=json.dumps(payload).encode(),
        )
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("message")
    args = parser.parse_args()
    endpoint = f"repos/{os.environ['GITHUB_REPOSITORY']}/git"
    parent = git("rev-parse", "HEAD").decode().strip()
    base_tree = git("rev-parse", "HEAD^{tree}").decode().strip()
    staged_tree = git("write-tree").decode().strip()
    if staged_tree == base_tree:
        raise SystemExit("No staged changes to commit")

    # Disable rename detection so renames become an addition and a deletion.
    paths = git("diff", "--cached", "--no-renames", "--name-only", "-z").split(b"\0")
    entries = []
    for raw_path in filter(None, paths):
        path = raw_path.decode()
        staged = git("--literal-pathspecs", "ls-files", "--stage", "-z", "--", path)
        if not staged:
            entries.append({"path": path, "mode": "100644", "type": "blob", "sha": None})
            continue
        mode, sha, stage = staged.split(b"\t", 1)[0].decode().split()
        if stage != "0":
            raise SystemExit(f"Unresolved index entry: {path}")
        kind = "commit" if mode == "160000" else "blob"
        if kind == "blob":
            content = base64.b64encode(git("cat-file", "blob", sha)).decode()
            blob = api(f"{endpoint}/blobs", {"content": content, "encoding": "base64"})
            if blob["sha"] != sha:
                raise SystemExit(f"GitHub blob differs from staged blob: {path}")
        entries.append({"path": path, "mode": mode, "type": kind, "sha": sha})

    tree = api(f"{endpoint}/trees", {"base_tree": base_tree, "tree": entries})
    if tree["sha"] != staged_tree:
        raise SystemExit("GitHub tree differs from staged tree")
    # Custom author/committer fields disable GitHub's automatic App signing.
    commit = api(
        f"{endpoint}/commits",
        {"message": args.message, "tree": staged_tree, "parents": [parent]},
    )
    if not commit.get("verification", {}).get("verified"):
        raise SystemExit("GitHub did not verify the App commit")
    sha = commit["sha"]
    git("fetch", "origin", sha)
    if git("rev-parse", f"{sha}^{{tree}}").decode().strip() != staged_tree:
        raise SystemExit("Fetched commit differs from staged tree")
    if git("rev-parse", f"{sha}^").decode().strip() != parent:
        raise SystemExit("Fetched commit has an unexpected parent")
    # Move only the local branch. The caller retains control of publishing it.
    git("reset", "--soft", sha)
    print(sha)


if __name__ == "__main__":
    main()
