#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Check if all schema changes from pg-schema-diff are present in the migration file.
Compares SQL statements in an order-independent way since pg-schema-diff output order
is non-deterministic. Comments are stripped for comparison.
"""

import sys
import re


def normalize_sql(content):
    """Remove comments and normalize whitespace from SQL content."""
    # Remove workflow markers that may be in the diff output
    content = re.sub(
        r"---\s*(BEGIN|END)\s+SUGGESTED\s+UPGRADE\s+SCRIPT\s*---", "", content
    )
    # Remove single-line comments (-- ...)
    content = re.sub(r"--[^\n]*\n?", "", content)
    # Remove block comments (/* ... */)
    content = re.sub(r"/\*.*?\*/", "", content, flags=re.DOTALL)
    # Remove psql commands like \echo and \quit
    content = re.sub(r"\\[a-z]+[^\n]*\n?", "", content, flags=re.IGNORECASE)
    # Normalize whitespace
    content = re.sub(r"\s+", " ", content).strip()
    return content


def extract_statements(content):
    """Extract SQL statements (CREATE FUNCTION, etc.) from content."""
    # First normalize the content
    normalized = normalize_sql(content)

    # Split by semicolons to get individual statements
    statements = set()
    for stmt in normalized.split(";"):
        stmt = stmt.strip()
        # Only include actual SQL statements (CREATE, ALTER, DROP, etc.)
        if stmt and re.match(r"^(CREATE|ALTER|DROP)\s+", stmt, re.IGNORECASE):
            statements.add(stmt)

    return statements


def main():
    """Validate that migration file contains all schema changes from diff."""
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <diff_file> <migration_file> [--debug]")
        sys.exit(2)

    diff_file = sys.argv[1]
    migration_file = sys.argv[2]
    debug = len(sys.argv) > 3 and sys.argv[3] == "--debug"

    # Read files
    with open(diff_file, "r", encoding="utf-8") as f:
        diff_content = f.read()
    with open(migration_file, "r", encoding="utf-8") as f:
        mig_content = f.read()

    diff_statements = extract_statements(diff_content)
    mig_statements = extract_statements(mig_content)

    if debug:
        print(f"DEBUG: Found {len(diff_statements)} statements in diff")
        print(f"DEBUG: Found {len(mig_statements)} statements in migration")
        print("\nDEBUG: Diff statements:")
        for i, stmt in enumerate(sorted(diff_statements)):
            print(f"  [{i}]: {stmt[:80]}...")
        print("\nDEBUG: Migration statements:")
        for i, stmt in enumerate(sorted(mig_statements)):
            print(f"  [{i}]: {stmt[:80]}...")

    # Check if all diff statements are in migration
    missing_statements = diff_statements - mig_statements
    if missing_statements:
        print("❌ Missing schema changes in migration file:")
        for stmt in missing_statements:
            # Show first 100 chars of each missing statement
            print(f"  - {stmt[:100]}...")
        if debug:
            # Find closest match
            for missing in missing_statements:
                print(f"\nDEBUG: Looking for close match for: {missing[:60]}...")
                for mig_stmt in mig_statements:
                    # Check if they start similarly
                    if missing[:50] == mig_stmt[:50]:
                        print("  Close match found!")
                        print(f"  Diff: ...{missing[50:150]}...")
                        print(f"  Mig:  ...{mig_stmt[50:150]}...")
        sys.exit(1)

    print(f"✅ All {len(diff_statements)} schema statements found in migration file")
    sys.exit(0)


if __name__ == "__main__":
    main()
