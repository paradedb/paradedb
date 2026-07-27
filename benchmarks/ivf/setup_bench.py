#!/usr/bin/env python3
"""
Provision a local pg_search IVF benchmark fixture end to end, with an int
`category` fast field for BM25 filter predicates:

    (optional) download -> load table (id, category, embedding)
                        -> extensions -> bm25 index (category + vector) -> verify
                        -> (optional) cluster-size histogram + stats PNG

Dataset:
  cohere : VectorDBBench Cohere, 768d, cosine opclass (vector_cosine_ops).
           Sizes 100k/1m/... This is the target benchmark. (SIFT was removed;
           re-adding a dataset means restoring a resolver in bench_datasets.)

`category` is hash(id) % 100 -- deterministic, exactly-uniform 0..99 -- so the
sweep gets a clean selectivity dial: one term = 1%, range [0,10) = 10%,
range [0,50) = 50%.

The vector opclass is vector_cosine_ops (Cohere is cosine). The ORDER BY uses
the matching `<=>` operator, or the scan won't push down to the IVF TopK.

CONFIG + VALIDATION (new)
-------------------------
Every knob can come from a JSON config (--config), merged UNDER the CLI: an
explicit flag always wins, the file wins over the built-in default. Unlike the
old behavior, invalid input is now a HARD ERROR, never a silent coercion:
  * an unknown config key fails (it does not warn-and-ignore);
  * an out-of-range value fails (e.g. centroid_ratio must be in (0, 1];
    target_segments must be >= 4 -- it is NOT clamped up to 4 silently);
  * a malformed maintenance_work_mem (e.g. "8 gigs") fails.
So a typo'd config can never quietly build a different index than you asked for.

A config file is just the long-form flags as JSON, e.g.:

    {
      "size": 1000000, "skip_load": true,
      "centroid_ratio": 0.16, "replicas": 1,
      "cluster_plot": true, "outdir": "plots"
    }

Examples
--------
  python setup_bench.py --size 100000 --download
  python setup_bench.py --config configs/cohere_1m_repl_off.json
  python setup_bench.py --size 1000000 --skip-load --replicas 1
  python setup_bench.py --size 1000000 --skip-load --replicas 8

Vector index options (pg_search WITH clause), all optional -- omit for defaults:
  --replicas    fixed-k replication: total cells a vector lands in. 1 = OFF
                (primary only, no selector graph built; byte-identical baseline).
                8 = primary + 7 nearest centroids (RC=8). Default 1. The extra
                k-1 cells come from the in-house RNG (RelativeNeighborhoodGraph)
                over the centroids, in the field's raw metric -- router-consistent
                with query-time centroid ranking. Primary deduped out.

REMOVED (engine): the cluster-balancing band (max_posting_len/min_posting_len)
no longer exists upstream -- superseded by replica-based boundary replication.
A config carrying those keys is a hard error here, by design.
"""
import argparse
import json
import os
import re
import sys
import time

import psycopg

from bench_datasets import resolve_dataset, numerize

N_CATEGORIES = 100

# Cohere is cosine. (Kept as a constant rather than a metric->opclass dict now
# that SIFT/L2 is gone; restore the dict if a second metric returns.)
COHERE_OPCLASS = "vector_cosine_ops"
COHERE_DIST_OP = "<=>"

# Minimum a target_segment_count reloption is allowed to be. pg_search floors
# this at 4 internally; we reject below it here rather than silently clamp, so
# the built index always matches what was asked for.
MIN_TARGET_SEGMENTS = 4

_MWM_RE = re.compile(r"^\d+\s*(kB|MB|GB|TB)$", re.IGNORECASE)


def category_for(i: int) -> int:
    """Deterministic uniform 0..99 (multiplicative hash, decorrelated from id order)."""
    return ((i * 2654435761) & 0xFFFFFFFF) % N_CATEGORIES


def log(msg: str) -> None:
    print(f"[setup] {msg}", flush=True)


def die(msg: str):
    sys.exit(f"[setup] FATAL: {msg}")


# The engine emits one NOTICE per IVF segment build, e.g.:
#   ivf_build timings_ms train=812 selector_build=240 assign=190 replica_knn=8810 \
#       posting_write=45 total=10100 replicas=8 centroids=160000 vectors=1000000
# It fires on the backend running CREATE INDEX (no background worker for the bulk
# build), so a notice handler on this connection captures it. With replicas=1 no
# selector graph is built and selector_build/replica_knn are 0. (Pre-RNG engines
# emitted the key as hnsw_build; summarize_ivf_build normalizes it.) Multi-segment
# builds emit one line per segment; we keep them all and sum for the report.
_IVF_BUILD_RE = re.compile(r"(\w+)=(\d+)")


def make_ivf_build_collector():
    """Return (handler, lines). Register `handler` via conn.add_notice_handler
    BEFORE CREATE INDEX; afterwards `lines` holds one parsed dict per emitted
    `ivf_build` NOTICE (empty if the engine predates the timings instrumentation
    or the build produced only flat segments)."""
    lines = []

    def handler(diag):
        msg = (diag.message_primary or "")
        if msg.startswith("ivf_build timings_ms"):
            # extract attributes inside the callback -- the Diagnostic is
            # deallocated once the handler returns (psycopg3 contract).
            lines.append({k: int(v) for k, v in _IVF_BUILD_RE.findall(msg)})

    return handler, lines


def summarize_ivf_build(lines):
    """Collapse the per-segment ivf_build dicts into a single report block:
    phase times summed across segments, plus the per-segment count. Returns None
    if nothing was captured (older engine, or flat-only build)."""
    if not lines:
        return None
    phases = ("train", "selector_build", "assign", "replica_knn", "posting_write", "total")
    # normalize the pre-RNG key so old-engine captures still report
    lines = [{("selector_build" if k == "hnsw_build" else k): v
              for k, v in d.items()} for d in lines]
    summed = {p: sum(d.get(p, 0) for d in lines) for p in phases}
    summed["segments"] = len(lines)
    # replicas/centroids/vectors are per-segment build geometry; report the max
    # (they're equal across a uniform build) so the block reads sensibly.
    for g in ("replicas", "centroids", "vectors"):
        vals = [d[g] for d in lines if g in d]
        if vals:
            summed[g] = max(vals)
    return summed


def vlit(emb) -> str:
    return "[" + ",".join(map(repr, emb)) + "]"


# --------------------------------------------------------------- resolve/load

def _download_streaming(fs, remote, local, label, *, chunk=16 << 20, workers=16):
    """Fetch one remote file to `local` with a byte bar that advances as bytes
    arrive, fast over a high-RTT link. We drive tqdm ourselves instead of
    relying on s3fs's get_file callback (which, depending on version +
    max_concurrency, fires only once per large batch -> a bar parked at 0% that
    jumps at the end).

    Design (fixes the two things the naive version got wrong):
      * CONCURRENCY: keep `workers` ranged GETs in flight via a sliding window
        (fs.cat_file per range). More parallel streams is the main lever for S3
        throughput from far away -- a single stream is RTT-bound and crawls.
      * NO HEAD-OF-LINE STALL: progress is tracked on COMPLETION (bytes off the
        wire), not on in-order writes, so one slow range can't freeze the bar
        while other ranges are downloading. Disk writes are flushed in
        contiguous order as the prefix fills (the file grows 0->size for real --
        no sparse pre-truncate that shows full size / 0 blocks), but the writer
        never gates the downloaders.
    `.part` temp + size verify + atomic rename: an interrupted or short pull
    never leaves a wrong-but-right-sized file the skip check would accept; a
    mismatch raises so the caller falls back to vectordb_bench's downloader.
    chunk/workers are tunable -- raise workers to push a fat pipe harder."""
    import concurrent.futures as cf
    from collections import deque
    from tqdm import tqdm

    rp = str(remote)
    size = int(fs.info(rp)["size"])
    queue = deque((s, min(s + chunk, size)) for s in range(0, size, chunk))
    window = workers + 4           # a little queue depth so workers never idle
    part = local.with_name(local.name + ".part")
    pending, next_write, written = {}, 0, 0

    def _fetch(s, e):
        # fsspec cat_file is end-EXCLUSIVE (Pythonic slice); ranges sum to size.
        return s, fs.cat_file(rp, start=s, end=e)

    with open(part, "wb") as f, \
            tqdm(total=size, unit="B", unit_scale=True, unit_divisor=1024,
                 desc=label, leave=True) as bar, \
            cf.ThreadPoolExecutor(max_workers=workers) as ex:
        inflight = set()
        for _ in range(window):
            if queue:
                s, e = queue.popleft()
                inflight.add(ex.submit(_fetch, s, e))

        while inflight:
            done, not_done = cf.wait(inflight, return_when=cf.FIRST_COMPLETED)
            inflight = set(not_done)
            for fut in done:
                offset, data = fut.result()
                pending[offset] = data
                written += len(data)
                bar.update(len(data))          # progress = bytes downloaded
                if queue:                      # refill the window
                    s, e = queue.popleft()
                    inflight.add(ex.submit(_fetch, s, e))
            # flush the now-contiguous prefix to disk, in order (file grows 0->N)
            while next_write in pending:
                d = pending.pop(next_write)
                f.write(d)
                f.flush()
                next_write += len(d)

    if written != size:
        part.unlink(missing_ok=True)
        raise IOError(f"incomplete download for {remote}: {written} != {size} bytes")
    os.replace(part, local)


def _download_cohere_with_progress(m):
    """Download exactly the files m.prepare(S3) would -- train shards + test +
    groundtruth -- but with a PER-FILE BYTE progress bar instead of vectordb_
    bench's one-tick-per-completed-file bar (which parks on the multi-GB train
    shard, then jumps). Reuses vectordb_bench's own S3 reader, so the bucket
    URL, anonymous creds and region come from THEIR config -- we add only the
    progress (and the concurrent ranged fetch in _download_streaming).

    Raises on any problem so the caller can fall back to the stock prepare().
    Skips already-present files via the reader's own validate_file, matching
    prepare()'s idempotency, so a re-run (or a fallback after a partial run)
    only fetches what's missing."""
    import pathlib
    from vectordb_bench.backend.data_source import DatasetSource
    from vectordb_bench.backend.filter import non_filter

    if not getattr(m.data, "with_remote_resource", True):
        return  # nothing to fetch from S3 for this dataset variant

    reader = DatasetSource.S3.reader()       # AwsS3Reader: .fs, .remote_root, .validate_file
    fs, remote_root = reader.fs, reader.remote_root
    dataset = m.data.dir_name.lower()
    dest = m.data_dir
    dest.mkdir(parents=True, exist_ok=True)

    # the same set prepare() assembles (train shards + groundtruth + test)
    files = list(m.data.train_files)
    if m.data.with_gt:
        files += [non_filter.groundtruth_file, m.data.test_file]
    files = [f for f in files if f]

    n = len(files)
    for i, f in enumerate(files, 1):
        remote = pathlib.PurePosixPath(remote_root, dataset, f)
        local = dest / f
        if local.exists() and reader.validate_file(remote, local):
            log(f"  [{i}/{n}] {f}: present + validated, skip")
            continue
        local.parent.mkdir(parents=True, exist_ok=True)
        _download_streaming(fs, remote, local, f"[{i}/{n}] {f}")


def _download_cohere(m):
    """Fetch the Cohere files for this size. Tries the per-file byte-progress
    path first; on ANY failure (old fsspec, no callback kwarg, attribute drift)
    falls back to vectordb_bench's stock m.prepare() with its file-level bar --
    so the byte progress is purely additive and the download itself never
    depends on it. prepare() is idempotent, so the fallback finishes a partial
    byte-progress run rather than redoing it."""
    from vectordb_bench.backend.data_source import DatasetSource
    try:
        _download_cohere_with_progress(m)
    except Exception as exc:
        log(f"  per-file byte progress unavailable ({type(exc).__name__}: {exc}); "
            "falling back to vectordb_bench's file-level bar")
        m.prepare(source=DatasetSource.S3)


def resolve_cohere(size, do_download):
    """Returns (dim, metric, train_row_iter_factory, test_path, gt_path)."""
    try:
        from vectordb_bench.backend.dataset import Dataset
        from vectordb_bench.backend.data_source import DatasetSource
    except ImportError:
        die("vectordb_bench not importable -- activate the venv "
            "(source ~/vdb/bin/activate) or pip install 'vectordb-bench[pgvector]'")

    m = Dataset.COHERE.manager(size)
    if do_download:
        log(f"downloading Cohere {numerize(size)} (train/test/neighbors) from S3 ...")
        _download_cohere(m)

    ds = resolve_dataset("cohere", size)
    train_paths = [str(m.data_dir / f) for f in m.data.train_files]

    def train_rows(batch_size):
        import pyarrow.parquet as pq
        for path in train_paths:
            pf = pq.ParquetFile(path)
            for b in pf.iter_batches(batch_size=batch_size, columns=["id", "emb"]):
                ids = b.column("id").to_pylist()
                embs = b.column("emb").to_pylist()
                for i, e in zip(ids, embs):
                    yield int(i), e

    return m.data.dim, "cosine", train_rows, ds.test_path, ds.gt_path


def warn_if_debug(conn) -> None:
    try:
        ver, mode = conn.execute(
            "SELECT version, build_mode FROM paradedb.version_info()"
        ).fetchone()
    except Exception:
        return
    log(f"pg_search {ver} ({mode} build)")
    if mode != "release":
        log("!!! DEBUG BUILD -- query latency will be ~10-50x inflated. "
            "Rebuild with: cd pg_search && cargo pgrx run --release pg18")


def ensure_extensions(conn, opclass) -> None:
    conn.execute("CREATE EXTENSION IF NOT EXISTS vector")
    conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search")
    have = conn.execute(
        "SELECT 1 FROM pg_opclass oc JOIN pg_am am ON am.oid = oc.opcmethod "
        "WHERE am.amname = 'bm25' AND oc.opcname = %s", (opclass,)
    ).fetchone()
    if not have:
        log(f"{opclass}(bm25) missing -- recreating pg_search after vector "
            "(CASCADE; drops any other bm25 indexes in this DB)")
        conn.execute("DROP EXTENSION pg_search CASCADE")
        conn.execute("CREATE EXTENSION pg_search")


def load_table(conn, table: str, dim: int, train_rows, batch_size: int) -> int:
    conn.execute(f"DROP TABLE IF EXISTS {table} CASCADE")
    conn.execute(
        f"CREATE TABLE {table} "
        f"(id int PRIMARY KEY, category int NOT NULL, embedding vector({dim}))")
    total = 0
    with conn.cursor().copy(f"COPY {table} (id, category, embedding) FROM STDIN") as cp:
        for i, e in train_rows(batch_size):
            cp.write_row((i, category_for(i), vlit(e)))
            total += 1
            if total % batch_size == 0:
                print(f"\r[setup]   loaded {total:,}", end="", flush=True)
    print(f"\r[setup]   loaded {total:,}")
    return total


def table_has_category(conn, table: str) -> bool:
    return conn.execute(
        "SELECT 1 FROM information_schema.columns "
        "WHERE table_name = %s AND column_name = 'category'", (table,)
    ).fetchone() is not None


def build_index(conn, table, index, opclass, centroid_ratio, target_segments,
                mwm, workers, replicas=None):
    """Build the IVF index. `replicas`
    = the fixed-k total cells per vector, or None to leave at the pg_search
    default (1 = off). `target_segments` is validated >= MIN_TARGET_SEGMENTS by
    the caller -- passed through as-is, never clamped here.

    Captures the engine's per-segment `ivf_build timings_ms ...` NOTICE so the
    phase breakdown (train / selector_build / assign / replica_knn / posting_write)
    is reported alongside the wall-clock CREATE INDEX time -- replica_knn is the
    one to watch at replicas>1 (the per-vector RNG top-k queries).

    Returns (create_secs, vacuum_secs, build_timings): CREATE INDEX time and
    VACUUM ANALYZE time (timed separately so a slow build is distinguishable
    from a slow vacuum), plus the summarized ivf_build NOTICE (or None if the
    engine didn't emit one)."""
    conn.execute(f"SET maintenance_work_mem = '{mwm}'")
    conn.execute(f"SET max_parallel_maintenance_workers = {workers}")
    conn.execute(f"SET max_parallel_workers = {workers}")
    conn.execute(f"DROP INDEX IF EXISTS {index}")

    opts = ["key_field = id", f"centroid_ratio = {centroid_ratio}"]
    if target_segments is not None:
        opts.append(f"target_segment_count = {target_segments}")

    # Fixed-k replication: a single count. 1 (or unset->default 1) builds no
    # selector graph and is byte-identical baseline; N replicates each vector
    # into its N nearest centroids (primary from superkmeans + N-1 from the
    # in-house RNG over the centroids, field's raw metric).
    if replicas is not None:
        opts.append(f"cluster_replication = {replicas}")

    with_clause = ", ".join(opts)
    log(f"  WITH ({with_clause})")

    handler, build_lines = make_ivf_build_collector()
    conn.add_notice_handler(handler)
    try:
        t0 = time.time()
        # category indexed alongside the vector -> term/range predicates on it
        # drive the filtered probe path. opclass follows the dataset metric.
        conn.execute(
            f"CREATE INDEX {index} ON {table} "
            f"USING bm25 (id, category, embedding {opclass}) WITH ({with_clause})"
        )
        create_secs = time.time() - t0
    finally:
        # psycopg3 has no remove-by-handle; the connection closes after this
        # function's caller scope, so the dangling handler is harmless. (If you
        # reuse the connection for queries, register on a fresh conn instead.)
        pass

    t1 = time.time()
    conn.execute(f"VACUUM ANALYZE {table}")
    vacuum_secs = time.time() - t1

    timings = summarize_ivf_build(build_lines)
    if timings is not None:
        log(f"  ivf_build phases (ms, summed over {timings['segments']} seg): "
            f"train={timings['train']} selector_build={timings['selector_build']} "
            f"assign={timings['assign']} replica_knn={timings['replica_knn']} "
            f"posting_write={timings['posting_write']} total={timings['total']}")
    else:
        log("  (no ivf_build NOTICE captured -- engine predates the timings "
            "instrumentation, or the build produced only flat segments)")
    return create_secs, vacuum_secs, timings


def verify(conn, table, index, dist_op) -> None:
    """Correctness gate only: filter selectivities land on target, and the
    ORDER BY pushes to the IVF TopK scan for unfiltered/term/range. Size,
    centroid, and replication numbers live in collect_build_stats()."""
    total = conn.execute(f"SELECT count(*) FROM {table}").fetchone()[0]

    for label, pred, target in [
        ("sel1  (category = 5)",   "category = 5",  0.01),
        ("sel10 (category < 10)",  "category < 10", 0.10),
        ("sel50 (category < 50)",  "category < 50", 0.50),
    ]:
        n = conn.execute(f"SELECT count(*) FROM {table} WHERE {pred}").fetchone()[0]
        log(f"{label}: {n:,}/{total:,} = {n/total:.4f} (target {target})")

    qvec = conn.execute(
        f"SELECT embedding FROM {table} ORDER BY id LIMIT 1").fetchone()[0]

    def pushed(bm25_pred: str) -> bool:
        plan = "\n".join(r[0] for r in conn.execute(
            f"EXPLAIN (COSTS OFF) SELECT id FROM {table} WHERE {bm25_pred} "
            f"ORDER BY embedding {dist_op} %(q)s::vector LIMIT 10", {"q": qvec}
        ).fetchall())
        return "NormalScanExecState" not in plan and "not using Top K" not in plan

    term_pred = "id @@@ paradedb.term('category', 5)"
    range_pred = "id @@@ paradedb.range('category', int4range(0, 10, '[)'))"
    log(f"unfiltered ORDER BY pushed to IVF TopK: "
        f"{'YES' if pushed('id @@@ paradedb.all()') else 'NO  <-- check opclass/pushdown'}")
    log(f"term-filtered ORDER BY pushed to IVF TopK: "
        f"{'YES' if pushed(term_pred) else 'NO  <-- filtered pushdown broken'}")
    log(f"range-filtered ORDER BY pushed to IVF TopK: "
        f"{'YES' if pushed(range_pred) else 'NO  <-- range pushdown broken'}")


# ============================================================ build stats

def _f(x, default=0.0):
    """Numeric coercion: index_info returns Decimal/AnyNumeric and NULLs."""
    return default if x is None else float(x)


def human_bytes(n):
    if n is None:
        return "n/a"
    n = float(n)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if n < 1024 or unit == "TB":
            return f"{n:.0f}{unit}" if unit == "B" else f"{n:.2f}{unit}"
        n /= 1024


def collect_build_stats(conn, table, index, *, load_secs, create_secs,
                        vacuum_secs, cfg, build_timings=None):
    """Timings + size + structure for one build, as a flat dict. Size totals
    come from index_info.byte_size, which includes the IVF vector storage
    (.vec rows/id-map + .centroids) -- so the vector-side bytes, and any
    replication bloat, are captured. The bm25 side is the sum of the named
    component columns; vector_storage = total - bm25, the part that grows with
    replication.

    COUNT SEMANTICS: vector_info.vector_num_vectors is
    DISTINCT DOCS (flat and IVF agree), while per-cluster sizes are MEMBERSHIPS
    (rows incl. replicas) -- so the replication fraction must come from
    vector_total_memberships vs num_vectors, NOT num_vectors vs table rows
    (which is now a coverage check and always ~0 bloat). `build_timings` is
    the per-phase ms breakdown from the engine's ivf_build NOTICE (or None)."""
    # Current pg_search splits the per-field vector statistics out of
    # index_info into vector_info(index, field); sizes stay on index_info.
    row = conn.execute(
        "SELECT count(*), "
        "coalesce(sum(byte_size),0), "
        "coalesce(sum(postings_bytes),0), coalesce(sum(positions_bytes),0), "
        "coalesce(sum(fast_fields_bytes),0), coalesce(sum(fieldnorms_bytes),0), "
        "coalesce(sum(termdict_bytes),0), coalesce(sum(store_bytes),0) "
        f"FROM paradedb.index_info('{index}')"
    ).fetchone()
    (segments, total_b, post_b, pos_b, ff_b, fn_b, term_b, store_b) = row
    vrow = conn.execute(
        "SELECT coalesce(sum(vector_num_vectors),0), "
        "coalesce(sum(vector_num_centroids),0), "
        "min(vector_min_cluster_size), max(vector_max_cluster_size), "
        "avg(vector_avg_cluster_size), coalesce(sum(vector_empty_clusters),0) "
        f"FROM paradedb.vector_info('{index}', 'embedding')"
    ).fetchone()
    (n_vectors, n_centroids, cmin, cmax, cavg, empties) = vrow

    total_b = int(_f(total_b)); n_vectors = int(_f(n_vectors))
    n_centroids = int(_f(n_centroids))
    bm25_b = int(sum(_f(x) for x in (post_b, pos_b, ff_b, fn_b, term_b, store_b)))
    vector_b = max(total_b - bm25_b, 0)

    table_rows = conn.execute(f"SELECT count(*) FROM {table}").fetchone()[0]

    # Memberships (rows incl. replicas) and distinct docs both come from
    # vector_info. Their ratio is the real replication factor.
    try:
        memberships = conn.execute(
            "SELECT coalesce(sum(vector_total_memberships),0) "
            "FROM paradedb.vector_info(%s, 'embedding')",
            (index,)).fetchone()[0]
        memberships = int(_f(memberships)) or None  # 0 => flat-only build
    except Exception:
        memberships = None  # vector_info absent on this build
    try:
        pg_b = conn.execute(
            "SELECT pg_relation_size(%s::regclass)", (index,)).fetchone()[0]
    except Exception:
        pg_b = None

    build_secs = create_secs + vacuum_secs
    return {
        "index": index, "table": table,
        "config": {
            "dataset": cfg.dataset, "size": cfg.size,
            "centroid_ratio": cfg.centroid_ratio,
            "target_segments": cfg.target_segments,
            "replicas": cfg.replicas,
            "maintenance_work_mem": cfg.maintenance_work_mem,
            "workers": cfg.workers,
        },
        "timings_sec": {
            "load": round(load_secs, 2), "create_index": round(create_secs, 2),
            "vacuum_analyze": round(vacuum_secs, 2),
            "build_total": round(build_secs, 2),
        },
        # Per-phase engine breakdown from the ivf_build NOTICE (ms), summed over
        # segments. Diff replica_knn + selector_build across the A/B to price
        # replication (RNG build + per-vector top-k).
        "ivf_build_timings_ms": build_timings,
        "throughput": {
            "load_rows_per_sec": round(table_rows / load_secs) if load_secs else None,
            "build_vectors_per_sec": round(n_vectors / create_secs) if create_secs else None,
        },
        "size_bytes": {
            "total": total_b, "vector_storage": vector_b, "bm25": bm25_b,
            "pg_relation_size": int(pg_b) if pg_b is not None else None,
            "postings": int(_f(post_b)), "positions": int(_f(pos_b)),
            "fast_fields": int(_f(ff_b)), "fieldnorms": int(_f(fn_b)),
            "termdict": int(_f(term_b)), "store": int(_f(store_b)),
            "bytes_per_vector": round(total_b / n_vectors, 1) if n_vectors else None,
            "vector_fraction": round(vector_b / total_b, 3) if total_b else None,
        },
        "structure": {
            "segments": int(segments),
            "table_rows": int(table_rows),
            "distinct_docs": n_vectors,          # vector_info.vector_num_vectors
            "stored_entries": memberships,        # vector_total_memberships; None if absent
            "coverage": round(n_vectors / table_rows, 4) if table_rows else None,
            "replication_fraction": round((memberships - n_vectors) / n_vectors, 4)
                if memberships and n_vectors else None,
            "num_centroids": n_centroids,
            "target_centroids": int(round(cfg.centroid_ratio * table_rows)),
            "cluster_min": int(_f(cmin)), "cluster_max": int(_f(cmax)),
            "cluster_avg": round(_f(cavg), 2), "empty_clusters": int(_f(empties)),
        },
    }


def report_build_stats(stats, outdir, index):
    t = stats["timings_sec"]; tp = stats["throughput"]
    s = stats["size_bytes"]; st = stats["structure"]

    log(f"build timings: load={t['load']}s  create_index={t['create_index']}s  "
        f"vacuum={t['vacuum_analyze']}s  total(build+vac)={t['build_total']}s")
    if tp["build_vectors_per_sec"]:
        extra = (f"  load {tp['load_rows_per_sec']:,} rows/s"
                 if tp["load_rows_per_sec"] else "")
        log(f"  throughput: build {tp['build_vectors_per_sec']:,} vectors/s{extra}")

    rel = (f"  (pg_relation_size={human_bytes(s['pg_relation_size'])})"
           if s["pg_relation_size"] is not None else "")
    log(f"index size: total={human_bytes(s['total'])}{rel}")
    if s["vector_fraction"] is not None:
        log(f"  vector storage={human_bytes(s['vector_storage'])} "
            f"({s['vector_fraction']:.0%})  bm25={human_bytes(s['bm25'])}  "
            f"bytes/vector={s['bytes_per_vector']}")
    log(f"  components: postings={human_bytes(s['postings'])} "
        f"fast_fields={human_bytes(s['fast_fields'])} "
        f"terms={human_bytes(s['termdict'])} "
        f"fieldnorms={human_bytes(s['fieldnorms'])} "
        f"positions={human_bytes(s['positions'])} store={human_bytes(s['store'])}")

    rf = st["replication_fraction"]
    replicas = stats["config"].get("replicas")
    expected = (replicas - 1) if replicas and replicas > 1 else 0
    if st["stored_entries"] is None:
        rf_line = ("memberships unavailable (vector_info absent or "
                   "flat-only build) -> replication fraction not measured")
    else:
        rf_note = ("  (no replication)" if rf is None or rf <= 0.0001 else
                   f"  (~{rf:.1f}x extra; expect ~{expected}x at replicas={replicas})"
                   if expected else "  (unexpected: replicas<=1 but memberships > docs)")
        rf_line = (f"memberships={st['stored_entries']:,} vs docs="
                   f"{st['distinct_docs']:,} -> replication "
                   f"{0.0 if rf is None else rf:+.2%}{rf_note}")
    log(f"structure: segments={st['segments']}  "
        f"centroids={st['num_centroids']:,} (target≈{st['target_centroids']:,})  "
        f"docs={st['distinct_docs']:,} / rows={st['table_rows']:,} "
        f"(coverage {st['coverage']:.1%})")
    log(f"  {rf_line}")
    log(f"  cluster_size[min={st['cluster_min']} max={st['cluster_max']} "
        f"avg={st['cluster_avg']:.1f} empty={st['empty_clusters']:,}]")

    bt = stats.get("ivf_build_timings_ms")
    if bt:
        log(f"  ivf_build (ms): train={bt['train']} selector_build={bt['selector_build']} "
            f"assign={bt['assign']} replica_knn={bt['replica_knn']} "
            f"posting_write={bt['posting_write']} total={bt['total']} "
            f"[replicas={bt.get('replicas','?')} over {bt['segments']} seg]")

    os.makedirs(outdir, exist_ok=True)
    path = os.path.join(outdir, f"build_stats_{index}.json")
    with open(path, "w") as f:
        json.dump(stats, f, indent=2)
    log(f"wrote build stats -> {path}")


# ============================================================ config + schema

# dest -> (python types, hard default, validator). The validator is a callable
# (value) -> None that raises ValueError on a bad value. `None` is allowed only
# for keys whose default is None (the "leave at pg_search default" knobs).
def _pos_int(name, allow_zero=False, minimum=None):
    lo = 0 if allow_zero else 1
    if minimum is not None:
        lo = minimum
    def v(x):
        if isinstance(x, bool) or not isinstance(x, int):
            raise ValueError(f"{name} must be an integer, got {x!r}")
        if x < lo:
            raise ValueError(f"{name} must be >= {lo}, got {x}")
    return v


def _ratio(name):
    def v(x):
        if isinstance(x, bool) or not isinstance(x, (int, float)):
            raise ValueError(f"{name} must be a number, got {x!r}")
        if not (0.0 < x <= 1.0):
            raise ValueError(f"{name} must be in (0, 1], got {x}")
    return v


def _pos_float(name):
    def v(x):
        if isinstance(x, bool) or not isinstance(x, (int, float)):
            raise ValueError(f"{name} must be a number, got {x!r}")
        if x <= 0.0:
            raise ValueError(f"{name} must be > 0, got {x}")
    return v


def _choice(name, choices):
    def v(x):
        if x not in choices:
            raise ValueError(f"{name} must be one of {choices}, got {x!r}")
    return v


def _str(name):
    def v(x):
        if not isinstance(x, str) or not x:
            raise ValueError(f"{name} must be a non-empty string, got {x!r}")
    return v


def _bool(name):
    def v(x):
        if not isinstance(x, bool):
            raise ValueError(f"{name} must be true/false, got {x!r}")
    return v


def _mwm(name):
    def v(x):
        if not isinstance(x, str) or not _MWM_RE.match(x.strip()):
            raise ValueError(f"{name} must look like '8GB'/'512MB'/'1024kB', got {x!r}")
    return v


# Every tunable. Anything not here is an unknown key and is REJECTED.
SCHEMA = {
    "dataset":                 (str,           "cohere", _choice("dataset", ("cohere",))),
    "size":                    (int,           100_000,  _pos_int("size")),
    "download":                (bool,          False,    _bool("download")),
    "skip_load":               (bool,          False,    _bool("skip_load")),
    "dsn":                     (str,           "postgresql://localhost:28818/pg_search", _str("dsn")),
    "table":                   ((str, type(None)),  None, _str("table")),
    "centroid_ratio":          ((int, float),  0.01,     _ratio("centroid_ratio")),
    "target_segments":         ((int, type(None)),  None, _pos_int("target_segments", minimum=MIN_TARGET_SEGMENTS)),
    "replicas":                ((int, type(None)),  None, _pos_int("replicas", minimum=1)),
    "maintenance_work_mem":    (str,           "8GB",    _mwm("maintenance_work_mem")),
    "workers":                 (int,           8,        _pos_int("workers", allow_zero=True)),
    "batch_size":              (int,           10_000,   _pos_int("batch_size")),
    "cluster_plot":            (bool,          True,     _bool("cluster_plot")),
    "outdir":                  (str,           ".",      _str("outdir")),
}


def resolve_settings(args):
    """Merge defaults <- JSON config <- CLI, then validate. CLI flags use a
    sentinel default so 'user passed it' is unambiguous. Raises SystemExit with
    a precise message on any unknown key or out-of-range value."""
    cli = {k: v for k, v in vars(args).items()
           if k not in ("config",) and v is not _UNSET}

    file_cfg = {}
    if args.config is not _UNSET and args.config is not None:
        try:
            with open(args.config) as f:
                raw = json.load(f)
        except (OSError, json.JSONDecodeError) as exc:
            die(f"cannot read config {args.config!r}: {exc}")
        for key, val in raw.items():
            if key.startswith("_"):           # _comment etc. (JSON has no comments)
                continue
            dest = key.replace("-", "_")
            if dest not in SCHEMA:
                die(f"config {args.config}: unknown key '{key}' "
                    f"(valid keys: {', '.join(sorted(SCHEMA))})")
            file_cfg[dest] = val

    eff = {k: default for k, (_t, default, _v) in SCHEMA.items()}
    eff.update(file_cfg)   # file over default
    eff.update(cli)        # CLI over file

    # per-key validation
    for dest, (_types, _default, validate) in SCHEMA.items():
        val = eff[dest]
        if val is None:
            # None is only legal where the default is None (the "leave unset" knobs)
            if _default is not None:
                die(f"{dest} may not be null")
            continue
        try:
            validate(val)
        except ValueError as exc:
            die(str(exc))

    return argparse.Namespace(**eff, config=(None if args.config is _UNSET else args.config))


_UNSET = object()


def build_parser():
    ap = argparse.ArgumentParser(
        description="Provision a local pg_search IVF benchmark fixture (with category filter field).")
    # config path is special-cased (not a tunable in SCHEMA)
    ap.add_argument("--config", default=_UNSET, help="JSON config; CLI flags override it")
    # Every tunable defaults to the _UNSET sentinel so we can tell a real CLI
    # value from 'not passed' when merging the config underneath.
    ap.add_argument("--dataset", default=_UNSET, help="cohere (only supported dataset)")
    ap.add_argument("--size", type=int, default=_UNSET)
    ap.add_argument("--download", action="store_true", default=_UNSET)
    ap.add_argument("--skip-load", action="store_true", default=_UNSET)
    ap.add_argument("--dsn", default=_UNSET)
    ap.add_argument("--table", default=_UNSET)
    ap.add_argument("--centroid-ratio", type=float, default=_UNSET)
    ap.add_argument("--target-segments", type=int, default=_UNSET,
                    help=f"merger target segment count; must be >= {MIN_TARGET_SEGMENTS} "
                         "(rejected, not clamped, below that)")
    ap.add_argument("--replicas", type=int, default=_UNSET,
                    help="fixed-k replication: total cells per vector. 1 = OFF (no selector "
                         "graph); 8 = primary + 7 RNG-nearest centroids (RC=8). Default 1.")
    ap.add_argument("--maintenance-work-mem", default=_UNSET)
    ap.add_argument("--workers", type=int, default=_UNSET)
    ap.add_argument("--batch-size", type=int, default=_UNSET)
    ap.add_argument("--cluster-plot", action="store_true", default=_UNSET,
                    help="emit a cluster-size histogram + stats PNG after build (needs the "
                         "vector_info SRF; default on)")
    ap.add_argument("--no-cluster-plot", action="store_true", default=_UNSET,
                    help="suppress the cluster-stats PNG")
    ap.add_argument("--outdir", default=_UNSET, help="where the cluster-stats PNG is written")
    return ap


def main() -> None:
    args = build_parser().parse_args()

    # fold --no-cluster-plot into the cluster_plot bool before schema merge
    no_plot = getattr(args, "no_cluster_plot", _UNSET)
    if no_plot is not _UNSET and no_plot:
        if getattr(args, "cluster_plot", _UNSET) is not _UNSET and args.cluster_plot:
            die("pass only one of --cluster-plot / --no-cluster-plot")
        args.cluster_plot = False
    delattr(args, "no_cluster_plot")

    cfg = resolve_settings(args)

    table = cfg.table or f"{cfg.dataset}_{numerize(cfg.size)}"
    index = f"{table}_idx"

    dim, metric, train_rows, test_path, gt_path = resolve_cohere(cfg.size, cfg.download)
    opclass, dist_op = COHERE_OPCLASS, COHERE_DIST_OP
    log(f"dataset={cfg.dataset} dim={dim} metric={metric} "
        f"opclass={opclass} dist_op={dist_op}")

    with psycopg.connect(cfg.dsn, autocommit=True) as conn:
        warn_if_debug(conn)
        ensure_extensions(conn, opclass)

        load_secs = 0.0
        if not cfg.skip_load:
            log(f"loading {table} (+ category field) ...")
            _t = time.time()
            n = load_table(conn, table, dim, train_rows, cfg.batch_size)
            load_secs = time.time() - _t
            log(f"loaded {n:,} rows into {table} in {load_secs:.1f}s")
        else:
            if not table_has_category(conn, table):
                die(f"{table} exists but has no `category` column -- it predates the "
                    "filtered benchmark. Either re-run WITHOUT --skip-load, or backfill:\n"
                    f"  ALTER TABLE {table} ADD COLUMN category int;\n"
                    f"  UPDATE {table} SET category = (hashint4(id) & 2147483647) % 100;\n"
                    f"  ALTER TABLE {table} ALTER COLUMN category SET NOT NULL;")
            log(f"--skip-load: reusing existing {table} (load time not measured)")

        replicas = cfg.replicas   # None => engine default (1, off)

        rep_note = ""
        if replicas == 1:
            rep_note = " [replication OFF]"
        elif replicas is not None and replicas > 1:
            rep_note = f" [replicas={replicas}]"

        log(f"building {index} (opclass={opclass}, centroid_ratio={cfg.centroid_ratio}"
            + (f", target_segments={cfg.target_segments}" if cfg.target_segments else "")
            + rep_note + ") ...")
        create_secs, vacuum_secs, build_timings = build_index(
            conn, table, index, opclass, cfg.centroid_ratio,
            cfg.target_segments, cfg.maintenance_work_mem,
            cfg.workers, replicas=replicas)
        log(f"index built in {create_secs:.1f}s, vacuum+analyze in {vacuum_secs:.1f}s")

        verify(conn, table, index, dist_op)

        stats = collect_build_stats(
            conn, table, index, load_secs=load_secs, create_secs=create_secs,
            vacuum_secs=vacuum_secs, cfg=cfg, build_timings=build_timings)
        report_build_stats(stats, cfg.outdir, index)

        if cfg.cluster_plot:
            try:
                from cluster_stats import plot_index_cluster_stats
                plot_index_cluster_stats(
                    conn, index, cfg.outdir,
                    title=f"{table}  (centroid_ratio={cfg.centroid_ratio}{rep_note})")
            except SystemExit as exc:
                # SRF not landed yet, or index entirely flat -- don't abort the build
                log(f"cluster-stats plot skipped: {str(exc).splitlines()[0]}")
            except Exception as exc:
                log(f"cluster-stats plot skipped: {exc}")

    print()
    log("done. point the recall harness at:")
    print(f"    --size {cfg.size}")
    print(f"    (test={test_path}")
    print(f"     gt={gt_path})")


if __name__ == "__main__":
    main()
