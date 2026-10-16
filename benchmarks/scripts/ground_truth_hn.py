"""Prepare PostgreSQL filter masks, compute exhaustive HN GT, and publish fixtures."""

# pylint: disable=import-error

import argparse
import importlib.metadata
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

import duckdb
import psycopg
import pyarrow as pa
import pyarrow.parquet as pq
from psycopg import sql

import prepare_hn as hn
from assemble_hn import checked_file
from embed_hn import Store
from plan_hn_embeddings import write_json

SUPERNOVA_REV = "695841a0fc29f8da303e8dccb80eb21cb732e280"
DEFAULT_FILTERS = hn.DEFAULT_CONFIG.parent / "filters.toml"
GT_SCHEMA = pa.schema([("query_id", pa.int32()), ("gt_ids", pa.list_(pa.string()))])
CORPUS_PREFIX = "sampled/full/parquet/hn_items/"


def inventory(root, paths):
    """Bind relative artifact paths to their exact bytes."""
    return {
        path.relative_to(root).as_posix(): {
            "sha256": hn.file_digest(path),
            "bytes": path.stat().st_size,
        }
        for path in sorted(paths)
    }


def restore_dataset(store):
    """Restore only the full corpus and its query/holdout fixtures."""
    manifest = json.loads(store.fetch("manifest.json").read_text())
    if (
        manifest["format"] != 1
        or manifest["state"] != "assembled"
        or (manifest["dataset"], manifest["version"]) != ("hn", "v1")
        or manifest["embedding"]["dimensions"] != 1536
        or manifest["embedding"]["distance"] != "cosine"
    ):
        raise ValueError("Expected the assembled HN v1 cosine dataset")
    paths = []
    for relative, entry in manifest["artifacts"].items():
        if relative.startswith(CORPUS_PREFIX) and relative.endswith(".parquet"):
            paths.append(checked_file(store, relative, entry["sha256"]))
    for relative in ("queries/hn_queries.parquet", "queries/holdout.parquet"):
        checked_file(store, relative, manifest["artifacts"][relative]["sha256"])
    if (
        not paths
        or sum(pq.ParquetFile(p).metadata.num_rows for p in paths)
        != manifest["corpora"]["full"]["rows"]
    ):
        raise ValueError("Full corpus inventory/count mismatch")
    return manifest, sorted(paths)


def validate_vectors(table):
    """Reject null, zero, nonfinite, or incorrectly sized query/corpus vectors."""
    with duckdb.connect() as conn:
        conn.register("vectors", table)
        if conn.execute("""
            SELECT count(*) FROM vectors WHERE emb IS NULL OR len(emb) != 1536
                OR NOT list_bool_and(list_transform(emb, x -> x IS NOT NULL AND isfinite(x)))
                OR NOT list_bool_or(list_transform(emb, x -> x != 0))
        """).fetchone()[0]:
            raise ValueError("Invalid embedding values/dimensions")


def load_metadata(db, paths, holdout):
    """Copy scalar metadata into an isolated temporary PostgreSQL table."""
    db.execute("""
        CREATE TEMP TABLE hn_gt_items (
            id bigint PRIMARY KEY, type text COLLATE "C", "by" text COLLATE "C",
            score integer, time timestamptz
        ) ON COMMIT DROP
    """)
    held_out = set(holdout.column("hn_id").to_pylist())
    for index, path in enumerate(paths):
        with duckdb.connect() as conn:
            conn.read_parquet(str(path)).create_view("shard")
            batches = conn.execute("""
                SELECT id, type, "by", score, time FROM shard WHERE emb IS NOT NULL
            """).fetch_record_batch(8192)
            with db.cursor().copy(
                'COPY hn_gt_items (id, type, "by", score, time) FROM STDIN'
            ) as copy:
                for batch in batches:
                    for row in zip(
                        *(column.to_pylist() for column in batch.columns), strict=True
                    ):
                        if row[0] in held_out:
                            raise ValueError("Query holdout overlaps the corpus")
                        copy.write_row(row)
        print(f"Loaded filter metadata {index + 1}/{len(paths)}", flush=True)
    db.execute("ANALYZE hn_gt_items")
    return db.execute("SELECT count(*) FROM hn_gt_items").fetchone()[0]


def calibrate_filters(db, config, total):
    """Freeze equality/range predicates and measure their actual vector selectivity."""
    records = [dict(item) for item in config["filters"]]
    for index, target in enumerate(config["time_targets"]):
        if not 0 < target < 1:
            raise ValueError("Selectivity targets must be between zero and one")
        cutoff = db.execute(
            "SELECT percentile_disc(%s) WITHIN GROUP (ORDER BY time) FROM hn_gt_items",
            (1 - target,),
        ).fetchone()[0]
        if cutoff is None:
            raise ValueError("No nonnull timestamps for time filters")
        records.append(
            {
                "name": f"knn_top10_time_{index + 1}",
                "predicate": sql.SQL("time >= {}::timestamptz")
                .format(sql.Literal(cutoff.isoformat()))
                .as_string(db),
                "target_selectivity": target,
            }
        )
    db.execute("""
        CREATE TEMP TABLE hn_gt_authors ON COMMIT DROP AS
        SELECT "by", count(*) AS n FROM hn_gt_items
        WHERE "by" IS NOT NULL GROUP BY "by"
    """)
    authors = []
    for index, target in enumerate(config["author_targets"]):
        if not 0 < target < 1:
            raise ValueError("Selectivity targets must be between zero and one")
        result = db.execute(
            """
            SELECT "by" FROM hn_gt_authors
            WHERE n >= %s AND NOT ("by" = ANY(%s::text[]))
            ORDER BY abs(n::double precision / %s - %s), "by" COLLATE "C" LIMIT 1
        """,
            (config["k"] + 1, authors, total, target),
        ).fetchone()
        if result is None:
            raise ValueError("Not enough distinct authors with k+1 eligible rows")
        authors.append(result[0])
        records.append(
            {
                "name": f"knn_top10_author_{index + 1}",
                "predicate": sql.SQL('"by" = {}')
                .format(sql.Literal(result[0]))
                .as_string(db),
                "target_selectivity": target,
            }
        )
    names = set()
    for item in records:
        if (
            not re.fullmatch(r"knn_top10_[a-z0-9_]+", item["name"])
            or item["name"] in names
        ):
            raise ValueError("Invalid or duplicate filter name")
        names.add(item["name"])
        item["matched_rows"] = db.execute(
            sql.SQL("SELECT count(*) FROM hn_gt_items WHERE ({})").format(
                sql.SQL(item["predicate"])
            )
        ).fetchone()[0]
        if item["matched_rows"] <= config["k"]:
            raise ValueError(f"Filter has fewer than k+1 vectors: {item['name']}")
        item["selectivity"] = item["matched_rows"] / total
    return records


def export_membership(db, records, path):
    """Persist PostgreSQL predicate truth values without reinterpreting them elsewhere."""
    fields = sql.SQL(", ").join(
        sql.SQL("COALESCE(({}), false) AS {}").format(
            sql.SQL(item["predicate"]), sql.Identifier(item["name"])
        )
        for item in records
    )
    schema = pa.schema(
        [("id", pa.int64())] + [(item["name"], pa.bool_()) for item in records]
    )
    with (
        db.cursor(name="hn_gt_membership") as cursor,
        pq.ParquetWriter(path, schema, compression="zstd") as writer,
    ):
        cursor.execute(
            sql.SQL("SELECT id, {} FROM hn_gt_items ORDER BY id").format(fields)
        )
        while rows := cursor.fetchmany(8192):
            writer.write_table(
                pa.Table.from_arrays(
                    [
                        pa.array(values, type=field.type)
                        for values, field in zip(
                            zip(*rows, strict=True), schema, strict=True
                        )
                    ],
                    schema=schema,
                )
            )


def stage_corpus(paths, output, records, memory_limit):
    """Join masks to vector-bearing corpus rows in bounded, independent shards."""
    destination = output / "staging/corpus"
    destination.mkdir(parents=True)
    flags = ", ".join(f'm."{item["name"]}"' for item in records)
    counts = []
    with (
        tempfile.TemporaryDirectory(dir=output) as work,
        duckdb.connect(str(Path(work) / "masks.duckdb")) as conn,
    ):
        conn.execute("SET memory_limit = ?", [memory_limit])
        conn.execute("SET threads = 1")
        conn.read_parquet(str(output / "ground_truth/membership.parquet")).create_view(
            "source_masks"
        )
        conn.execute("CREATE TABLE masks AS SELECT * FROM source_masks")
        for index, path in enumerate(paths):
            conn.read_parquet(str(path)).create_view("shard")
            count = conn.execute(
                "SELECT count(*) FROM shard WHERE emb IS NOT NULL"
            ).fetchone()[0]
            if not count:
                continue
            if conn.execute("""
                SELECT count(*) FROM shard LEFT JOIN masks USING (id)
                WHERE shard.emb IS NOT NULL AND masks.id IS NULL
            """).fetchone()[0]:
                raise ValueError("Missing PostgreSQL membership")
            if conn.execute("""
                SELECT count(*) FROM shard WHERE emb IS NOT NULL AND (
                    len(emb) != 1536
                    OR NOT list_bool_and(list_transform(emb, x -> x IS NOT NULL AND isfinite(x)))
                    OR NOT list_bool_or(list_transform(emb, x -> x != 0)))
            """).fetchone()[0]:
                raise ValueError("Invalid corpus embedding")
            target = destination / path.name
            conn.execute(
                f"""
                COPY (SELECT s.id, s.emb, {flags} FROM shard s JOIN masks m USING (id)
                      WHERE s.emb IS NOT NULL ORDER BY s.id)
                TO ? (FORMAT PARQUET, COMPRESSION ZSTD, ROW_GROUP_SIZE 8192)
            """,
                [str(target)],
            )
            if pq.ParquetFile(target).metadata.num_rows != count:
                raise ValueError("Staged corpus row count mismatch")
            counts.append(count)
            print(f"Staged GT vectors {index + 1}/{len(paths)}", flush=True)
    return sum(counts)


def prepare(dataset, output, dsn, *, filters=DEFAULT_FILTERS, memory_limit="4GB"):
    """Prepare frozen SQL workloads and full-corpus inputs without running search."""
    # pylint: disable=too-many-locals
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError("GT output directory must be empty")
    config = tomllib.loads(Path(filters).read_text(encoding="utf-8"))
    if config["k"] != 10:
        raise ValueError("pdbench HN fixtures require k=10")
    manifest, paths = restore_dataset(dataset)
    queries = pq.read_table(dataset.path("queries/hn_queries.parquet"))
    holdout = pq.read_table(dataset.path("queries/holdout.parquet"))
    expected_ids = list(range(manifest["counts"]["query_rows"]))
    if (
        queries.schema.names != ["id", "emb"]
        or queries.schema.field("id").type != pa.int32()
        or queries.column("id").to_pylist() != expected_ids
        or holdout.column("id").to_pylist() != expected_ids
        or len(set(holdout.column("hn_id").to_pylist())) != len(expected_ids)
    ):
        raise ValueError("Invalid query/holdout identities")
    validate_vectors(queries)
    metadata = output / "ground_truth"
    metadata.mkdir()
    with psycopg.connect(dsn) as db:
        db.execute("SET TIME ZONE 'UTC'")
        total = load_metadata(db, paths, holdout)
        if total != manifest["corpora"]["full"]["vector_rows"]:
            raise ValueError("Corpus vector count mismatch")
        records = calibrate_filters(db, config, total)
        export_membership(db, records, metadata / "membership.parquet")
        postgres_version = db.info.server_version
    if stage_corpus(paths, output, records, memory_limit) != total:
        raise ValueError("Staged vector count mismatch")
    (output / "staging/queries").mkdir()
    shutil.copyfile(
        dataset.path("queries/hn_queries.parquet"),
        output / "staging/queries/hn_queries.parquet",
    )
    shutil.copyfile(dataset.path("manifest.json"), metadata / "dataset-manifest.json")
    shutil.copyfile(filters, metadata / "filters.toml")
    shutil.copyfile(
        hn.DEFAULT_CONFIG.parent / "indexes/pg_search.sql", metadata / "index.sql"
    )
    write_json(metadata / "filters.json", records)
    template = (hn.DEFAULT_CONFIG.parent / "query.sql").read_text()
    (metadata / "queries").mkdir()
    for item in records:
        (metadata / "queries" / f"{item['name']}.sql").write_text(
            template.replace("{{ predicate }}", item["predicate"])
        )
    prepared = {
        "format": 1,
        "state": "prepared",
        "k": 10,
        "metric": "cosine",
        "dataset_manifest_sha256": hn.file_digest(dataset.path("manifest.json")),
        "corpus_rows": total,
        "query_rows": len(expected_ids),
        "filters": records,
        "postgres_version": postgres_version,
        "script_sha256": hn.file_digest(__file__),
        "supernova_revision": SUPERNOVA_REV,
        "artifacts": inventory(output, [p for p in output.rglob("*") if p.is_file()]),
    }
    write_json(output / "prepared.json", prepared)
    return prepared


def supernova_config(output, prepared):
    """Use a full float32 scan, numeric ID ties, and one extra boundary neighbor."""
    return {
        "corpus": {
            "path": str(output / "staging/corpus"),
            "dense_column": "emb",
            "id_column": "id",
        },
        "queries": {
            "path": str(output / "staging/queries/hn_queries.parquet"),
            "dense_column": "emb",
            "id_column": "id",
        },
        "output": {"path": str(output / "ground_truth/supernova")},
        "params": {
            "io_workers": 2,
            "dense_batch_size": 4096,
            "allow_tf32": False,
            "two_pass": "off",
            "tiebreak": "id",
        },
        "searches": [
            {
                "name": item["name"],
                "vector_type": "dense",
                "metric": "cosine",
                "k": prepared["k"] + 1,
                "filter": {"must": [{"field": item["name"], "match": True}]},
            }
            for item in prepared["filters"]
        ],
    }


def validate_run(manifest, prepared, file_count):
    """Reject partial scans and incomplete query coverage."""
    # pylint: disable=too-many-boolean-expressions
    shard, counts = manifest["sharding"], manifest["counts"]
    if (
        shard["partial_slice"]
        or shard["max_files"] is not None
        or shard["num_jobs"] not in (None, 1)
        or shard["job_rank"] not in (None, 0)
        or shard["corpus_files_total"] != file_count
        or shard["corpus_files_this_worker"] != file_count
        or counts["corpus_rows_scanned"] != prepared["corpus_rows"]
        or counts["queries_in_file"] != prepared["query_rows"]
        or counts["queries_searched"] != prepared["query_rows"]
    ):
        raise ValueError("Supernova did not complete the full corpus/query scan")


def convert_result(output, prepared, item, path):
    """Validate eligible unique hits and export pdbench's exact two-column schema."""
    # pylint: disable=too-many-locals,too-many-boolean-expressions
    rows = pq.read_table(path).to_pylist()
    count, k = prepared["query_rows"], prepared["k"]
    if len(rows) != count or {row["query_id"] for row in rows} != {
        str(i) for i in range(count)
    }:
        raise ValueError("Ground truth query IDs do not match the query fixture")
    rows.sort(key=lambda row: int(row["query_id"]))
    audit = []
    for row in rows:
        ids, scores = row["hit_ids"], row["hit_scores"]
        if (
            len(ids) != k + 1
            or len(set(ids)) != len(ids)
            or len(scores) != len(ids)
            or any(not re.fullmatch(r"[1-9][0-9]*", value) for value in ids)
            or any(not math.isfinite(value) or abs(value) > 1.00001 for value in scores)
            or scores != sorted(scores, reverse=True)
            or any(
                scores[i] == scores[i + 1] and int(ids[i]) > int(ids[i + 1])
                for i in range(k)
            )
        ):
            raise ValueError(
                "Invalid, duplicate, unordered, or incomplete ground truth hits"
            )
        gap = scores[k - 1] - scores[k]
        audit.append(
            {
                "query_id": int(row["query_id"]),
                "boundary_gap": gap,
                "boundary_tie": gap == 0,
                "near_boundary": gap <= 1e-6,
            }
        )
    wanted = pa.table(
        {
            "id": pa.array(
                sorted({int(value) for row in rows for value in row["hit_ids"]}),
                pa.int64(),
            )
        }
    )
    with duckdb.connect() as conn:
        conn.register("hits", wanted)
        conn.read_parquet(str(output / "ground_truth/membership.parquet")).create_view(
            "members"
        )
        if conn.execute(f'''
            SELECT count(*) FROM hits LEFT JOIN members USING (id)
            WHERE members.id IS NULL OR NOT members."{item["name"]}"
        ''').fetchone()[0]:
            raise ValueError("Ground truth contains IDs outside the PostgreSQL filter")
    fixture = output / f"queries/ground_truth_{item['name']}_full.parquet"
    fixture.parent.mkdir(exist_ok=True)
    pq.write_table(
        pa.Table.from_pylist(
            [
                {"query_id": int(row["query_id"]), "gt_ids": row["hit_ids"][:k]}
                for row in rows
            ],
            schema=GT_SCHEMA,
        ),
        fixture,
        compression="zstd",
    )
    pq.write_table(
        pa.Table.from_pylist(audit),
        output / f"ground_truth/{item['name']}_boundary.parquet",
        compression="zstd",
    )
    return {
        "boundary_ties": sum(row["boundary_tie"] for row in audit),
        "near_boundary": sum(row["near_boundary"] for row in audit),
    }


def compute(output, supernova):
    """Run the pinned exhaustive search implementation and certify the GT output."""
    # pylint: disable=too-many-locals,import-outside-toplevel
    output, supernova = Path(output).resolve(), Path(supernova).resolve()
    if (output / "ground_truth/manifest.json").exists():
        raise ValueError("Ground truth is already complete; publish or reuse it")
    prepared = json.loads((output / "prepared.json").read_text())
    if (
        prepared["state"] != "prepared"
        or prepared["supernova_revision"] != SUPERNOVA_REV
        or prepared["script_sha256"] != hn.file_digest(__file__)
    ):
        raise ValueError("Invalid GT preparation")
    for relative, entry in prepared["artifacts"].items():
        checked_file(Store(output), relative, entry["sha256"])
    revision = subprocess.check_output(
        ["git", "-C", str(supernova), "rev-parse", "HEAD"], text=True
    ).strip()
    if revision != SUPERNOVA_REV or subprocess.check_output(
        ["git", "-C", str(supernova), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    ):
        raise ValueError(
            "Supernova must be an unmodified checkout at the pinned revision"
        )
    destination = output / "ground_truth/supernova"
    if destination.exists():
        raise ValueError(
            "Compute output exists; remove failed compute outputs before retrying"
        )
    sys.path.insert(0, str(supernova / "python/nova-bf/src"))
    from nova_bf.compute import run_compute
    from nova_bf.config import load_config
    from nova_bf.manifest import manifest_name
    from nova_bf.results import result_name

    config_path = output / "ground_truth/supernova-config.json"
    write_json(config_path, supernova_config(output, prepared))
    config = load_config(str(config_path))
    run_compute(config, num_jobs=None, job_rank=None)
    run_path = destination / manifest_name(config, "compute")
    runtime = json.loads(run_path.read_text())
    validate_run(
        runtime, prepared, len(list((output / "staging/corpus").glob("*.parquet")))
    )
    audits = {}
    for spec, item in zip(config.searches, prepared["filters"], strict=True):
        audits[item["name"]] = convert_result(
            output, prepared, item, destination / result_name(config, spec)
        )
    shutil.copyfile(output / "prepared.json", output / "ground_truth/prepared.json")
    manifest = {
        "format": 1,
        "state": "complete",
        "dataset": "hn",
        "version": "v1",
        "size": "full",
        "dataset_manifest_sha256": prepared["dataset_manifest_sha256"],
        "corpus_rows": prepared["corpus_rows"],
        "query_rows": prepared["query_rows"],
        "k": prepared["k"],
        "metric": "cosine",
        "filters": prepared["filters"],
        "tie_policy": "float32 cosine score descending, numeric HN ID ascending",
        "boundary_audit": audits,
        "supernova_revision": revision,
        "script_sha256": hn.file_digest(__file__),
        "dependencies": {
            name: importlib.metadata.version(name)
            for name in ("torch", "numpy", "pyarrow", "duckdb", "psycopg")
        },
        "artifacts": inventory(
            output,
            [
                p
                for folder in ("queries", "ground_truth")
                for p in (output / folder).rglob("*")
                if p.is_file()
            ],
        ),
    }
    write_json(output / "ground_truth/manifest.json", manifest)
    return manifest


def publish(store):
    """Publish GT separately from the dataset, with an immutable completion marker."""
    expected = hn.load_config(hn.DEFAULT_CONFIG)["artifact_prefix"]
    if store.remote != expected:
        raise ValueError("GT publishing requires the configured HN artifact prefix")
    path = store.path("ground_truth/manifest.json")
    manifest = json.loads(path.read_text())
    if manifest["state"] != "complete":
        raise ValueError("Ground truth is not complete")
    intent = {
        "manifest_sha256": hn.file_digest(path),
        "dataset_manifest_sha256": manifest["dataset_manifest_sha256"],
    }
    for relative, entry in manifest["artifacts"].items():
        checked_file(store, relative, entry["sha256"])
    with tempfile.TemporaryDirectory(dir=store.root) as temporary:
        remote = Store(temporary, store.remote)
        dataset = checked_file(
            remote, "manifest.json", intent["dataset_manifest_sha256"]
        )
        if (
            json.loads(dataset.read_text())["source"].get("archive_verified")
            is not True
        ):
            raise ValueError("Publishing requires an archive-verified dataset")
        existing = remote.fetch("ground_truth/manifest.json", optional=True)
        if existing:
            if hn.file_digest(existing) != intent["manifest_sha256"]:
                raise ValueError("Dataset already contains different ground truth")
            return
        previous = remote.fetch("ground_truth/intent.json", optional=True)
        if previous and json.loads(previous.read_text()) != intent:
            raise ValueError("A different GT run owns this dataset prefix")
    store.save("ground_truth/intent.json", intent)
    for relative in manifest["artifacts"]:
        store.publish(relative)
    store.publish("ground_truth/manifest.json")


def main():
    """Separate expensive preparation/compute from immutable publication."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "compute", "publish"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dataset-directory", type=Path)
    parser.add_argument("--s3-prefix")
    parser.add_argument("--supernova", type=Path)
    parser.add_argument("--filters", type=Path, default=DEFAULT_FILTERS)
    parser.add_argument("--memory-limit", default="4GB")
    args = parser.parse_args()
    if args.action == "prepare":
        dsn = os.environ.get("HN_GT_DATABASE_URL")
        if not dsn or not args.dataset_directory:
            parser.error("prepare requires HN_GT_DATABASE_URL and --dataset-directory")
        dataset = Store(args.dataset_directory, args.s3_prefix)
        with dataset.lock():
            prepare(
                dataset,
                args.output,
                dsn,
                filters=args.filters,
                memory_limit=args.memory_limit,
            )
    elif args.action == "compute":
        if not args.supernova:
            parser.error("compute requires --supernova")
        with Store(args.output).lock():
            compute(args.output, args.supernova)
    else:
        store = Store(args.output, args.s3_prefix)
        with store.lock():
            publish(store)


if __name__ == "__main__":
    main()
