"""Verify published HN artifacts and stage them for the shared heap loader."""

import argparse
import hashlib
import json
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path, PurePosixPath

import duckdb  # pylint: disable=import-error

CORPUS = "sampled/full/parquet/hn/"
FIXTURES = ("queries/holdout.parquet", "queries/hn_queries.parquet")


def fetch(source, relative, root, entry=None):
    """Fetch one artifact and verify its published size and SHA-256."""
    path = PurePosixPath(relative)
    if path.is_absolute() or ".." in path.parts or str(path) != relative:
        raise ValueError(f"Invalid artifact path: {relative}")
    destination = root / relative
    destination.parent.mkdir(parents=True, exist_ok=True)
    if source.startswith("s3://"):
        subprocess.run(
            [
                "aws",
                "s3",
                "cp",
                f"{source.rstrip('/')}/{relative}",
                str(destination),
                "--only-show-errors",
            ],
            check=True,
        )
    else:
        shutil.copyfile(Path(source) / relative, destination)
    if entry:
        with destination.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        if destination.stat().st_size != entry["bytes"] or digest != entry["sha256"]:
            raise ValueError(f"Artifact checksum/size mismatch: {relative}")
    return destination


def prepare(source, output, workers=16):
    """Bind a loadable corpus and verification SQL to the assembled manifest."""
    # pylint: disable=too-many-locals
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()) or workers < 1:
        raise ValueError("Require an empty output directory and positive workers")
    root = output / "dataset"
    manifest_path = fetch(source, "manifest.json", root)
    manifest = json.loads(manifest_path.read_text())
    identity = (
        manifest["format"],
        manifest["state"],
        manifest["dataset"],
        manifest["version"],
        manifest["embedding"]["dimensions"],
        manifest["embedding"]["distance"],
    )
    if identity != (1, "assembled", "hn", "v1", 1536, "cosine") or (
        manifest["source"].get("archive_verified") is not True
    ):
        raise ValueError("Expected an archive-verified assembled HN v1 cosine dataset")
    artifacts = manifest["artifacts"]
    shards = sorted(
        name
        for name in artifacts
        if name.startswith(CORPUS) and name.endswith(".parquet")
    )
    rows = manifest["corpora"]["full"]["rows"]
    vectors = manifest["corpora"]["full"]["vector_rows"]
    queries = manifest["counts"]["query_rows"]
    if any(
        not isinstance(n, int) or isinstance(n, bool) for n in (rows, vectors, queries)
    ):
        raise ValueError("Manifest counts must be integers")
    if not shards or not 0 < vectors <= rows or queries < 1:
        raise ValueError("Incomplete full corpus or query counts")
    with ThreadPoolExecutor(max_workers=workers) as pool:
        downloads = [
            pool.submit(fetch, source, name, root, artifacts[name])
            for name in [*shards, *FIXTURES]
        ]
        for index, download in enumerate(downloads, 1):
            download.result()
            print(f"Verified artifact {index}/{len(downloads)}", flush=True)
    with duckdb.connect() as db:
        shard_rows = db.execute(
            "SELECT sum(num_rows) FROM parquet_file_metadata(?)",
            [[str(root / name) for name in shards]],
        ).fetchone()[0]
        holdout = db.execute(
            "SELECT id, hn_id FROM read_parquet(?) ORDER BY id",
            [str(root / FIXTURES[0])],
        ).fetchall()
        query_ids = db.execute(
            "SELECT id FROM read_parquet(?) ORDER BY id",
            [str(root / FIXTURES[1])],
        ).fetchall()
    if shard_rows != rows:
        raise ValueError("Corpus Parquet row count differs from the manifest")
    expected_ids = list(range(queries))
    if (
        [row[0] for row in holdout] != expected_ids
        or [row[0] for row in query_ids] != expected_ids
        or len({row[1] for row in holdout}) != queries
        or any(
            not isinstance(row[1], int) or isinstance(row[1], bool) for row in holdout
        )
    ):
        raise ValueError("Query and holdout identities do not match the manifest")
    with manifest_path.open("rb") as stream:
        manifest_hash = hashlib.file_digest(stream, "sha256").hexdigest()
    ids = ",".join(str(row[1]) for row in holdout)
    sql = f"""DO $$
DECLARE
    actual_rows bigint;
    actual_vectors bigint;
BEGIN
    SELECT count(*), count(emb) INTO actual_rows, actual_vectors FROM hn_items;
    IF actual_rows <> {rows} OR actual_vectors <> {vectors} THEN
        RAISE EXCEPTION 'HN heap counts differ from manifest: % rows, % vectors', actual_rows, actual_vectors;
    END IF;
    IF EXISTS (SELECT 1 FROM hn_items WHERE id = ANY(ARRAY[{ids}]::bigint[])) THEN
        RAISE EXCEPTION 'Held-out HN queries overlap the corpus';
    END IF;
END
$$;

CREATE TABLE hn_dataset_identity (manifest_sha256 text PRIMARY KEY);
INSERT INTO hn_dataset_identity VALUES ('{manifest_hash}');
VACUUM (FREEZE, ANALYZE) hn_items;
"""
    (output / "verify_heap.sql").write_text(sql)
    table = output / "load-source/sampled/full/parquet/hn_items"
    table.parent.mkdir(parents=True)
    table.symlink_to(root / CORPUS, target_is_directory=True)
    print(
        f"HN manifest {manifest_hash}: {rows} rows, {vectors} vectors, {queries} queries"
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=16)
    args = parser.parse_args()
    prepare(args.source, args.output, args.workers)
