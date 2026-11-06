#!/usr/bin/env python3
"""Export the full restored Cohere heap to memory-mapped evaluation input."""

import argparse
import json
import struct
from pathlib import Path

import numpy as np
import psycopg
import pyarrow.parquet as pq
from psycopg import sql


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True)
    parser.add_argument("--table", default="public.cohere_wiki")
    parser.add_argument("--queries", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected-rows", type=int, default=10_000_000)
    args = parser.parse_args()
    relation = sql.Identifier(*args.table.split("."))
    args.output.mkdir(parents=True, exist_ok=False)
    with psycopg.connect(args.url) as conn:
        conn.execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        count = conn.execute(
            sql.SQL("SELECT count(*) FROM {}").format(relation)
        ).fetchone()[0]
        if count != args.expected_rows:
            raise ValueError(
                f"Expected {args.expected_rows:,} corpus rows, got {count:,}"
            )
        dim = conn.execute(
            sql.SQL("SELECT vector_dims(emb) FROM {} LIMIT 1").format(relation)
        ).fetchone()[0]
        vectors = np.lib.format.open_memmap(
            args.output / "vectors.npy", mode="w+", dtype=np.float32, shape=(count, dim)
        )
        copied = 0
        with conn.cursor().copy(
            sql.SQL("COPY (SELECT emb FROM {}) TO STDOUT (FORMAT BINARY)").format(
                relation
            )
        ) as stream:
            for (value,) in stream.rows():
                if (
                    value is None
                    or len(value) != 4 + dim * 4
                    or struct.unpack_from("!HH", value) != (dim, 0)
                ):
                    raise ValueError(f"Invalid vector at source row {copied}")
                vectors[copied] = np.frombuffer(value, dtype=">f4", offset=4)
                copied += 1
                if copied % 100_000 == 0:
                    print(f"Exported {copied:,}/{count:,} source vectors", flush=True)
        if copied != count:
            raise ValueError(f"Expected {count:,} rows, copied {copied:,}")
        vectors.flush()
    table = pq.read_table(args.queries)
    queries = np.asarray(table.column("emb").to_pylist(), dtype=np.float32)
    if queries.shape != (100, dim):
        raise ValueError(f"Expected 100 x {dim} query vectors, got {queries.shape}")
    np.save(args.output / "queries.npy", queries)
    (args.output / "source.json").write_text(
        json.dumps(
            {
                "table": args.table,
                "rows": count,
                "dimensions": dim,
                "queries": str(args.queries),
                "row_order": "heap COPY order; array offsets become stable evaluation IDs",
            },
            indent=2,
        )
        + "\n"
    )


if __name__ == "__main__":
    main()
