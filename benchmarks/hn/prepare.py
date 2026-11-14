"""Verify the published HN fixtures and prepare the restored heap checks."""

import argparse
import csv
import hashlib
import json
import math
import struct
from pathlib import Path
from urllib.request import urlopen

import duckdb

SOURCE = "https://paradedb-benchmark.s3.us-east-1.amazonaws.com/datasets/hackernews/vector/v1"
MANIFEST_SHA256 = "1394cce89acddb820d564ace048179184e6f053f6f0b8192dca82a375851e2b6"
GROUND_TRUTH_SHA256 = "f3372b78ab359abc121666b053c30e1ce0328efabcd461c6f9ae91bafdafc0dd"
ASSETS = Path(__file__).parent.parent / "assets/hn"
WORKLOADS = {
    "range_10pct": "knn_top10_time_1",
    "range_1pct": "knn_top10_time_2",
    "range_0.1pct": "knn_top10_time_3",
    "single_term_0.1pct": "knn_top10_author_1",
    "single_term_0.01pct": "knn_top10_author_2",
    "single_term_0.001pct": "knn_top10_author_3",
}


def fetch(key, checksum, output):
    """Save an artifact only after its checksum matches."""
    path = output / key
    with urlopen(f"{SOURCE}/{key}", timeout=120) as response:
        data = response.read()
    if hashlib.sha256(data).hexdigest() != checksum:
        raise ValueError(f"Checksum mismatch: {key}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    return path


def validate_ground_truth(rows, query_ids, holdout_ids):
    """Require ten distinct corpus neighbors for every held-out query."""
    if [row[0] for row in rows] != query_ids:
        raise ValueError("Ground truth must cover every query ID exactly once")
    for query_id, ids in rows:
        if (
            len(ids) != 10
            or len(set(ids)) != 10
            or any(not value.isdecimal() for value in ids)
            or {int(value) for value in ids} & holdout_ids
        ):
            raise ValueError(
                f"Invalid or held-out ground-truth IDs for query {query_id}"
            )


def prepare(output):
    """Bind the local fixtures and heap checks to the published manifests."""
    # pylint: disable=too-many-locals
    output = output.resolve()
    manifest = json.loads(fetch("manifest.json", MANIFEST_SHA256, output).read_text())
    truth = json.loads(
        fetch("ground_truth/manifest.json", GROUND_TRUTH_SHA256, output).read_text()
    )
    if truth["dataset_manifest_sha256"] != MANIFEST_SHA256:
        raise ValueError("Ground truth belongs to another dataset")
    if (
        manifest["corpora"]["28m"] != {"rows": 28736557, "vector_rows": 24138153}
        or manifest["embedding"]["dimensions"] != 1536
        or manifest["counts"]["query_rows"] != 1000
        or (truth["state"], truth["size"], truth["k"], truth["metric"])
        != ("complete", "28m", 10, "cosine")
    ):
        raise ValueError("Expected the published HN 28m cosine top-10 fixtures")
    for key in (
        "queries/hn_queries.parquet",
        "queries/holdout.parquet",
        "provenance/original-dataset-manifest.json",
    ):
        fetch(key, manifest["artifacts"][key]["sha256"], output)
    fetch(
        "ground_truth/index.sql",
        truth["artifacts"]["ground_truth/index.sql"]["sha256"],
        output,
    )

    with duckdb.connect() as db:
        vectors = db.execute(
            "SELECT id, emb FROM read_parquet(?) ORDER BY id",
            [str(output / "queries/hn_queries.parquet")],
        ).fetchall()
        holdout = db.execute(
            "SELECT id, hn_id FROM read_parquet(?) ORDER BY id",
            [str(output / "queries/holdout.parquet")],
        ).fetchall()
        query_ids = list(range(1000))
        holdout_ids = {row[1] for row in holdout}
        if (
            [row[0] for row in vectors] != query_ids
            or [row[0] for row in holdout] != query_ids
            or len(holdout_ids) != 1000
            or any(
                len(vector) != 1536 or not all(math.isfinite(x) for x in vector)
                for _, vector in vectors
            )
        ):
            raise ValueError(
                "Query vectors and holdout do not match the published identities"
            )
        with (output / "ground_truth.csv").open("w", newline="") as stream:
            writer = csv.writer(stream)
            for query, published in WORKLOADS.items():
                key = f"ground_truth/queries/{published}.sql"
                if (
                    hashlib.sha256(
                        (ASSETS / "queries" / f"{query}.sql").read_bytes()
                    ).hexdigest()
                    != truth["artifacts"][key]["sha256"]
                ):
                    raise ValueError(
                        f"Query differs from published ground truth: {query}"
                    )
                key = f"queries/ground_truth_{published}_28m.parquet"
                path = fetch(key, truth["artifacts"][key]["sha256"], output)
                rows = db.execute(
                    "SELECT query_id, gt_ids FROM read_parquet(?) ORDER BY query_id",
                    [str(path)],
                ).fetchall()
                validate_ground_truth(rows, query_ids, holdout_ids)
                writer.writerows(
                    ("hn_items", query, query_id, "{" + ",".join(ids) + "}")
                    for query_id, ids in rows
                )
                key = f"ground_truth/{published}_boundary.parquet"
                fetch(key, truth["artifacts"][key]["sha256"], output)

    schema = (
        (ASSETS / "create_table.sql")
        .read_text()
        .replace("{{ table }}", "pg_temp.hn_schema_probe")
        .replace("{{ dims }}", "1536")
    )
    ids = ",".join(str(value) for value in sorted(holdout_ids))
    (output / "verify_heap.sql").write_text(f"""{schema}
DO $$
DECLARE
    actual_rows bigint;
    actual_vectors bigint;
    expected_schema jsonb;
    actual_schema jsonb;
BEGIN
    SELECT jsonb_agg(jsonb_build_array(attname, format_type(atttypid, atttypmod)) ORDER BY attnum)
      INTO expected_schema FROM pg_attribute
      WHERE attrelid = 'pg_temp.hn_schema_probe'::regclass AND attnum > 0 AND NOT attisdropped;
    SELECT jsonb_agg(jsonb_build_array(attname, format_type(atttypid, atttypmod)) ORDER BY attnum)
      INTO actual_schema FROM pg_attribute
      WHERE attrelid = 'hn_items'::regclass AND attnum > 0 AND NOT attisdropped;
    IF actual_schema IS DISTINCT FROM expected_schema THEN
        RAISE EXCEPTION 'HN heap schema differs from the benchmark assets';
    END IF;
    SELECT count(*), count(emb) INTO actual_rows, actual_vectors FROM hn_items;
    IF actual_rows <> 28736557 OR actual_vectors <> 24138153 THEN
        RAISE EXCEPTION 'HN heap counts differ: % rows, % vectors', actual_rows, actual_vectors;
    END IF;
    IF EXISTS (SELECT 1 FROM hn_items WHERE id = ANY(ARRAY[{ids}]::bigint[])) THEN
        RAISE EXCEPTION 'Held-out HN queries overlap the corpus';
    END IF;
END
$$;
ANALYZE hn_items;
""")
    csv_path = str(output / "ground_truth.csv").replace("'", "''")
    (
        output / "import_ground_truth.sql"
    ).write_text(f"""CREATE TABLE IF NOT EXISTS pdbench_ground_truth (
    corpus text, query_stem text, query_id int, gt_ids text[],
    PRIMARY KEY (corpus, query_stem, query_id)
);
DELETE FROM pdbench_ground_truth WHERE corpus = 'hn_items';
\\copy pdbench_ground_truth FROM '{csv_path}' WITH (FORMAT csv)
""")
    print(
        "Verified HN fixtures: 1,000 held-out queries, six published top-10 workloads"
    )


def verify_vectors(output, exported):
    """Compare k6's PostgreSQL text vectors with the published float32 values."""
    with duckdb.connect() as db:
        expected = db.execute(
            "SELECT emb FROM read_parquet(?) ORDER BY id LIMIT 100",
            [str(output / "queries/hn_queries.parquet")],
        ).fetchall()
    actual = json.loads(exported.read_text())
    if len(actual) != 100 or len(expected) != 100:
        raise ValueError("Expected the first 100 published HN query vectors")
    for index, (text, (vector,)) in enumerate(zip(actual, expected)):
        values = json.loads(text)
        if len(values) != 1536 or struct.pack("<1536f", *values) != struct.pack(
            "<1536f", *vector
        ):
            raise ValueError(f"Query vector {index} differs from the published fixture")
    (output / "query-vectors.sha256").write_text(
        hashlib.sha256(exported.read_bytes()).hexdigest() + "\n"
    )
    print("Verified all 100 k6 vectors against the published float32 embeddings")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify-vectors", type=Path)
    args = parser.parse_args()
    if args.verify_vectors:
        verify_vectors(args.output, args.verify_vectors)
    else:
        prepare(args.output)
