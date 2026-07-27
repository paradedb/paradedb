#!/usr/bin/env python3
"""
Cluster-size distribution for a pg_search IVF index: the histogram + the full
stats table (mean/median/std/min/max/empty/p90/p99/p99.9), matching the SPANN
reference plots we compare against.

WHY THIS NEEDS AN ENGINE FUNCTION
---------------------------------
`paradedb.index_info(index)` exposes only per-SEGMENT summaries --
vector_num_centroids, vector_{min,max,avg}_cluster_size, vector_empty_clusters.
A histogram and the median/std/p90/p99/p99.9 need the FULL per-cluster posting
lengths, which index_info collapses before anything reaches SQL.

The data already exists: tantivy's cluster_sizes() yields every posting
length. It's exposed through the read-only SRF (landed on mvp/vector-search):

    paradedb.ivf_cluster_sizes(index regclass)
        RETURNS TABLE(segno text, field text, cluster_ord int, size bigint)

COUNT SEMANTICS: sizes are MEMBERSHIPS (rows incl. replicas), deliberately --
they describe physical postings / scan cost. Under replicas=N their sum is
~N x index_info.vector_num_vectors (which is distinct docs). That gap is the
replication factor, not an error.

WHY NOT FAISS
-------------
You could approximate the distribution by re-clustering the loaded vectors with
faiss (`--source faiss`), the way Ming's reference plots were made. But that
measures a faiss kmeans over the raw vectors -- NOT your index's posting lists,
which are shaped by superkmeans AND fixed-k RNG replication (the rebalance
split/merge band was removed upstream -- expect more skew than old balanced
builds). The whole question ("does MY pipeline produce SPANN-like postings?")
is exactly what a faiss reference cannot answer. Use faiss only to draw the
target curve; use srf to measure the real index. The faiss path here is a
deliberately thin stub that prints this caveat.

Examples
--------
  python cluster_stats.py --index cohere_100k_idx
  python cluster_stats.py --index cohere_1m_idx --outdir plots --title "Cohere 1M RC=8"
  # called from setup_bench after a build (see plot_index_cluster_stats)
"""

import argparse
import os

import numpy as np
import psycopg


def log(msg):
    print(f"[cluster] {msg}", flush=True)


# ----------------------------------------------------------------- data source

def fetch_cluster_sizes_srf(conn, index):
    """Per-cluster posting lengths from the read-only SRF. Returns a flat numpy
    array of sizes (across all IVF segments) plus the segment count. Flat
    segments (no IVF) contribute no rows. Raises a clear error if the SRF isn't
    present on this build."""
    try:
        rows = conn.execute(
            "SELECT segno, size FROM paradedb.ivf_cluster_sizes(%s)", (index,)
        ).fetchall()
    except psycopg.errors.UndefinedFunction as exc:
        raise SystemExit(
            "paradedb.ivf_cluster_sizes(regclass) is not present on this build.\n"
            "  Current pg_search consolidated per-cluster sizes into the\n"
            "  vector_info(index, field) per-segment aggregates (min/max/avg/\n"
            "  empty/total) -- the full per-cluster size histogram is no longer\n"
            "  exposed via SQL. Run against an older build that still has the\n"
            "  SRF, or run with --source faiss for a REFERENCE distribution\n"
            f"  (not your index).\n  underlying error: {exc}")
    if not rows:
        raise SystemExit(
            f"{index}: SRF returned no IVF clusters -- the index is entirely flat "
            "segments (below the clustering threshold). Build a larger / "
            "consolidated segment first, or check index_info().")
    sizes = np.array([int(sz) for _, sz in rows], dtype=np.int64)
    n_segments = len({seg for seg, _ in rows})
    return sizes, n_segments


def fetch_cluster_sizes_faiss(conn, index):
    """REFERENCE distribution -- a faiss kmeans over the table's vectors. NOT the
    index's posting lists. Intentionally minimal; exists only to draw a target
    curve when the SRF isn't available. See the module docstring."""
    raise SystemExit(
        "--source faiss is a stub on purpose: it would measure a faiss reference "
        "clustering, not your index's postings (no rebalance band anymore, no replication), so "
        "it cannot answer 'does my pipeline match SPANN'. Land the SRF and use "
        "--source srf. If you genuinely want the reference target curve, wire "
        "faiss.Kmeans over the table embeddings here.")


# ----------------------------------------------------------------- stats + plot

def compute_stats(sizes):
    nonempty = sizes[sizes > 0]
    return {
        "clusters": int(sizes.size),
        "mean": float(sizes.mean()),
        "median": float(np.median(sizes)),
        "std": float(sizes.std()),
        "min": int(sizes.min()),
        "max": int(sizes.max()),
        "empty": int((sizes == 0).sum()),
        "p90": float(np.percentile(nonempty, 90)) if nonempty.size else 0.0,
        "p99": float(np.percentile(nonempty, 99)) if nonempty.size else 0.0,
        "p99.9": float(np.percentile(nonempty, 99.9)) if nonempty.size else 0.0,
        "cv": float(sizes.std() / sizes.mean()) if sizes.mean() else 0.0,
        "max_over_mean": float(sizes.max() / sizes.mean()) if sizes.mean() else 0.0,
    }


def plot(sizes, stats, title, outpath):
    """Two stacked panels: a stats table (Ming's table 1) on top, the cluster-
    size histogram (Ming's plot 2) below, with mean/median guide lines."""
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig = plt.figure(figsize=(9, 6.2))
    gs = fig.add_gridspec(2, 1, height_ratios=[1, 3], hspace=0.32)

    # --- stats table ---
    ax_t = fig.add_subplot(gs[0])
    ax_t.axis("off")
    # Ming's columns + the cluster count. cv and max/mean (skew indicators) go
    # in the log line and the caption, not the table -- keeps it from overflowing.
    order = ["clusters", "mean", "median", "std", "min", "max",
             "empty", "p90", "p99", "p99.9"]
    def fmt(k):
        v = stats[k]
        if k in ("clusters", "min", "max", "empty"):
            return f"{int(v):,}"
        return f"{v:.2f}"
    tbl = ax_t.table(
        cellText=[[fmt(k) for k in order]],
        colLabels=order, cellLoc="center", loc="center",
        colWidths=[1.0 / len(order)] * len(order))
    tbl.auto_set_font_size(False)
    tbl.set_fontsize(9)
    tbl.scale(1, 1.6)
    ax_t.set_title(f"cv={stats['cv']:.2f}   max/mean={stats['max_over_mean']:.1f}",
                   fontsize=9, loc="right", pad=2)

    # --- histogram ---
    ax = fig.add_subplot(gs[1])
    upper = int(np.percentile(sizes, 99.5)) + 1  # clip the long tail for readability
    upper = max(upper, stats["min"] + 1)
    bins = np.arange(0, upper + 2)
    ax.hist(sizes, bins=bins, color="#377AB5", edgecolor="none")
    ax.axvline(stats["mean"], color="#D62728", linestyle="--", linewidth=1.6,
               label=f"mean={stats['mean']:.1f}")
    ax.axvline(stats["median"], color="#2CA02C", linestyle=":", linewidth=1.6,
               label=f"median={stats['median']:.0f}")
    ax.set_xlabel("cluster size (# vectors in posting list)")
    ax.set_ylabel("# clusters")
    ax.set_xlim(0, upper)
    ax.legend()
    tail = sizes[sizes > upper]
    sub = (f"  (x clipped at p99.5={upper}; {tail.size:,} clusters with larger "
           f"postings up to {stats['max']:,})") if tail.size else ""
    ax.set_title("cluster size distribution" + sub, fontsize=10)

    fig.suptitle(title, fontsize=13)
    os.makedirs(os.path.dirname(outpath) or ".", exist_ok=True)
    fig.savefig(outpath, dpi=130, bbox_inches="tight")
    plt.close(fig)
    log(f"wrote cluster-stats plot -> {outpath}")


# ----------------------------------------------------------------- public hook

def plot_index_cluster_stats(conn, index, outdir, title=None, file=None, source="srf"):
    """Fetch -> stats -> plot for one index. Importable so setup_bench can call
    it right after a build. Returns the stats dict (also logged)."""
    fetch = fetch_cluster_sizes_srf if source == "srf" else fetch_cluster_sizes_faiss
    sizes, n_segments = fetch(conn, index)
    stats = compute_stats(sizes)
    log(f"{index}: {stats['clusters']:,} clusters over {n_segments} segment(s)  "
        f"mean={stats['mean']:.2f} median={stats['median']:.0f} "
        f"std={stats['std']:.2f} max={stats['max']:,} "
        f"empty={stats['empty']:,} cv={stats['cv']:.2f} "
        f"max/mean={stats['max_over_mean']:.1f}")
    file = file or f"cluster_stats_{index}.png"
    out = os.path.join(outdir, file)
    plot(sizes, stats, title or f"{index} -- cluster size distribution", out)
    return stats


def main():
    ap = argparse.ArgumentParser(description="cluster-size histogram + stats for a pg_search IVF index")
    ap.add_argument("--index", required=True, help="index name, e.g. cohere_100k_idx")
    ap.add_argument("--dsn", default="postgresql://localhost:28818/pg_search")
    ap.add_argument("--outdir", default=".")
    ap.add_argument("--title", default=None)
    ap.add_argument("--file", default=None)
    ap.add_argument("--source", choices=["srf", "faiss"], default="srf",
                    help="srf = real index postings (needs the SRF); faiss = reference (stub)")
    args = ap.parse_args()
    with psycopg.connect(args.dsn, autocommit=True) as conn:
        plot_index_cluster_stats(conn, args.index, args.outdir, args.title, args.file, args.source)


if __name__ == "__main__":
    main()
