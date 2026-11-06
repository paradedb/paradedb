#!/usr/bin/env python3
"""Writes queries.jsonl: one {query_id, set, emb, gt_ids} per audited query.

1m: the held-out queries and their ground truth from --data, plus --self-held-out rows sampled
uniformly from the base vectors, each with its exact top-10 by brute force over the base vectors,
excluding the query's own id. Also loads the held-out ground truth into pdbench_ground_truth.

10m: the held-out queries from pdbench_queries and pdbench_ground_truth, which CI imported.

The sha256 of the held-out vectors, in id order, goes to queries.meta.json, so a local and a CI run
can confirm they share them.
"""
import argparse
import hashlib
import json
import struct
import subprocess
import sys

TOP = 10
SEED = 20261106
CHUNK = 64


def psql(url, sql, stdin=None):
    return subprocess.run(
        ["psql", url, "-X", "-q", "-At", "-v", "ON_ERROR_STOP=1", "-c", sql],
        check=True,
        input=stdin,
        capture_output=True,
        text=True,
    ).stdout


def vector_digest(rows):
    """sha256 of (query_id, emb) rows' vectors as f32s, in id order. Adding 0.0 folds -0.0 into
    0.0, which pgvector's text form does not keep."""
    digest = hashlib.sha256()
    for _, emb in sorted(rows, key=lambda row: row[0]):
        digest.update(struct.pack(f"<{len(emb)}f", *(value + 0.0 for value in emb)))
    return digest.hexdigest()


def loaded_queries(url):
    """(id, emb) of the query vectors pdbench loaded."""
    rows = []
    for line in psql(url, "SELECT id, emb::text FROM pdbench_queries ORDER BY id").splitlines():
        query_id, emb = line.split("|", 1)
        rows.append((int(query_id), json.loads(emb)))
    return rows


def from_database(url, corpus, stem):
    gt = {}
    for line in psql(
        url,
        f"SELECT query_id, to_json(gt_ids) FROM pdbench_ground_truth "
        f"WHERE corpus = '{corpus}' AND query_stem = '{stem}'",
    ).splitlines():
        query_id, ids = line.split("|", 1)
        gt[int(query_id)] = json.loads(ids)
    return [
        {"query_id": query_id, "set": "held_out", "emb": emb, "gt_ids": gt[query_id]}
        for query_id, emb in loaded_queries(url)
    ]


def from_files(url, data, corpus, stem, self_held_out):
    import numpy as np

    base = np.load(f"{data}/base.npy", mmap_mode="r")
    ids = np.load(f"{data}/ids.npy")
    queries = np.load(f"{data}/queries.npy")
    query_ids = np.load(f"{data}/query_ids.npy")
    gt_rows = np.load(f"{data}/gt_rows.npy")
    rows = [
        {
            "query_id": int(query_id),
            "set": "held_out",
            "emb": [float(x) for x in query],
            "gt_ids": [str(ids[row]) for row in gt],
        }
        for query_id, query, gt in zip(query_ids, queries, gt_rows)
    ]
    psql(
        url,
        "CREATE TABLE IF NOT EXISTS pdbench_ground_truth (corpus text, query_stem text, "
        "query_id int, gt_ids text[], PRIMARY KEY (corpus, query_stem, query_id))",
    )
    psql(url, f"DELETE FROM pdbench_ground_truth WHERE corpus = '{corpus}' AND query_stem = '{stem}'")
    values = ",".join(
        "('{}','{}',{},ARRAY[{}])".format(
            corpus,
            stem,
            row["query_id"],
            ",".join("'" + doc.replace("'", "''") + "'" for doc in row["gt_ids"]),
        )
        for row in rows
    )
    psql(url, f"INSERT INTO pdbench_ground_truth VALUES {values}")

    picks = np.random.default_rng(SEED).choice(len(base), size=self_held_out, replace=False)
    norms = np.linalg.norm(base, axis=1, keepdims=True).astype(np.float32)
    unit = np.asarray(base, dtype=np.float32) / norms
    for start in range(0, len(picks), CHUNK):
        chunk = picks[start : start + CHUNK]
        scores = unit @ unit[chunk].T
        scores[chunk, np.arange(len(chunk))] = -np.inf
        top = np.argpartition(-scores, TOP, axis=0)[:TOP]
        order = np.argsort(-np.take_along_axis(scores, top, axis=0), axis=0)
        top = np.take_along_axis(top, order, axis=0)
        for column, row in enumerate(chunk):
            rows.append(
                {
                    "query_id": int(row),
                    "set": "self_held_out",
                    "emb": [float(x) for x in base[row]],
                    "gt_ids": [str(ids[doc]) for doc in top[:, column]],
                }
            )
        print(f"self-held-out {start + len(chunk)}/{len(picks)}", file=sys.stderr, flush=True)
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--size", required=True, choices=["1m", "10m"])
    parser.add_argument("--url", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--data")
    parser.add_argument("--corpus", default="cohere_wiki")
    parser.add_argument("--query-stem", default="knn_top10_unfiltered")
    parser.add_argument("--self-held-out", type=int, default=900)
    parser.add_argument(
        "--check-loaded",
        action="store_true",
        help="only check pdbench_queries holds the audited held-out vectors",
    )
    args = parser.parse_args()

    if args.check_loaded:
        with open(f"{args.out}/queries.meta.json") as handle:
            expected = json.load(handle)["held_out_sha256"]
        actual = vector_digest(loaded_queries(args.url))
        if actual != expected:
            raise SystemExit(f"pdbench loaded other queries: {actual} != {expected}")
        return
    if args.size == "1m":
        rows = from_files(args.url, args.data, args.corpus, args.query_stem, args.self_held_out)
    else:
        rows = from_database(args.url, args.corpus, args.query_stem)
    with open(f"{args.out}/queries.jsonl", "w") as out:
        for row in rows:
            out.write(json.dumps(row) + "\n")
    held_out = [(row["query_id"], row["emb"]) for row in rows if row["set"] == "held_out"]
    with open(f"{args.out}/queries.meta.json", "w") as out:
        json.dump(
            {
                "held_out": len(held_out),
                "self_held_out": len(rows) - len(held_out),
                "held_out_sha256": vector_digest(held_out),
            },
            out,
            indent=2,
        )


if __name__ == "__main__":
    main()
