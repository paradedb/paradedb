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
    # Remove single-line comments (-- ...)
    content = re.sub(r"--[^\n]*\n?", "", content)
    # Remove block comments (/* ... */)
    content = re.sub(r"/\*.*?\*/", "", content, flags=re.DOTALL)
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

    diff_statements = extract_statements(diff_content)
    mig_statements = extract_statements(mig_content)

    # Check if all diff statements are in migration
    missing_statements = diff_statements - mig_statements
    if missing_statements:
        print("❌ Missing schema changes in migration file:")
        for stmt in missing_statements:
            # Show first 100 chars of each missing statement
            print(f"  - {stmt[:100]}...")
        sys.exit(1)

    print(f"✅ All {len(diff_statements)} schema statements found in migration file")
    sys.exit(0)


if __name__ == "__main__":
    main()
