#!/usr/bin/env python3
# Copyright (c) 2023-2026 ParadeDB, Inc.
#
# This file is part of ParadeDB - Postgres for Search and Analytics
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <http://www.gnu.org/licenses/>.

"""Read Parquet files, generate a CREATE TABLE script, and write a combined CSV.

Usage: python3 load_parquet.py <parquet_dir> <table_name> <max_rows>

Outputs:
  <parquet_dir>/create.sql   — DROP + CREATE TABLE statement
  <parquet_dir>/combined.csv — Row-limited CSV ready for \\COPY
"""

import glob
import os
import sys

import pyarrow as pa  # pylint: disable=import-error
import pyarrow.csv as pa_csv  # pylint: disable=import-error
import pyarrow.parquet as pq  # pylint: disable=import-error

ARROW_TO_PG = {
    "int8": "SMALLINT",
    "int16": "SMALLINT",
    "int32": "INTEGER",
    "int64": "BIGINT",
    "uint8": "SMALLINT",
    "uint16": "INTEGER",
    "uint32": "BIGINT",
    "uint64": "NUMERIC",
    "float16": "REAL",
    "float32": "REAL",
    "float": "REAL",
    "float64": "DOUBLE PRECISION",
    "double": "DOUBLE PRECISION",
    "bool": "BOOLEAN",
    "string": "TEXT",
    "utf8": "TEXT",
    "large_string": "TEXT",
    "large_utf8": "TEXT",
    "binary": "BYTEA",
    "large_binary": "BYTEA",
    "date32": "DATE",
    "date64": "DATE",
    "date32[day]": "DATE",
}


def arrow_to_pg(arrow_type):
    """Map an Arrow data type to a PostgreSQL type name."""
    s = str(arrow_type)
    if s in ARROW_TO_PG:
        return ARROW_TO_PG[s]
    if "timestamp" in s:
        return "TIMESTAMP"
    if "date" in s:
        return "DATE"
    if "decimal" in s:
        return "NUMERIC"
    return "TEXT"


def main():
    """Entry point: read parquet, emit CREATE TABLE SQL and combined CSV."""
    parquet_dir = sys.argv[1]
    table_name = sys.argv[2]
    max_rows = int(sys.argv[3])

    files = sorted(glob.glob(os.path.join(parquet_dir, "*.parquet")))
    if not files:
        print(f"No parquet files found in {parquet_dir}", file=sys.stderr)
        sys.exit(1)

    # --- Generate CREATE TABLE SQL from the first file's schema ---
    schema = pq.read_schema(files[0])
    col_defs = [f'"{f.name}" {arrow_to_pg(f.type)}' for f in schema]

    sql_path = os.path.join(parquet_dir, "create.sql")
    with open(sql_path, "w", encoding="utf-8") as f:
        f.write(f"DROP TABLE IF EXISTS {table_name} CASCADE;\n")
        f.write(f"CREATE TABLE {table_name} ({', '.join(col_defs)});\n")

    # --- Read parquet files and write a single CSV, respecting max_rows ---
    chunks = []
    total = 0
    for fpath in files:
        if total >= max_rows:
            break
        table = pq.read_table(fpath)
        remaining = max_rows - total
        if len(table) > remaining:
            table = table.slice(0, remaining)
        chunks.append(table)
        total += len(table)

    combined = pa.concat_tables(chunks)
    csv_path = os.path.join(parquet_dir, "combined.csv")
    pa_csv.write_csv(combined, csv_path)

    print(f"{total} rows written for {table_name}", file=sys.stderr)


if __name__ == "__main__":
    main()
