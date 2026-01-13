#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Check if all schema changes from pg-schema-diff are present in the migration file.
Compares blocks in an order-independent way since pg-schema-diff output order is non-deterministic.
"""

import sys
import re


def extract_blocks(content):
    """Extract connected object blocks and other content, normalized."""
    # Split by the begin marker, keeping the marker
    parts = re.split(r"(/\* <begin connected objects> \*/)", content)

    blocks = set()
    other_content = []

    i = 0
    while i < len(parts):
        part = parts[i].strip()
        if part == "/* <begin connected objects> */":
            # Next part is the block content until </end connected objects>
            if i + 1 < len(parts):
                block_content = parts[i + 1]
                # Find the end marker
                end_idx = block_content.find("/* </end connected objects> */")
                if end_idx != -1:
                    block = block_content[:end_idx].strip()
                    # Normalize: remove comments with line numbers (they can vary)
                    block = re.sub(r"-- [^\n]+:\d+\n", "", block)
                    block = re.sub(r"\s+", " ", block).strip()
                    if block:
                        blocks.add(block)
                    # Rest goes to other_content or next iteration
                    remaining = block_content[
                        end_idx + len("/* </end connected objects> */") :
                    ].strip()
                    if remaining:
                        other_content.append(remaining)
                i += 2
            else:
                i += 1
        else:
            # Non-block content (like initial comments)
            if part and not part.startswith("/* </end connected objects>"):
                normalized = re.sub(r"\s+", " ", part).strip()
                if normalized:
                    other_content.append(normalized)
            i += 1

    return blocks, " ".join(other_content)


def main():
    """Validate that migration file contains all schema changes from diff."""
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <diff_file> <migration_file>")
        sys.exit(2)

    diff_file = sys.argv[1]
    migration_file = sys.argv[2]

    # Read files
    with open(diff_file, "r", encoding="utf-8") as f:
        diff_content = f.read()
    with open(migration_file, "r", encoding="utf-8") as f:
        mig_content = f.read()

    diff_blocks, diff_other = extract_blocks(diff_content)
    mig_blocks, mig_other = extract_blocks(mig_content)

    # Check if all diff blocks are in migration
    missing_blocks = diff_blocks - mig_blocks
    if missing_blocks:
        print("❌ Missing schema changes in migration file:")
        for block in missing_blocks:
            print(f"  - {block[:100]}...")
        sys.exit(1)

    # Check non-block content (like initial schema comments)
    diff_other_normalized = re.sub(r"\s+", " ", diff_other).strip()
    mig_other_normalized = re.sub(r"\s+", " ", mig_other).strip()
    if diff_other_normalized and diff_other_normalized not in mig_other_normalized:
        # Check if it's just the schema header which might be slightly different
        if not all(
            part.strip() in mig_content
            for part in diff_other.split("/*")
            if part.strip()
        ):
            print(f"❌ Missing non-block content: {diff_other_normalized[:100]}...")
            sys.exit(1)

    print(f"✅ All {len(diff_blocks)} schema change blocks found in migration file")
    sys.exit(0)


if __name__ == "__main__":
    main()
