#!/usr/bin/env python3
"""Generate GitHub release notes with original PR authors credited for backports."""

import argparse
from functools import cache
import json
from pathlib import Path
import re
import subprocess


def api(endpoint, payload=None):
    command = ["gh", "api", endpoint]
    if payload is not None:
        command += ["--method", "POST", "--input", "-"]
    result = subprocess.run(
        command,
        input=json.dumps(payload) if payload is not None else None,
        text=True,
        stdout=subprocess.PIPE,
        check=True,
    )
    return json.loads(result.stdout)


def credit_original_authors(body, repository, fetch_pull):
    @cache
    def original_author(number, visited=()):
        if number in visited:
            raise ValueError(f"Backport reference cycle at PR #{number}")
        pull = fetch_pull(number)
        labels = {label["name"] for label in pull["labels"]}
        if "automated-cherry-pick" in labels:
            source = re.search(
                r"^Backport of #(\d+) to `[^`]+`\.",
                pull.get("body") or "",
                re.MULTILINE,
            )
            if source is None:
                raise ValueError(f"Cannot find original PR for backport #{number}")
            return original_author(int(source[1]), (*visited, number))
        return pull["user"]["login"]

    pattern = re.compile(
        r"(?m)^(\* .* by )@\S+( in https://github\.com/"
        + re.escape(repository)
        + r"/pull/(\d+))$"
    )
    return pattern.sub(
        lambda match: f"{match[1]}@{original_author(int(match[3]))}{match[2]}",
        body,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--previous-tag")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    endpoint = f"repos/{args.repository}"
    payload = {"tag_name": args.tag}
    if args.previous_tag:
        payload["previous_tag_name"] = args.previous_tag
    notes = api(f"{endpoint}/releases/generate-notes", payload)
    body = credit_original_authors(
        notes["body"], args.repository, lambda number: api(f"{endpoint}/pulls/{number}")
    )
    args.output.write_text(body, encoding="utf-8")


if __name__ == "__main__":
    main()
