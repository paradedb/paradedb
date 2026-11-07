#!/usr/bin/env python3
"""Build both extensions and evaluate exported, stored cluster memberships."""

import argparse
import hashlib
import json
import os
import struct
import time
from pathlib import Path

os.environ.setdefault("VECLIB_MAXIMUM_THREADS", "4")
os.environ.setdefault("OPENBLAS_NUM_THREADS", "4")
import numpy as np
import psycopg
from psycopg import sql


def unit_rows(values):
    values = np.ascontiguousarray(values, dtype=np.float32)
    norms = np.linalg.norm(values, axis=1, keepdims=True)
    if not np.isfinite(values).all() or not (norms > 0).all():
        raise ValueError("Expected finite, nonzero vectors")
    return values / norms


def exact_topk(vectors, queries, k=10, row_ids=None, batch_size=16384):
    count = len(vectors) if row_ids is None else len(row_ids)
    best_ids = np.empty((len(queries), 0), dtype=np.int64)
    best_scores = np.empty((len(queries), 0), dtype=np.float32)
    for start in range(0, count, batch_size):
        ids = (
            np.arange(start, min(start + batch_size, count))
            if row_ids is None
            else row_ids[start : start + batch_size]
        )
        scores = queries @ unit_rows(vectors[ids]).T
        local_order = np.argsort(-scores, axis=1, kind="stable")[:, :k]
        candidate_ids = np.concatenate((best_ids, ids[local_order]), axis=1)
        candidate_scores = np.concatenate(
            (best_scores, np.take_along_axis(scores, local_order, axis=1)), axis=1
        )
        order = np.lexsort((candidate_ids, -candidate_scores), axis=1)[:, :k]
        best_ids = np.take_along_axis(candidate_ids, order, axis=1)
        best_scores = np.take_along_axis(candidate_scores, order, axis=1)
    return best_ids


def load(conn, schema, vectors):
    dim = vectors.shape[1]
    conn.execute(sql.SQL("CREATE SCHEMA {}").format(sql.Identifier(schema)))
    conn.execute(
        sql.SQL("SET search_path TO {}, public").format(sql.Identifier(schema))
    )
    conn.execute(
        sql.SQL("CREATE TABLE items (id integer PRIMARY KEY, emb vector({}))").format(
            sql.Literal(dim)
        )
    )
    with conn.cursor().copy("COPY items (id, emb) FROM STDIN (FORMAT BINARY)") as copy:
        copy.write(b"PGCOPY\n\xff\r\n\0" + struct.pack("!ii", 0, 0))
        for start in range(0, len(vectors), 1024):
            batch = bytearray()
            for row, vector in enumerate(
                unit_rows(vectors[start : start + 1024]), start
            ):
                batch.extend(struct.pack("!hiiiHH", 2, 4, row, 4 + dim * 4, dim, 0))
                batch.extend(vector.astype(">f4").tobytes())
            copy.write(batch)
        copy.write(struct.pack("!h", -1))
    conn.execute("ANALYZE items")


def export(conn, engine, count, dim, output):
    with conn.transaction():
        conn.execute("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        conn.execute("LOCK TABLE items IN SHARE MODE")
        if engine == "pg_search":
            centers = conn.execute("""
                SELECT index_oid::text, segno, cluster_id, centroid, vector_partitioned
                FROM paradedb.vector_cluster_centroids('pdb_clusters', 'emb')
                ORDER BY index_oid, segno, cluster_id
            """).fetchall()
            member_query = """
                SELECT m.index_oid::text, m.segno, m.cluster_id, t.id
                FROM paradedb.vector_cluster_members('pdb_clusters', 'emb') m
                JOIN items t ON t.ctid=m.heap_tid
            """
            stored_counts = conn.execute("""
                SELECT segno, cluster_sizes FROM paradedb.vector_clusters('pdb_clusters', 'emb')
            """).fetchall()
            segment_counts = {}
            for _, segment, _, _, _ in centers:
                if segment in segment_counts:
                    continue
                matches = [
                    sizes
                    for short, sizes in stored_counts
                    if segment.replace("-", "").startswith(short)
                ]
                if len(matches) != 1 or matches[0] is None:
                    raise ValueError(
                        f"Cannot uniquely match cluster sizes for segment {segment}"
                    )
                segment_counts[segment] = matches[0]
            expected = {c[:3]: int(segment_counts[c[1]][c[2]]) for c in centers}
        else:
            centers = [
                ("prism", "0", c, vector)
                for c, vector, kind in conn.execute(
                    "SELECT * FROM prism_cluster_centroids('prism_clusters') ORDER BY cluster_id"
                ).fetchall()
                if kind == "encoding_reference"
            ]
            member_query = """
                SELECT 'prism', '0', m.cluster_id, t.id FROM prism_cluster_members('prism_clusters') m
                JOIN items t ON t.ctid=m.heap_tid
            """
            expected = {
                ("prism", "0", c): n
                for c, n in conn.execute("""
                SELECT cluster_id, sum(entry_count - dead_count)
                FROM prism_posting_pages('prism_clusters') WHERE NOT tombstoned
                GROUP BY cluster_id
            """).fetchall()
            }
        mapping = {c[:3]: i for i, c in enumerate(centers)}
        raw = np.asarray([c[3] for c in centers], dtype=np.float32)
        if raw.shape != (len(mapping), dim) or not len(mapping):
            raise ValueError("Invalid centroid export")
        total = sum(expected.values())
        labels = np.empty(total, dtype=np.int32)
        row_ids = np.empty(total, dtype=np.int32)
        seen = 0
        with conn.cursor(name=f"{engine}_members") as members:
            members.itersize = 8192
            members.execute(member_query)
            for index, segment, cluster, row in members:
                if not 0 <= row < count or seen >= total:
                    raise ValueError(f"Unexpected membership for row {row}")
                labels[seen] = mapping[(index, segment, cluster)]
                row_ids[seen] = row
                seen += 1
        if seen != total:
            raise ValueError(f"Expected {total} postings, exported {seen}")
        multiplicity = np.bincount(row_ids, minlength=count)
        if (multiplicity == 0).any():
            raise ValueError(
                "Some rows have no IVF membership (possibly flat segments); refusing an incomplete comparison"
            )
        if engine == "pg_search" and (multiplicity != 1).any():
            raise ValueError("Unexpected overlapping pg_search memberships")
        pairs = np.sort(row_ids.astype(np.int64) * len(raw) + labels)
        if (pairs[1:] == pairs[:-1]).any():
            raise ValueError("Duplicate row/cluster membership")
        del pairs
        order = np.argsort(row_ids, kind="stable")
        labels, row_ids = labels[order], row_ids[order]
        sizes = np.bincount(labels, minlength=len(raw))
        for identity, cluster in mapping.items():
            if sizes[cluster] != expected.get(identity, 0):
                raise ValueError(f"Posting-size mismatch for {identity}")
        np.savez(
            output / f"{engine}-clusters.npz",
            centroids=raw,
            labels=labels,
            row_ids=row_ids,
            cluster_keys=np.asarray(list(mapping), dtype=str),
        )
        metadata = {}
        if engine == "pg_search":
            metadata = {
                "segments": len(segment_counts),
                "vector_partitioned_segments": sorted({c[1] for c in centers if c[4]}),
            }
        return raw, labels, sizes, row_ids, metadata


def evaluate(
    vectors, queries, truth, raw, labels, counts, nearest_sample_rows=None, row_ids=None
):
    centers = unit_rows(raw)
    if row_ids is None:
        row_ids = np.arange(len(vectors))
    if len(row_ids) != len(labels) or np.any(row_ids[1:] < row_ids[:-1]):
        raise ValueError("Memberships must be ordered by stable row ID")
    multiplicity = np.bincount(row_ids, minlength=len(vectors))
    if not (multiplicity > 0).all():
        raise ValueError("Every corpus row must have a stored membership")
    offsets = np.r_[0, np.cumsum(multiplicity)]
    losses = np.full(len(vectors), np.inf)
    posting_loss = 0.0
    for start in range(0, len(labels), 2048):
        ids = row_ids[start : start + 2048]
        batch = unit_rows(vectors[ids])
        assigned = np.einsum(
            "ij,ij->i", batch, centers[labels[start : start + len(batch)]]
        )
        np.minimum.at(losses, ids, 1 - assigned)
        posting_loss += np.sum(1 - assigned, dtype=np.float64)
    nearest_count = min(len(vectors), nearest_sample_rows or len(vectors))
    nearest_ids = np.sort(
        np.random.default_rng(0).choice(len(vectors), nearest_count, replace=False)
    )
    gaps = np.empty(nearest_count, dtype=np.float64)
    batch_size = max(1, min(2048, 64 * 1024**2 // (4 * len(centers))))
    for start in range(0, nearest_count, batch_size):
        ids = nearest_ids[start : start + batch_size]
        scores = unit_rows(vectors[ids]) @ centers.T
        gaps[start : start + len(ids)] = scores.max(axis=1) - (1 - losses[ids])
    order = np.argsort(-(queries @ centers.T), axis=1, kind="stable")
    ranks = np.empty_like(order)
    ranks[np.arange(len(queries))[:, None], order] = np.arange(len(centers))
    hits = np.zeros_like(order)
    for query in range(len(queries)):
        first_ranks = [
            ranks[query, labels[offsets[row] : offsets[row + 1]]].min()
            for row in truth[query]
        ]
        np.add.at(hits[query], first_ranks, 1)
    coverage = np.cumsum(hits, axis=1) / truth.shape[1]
    scanned = np.cumsum(counts[order], axis=1)
    assert np.all(coverage[:, -1] == 1) and np.all(scanned[:, -1] == len(labels))
    checked = 0
    for query in range(min(5, len(queries))):
        for probes in sorted({1, min(8, len(centers)), max(1, len(centers) // 10)}):
            ids = np.unique(row_ids[ranks[query, labels] < probes])
            exact = exact_topk(
                vectors, queries[query : query + 1], truth.shape[1], ids
            )[0]
            recall = len(set(exact) & set(truth[query])) / truth.shape[1]
            assert np.isclose(recall, coverage[query, probes - 1])
            checked += 1
    recall = np.r_[0.0, coverage.mean(axis=0)]
    work = np.r_[0.0, scanned.mean(axis=0)]
    result = {
        "clusters": len(centers),
        "memberships": len(labels),
        "unique_rows": len(vectors),
        "replication_factor": len(labels) / len(vectors),
        "replicated_row_fraction": float((multiplicity > 1).mean()),
        "mean_cosine_loss": float(losses.mean()),
        "p95_cosine_loss": float(np.quantile(losses, 0.95)),
        "mean_posting_cosine_loss": float(posting_loss / len(labels)),
        "non_nearest_membership_fraction": float((gaps > 1e-5).mean()),
        "mean_excess_cosine_loss": float(gaps.mean()),
        "nearest_membership_diagnostic_rows": nearest_count,
        "loss_evaluation_rows": len(vectors),
        "cluster_size_mean": float(counts.mean()),
        "cluster_size_cv": float(counts.std() / counts.mean()),
        "cluster_size_p99": float(np.quantile(counts, 0.99)),
        "cluster_size_max": int(counts.max()),
        "empty_clusters": int((counts == 0).sum()),
        "centroid_norm_min": float(np.linalg.norm(raw, axis=1).min()),
        "centroid_norm_max": float(np.linalg.norm(raw, axis=1).max()),
        "exact_rerank_checks": checked,
    }
    for fraction in [0.01, 0.05, 0.1]:
        result[f"recall_at_scan_fraction_{fraction:g}"] = float(
            np.interp(len(vectors) * fraction, work, recall)
        )
    for target in [0.9, 0.95, 0.99]:
        result[f"vectors_at_recall_{target:g}"] = float(np.interp(target, recall, work))
    return result, recall, work


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--url",
        required=True,
        help="Local PostgreSQL with both patched extensions installed",
    )
    parser.add_argument(
        "--dataset",
        required=True,
        type=Path,
        help="NPZ, or directory with memory-mapped vectors.npy and queries.npy",
    )
    parser.add_argument("--dataset-name", help="Label for reports")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--schema", default=f"clustering_quality_{time.time_ns()}")
    parser.add_argument("--target-segment-count", type=int)
    parser.add_argument(
        "--rows",
        type=int,
        help="Explicit prefix limit; omitted means the entire dataset",
    )
    parser.add_argument(
        "--nearest-sample-rows",
        type=int,
        default=16384,
        help="Rows for the expensive global nearest-centroid diagnostic; 0 means all. Loss and recall always use the full corpus.",
    )
    args = parser.parse_args()
    if args.rows is not None and args.rows < 10:
        parser.error("--rows must be at least 10")
    if args.nearest_sample_rows < 0:
        parser.error("Invalid diagnostic sample size")
    if args.target_segment_count is not None and args.target_segment_count < 1:
        parser.error("--target-segment-count must be positive")
    if args.dataset.is_dir():
        vectors = np.load(args.dataset / "vectors.npy", mmap_mode="r")
        queries = unit_rows(np.load(args.dataset / "queries.npy", mmap_mode="r"))
        source_files = [args.dataset / name for name in ["vectors.npy", "queries.npy"]]
    else:
        with np.load(args.dataset) as data:
            vectors, queries = data["vectors"], unit_rows(data["queries"])
        source_files = [args.dataset]
    source_rows = len(vectors)
    if args.rows is not None and args.rows > source_rows:
        raise ValueError(
            f"Requested {args.rows:,} rows but the dataset contains only {source_rows:,}"
        )
    vectors = vectors[: args.rows]
    if (
        vectors.ndim != 2
        or queries.ndim != 2
        or len(vectors) < 10
        or len(queries) == 0
        or queries.shape[1] != vectors.shape[1]
    ):
        raise ValueError("Need at least ten base vectors and compatible queries")
    args.output.mkdir(parents=True, exist_ok=False)
    print(f"Preparing exact ground truth for {len(vectors):,} rows", flush=True)
    truth = exact_topk(vectors, queries)
    np.save(args.output / "ground_truth.npy", truth)
    normalized_hash = hashlib.sha256()
    for start in range(0, len(vectors), 16384):
        normalized_hash.update(unit_rows(vectors[start : start + 16384]).tobytes())
    source_hashes = {}
    for path in source_files:
        with path.open("rb") as source:
            source_hashes[path.name] = hashlib.file_digest(source, "sha256").hexdigest()
    manifest = {
        "dataset": str(args.dataset.resolve()),
        "dataset_name": args.dataset_name or args.dataset.stem,
        "source_file_sha256": source_hashes,
        "source_rows": source_rows,
        "explicit_row_limit": args.rows,
        "normalized_vectors_sha256": normalized_hash.hexdigest(),
        "rows": len(vectors),
        "dimensions": vectors.shape[1],
        "queries": len(queries),
        "top_k": 10,
        "schema": args.schema,
        "index_configuration": (
            f"extension defaults except pg_search target_segment_count={args.target_segment_count}; no extension GUC overrides"
            if args.target_segment_count is not None
            else "extension defaults; no index options or extension GUC overrides"
        ),
        "target_segment_count": args.target_segment_count,
        "training": "native default sampling and refinement, independently for each extension",
        "nearest_diagnostic_seed": 0,
        "assignment": "actual stored memberships exported from the built indexes; never reassigned",
        "evaluation_routing": "exact ranking of exported leaf centroids; not production ANN routing or query latency",
        "centroids": "pg_search stored routing centroids; PRISM unrotated encoding references (check leaf_refine_threshold)",
        "replication": "native defaults; retain every stored membership and count duplicate posting work",
        "loss": "minimum cosine loss among each row's actual stored memberships; posting-weighted loss also reported",
        "build_sql": {},
        "operating_points": "linear interpolation at equal mean posting rows scanned",
        "limitations": "one build per engine; different hierarchical cluster counts and iteration defaults; no seed variance estimate",
    }
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    summary, curves = {}, {}
    with psycopg.connect(args.url, autocommit=True) as conn:
        conn.execute("SET client_encoding='UTF8'")
        conn.execute("CREATE EXTENSION IF NOT EXISTS vector")
        conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search")
        conn.execute("CREATE EXTENSION IF NOT EXISTS pg_vectorsearch")
        manifest["extensions"] = conn.execute(
            "SELECT extname, extversion FROM pg_extension ORDER BY extname"
        ).fetchall()
        manifest["postgres_version"] = conn.execute("SELECT version()").fetchone()[0]
        manifest["vectorsearch_revision"] = conn.execute(
            "SELECT pg_vectorsearch_git_commit()"
        ).fetchone()[0]
        manifest["server_settings"] = conn.execute("""
            SELECT name, setting, unit, boot_val, source FROM pg_settings
            WHERE name IN ('maintenance_work_mem', 'shared_buffers', 'work_mem',
                'max_parallel_workers', 'max_worker_processes', 'max_parallel_maintenance_workers',
                'max_parallel_workers_per_gather')
                OR name LIKE 'prism.%' OR name LIKE 'paradedb.%'
            ORDER BY name
        """).fetchall()
        engine_overrides = [
            name
            for name, setting, _, boot, _ in manifest["server_settings"]
            if name.startswith(("prism.", "paradedb.")) and setting != boot
        ]
        if engine_overrides:
            raise ValueError(f"Extension GUCs differ from defaults: {engine_overrides}")
        load(conn, args.schema, vectors)
        print(
            f"Loaded {len(vectors):,} x {vectors.shape[1]} vectors into {args.schema}",
            flush=True,
        )
        for engine in ["pg_search", "vectorsearch"]:
            if engine == "pg_search":
                statement = "CREATE INDEX pdb_clusters ON items USING paradedb (id, emb vector_cosine_ops)"
                if args.target_segment_count is not None:
                    statement += f" WITH (target_segment_count={args.target_segment_count})"
            else:
                statement = "CREATE INDEX prism_clusters ON items USING prism (emb vec32_cosine_ops)"
            print(f"Building {engine}...", flush=True)
            manifest["build_sql"][engine] = statement
            started = time.monotonic()
            (args.output / "manifest.json").write_text(
                json.dumps(manifest, indent=2) + "\n"
            )
            conn.execute(statement)
            seconds = time.monotonic() - started
            print(f"Exporting {engine} stored clusters", flush=True)
            raw, labels, counts, row_ids, metadata = export(
                conn, engine, len(vectors), vectors.shape[1], args.output
            )
            if engine == "pg_search":
                manifest["pg_search_export"] = metadata
                if args.target_segment_count is not None and args.target_segment_count > 1:
                    if len(metadata["vector_partitioned_segments"]) != metadata["segments"]:
                        raise ValueError("Expected stored vector partition metadata on every segment")
            print(
                f"Evaluating {engine}: {len(raw):,} clusters, {len(labels):,} memberships",
                flush=True,
            )
            result, recall, work = evaluate(
                vectors,
                queries,
                truth,
                raw,
                labels,
                counts,
                args.nearest_sample_rows,
                row_ids,
            )
            result["index_build_seconds"] = seconds
            summary[engine] = result
            curves[f"{engine}_recall"], curves[f"{engine}_vectors_scanned"] = (
                recall,
                work,
            )
            print(json.dumps({engine: result}, indent=2), flush=True)
            (args.output / "summary.json").write_text(
                json.dumps(summary, indent=2) + "\n"
            )
            (args.output / "manifest.json").write_text(
                json.dumps(manifest, indent=2) + "\n"
            )
        manifest["vectorsearch_settings"] = conn.execute(
            "SELECT * FROM prism_index_settings('prism_clusters')"
        ).fetchall()
        manifest["pg_search_reloptions"] = conn.execute(
            "SELECT reloptions FROM pg_class WHERE oid='pdb_clusters'::regclass"
        ).fetchone()[0]
        manifest["vectorsearch_reloptions"] = conn.execute(
            "SELECT reloptions FROM pg_class WHERE oid='prism_clusters'::regclass"
        ).fetchone()[0]
        expected_options = (
            [f"target_segment_count={args.target_segment_count}"]
            if args.target_segment_count is not None
            else None
        )
        if manifest["pg_search_reloptions"] != expected_options or manifest["vectorsearch_reloptions"]:
            raise ValueError("Unexpected index options")
        manifest["pg_search_vector_info"] = conn.execute("""
            SELECT segno, vector_format, vector_num_vectors::bigint, vector_num_centroids::bigint,
                   quantized, layers, bytes_per_row, vector_router, vector_router_levels,
                   vector_total_rows::bigint
            FROM paradedb.vector_info('pdb_clusters', 'emb')
        """).fetchall()
    np.savez(args.output / "curves.npz", **curves)
    (args.output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
