#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Check if all schema changes from pg-schema-diff are present in the migration file.
Compares SQL statements in an order-independent way since pg-schema-diff output order
is non-deterministic. Comments are stripped for comparison.
"""

import sys
import re


def extract_statements(content):
    """Extract normalized SQL statements (CREATE, ALTER, DROP) from content."""
    # Remove single-line comments (-- ...)
    content = re.sub(r"--[^\n]*", "", content)
    # Remove block comments (/* ... */)
    content = re.sub(r"/\*.*?\*/", "", content, flags=re.DOTALL)
    # Remove psql meta-commands (\echo, \quit, etc.) - they don't end with semicolons
    content = re.sub(r"\\[a-zA-Z]+[^\n]*", "", content)
    # Normalize whitespace
    content = re.sub(r"\s+", " ", content).strip()

    # Split by semicolons and extract SQL statements
    statements = set()
    for stmt in content.split(";"):
        stmt = stmt.strip()
        if re.match(r"^(CREATE|ALTER|DROP)\s+", stmt, re.IGNORECASE):
            statements.add(stmt)

    return statements


def main():
    """Validate that migration file contains all schema changes from diff."""
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <diff_file> <migration_file> [--debug]")
        sys.exit(2)

    debug = "--debug" in sys.argv

    with open(sys.argv[1], "r", encoding="utf-8") as f:
        diff_statements = extract_statements(f.read())
    with open(sys.argv[2], "r", encoding="utf-8") as f:
        mig_statements = extract_statements(f.read())

    if debug:
        print(
            f"DEBUG: {len(diff_statements)} in diff, {len(mig_statements)} in migration"
        )
        for i, stmt in enumerate(sorted(diff_statements)):
            print(f"  diff[{i}]: {stmt[:80]}...")
        for i, stmt in enumerate(sorted(mig_statements)):
            print(f"  mig[{i}]: {stmt[:80]}...")

    missing = diff_statements - mig_statements
    if missing:
        print("❌ Missing schema changes in migration file:")
        for stmt in missing:
            print(f"  - {stmt[:100]}...")
        sys.exit(1)

    print(f"✅ All {len(diff_statements)} schema statements found in migration file")


if __name__ == "__main__":
    main()
