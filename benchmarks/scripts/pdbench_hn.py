"""Bind HN artifacts to pdbench assets, heap snapshots, and filtered search runs."""

# pylint: disable=import-error

import argparse
import json
import os
import re
import shutil
import subprocess
from pathlib import Path

import duckdb
import psycopg
import pyarrow.parquet as pq

import prepare_hn as hn
from assemble_hn import checked_file
from embed_hn import Store
from ground_truth_hn import CORPUS_PREFIX, GT_SCHEMA, inventory, restore_dataset
from plan_hn_embeddings import write_json

PDBENCH_REV = "059615191b0890fea8c3693567b145d0bc63086e"
ASSETS = hn.DEFAULT_CONFIG.parent


def prepare(store, output, *, corpus=False):
    """Verify fixtures and produce a self-contained pdbench profile/assets bundle."""
    # pylint: disable=too-many-locals,too-many-branches,too-many-statements
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError("pdbench output directory must be empty")
    manifest, _ = restore_dataset(store, corpus=False)
    dataset_hash = hn.file_digest(store.path("manifest.json"))
    if not manifest["source"]["archive_verified"]:
        raise ValueError("Expected archive-verified HN v1 cosine data")
    ground_truth = json.loads(store.fetch("ground_truth/manifest.json").read_text())
    expected_gt = {
        "format": 1,
        "state": "complete",
        "dataset": "hn",
        "version": "v1",
        "dataset_manifest_sha256": dataset_hash,
        "size": "full",
        "k": 10,
        "metric": "cosine",
        "query_rows": manifest["counts"]["query_rows"],
        "corpus_rows": manifest["corpora"]["full"]["vector_rows"],
    }
    if any(ground_truth.get(key) != value for key, value in expected_gt.items()):
        raise ValueError("Ground truth does not match the complete HN dataset")
    if corpus:
        _, paths = restore_dataset(store)
        if set(store.path(CORPUS_PREFIX).glob("*.parquet")) != set(paths):
            raise ValueError("Unmanifested corpus files would enter pdbench's glob")
    metadata = output / "provenance"
    metadata.mkdir()
    shutil.copyfile(store.path("manifest.json"), metadata / "dataset-manifest.json")
    shutil.copyfile(
        store.path("ground_truth/manifest.json"),
        metadata / "ground-truth-manifest.json",
    )
    assets = output / "assets/hn"
    (assets / "queries").mkdir(parents=True)
    table_sql = (
        (ASSETS / "create_tables.sql")
        .read_text()
        .replace("DROP TABLE IF EXISTS hn_items;\n", "")
    )
    (assets / "create_table.sql").write_text(
        table_sql.replace("CREATE TABLE hn_items", "CREATE TABLE {{ table }}").replace(
            "vector(1536)", "vector({{ dims }})"
        )
    )
    index_path = "ground_truth/index.sql"
    index_sql = checked_file(
        store, index_path, ground_truth["artifacts"][index_path]["sha256"]
    ).read_text()
    if index_sql != (ASSETS / "indexes/pg_search.sql").read_text():
        raise ValueError("Frozen GT index differs from the benchmark index")
    (assets / "index.sql").write_text(
        index_sql.split(") WITH (")[0].replace(
            "CREATE INDEX hn_items_bm25_idx ON hn_items",
            "CREATE INDEX {{ index_name }} ON {{ table }}",
        )
        + ") WITH ({{ with_options }});\n"
    )
    names = set()
    expected_ids = list(range(ground_truth["query_rows"]))
    for item in ground_truth["filters"]:
        name = item["name"]
        if not re.fullmatch(r"knn_top10_[a-z0-9_]+", name) or name in names:
            raise ValueError("Invalid or duplicate workload name")
        names.add(name)
        relatives = (
            f"ground_truth/queries/{name}.sql",
            f"queries/ground_truth_{name}_full.parquet",
            f"ground_truth/{name}_boundary.parquet",
        )
        paths = [
            checked_file(store, path, ground_truth["artifacts"][path]["sha256"])
            for path in relatives
        ]
        query = paths[0].read_text()
        expected = (
            (ASSETS / "query.sql")
            .read_text()
            .replace("{{ predicate }}", item["predicate"])
        )
        if query != expected or query.count(";") != 1 or "{{" in query:
            raise ValueError("Frozen query is incompatible with pdbench's SQL parser")
        (assets / "queries" / f"{name}.sql").write_text(query)
        fixture = pq.read_table(paths[1])
        if (
            not fixture.schema.equals(GT_SCHEMA)
            or fixture["query_id"].to_pylist() != expected_ids
        ):
            raise ValueError("Invalid ground-truth identities/schema")
        for row in fixture["gt_ids"].to_pylist():
            if row is None or len(row) != 10 or len(set(row)) != 10 or None in row:
                raise ValueError("Ground truth must contain ten distinct nonnull IDs")
        shutil.copyfile(paths[2], metadata / paths[2].name)
    if not names:
        raise ValueError("No frozen workloads")
    profile = (
        (ASSETS / "pdbench.toml.template")
        .read_text()
        .replace("{{ expected_rows }}", str(manifest["corpora"]["full"]["rows"]))
        .replace("{{ data_path }}", json.dumps(str(store.root.resolve())))
    )
    (output / "hn-full.toml").write_text(profile)
    binding = {
        "dataset_manifest_sha256": dataset_hash,
        "ground_truth_manifest_sha256": hn.file_digest(
            store.path("ground_truth/manifest.json")
        ),
        "pdbench_revision": PDBENCH_REV,
        "dataset_directory": str(store.root.resolve()),
        "corpus_downloaded": corpus,
        "rows": manifest["corpora"]["full"]["rows"],
        "vector_rows": manifest["corpora"]["full"]["vector_rows"],
        "filters": ground_truth["filters"],
        "boundary_audit": ground_truth["boundary_audit"],
        "artifacts": inventory(output, (p for p in output.rglob("*") if p.is_file())),
    }
    write_json(output / "bundle.json", binding)
    return binding


def read_bundle(output):
    """Reject changes to the generated SQL/profile/provenance before execution."""
    binding = json.loads((output / "bundle.json").read_text())
    for relative, entry in binding["artifacts"].items():
        checked_file(Store(output), relative, entry["sha256"])
    return binding


def load_heap(output, dsn):
    """Load a verified corpus into a new table and bind its snapshot identity."""
    binding = read_bundle(output)
    if not binding["corpus_downloaded"]:
        raise ValueError("Loading requires prepare --corpus")
    store = Store(binding["dataset_directory"])
    _, paths = restore_dataset(store)
    table_sql = (
        (output / "assets/hn/create_table.sql")
        .read_text()
        .replace("{{ table }}", "hn_items")
        .replace("{{ dims }}", "1536")
    )
    with psycopg.connect(dsn) as db:
        if db.execute("SELECT to_regclass('hn_items')").fetchone()[0]:
            raise ValueError("Refusing to replace an existing HN heap")
        db.execute(table_sql)
    with duckdb.connect() as conn:
        conn.execute("INSTALL postgres; LOAD postgres")
        conn.execute("ATTACH '" + dsn.replace("'", "''") + "' AS pg (TYPE postgres)")
        conn.read_parquet([str(path) for path in paths]).create_view("corpus")
        conn.execute("INSERT INTO pg.public.hn_items SELECT * FROM corpus")
    holdout = pq.read_table(store.path("queries/holdout.parquet"))["hn_id"].to_pylist()
    with psycopg.connect(dsn) as db:
        counts = db.execute("SELECT count(*), count(emb) FROM hn_items").fetchone()
        if counts != (binding["rows"], binding["vector_rows"]):
            raise ValueError("Loaded heap counts do not match the dataset")
        if db.execute(
            "SELECT 1 FROM hn_items WHERE id = ANY(%s) LIMIT 1", (holdout,)
        ).fetchone():
            raise ValueError("Held-out queries overlap the loaded heap")
        db.execute(
            "CREATE TABLE hn_dataset_identity (manifest_sha256 text PRIMARY KEY)"
        )
        db.execute(
            "INSERT INTO hn_dataset_identity VALUES (%s)",
            (binding["dataset_manifest_sha256"],),
        )
    with psycopg.connect(dsn, autocommit=True) as db:
        db.execute("VACUUM ANALYZE hn_items")


def verify_heap(binding, db):
    """Refuse a same-shaped heap from another dataset version or an incomplete load."""
    identity = db.execute("SELECT manifest_sha256 FROM hn_dataset_identity").fetchall()
    if identity != [(binding["dataset_manifest_sha256"],)]:
        raise ValueError("Heap snapshot is bound to a different dataset")
    counts = db.execute("SELECT count(*), count(emb) FROM hn_items").fetchone()
    if counts != (binding["rows"], binding["vector_rows"]):
        raise ValueError("Heap row/vector counts differ from the bound dataset")


def check_plan(plan, predicate):
    """Require native vector TopK and indexed filters without a residual heap filter."""
    nodes = [plan[0]["Plan"]]
    scans = []
    while nodes:
        node = nodes.pop()
        if any(key in node for key in ("Filter", "Index Cond", "Recheck Cond")):
            raise ValueError("Predicate remains outside the ParadeDB index")
        if node.get("Custom Plan Provider") == "ParadeDB Base Scan":
            scans.append(node)
        nodes.extend(node.get("Plans", []))
    if len(scans) != 1:
        raise ValueError("Expected one ParadeDB Base Scan")
    scan = {key.strip(): value for key, value in scans[0].items()}
    order = scan.get("TopK Order By", "")
    if "emb <=> vector" not in order or scan.get("TopK Limit") != 10:
        raise ValueError("Vector TopK is not pushed into the ParadeDB scan")
    query = scan.get("Tantivy Query", "")
    parsed = json.loads(query) if isinstance(query, str) else query
    fields = set()
    pending = [parsed]
    while pending:
        value = pending.pop()
        if isinstance(value, dict):
            if "heap_filter" in value or "postgres_expression" in value:
                raise ValueError("Filter uses heap evaluation inside the ParadeDB scan")
            for key in ("field", "field_name"):
                if key in value:
                    fields.add(value[key])
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    required = set(
        re.findall(r'(?<!\w)(type|score|time|"by")\s*(?:=|>=|IN\b)', predicate)
    )
    if not {field.strip('"') for field in required}.issubset(fields):
        raise ValueError("Filter fields are missing from the indexed Tantivy query")


def run(output, checkout, dsn):
    """Build with pinned pdbench, verify pushdown, and measure all frozen workloads."""
    # pylint: disable=too-many-locals
    binding = read_bundle(output)
    revision = subprocess.check_output(
        ["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True
    ).strip()
    dirty = subprocess.check_output(
        ["git", "-C", str(checkout), "status", "--porcelain", "--untracked-files=no"],
        text=True,
    )
    if revision != PDBENCH_REV or dirty:
        raise ValueError(
            "pdbench must be an unmodified checkout at the pinned revision"
        )
    reports = output / "runs"
    reports.mkdir(exist_ok=True)
    with psycopg.connect(dsn) as db:
        verify_heap(binding, db)
    binary = checkout / "target/release/pdbench"
    common = [
        "--config",
        str(output / "hn-full.toml"),
        "--assets-dir",
        str(output / "assets"),
        "--runs-dir",
        str(reports),
    ]
    env = dict(os.environ, PDBENCH_URL=dsn)
    subprocess.run([str(binary), "build", *common], env=env, check=True)
    vector = pq.read_table(
        Path(binding["dataset_directory"]) / "queries/hn_queries.parquet"
    )["emb"][0].as_py()
    with psycopg.connect(dsn) as db:
        db.execute("SELECT set_config('hn.qvec', %s, false)", (json.dumps(vector),))
        for item in binding["filters"]:
            query = (output / "assets/hn/queries" / f"{item['name']}.sql").read_text()
            plan = db.execute(
                "EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT JSON) " + query
            ).fetchone()[0]
            write_json(reports / f"{item['name']}-plan.json", plan)
            check_plan(plan, item["predicate"])
    for item in binding["filters"]:
        subprocess.run(
            [
                str(binary),
                "search",
                *common,
                "--query",
                item["name"],
                "--html",
                "--no-compare",
            ],
            env=env,
            check=True,
        )
    write_json(
        reports / "complete.json",
        {
            "dataset_manifest_sha256": binding["dataset_manifest_sha256"],
            "workloads": [item["name"] for item in binding["filters"]],
        },
    )


def main():
    """Expose artifact preparation, heap loading, and benchmark execution."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("prepare", "load-heap", "run"))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--dataset-directory", type=Path)
    parser.add_argument("--s3-prefix")
    parser.add_argument("--corpus", action="store_true")
    parser.add_argument("--pdbench", type=Path)
    args = parser.parse_args()
    if args.action == "prepare":
        if not args.dataset_directory:
            parser.error("prepare requires --dataset-directory")
        prepare(
            Store(args.dataset_directory, args.s3_prefix),
            args.output,
            corpus=args.corpus,
        )
    elif args.action == "load-heap":
        load_heap(args.output, os.environ["PDBENCH_URL"])
    else:
        if not args.pdbench:
            parser.error("run requires --pdbench")
        run(args.output, args.pdbench, os.environ["PDBENCH_URL"])


if __name__ == "__main__":
    main()
