# /// script
# requires-python = ">=3.12,<3.13"
# dependencies = ["duckdb==1.4.3", "pyarrow==21.0.0", "tiktoken==0.12.0", "openai==3.31.0"]
# ///
"""Assemble verified HN corpus/query Parquet files and publish them immutably."""

# pylint: disable=import-error

import argparse
import csv
import json
import re
import shutil
import tempfile
import tomllib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import duckdb
import pyarrow as pa
import pyarrow.parquet as pq

import prepare_hn as hn
from embed_hn import Store
from plan_hn_embeddings import validate_profile, write_json

COLUMNS = {
    "id": "BIGINT",
    "deleted": "BOOLEAN",
    "type": "VARCHAR",
    "by": "VARCHAR",
    "time": "TIMESTAMPTZ",
    "text": "VARCHAR",
    "dead": "BOOLEAN",
    "parent": "BIGINT",
    "poll": "BIGINT",
    "kids": "BIGINT[]",
    "url": "VARCHAR",
    "score": "INTEGER",
    "title": "VARCHAR",
    "parts": "BIGINT[]",
    "descendants": "INTEGER",
}


def checked_file(store, relative, expected):
    """Restore an artifact only if its bytes match its manifest."""
    path = store.fetch(relative)
    if hn.file_digest(path) != expected:
        raise ValueError(f"Checksum mismatch: {relative}")
    return path


def load_cache(store):
    """Bind the completed cache to its exact plan, profile, and query holdout."""
    embedded = json.loads(store.fetch("embeddings.json").read_text())
    plan_path = checked_file(store, "plan.json", embedded["plan_sha256"])
    plan = json.loads(plan_path.read_text())
    profile_path = checked_file(store, "profile.json", plan["profile_sha256"])
    store.fetch("queries/holdout.parquet")
    profile = validate_profile(profile_path)
    if (
        embedded["state"] != "embedded"
        or plan["format"] != 1
        or embedded["profile_sha256"] != plan["profile_sha256"]
        or embedded["embedding"] != plan["embedding"]
        or embedded["rows"] != profile["counts"]["unique_inputs"]
    ):
        raise ValueError("Embedding cache does not match its plan/profile")
    if (
        plan["embedding"] != profile["config"]["embedding"]
        or plan["source_sha256"] != profile["source"]["csv_sha256"]
        or any(value != profile["counts"][key] for key, value in plan["counts"].items())
    ):
        raise ValueError("Embedding cache does not match its plan/profile")
    paths = [part["path"] for part in embedded["parts"]]
    if (
        not paths
        or len(set(paths)) != len(paths)
        or any(
            not re.fullmatch(r"vectors/[A-Za-z0-9_-]+\.parquet", path) for path in paths
        )
        or sum(part["rows"] for part in embedded["parts"]) != embedded["rows"]
    ):
        raise ValueError("Invalid embedding shard inventory")
    holdout = pq.read_table(store.path("queries/holdout.parquet"))
    records = holdout.to_pylist()
    count = profile["counts"]["query_rows"]
    if (
        not holdout.schema.equals(hn.HOLDOUT_SCHEMA)
        or [row["id"] for row in records] != list(range(count))
        or len({row["hn_id"] for row in records}) != count
        or len({row["input_sha256"] for row in records}) != count
        or any(
            hn.input_digest(row["token_ids"]).hex() != row["input_sha256"]
            for row in records
        )
    ):
        raise ValueError("Invalid query holdout")
    return profile, embedded


def sample_sizes(labels, corpus_rows):
    """Resolve exact sample sizes, with full retaining every non-query row."""
    sizes = {}
    for label in labels:
        if label == "full":
            size = corpus_rows
        elif re.fullmatch(r"[1-9][0-9]*[km]?", label):
            multiplier = {"k": 1000, "m": 1000000}.get(label[-1], 1)
            size = int(label[:-1] if multiplier != 1 else label) * multiplier
        else:
            raise ValueError(f"Invalid sample size: {label}")
        if size > corpus_rows or label in sizes:
            raise ValueError(f"Duplicate or oversized sample: {label}")
        sizes[label] = size
    if not sizes:
        raise ValueError("At least one corpus size is required")
    return sizes


def partition_source(conn, source, work, profile, workers):
    """Verify source identity and partition original columns with canonical input hashes."""
    before = source.stat()
    if before.st_size != profile["source"]["csv_bytes"]:
        raise ValueError("Source CSV size does not match the profile")
    with source.open(newline="", encoding="utf-8") as stream:
        if next(csv.reader(stream)) != list(COLUMNS):
            raise ValueError("Source CSV columns do not match the HN table schema")
    hashes = work / "hashes"
    hashes.mkdir()
    with ThreadPoolExecutor(max_workers=1) as hashing:
        checksum = hashing.submit(hn.file_digest, source)
        hn.profile_csv(
            source, hashes, profile["config"]["embedding"]["max_tokens"], workers
        )
        if checksum.result() != profile["source"]["csv_sha256"]:
            raise ValueError("Source CSV checksum does not match the profile")
    conn.read_parquet(str(hashes / "*.parquet")).create_view("hashes")
    measured = conn.execute("""
        SELECT count(*), count(DISTINCT hn_id), count(input_sha256),
               count(DISTINCT input_sha256) FROM hashes
    """).fetchone()
    counts = profile["counts"]
    if measured != (
        counts["source_rows"],
        counts["source_rows"],
        counts["source_rows"] - counts["empty_rows"],
        counts["unique_inputs"],
    ):
        raise ValueError("Source identities/input counts do not match the profile")
    if conn.execute("""
        SELECT count(*) FROM holdout q LEFT JOIN hashes h ON q.hn_id = h.hn_id
        WHERE h.hn_id IS NULL OR q.input_sha256 IS DISTINCT FROM lower(hex(h.input_sha256))
           OR q.normalized_sha256 IS DISTINCT FROM lower(hex(h.normalized_sha256))
    """).fetchone()[0]:
        raise ValueError("Held-out inputs do not match the source")
    seed = tomllib.loads((hn.DEFAULT_CONFIG.parent / "config.toml").read_text())[
        "sampling_seed"
    ]
    conn.execute(
        """
        CREATE TABLE ranks AS
        SELECT hn_id, row_number() OVER (
            ORDER BY sha256(? || ':' || hn_id::VARCHAR), hn_id
        ) AS sample_rank
        FROM hashes ANTI JOIN holdout USING (hn_id)
    """,
        [str(seed)],
    )
    conn.read_csv(
        str(source),
        header=True,
        columns=dict.fromkeys(COLUMNS, "VARCHAR"),
        na_values="\\N",
        auto_detect=False,
        quotechar='"',
        escapechar='"',
    ).create_view("raw_source")
    columns = ", ".join(
        f'r."{name}"'
        if kind == "VARCHAR"
        else f'nullif(r."{name}", \'\')::{kind} AS "{name}"'
        for name, kind in COLUMNS.items()
    )
    conn.execute(
        f"""
        COPY (
            SELECT {columns}, lower(hex(h.input_sha256)) AS input_sha256,
                   q.id AS query_id, ranks.sample_rank,
                   coalesce(substr(lower(hex(h.input_sha256)), 1, 2), 'empty') AS bucket
            FROM raw_source r JOIN hashes h ON r.id::BIGINT = h.hn_id
            LEFT JOIN holdout q ON h.hn_id = q.hn_id
            LEFT JOIN ranks ON h.hn_id = ranks.hn_id
        ) TO ? (FORMAT PARQUET, PARTITION_BY (bucket), COMPRESSION ZSTD, ROW_GROUP_SIZE 8192)
    """,
        [str(work / "source")],
    )
    after = source.stat()
    if (before.st_size, before.st_mtime_ns, before.st_ino) != (
        after.st_size,
        after.st_mtime_ns,
        after.st_ino,
    ):
        raise ValueError("Source CSV changed during assembly")
    conn.execute("DROP TABLE ranks")
    conn.execute("DROP VIEW hashes")
    conn.execute("DROP VIEW raw_source")
    shutil.rmtree(hashes)
    return seed


def partition_vectors(conn, store, embedded, work):
    """Validate cache shards, then partition vectors to keep individual joins small."""
    paths = []
    downloaded = []
    schema = pa.schema(
        [
            ("input_sha256", pa.string()),
            ("emb", pa.list_(pa.float32(), embedded["embedding"]["dimensions"])),
        ]
    )
    for part in embedded["parts"]:
        if store.remote and not store.path(part["path"]).exists():
            downloaded.append(store.path(part["path"]))
        path = checked_file(store, part["path"], part["sha256"])
        parquet = pq.ParquetFile(path)
        if (
            not parquet.schema_arrow.equals(schema)
            or parquet.metadata.num_rows != part["rows"]
        ):
            raise ValueError(f"Invalid cache shard schema/count: {part['path']}")
        conn.read_parquet(str(path)).create_view("shard")
        if conn.execute("""
            SELECT count(*) FROM shard WHERE input_sha256 IS NULL
                OR NOT regexp_full_match(input_sha256, '[0-9a-f]{64}')
                OR emb IS NULL
                OR NOT list_bool_and(list_transform(emb, x -> x IS NOT NULL AND isfinite(x)))
                OR NOT list_bool_or(list_transform(emb, x -> x != 0))
        """).fetchone()[0]:
            raise ValueError(f"Invalid embedding values: {part['path']}")
        paths.append(str(path))
        print(
            f"Verified vector shard {len(paths)}/{len(embedded['parts'])}", flush=True
        )
    conn.read_parquet(paths).create_view("vectors")
    destination = work / "vectors"
    destination.mkdir()
    for group, prefixes in enumerate(("0123", "4567", "89ab", "cdef")):
        staging = work / f"vector-group-{group}"
        conn.execute(
            """
            COPY (
                SELECT *, substr(input_sha256, 1, 2) AS bucket FROM vectors
                WHERE substr(input_sha256, 1, 1) = ANY($prefixes)
            ) TO $path (
                FORMAT PARQUET, PARTITION_BY (bucket), COMPRESSION ZSTD,
                ROW_GROUP_SIZE 2048
            )
        """,
            {"prefixes": list(prefixes), "path": str(staging)},
        )
        for partition in staging.iterdir():
            partition.rename(destination / partition.name)
        staging.rmdir()
        print(f"Partitioned vector group {group + 1}/4", flush=True)
    conn.execute("DROP VIEW vectors")
    conn.execute("DROP VIEW shard")
    for path in downloaded:
        path.unlink()


def write_bucket(conn, work, output, sizes, bucket):
    """Join one hash partition, rejecting duplicate, missing, and extraneous vectors."""
    source = sorted((work / "source" / bucket).glob("*.parquet"))
    vectors = sorted((work / "vectors" / bucket).glob("*.parquet"))
    if not source:
        raise ValueError("Embedding cache contains inputs absent from the source")
    conn.read_parquet(
        [str(path) for path in source], hive_partitioning=False
    ).create_view("items")
    if vectors:
        conn.read_parquet(
            [str(path) for path in vectors], hive_partitioning=False
        ).create_view("vectors")
        if conn.execute("""
            SELECT count(*) != count(DISTINCT input_sha256) FROM vectors
        """).fetchone()[0]:
            raise ValueError("Duplicate embedding input hashes")
        if conn.execute("""
            SELECT count(*) FROM vectors ANTI JOIN items USING (input_sha256)
        """).fetchone()[0]:
            raise ValueError("Embedding cache contains inputs absent from the source")
        conn.execute("""
            CREATE TEMP TABLE joined AS SELECT items.*, emb::FLOAT[] AS emb
            FROM items LEFT JOIN vectors USING (input_sha256)
        """)
    else:
        conn.execute(
            "CREATE TEMP TABLE joined AS SELECT *, NULL::FLOAT[] AS emb FROM items"
        )
    if conn.execute("""
        SELECT count(*) FROM joined WHERE (input_sha256 IS NULL) != (emb IS NULL)
    """).fetchone()[0]:
        raise ValueError("Missing embeddings for nonempty source inputs")
    queries = conn.execute(
        "SELECT query_id::INTEGER AS id, emb FROM joined WHERE query_id IS NOT NULL"
    ).fetch_arrow_table()
    stats = {}
    columns = ", ".join(f'"{name}"' for name in COLUMNS)
    for label, size in sizes.items():
        count, vector_count = conn.execute(
            """
            SELECT count(*), count(emb) FROM joined WHERE sample_rank <= ?
        """,
            [size],
        ).fetchone()
        stats[label] = {"rows": count, "vector_rows": vector_count}
        if count:
            path = (
                output
                / f"sampled/{label}/parquet/hn/part-{bucket.removeprefix('bucket=')}.parquet"
            )
            path.parent.mkdir(parents=True, exist_ok=True)
            conn.execute(
                f"""
                COPY (SELECT {columns}, emb FROM joined WHERE sample_rank <= $size ORDER BY id)
                TO $path (FORMAT PARQUET, COMPRESSION ZSTD, ROW_GROUP_SIZE 8192)
            """,
                {"size": size, "path": str(path)},
            )
    conn.execute("DROP TABLE joined")
    return queries, stats


def assemble_partitions(conn, work, output, sizes):
    """Consume hash partitions and accumulate exact corpus/query counts."""
    counts = {label: {"rows": 0, "vector_rows": 0} for label in sizes}
    queries = []
    buckets = sorted(
        {
            path.name
            for parent in ("source", "vectors")
            for path in (work / parent).iterdir()
        }
    )
    for index, bucket in enumerate(buckets):
        batch, stats = write_bucket(conn, work, output, sizes, bucket)
        queries.append(batch)
        for label in sizes:
            counts[label]["rows"] += stats[label]["rows"]
            counts[label]["vector_rows"] += stats[label]["vector_rows"]
        for parent in ("source", "vectors"):
            path = work / parent / bucket
            if path.exists():
                shutil.rmtree(path)
        print(f"Assembled partition {index + 1}/{len(buckets)}", flush=True)
    for label, size in sizes.items():
        if counts[label]["rows"] != size:
            raise ValueError(f"Corpus row count mismatch: {label}")
    return pa.concat_tables(queries).sort_by("id"), counts


def write_artifacts(output, cache, query_table):
    """Write query/provenance fixtures and inventory every assembled artifact."""
    (output / "queries").mkdir()
    pq.write_table(
        query_table, output / "queries/hn_queries.parquet", compression="zstd"
    )
    shutil.copyfile(
        cache.path("queries/holdout.parquet"), output / "queries/holdout.parquet"
    )
    (output / "provenance").mkdir()
    for name in ("plan.json", "profile.json", "embeddings.json"):
        shutil.copyfile(cache.path(name), output / "provenance" / name)
    artifacts = {}
    for path in sorted(output.rglob("*")):
        if path.is_file():
            entry = {"sha256": hn.file_digest(path), "bytes": path.stat().st_size}
            if path.suffix == ".parquet":
                entry["rows"] = pq.ParquetFile(path).metadata.num_rows
            artifacts[path.relative_to(output).as_posix()] = entry
    return artifacts


def assemble(
    source,
    cache,
    output,
    *,
    labels=("full",),
    workers=4,
    memory_limit="4GB",
):
    """Produce an immutable local dataset only after every source/cache check succeeds."""
    # pylint: disable=too-many-arguments,too-many-locals
    source, output = Path(source), Path(output)
    profile, embedded = load_cache(cache)
    sizes = sample_sizes(labels, profile["counts"]["corpus_rows"])
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError(f"Output directory must be empty: {output}")
    if workers < 1:
        raise ValueError("Workers must be positive")
    with tempfile.TemporaryDirectory(prefix=".assembly-", dir=output) as temporary:
        work = Path(temporary)
        with duckdb.connect(str(work / "assembly.duckdb")) as conn:
            conn.execute("SET memory_limit = ?", [memory_limit])
            conn.execute("SET threads = 1")
            conn.execute("SET TimeZone = 'UTC'")
            conn.execute("SET partitioned_write_max_open_files = 256")
            conn.execute("SET partitioned_write_flush_threshold = 16384")
            conn.execute("SET temp_directory = ?", [str(work / "spill")])
            conn.read_parquet(str(cache.path("queries/holdout.parquet"))).create_view(
                "holdout"
            )
            seed = partition_source(conn, source, work, profile, workers)
            partition_vectors(conn, cache, embedded, work)
            query_table, counts = assemble_partitions(conn, work, output, sizes)
    if query_table.column("id").to_pylist() != list(
        range(profile["counts"]["query_rows"])
    ):
        raise ValueError("Query output does not match the holdout")
    for label, size in sizes.items():
        if (
            size == profile["counts"]["corpus_rows"]
            and counts[label]["vector_rows"] != profile["counts"]["corpus_vector_rows"]
        ):
            raise ValueError("Full corpus vector count mismatch")
    manifest = {
        "format": 1,
        "state": "assembled",
        "dataset": "hn",
        "version": "v1",
        "source": profile["source"],
        "embedding": embedded["embedding"],
        "cache": {
            "prefix": cache.remote,
            "manifest_sha256": hn.file_digest(cache.path("embeddings.json")),
        },
        "assembly": {
            "script_sha256": hn.file_digest(__file__),
            "duckdb": duckdb.__version__,
            "pyarrow": pa.__version__,
            "sampling_seed": seed,
            "sample_order": "sha256(decimal_seed + ':' + decimal_hn_id), hn_id",
        },
        "counts": {
            "source_rows": profile["counts"]["source_rows"],
            "query_rows": query_table.num_rows,
        },
        "corpora": counts,
        "artifacts": write_artifacts(output, cache, query_table),
    }
    write_json(output / "manifest.json", manifest)
    return manifest


def publish_dataset(store):
    """Publish checked files before the manifest; never replace a different version."""
    if not store.remote:
        raise ValueError("Publishing requires --s3-prefix")
    manifest_path = store.path("manifest.json")
    manifest = json.loads(manifest_path.read_text())
    expected = hn.load_config(hn.DEFAULT_CONFIG)
    if (
        manifest["state"] != "assembled"
        or manifest["source"].get("archive_verified") is not True
        or store.remote != expected["artifact_prefix"]
    ):
        raise ValueError(
            "Publishing requires an archive-verified assembly at the configured artifact prefix"
        )
    intent = {"manifest_sha256": hn.file_digest(manifest_path)}
    with tempfile.TemporaryDirectory(dir=store.root) as temporary:
        remote = Store(temporary, store.remote)
        existing = remote.fetch("manifest.json", optional=True)
        if existing:
            if hn.file_digest(existing) != intent["manifest_sha256"]:
                raise ValueError(
                    "Dataset version already contains a different manifest"
                )
            return
        previous = remote.fetch("assembly-intent.json", optional=True)
        if previous and json.loads(previous.read_text()) != intent:
            raise ValueError("A different assembly already owns this dataset prefix")
    for relative, entry in manifest["artifacts"].items():
        if hn.file_digest(store.path(relative)) != entry["sha256"]:
            raise ValueError(f"Artifact checksum mismatch: {relative}")
    store.save("assembly-intent.json", intent)
    for relative in manifest["artifacts"]:
        store.publish(relative)
    store.publish("manifest.json")


def main():
    """Build locally or publish a completed, archive-verified assembly."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("assemble", "publish"))
    parser.add_argument("--source", type=Path)
    parser.add_argument("--cache-directory", type=Path)
    parser.add_argument("--cache-s3-prefix")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--s3-prefix")
    parser.add_argument("--sizes", nargs="+", default=["full"])
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--memory-limit", default="4GB")
    args = parser.parse_args()
    if args.action == "publish":
        store = Store(args.output, args.s3_prefix)
        with store.lock():
            publish_dataset(store)
    else:
        if not args.source or not args.cache_directory:
            parser.error("assemble requires --source and --cache-directory")
        cache = Store(args.cache_directory, args.cache_s3_prefix)
        with cache.lock():
            result = assemble(
                args.source,
                cache,
                args.output,
                labels=args.sizes,
                workers=args.workers,
                memory_limit=args.memory_limit,
            )
        print(json.dumps(result["corpora"], indent=2))


if __name__ == "__main__":
    main()
