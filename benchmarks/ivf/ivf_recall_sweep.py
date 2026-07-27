#!/usr/bin/env python3
"""
Recall + latency sweep for pg_search IVF, across BM25 FILTER SELECTIVITY x
GATE MODE x probe-ceiling fraction x epsilon x K, using the int `category`
fast field. Requires the candidate-gate engine (paradedb.vector_cluster_gate_mode
+ the FRACTIONAL paradedb.vector_cluster_max_probe ceiling): the
absolute max_probes GUC is gone, and under the default `candidate` gate mode
epsilon bands around the k-th best RESULT (squared-space L2), not the best
centroid -- values tuned for the centroid anchor DO NOT transfer.

Dataset (see bench_datasets.py):
  cohere : 768d cosine (the target benchmark; SIFT was removed)

Grid: for each filter in {none, sel50, sel10, sel1}, each gate mode, each
(fraction, epsilon), and each K, run every test query, score returned ids
against ground truth, and report recall@K, p50/p95/p99, serial QPS, plus the
measured selectivity.

THE THREE-RUNG GATE A/B (the current headline workflow): the same grid run
against two indexes gives the full table --
  centroid          this sweep with --gate-modes centroid (either index)
  candidate         --gate-modes candidate against an index built by a
                    PRE-RADIUS engine (its segments lack `.centroids` slot
                    [3] and load as all-zero radii); tag with
                    --rung-label candidate-noradii
  candidate+radius  --gate-modes candidate against a current-engine index;
                    tag with --rung-label candidate+radius
Epsilon stays at the shipped 0.5 on all rungs -- this harness reports the
table; re-deriving the default is the follow-up sweep's job.

K is a swept axis: recall@K moves with K (and in candidate mode the gate's
saturation precondition is exactly K), and the SPANN comparison is quoted at
recall@1 and recall@10 -- so `--ks 1,10` produces both in one pass. GT is
computed at max(K) once and truncated per K.

OUTPUTS (new): besides the printed grid, each run writes
  <outdir>/recall_<dataset>_<size>.csv            the full grid, one row per cell
  <outdir>/recall_<dataset>_<size>_eps<e>.png     recall vs probe fraction, one
                                                  line per filter/mode, one
                                                  subplot per K
Disable either with --no-csv / --no-plot.

Ground truth:
  - unfiltered: the dataset's neighbors.parquet (external truth) if given.
  - filtered: computed EXACTLY via plain Postgres (no @@@, no custom scan):
    seq scan + pgvector `<=>` top-K under the plain SQL predicate. Independent
    of the index under test; cached to JSON per (table, filter, K).

Works against any table with (id, category, embedding) and a bm25 index over
all three -- including a hand-rolled `cohere` with category backfilled via SQL,
since GT is computed from the live table.

Examples
--------
  python ivf_recall_sweep.py --size 100000
  python ivf_recall_sweep.py --size 1000000 --ks 1,10 \
      --probes 16,32,64,128,256,512,1024
  python ivf_recall_sweep.py --config configs/recall_filtered_collapse.json
  python ivf_recall_sweep.py --config configs/cohere_1m.json --filters none,sel1  # CLI wins
"""

import argparse
import csv
import json
import os
import re
import time

import pyarrow.parquet as pq
import psycopg

from bench_datasets import (resolve_dataset, load_config_into,
                            parse_int_list, parse_float_list, parse_str_list)

FILTER_PREDS = {
    # name -> (bm25 predicate for the measured query, plain-SQL predicate for GT)
    "none":  ("id @@@ paradedb.all()", "TRUE"),
    "sel50": ("id @@@ paradedb.range('category', int4range(0, 50, '[)'))", "category < 50"),
    "sel10": ("id @@@ paradedb.range('category', int4range(0, 10, '[)'))", "category < 10"),
    "sel1":  ("id @@@ paradedb.term('category', 5)", "category = 5"),
}

# Stable color + line style per filter so every plot reads the same way and a
# filter keeps its identity across K subplots and across datasets. Matches the
# in-chat recall curve: none green, sel10 amber, sel1 red, sel50 blue.
FILTER_STYLE = {
    "none":  ("#1D9E75", "-",  "o"),
    "sel50": ("#185FA5", "-.", "s"),
    "sel10": ("#BA7517", "--", "^"),
    "sel1":  ("#E24B4A", ":",  "D"),
}


def log(msg):
    print(f"[sweep] {msg}", flush=True)


# ----------------------------------------------------- probe-stats capture
#
# When --probe-stats is set we turn on `paradedb.log_probe_stats` and each
# query emits ONE NOTICE describing how its probe loop ran, e.g.:
#   probe_stats visited=9600 pruned_filter=9504 pruned_dead=0 pruned_seen=0 \
#       scored=96 clusters_probed=160 router_scored=1366 \
#       min_candidates=400 termination=ceiling:1,gate:0,exhausted:0
# This is the readout for the filtered-collapse experiment: `termination` says
# whether the loop was Ceiling-bound (raise max_probes) or Gate/Exhausted, and
# `scored < min_candidates` is the starvation signal -- the survivor floor
# couldn't be satisfied within the ceiling. NOTE: emitting a NOTICE per query
# adds protocol overhead, so latencies from a --probe-stats run are indicative,
# not authoritative -- run the clean sweep for the numbers, this for the why.

_PROBE_SCALAR_RE = re.compile(
    r"\b(visited|pruned_filter|pruned_dead|pruned_seen|scored|clusters_probed|"
    r"router_scored|centroids_ranked|min_candidates|exact_rows|"
    # candidate-gate era (tantivy pin >= radii/gate work): heap_saturated =
    # segments whose top-N heap ended full; gate_armed_at_ceiling = segments
    # where the gate's terminate condition held on the ceiling yield;
    # radius_skips = clusters the gate skipped without probing.
    r"heap_saturated|gate_armed_at_ceiling|radius_skips)=(\d+)")
_PROBE_TERM_RE = re.compile(r"termination=ceiling:(\d+),gate:(\d+),exhausted:(\d+)")
_PROBE_POSTINGS_RE = re.compile(r"postings=row:(\d+),skipped:(\d+)")


def make_probe_stats_collector():
    """Return (handler, lines). Register the handler on the connection; each
    `probe_stats` NOTICE becomes one dict appended to `lines`."""
    lines = []

    def handler(diag):
        msg = diag.message_primary or ""
        if not msg.startswith("probe_stats "):
            return
        d = {k: int(v) for k, v in _PROBE_SCALAR_RE.findall(msg)}
        m = _PROBE_TERM_RE.search(msg)
        if m:
            d["ceiling"], d["gate"], d["exhausted"] = (int(x) for x in m.groups())
        m = _PROBE_POSTINGS_RE.search(msg)
        if m:
            d["postings_row"], d["postings_skipped"] = (int(x) for x in m.groups())
        lines.append(d)

    return handler, lines


def aggregate_probe_stats(lines):
    """Collapse per-query probe_stats into per-cell summary fields. Scalar
    counters are averaged per query. The visited breakdown is kept in full so
    the engine invariant holds column-wise:
        visited == pruned_filter + pruned_dead + pruned_seen + scored
    (pruned_filter is the headline under a selective filter -- visited vectors
    the filter rejected). Termination is reported as the fraction of queries
    ps_pct_ceiling/gate/exhausted are segment-weighted shares of how probed
    segments terminated; they sum to 1.0. ps_pct_starved stays query-level
    fraction with scored < min_candidates -- summed across segments, a proxy,
    so pair it with pct_ceiling (under sel they coincide; under none they don't)."""
    if not lines:
        return {}
    # ---- self-check: engine invariants, verified on every query ----------
    # (1) visited == pruned_filter + pruned_dead + pruned_seen + scored
    # (2) when posting-mode tokens are present (pin >= filter-aware reads):
    #     postings bulk + row + skipped == clusters_probed
    # Violations indicate a parser/grammar drift or an engine bug -- either
    # way the cell's stats can't be trusted, so say so loudly.
    bad_vis = sum(1 for q in lines
                  if q.get("visited", 0) != q.get("pruned_filter", 0)
                  + q.get("pruned_dead", 0) + q.get("pruned_seen", 0)
                  + q.get("scored", 0))
    bad_post = sum(1 for q in lines
                   if "postings_bulk" in q and q.get("clusters_probed", 0)
                   != q["postings_bulk"] + q.get("postings_row", 0)
                   + q.get("postings_skipped", 0))
    if bad_vis or bad_post:
        print(f"[sweep]   !! STATS INVARIANT VIOLATED: "
              f"visited-identity broken on {bad_vis}/{len(lines)} queries, "
              f"postings-partition broken on {bad_post}/{len(lines)} -- "
              f"grammar drift or engine bug; treat this cell's ps_* as suspect")
    n = len(lines)

    def mean(key):
        return round(sum(d.get(key, 0) for d in lines) / n, 1)

    def frac(key):
        return round(sum(1 for d in lines if d.get(key, 0) > 0) / n, 3)

    pct_starved = sum(1 for d in lines
                      if d.get("scored", 0) < d.get("min_candidates", 0)) / n
    # Candidate-gate starvation: heap_saturated counts segments whose top-N
    # heap ended full, and ceiling+gate+exhausted counts segments probed —
    # a query with heap_saturated < segments left at least one segment
    # starved (returned < K passing docs from it).
    pct_heap_starved = sum(
        1 for d in lines
        if d.get("heap_saturated", 0)
        < d.get("ceiling", 0) + d.get("gate", 0) + d.get("exhausted", 0)) / n
    return {
        "ps_queries": n,
        # visited breakdown (sums to ps_visited)
        "ps_visited": mean("visited"),
        "ps_pruned_filter": mean("pruned_filter"),
        "ps_pruned_dead": mean("pruned_dead"),
        "ps_pruned_seen": mean("pruned_seen"),
        "ps_scored": mean("scored"),
        # navigation cost
        "ps_clusters_probed": mean("clusters_probed"),
        # router_scored (>= c82a9e44): centroids the beam router actually
        # scored. Pre-#170 engines emitted centroids_ranked (= all C, exact
        # routing); we report whichever is present under one column.
        "ps_router_scored": mean("router_scored") or mean("centroids_ranked"),
        # survivor floor + how the loop ended. Candidate mode reports the
        # floor as top_n (the saturation precondition), so ps_pct_starved
        # keeps its "scored < floor" reading across gate modes.
        "ps_min_candidates": mean("min_candidates"),
        # Segment-weighted: each probed segment ends exactly one way, so these
        # three are shares of segment-endings and sum to 1.0 (previously they
        # were "fraction of queries with ANY segment ending this way", which
        # overlaps on multi-segment indexes and could sum past 100%).
        **(lambda tc=sum(q.get("ceiling", 0) for q in lines),
                  tg=sum(q.get("gate", 0) for q in lines),
                  tx=sum(q.get("exhausted", 0) for q in lines):
           (lambda tot=max(tc + tg + tx, 1):
            {"ps_pct_ceiling": tc / tot,
             "ps_pct_gate": tg / tot,
             "ps_pct_exhausted": tx / tot})())(),
        "ps_pct_starved": round(pct_starved, 3),
        # candidate-gate telemetry (all 0 under gate_mode=centroid, and on
        # engines predating the candidate gate)
        "ps_radius_skips": mean("radius_skips"),
        "ps_heap_saturated": mean("heap_saturated"),
        "ps_gate_armed_at_ceiling": mean("gate_armed_at_ceiling"),
        "ps_pct_heap_starved": round(pct_heap_starved, 3),
    }


# probe-stats column order for the CSV (only written when --probe-stats).
# grouped: count | visited breakdown | navigation | floor + termination |
# candidate-gate telemetry.
PROBE_COLS = ["ps_queries",
              "ps_visited", "ps_pruned_filter", "ps_pruned_dead",
              "ps_pruned_seen", "ps_scored",
              "ps_clusters_probed", "ps_router_scored",
              "ps_min_candidates",
              "ps_pct_ceiling", "ps_pct_gate", "ps_pct_exhausted",
              "ps_pct_starved",
              "ps_radius_skips", "ps_heap_saturated",
              "ps_gate_armed_at_ceiling", "ps_pct_heap_starved"]


def vlit(emb):
    return emb if isinstance(emb, str) else "[" + ",".join(map(repr, emb)) + "]"


def pct(sorted_lat, q):
    return sorted_lat[min(len(sorted_lat) - 1, int(round(q * (len(sorted_lat) - 1))))]


def load_queries(test_path, n_q, vec_col, qid_col):
    t = pq.read_table(test_path)
    return list(zip(t.column(qid_col).to_pylist(),
                    t.column(vec_col).to_pylist()))[:n_q]


def external_gt(gt_path, qid_col, gt_col):
    g = pq.read_table(gt_path)
    return {qid: list(nb) for qid, nb in
            zip(g.column(qid_col).to_pylist(), g.column(gt_col).to_pylist())}


def compute_gt(conn, table, sql_pred, queries, k, cache_path, dist_op):
    """Exact filtered ground truth via plain Postgres (seq scan + pgvector
    top-K). `k` should be the MAX K in the sweep -- smaller K's truncate this.
    Cache is keyed by that K; a cache built at a smaller K is ignored."""
    if os.path.exists(cache_path):
        with open(cache_path) as f:
            cached = {int(q): ids for q, ids in json.load(f).items()}
        if all(qid in cached and len(cached[qid]) >= k for qid, _ in queries):
            log(f"  GT cache hit: {cache_path}")
            return cached
    log(f"  computing exact GT ({len(queries)} queries, pred = {sql_pred}) ...")
    gt_sql = (f"SELECT id FROM {table} WHERE {sql_pred} "
              f"ORDER BY embedding {dist_op} %(q)s::vector LIMIT {k}")
    out = {}
    t0 = time.time()
    for n, (qid, e) in enumerate(queries, 1):
        out[qid] = [r[0] for r in conn.execute(gt_sql, {"q": vlit(e)}).fetchall()]
        if n % 100 == 0:
            print(f"\r[sweep]   GT {n}/{len(queries)}", end="", flush=True)
    print()
    log(f"  GT computed in {time.time()-t0:.1f}s -> {cache_path}")
    os.makedirs(os.path.dirname(cache_path) or ".", exist_ok=True)
    with open(cache_path, "w") as f:
        json.dump(out, f)
    return out


def check_pushdown(conn, sql, qvec, label):
    plan = "\n".join(r[0] for r in conn.execute(
        "EXPLAIN (COSTS OFF) " + sql, {"q": qvec}).fetchall())
    if "NormalScanExecState" in plan or "not using Top K" in plan:
        log(f"WARNING [{label}]: ORDER BY NOT pushed down -- numbers below are "
            f"brute force, not IVF.")
        print(plan, "\n")
        return False
    return True


# ----------------------------------------------------------------- outputs

def write_csv(records, path):
    cols = ["filter", "sel_pct", "mode", "rung", "probe_fraction", "eps", "k",
            "recall", "p50ms", "p95ms", "p99ms", "qps",
            "repeats", "p50_spread", "p95_spread"]
    # extend with probe-stats columns iff any record carries them
    if records and any("ps_queries" in r for r in records):
        cols = cols + PROBE_COLS
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    with open(path, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=cols)
        w.writeheader()
        for r in records:
            w.writerow({c: r.get(c, "") for c in cols})
    log(f"wrote grid CSV -> {path}")


def plot_recall_curves(records, outdir, ds, epsilons, ks, filters):
    """One PNG per epsilon: a row of subplots, one per K, each plotting recall
    vs probe-ceiling FRACTION with one line per (filter, gate mode) — the
    color keeps the filter identity, the linestyle keys the mode (centroid
    dashed, candidate solid). Log-x because the fraction sweeps decades.
    Single-fraction runs (the gate-mode A/B table) still plot: they show the
    per-mode recall levels at the fixed operating point."""
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    mode_style = {"centroid": "--", "candidate": "-"}
    modes = sorted({r["mode"] for r in records})
    os.makedirs(outdir, exist_ok=True)
    for eps in epsilons:
        n = len(ks)
        fig, axes = plt.subplots(1, n, figsize=(5.2 * n, 4.4), squeeze=False)
        for col, k in enumerate(ks):
            ax = axes[0][col]
            for filt in filters:
                for mode in modes:
                    pts = sorted(
                        (r["probe_fraction"], r["recall"]) for r in records
                        if r["filter"] == filt and r["eps"] == eps
                        and r["k"] == k and r["mode"] == mode)
                    if not pts:
                        continue
                    xs, ys = zip(*pts)
                    color, _, marker = FILTER_STYLE.get(filt, ("#5F5E5A", "-", "o"))
                    ax.plot(xs, ys, color=color,
                            linestyle=mode_style.get(mode, "-"), marker=marker,
                            markersize=4, linewidth=1.8,
                            label=f"{filt}/{mode}")
            ax.set_xscale("log")
            ax.set_xlabel("probe ceiling (fraction of clusters)")
            ax.set_ylabel(f"recall@{k}")
            ax.set_ylim(0, 1.02)
            ax.grid(True, which="both", alpha=0.25, linewidth=0.5)
            ax.set_title(f"recall@{k}")
            ax.legend(title="filter/mode", fontsize=8, framealpha=0.9)
        fig.suptitle(f"{ds.family} {ds.size:,} -- recall vs probe fraction (eps={eps})",
                     fontsize=13)
        fig.tight_layout(rect=(0, 0, 1, 0.96))
        out = os.path.join(
            outdir, f"recall_{ds.family}_{numerize_safe(ds.size)}_eps{eps}.png")
        fig.savefig(out, dpi=130)
        plt.close(fig)
        log(f"wrote recall plot -> {out}")


def numerize_safe(n):
    if n % 1_000_000 == 0:
        return f"{n // 1_000_000}m"
    if n % 1_000 == 0:
        return f"{n // 1_000}k"
    return str(n)


def main():
    ap = argparse.ArgumentParser(description="filtered recall/latency sweep for pg_search IVF")
    ap.add_argument("--config", default=None,
                    help="JSON config file; CLI flags override it, it overrides defaults")
    ap.add_argument("--dataset", default="cohere", help="cohere (only supported dataset)")
    ap.add_argument("--size", type=int, default=100_000,
                    help="dataset size variant; derives the table name + parquet paths")
    ap.add_argument("--dsn", default="postgresql://localhost:28818/pg_search")
    ap.add_argument("--table", default=None, help="override table (default <dataset>_<size>)")
    ap.add_argument("--test", default=None, help="override test queries parquet")
    ap.add_argument("--gt", default=None,
                    help="override external UNFILTERED ground truth; filtered GT is "
                         "always computed+cached")
    ap.add_argument("--filters", default="none,sel50,sel10,sel1")
    ap.add_argument("--probe-fractions", default="0.05",
                    help="comma-separated FRACTIONAL probe ceilings "
                         "(paradedb.vector_cluster_max_probe): each segment "
                         "probes at most ceil(fraction * its cluster count), floored at "
                         "the engine's min_probe_clusters (16). 1.0 = exhaustive. "
                         "Old absolute probes p ~= fraction p / num_centroids.")
    ap.add_argument("--probes", default=None, help=argparse.SUPPRESS)   # removed (absolute cap era)
    ap.add_argument("--fanouts", default=None, help=argparse.SUPPRESS)  # removed
    ap.add_argument("--gate-modes", default="centroid,candidate",
                    help="comma-separated paradedb.vector_cluster_gate_mode values to "
                         "sweep (subset of centroid,candidate). 'centroid' is the "
                         "pre-candidate control arm; 'candidate' is result-anchored and "
                         "radius-aware. Against an index whose segments predate the "
                         "radius slot, 'candidate' IS the radii-absent rung -- label the "
                         "run with --rung-label to keep the three-rung table straight.")
    ap.add_argument("--rung-label", default=None,
                    help="free-text tag recorded in the 'rung' CSV column (e.g. "
                         "'candidate+radius' vs 'candidate-noradii' for two runs whose "
                         "gate_mode GUC value is identical but whose INDEX differs). "
                         "Defaults to the gate mode.")
    ap.add_argument("--epsilons", default="0.5",
                    help="pruning coefficients (paradedb.vector_cluster_probe_epsilon). "
                         "Default 0.5 = the shipped default; this sweep reports the "
                         "grid, it does not re-derive the default. Under gate-mode "
                         "'candidate' the band anchors on the k-th best result "
                         "(squared-space L2), so useful values sit far below the old "
                         "centroid-anchored 7.0.")
    ap.add_argument("--ks", default="10",
                    help="comma-separated K values, e.g. '1,10,100' (recall@K + 4*K floor)")
    ap.add_argument("--n-q", type=int, default=1000)
    ap.add_argument("--per-query-csv", default=None,
                    help="also dump one row PER QUERY PER REPEAT (rung, mode, "
                         "filter, eps, fraction, K, rep, qid, recall@K, and — "
                         "when --probe-stats is on — probed/ceiling/gate/"
                         "exhausted) to this path. The cell aggregates hide "
                         "per-query failure concentration; this exposes it.")
    ap.add_argument("--repeats", type=int, default=3,
                    help="latency passes per cell: each repeat runs the full query "
                         "set; the cell reports the MEDIAN across repeats of each "
                         "percentile, with max-min spread in the CSV "
                         "(p50_spread/p95_spread). Recall and probe_stats are "
                         "repeat-invariant (deterministic scans).")
    ap.add_argument("--gt-cache", default="gt_cache")
    ap.add_argument("--recompute-gt", action="store_true")
    ap.add_argument("--warmup-n", type=int, default=50,
                    help="warmup queries per filter (default 50; lower for "
                         "replicated builds whose working set exceeds RAM)")
    ap.add_argument("--outdir", default=".", help="where the CSV + PNGs are written")
    ap.add_argument("--no-plot", action="store_true", help="skip the recall-vs-probes PNGs")
    ap.add_argument("--no-csv", action="store_true", help="skip the grid CSV dump")
    ap.add_argument("--probe-stats", action="store_true",
                    help="capture per-query probe_stats via paradedb.log_probe_stats: "
                         "termination (ceiling/gate/exhausted) + starvation, into extra "
                         "CSV columns. Adds a NOTICE per query, so latencies become "
                         "indicative not authoritative -- use for the 'why', not numbers.")
    args = ap.parse_args()

    # ---- merge JSON config UNDER the CLI (explicit flag always wins) --------
    # defaults must match the add_argument defaults above so we can tell
    # "user passed it" from "still default".
    defaults = {"dataset": "cohere", "size": 100_000,
                "dsn": "postgresql://localhost:28818/pg_search",
                "table": None, "test": None, "gt": None,
                "filters": "none,sel50,sel10,sel1",
                "probe_fractions": "0.05", "probes": None, "fanouts": None,
                "gate_modes": "centroid,candidate", "rung_label": None,
                "epsilons": "0.5", "ks": "10", "n_q": 1000, "repeats": 3,
                "per_query_csv": None,
                "gt_cache": "gt_cache", "recompute_gt": False, "warmup_n": 50,
                "outdir": ".", "no_plot": False, "no_csv": False,
                "probe_stats": False}
    if args.config:
        cfg_raw, applied = load_config_into(args, args.config, defaults)
        log(f"config {args.config}: applied {applied or '(nothing -- all overridden on CLI)'}")
        if "fanouts" in cfg_raw:
            raise SystemExit(
                "[sweep] config sets 'fanouts', which is REMOVED (two engine "
                "generations back). The ceiling GUC is now the FRACTIONAL "
                "paradedb.vector_cluster_max_probe; set \"probe_fractions\".")
        if "probes" in cfg_raw:
            raise SystemExit(
                "[sweep] config sets 'probes', which is REMOVED: the absolute "
                "paradedb.vector_cluster_max_probes GUC was replaced by the FRACTIONAL "
                "paradedb.vector_cluster_max_probe. Replace with "
                "\"probe_fractions\": old probes p at C centroids/segment ~= fraction "
                "p/C (the engine floors the resolved ceiling at min_probe_clusters=16).")
    if args.fanouts is not None:
        raise SystemExit(
            "[sweep] --fanouts is REMOVED (the fanout GUC no longer exists). Use "
            "--probe-fractions; old fanout f is the SAME quantity (a fraction).")
    if args.probes is not None:
        raise SystemExit(
            "[sweep] --probes is REMOVED (the absolute max_probes GUC was replaced by "
            "the fractional ceiling). Use --probe-fractions; old probes p at C "
            "centroids/segment ~= fraction p/C.")

    ds = resolve_dataset(args.dataset, args.size, args.table, args.test, args.gt)
    if ds.test_path is None or not os.path.exists(ds.test_path):
        raise SystemExit(f"test parquet not found for {ds} "
                         f"(got {ds.test_path!r}); pass --test explicitly")
    log(f"dataset: {ds}")
    log(f"  test={ds.test_path}")
    log(f"  gt={ds.gt_path}")

    filters = parse_str_list(args.filters)
    fractions = parse_float_list(args.probe_fractions)
    gate_modes = parse_str_list(args.gate_modes)
    epsilons = parse_float_list(args.epsilons)
    ks = parse_int_list(args.ks)
    k_max = max(ks)
    for f in filters:
        if f not in FILTER_PREDS:
            raise SystemExit(f"unknown filter '{f}' (choose from {list(FILTER_PREDS)})")
    for m in gate_modes:
        if m not in ("centroid", "candidate"):
            raise SystemExit(f"unknown gate mode '{m}' (centroid or candidate)")
    for fr in fractions:
        if not 0.0 < fr <= 1.0:
            raise SystemExit(f"probe fraction {fr} out of (0, 1]")

    queries = load_queries(ds.test_path, args.n_q, ds.vec_col, ds.qid_col)
    log(f"{len(queries)} test queries, K in {ks} (GT at {k_max}), table={ds.table}")

    records = []  # structured rows for CSV + plotting (parallel to printed grid)
    per_query_rows = [] if args.per_query_csv else None

    with psycopg.connect(args.dsn, autocommit=True) as conn:
        total = conn.execute(f"SELECT count(*) FROM {ds.table}").fetchone()[0]

        # The gate-mode + fractional-ceiling GUCs are the two signatures of
        # the candidate-gate engine; fail loudly on an older one.
        try:
            conn.execute(f"SET paradedb.vector_cluster_gate_mode = {gate_modes[0]}")
            conn.execute(
                f"SET paradedb.vector_cluster_max_probe = {fractions[0]}")
        except Exception as exc:
            raise SystemExit(
                f"[sweep] engine rejects the candidate-gate GUCs ({exc}); this sweep "
                f"needs the gate-mode engine (paradedb.vector_cluster_gate_mode + "
                f"fractional ceiling). Older engines: use the pre-gate harness.")

        probe_lines = None
        if args.probe_stats:
            try:
                conn.execute("SET paradedb.log_probe_stats = on")
                # The capture rides the NOTICE channel; a server started with
                # client_min_messages=warning (pgrx does) silently suppresses
                # every line, so pin it for this session.
                conn.execute("SET client_min_messages = notice")
            except Exception as exc:
                log(f"--probe-stats: could not SET paradedb.log_probe_stats "
                    f"({exc}); is this engine built with the probe-stats GUC?")
                raise SystemExit(1)
            handler, probe_lines = make_probe_stats_collector()
            conn.add_notice_handler(handler)
            log("--probe-stats ON: per-query NOTICEs captured "
                "(latencies are now indicative, not authoritative)")

        hdr = (f"{'filter':>7}{'sel%':>7}{'mode':>10}{'frac':>7}{'eps':>6}{'K':>5}"
               f"{'recall':>9}{'p50ms':>8}{'p95ms':>8}{'p99ms':>8}{'qps':>8}")
        print(hdr)
        print("-" * len(hdr))
        rows_out = []

        for filt in filters:
            bm25_pred, sql_pred = FILTER_PREDS[filt]

            sel = conn.execute(
                f"SELECT count(*) FROM {ds.table} WHERE {sql_pred}").fetchone()[0] / total
            log(f"filter={filt}: selectivity {sel:.4f} ({int(sel*total):,} survivors)")

            # ground truth at K_MAX for this filter (smaller K's truncate it)
            if filt == "none" and ds.gt_path and os.path.exists(ds.gt_path) \
                    and not args.recompute_gt:
                gt = external_gt(ds.gt_path, ds.qid_col, ds.gt_col)
                log("  GT: external neighbors.parquet")
            else:
                cache = os.path.join(args.gt_cache,
                                     f"gt_{ds.table}_{filt}_k{k_max}.json")
                if args.recompute_gt and os.path.exists(cache):
                    os.remove(cache)
                gt = compute_gt(conn, ds.table, sql_pred, queries, k_max, cache,
                                ds.dist_op)

            # one warmup + one pushdown check per filter, at K_MAX
            sql_max = (f"SELECT id FROM {ds.table} WHERE {bm25_pred} "
                       f"ORDER BY embedding {ds.dist_op} %(q)s::vector LIMIT {k_max}")
            try:
                conn.execute(sql_max, {"q": vlit(queries[0][1])}).fetchall()
            except Exception as exc:
                log(f"FATAL [{filt}]: predicate failed to execute: {exc}")
                log("  if paradedb.range() is unavailable on this branch, replace the "
                    "sel10/sel50 predicates with a term_set or boolean OR of terms.")
                raise SystemExit(1)
            check_pushdown(conn, sql_max, vlit(queries[0][1]), filt)

            # Warm at the GRID's own max, not exhaustively: at replicas=N an
            # exhaustive scan touches ~N x the vector pages (dedup skips the
            # score, not the page fault), and a working set bigger than RAM
            # cannot be warmed -- 50 exhaustive queries would just thrash the
            # cache for many minutes with zero output. Warm what the sweep
            # will actually touch, and say so while doing it.
            warm_frac = max(fractions)
            conn.execute(
                f"SET paradedb.vector_cluster_max_probe = {warm_frac}")
            n_warm = min(args.warmup_n, len(queries))
            log(f"[{filt}] warming: {n_warm} queries at fraction={warm_frac}, "
                f"k={k_max} (--warmup-n to change)")
            t_w = time.time()
            for wi, (_, e) in enumerate(queries[:n_warm], 1):
                conn.execute(sql_max, {"q": vlit(e)}).fetchall()
                if wi % 10 == 0 or wi == n_warm:
                    log(f"[{filt}]   warmup {wi}/{n_warm} "
                        f"({(time.time()-t_w)/wi:.1f}s/query)")

            for mode in gate_modes:
                conn.execute(f"SET paradedb.vector_cluster_gate_mode = {mode}")
                rung = args.rung_label or mode
                for k in ks:
                    sql = (f"SELECT id FROM {ds.table} WHERE {bm25_pred} "
                           f"ORDER BY embedding {ds.dist_op} %(q)s::vector LIMIT {k}")
                    for eps in epsilons:
                        for frac in fractions:
                            conn.execute(
                                f"SET paradedb.vector_cluster_max_probe = {frac}")
                            conn.execute(
                                f"SET paradedb.vector_cluster_probe_epsilon = {eps}")

                            # snapshot the captured-NOTICE cursor so we
                            # attribute only THIS cell's per-query probe_stats
                            # lines (warmup/other cells excluded).
                            # Under a parallel plan each backend (leader +
                            # workers) emits its OWN probe_stats NOTICE covering
                            # the segments it scanned. Merge each query's
                            # notices into one dict so ps_ columns are
                            # per-QUERY, not per-backend.
                            cell_ps = [] if probe_lines is not None else None

                            # --repeats: each repeat is a full pass over the
                            # query set with its own percentile readings; the
                            # cell reports the median-across-repeats of each
                            # percentile (single-pass wobble was ±1 ms in the
                            # eps=0 sweep) plus the max-min spread. Recall is
                            # deterministic across repeats; probe_stats pool
                            # across repeats (per-query means are unchanged).
                            # cell_ps intentionally pools across repeats.
                            rep_p50, rep_p95, rep_p99 = [], [], []
                            all_lats, recs = [], []
                            cell_t0 = time.time()
                            for rep in range(max(1, args.repeats)):
                                lats, recs = [], []
                                for qi, (qid, e) in enumerate(queries, 1):
                                    q = vlit(e)
                                    ps_q0 = len(probe_lines) if probe_lines is not None else 0
                                    t0 = time.perf_counter()
                                    got = [r[0] for r in conn.execute(sql, {"q": q}).fetchall()]
                                    lats.append((time.perf_counter() - t0) * 1000.0)
                                    if qi % 100 == 0 and qi < len(queries):
                                        rate = (time.time() - cell_t0) / (rep * len(queries) + qi)
                                        left = rate * ((max(1, args.repeats) - rep) * len(queries) - qi)
                                        print(f"\r[sweep]   cell {filt}/{mode}/k{k}/"
                                              f"eps{eps}/f{frac} rep{rep + 1}: {qi}/{len(queries)} "
                                              f"({rate*1000:.0f}ms/q, ~{left:.0f}s left)",
                                              end="", flush=True)
                                    truth = gt[qid][:k]
                                    k_eff = min(k, len(truth)) or 1
                                    recs.append(len(set(truth) & set(got)) / k_eff)
                                    merged = None
                                    if probe_lines is not None and len(probe_lines) > ps_q0:
                                        merged = {}
                                        for d in probe_lines[ps_q0:]:
                                            for kk, vv in d.items():
                                                merged[kk] = merged.get(kk, 0) + vv
                                        cell_ps.append(merged)
                                    if per_query_rows is not None:
                                        pq = {"rung": rung, "mode": mode,
                                              "filter": filt, "eps": eps,
                                              "probe_fraction": frac, "k": k,
                                              "rep": rep, "qid": qid,
                                              "recall": round(recs[-1], 4)}
                                        if merged is not None:
                                            pq["probed"] = merged.get("clusters_probed", 0)
                                            pq["ceiling"] = merged.get("ceiling", 0)
                                            pq["gate"] = merged.get("gate", 0)
                                            pq["exhausted"] = merged.get("exhausted", 0)
                                        per_query_rows.append(pq)
                                lats.sort()
                                rep_p50.append(pct(lats, .50))
                                rep_p95.append(pct(lats, .95))
                                rep_p99.append(pct(lats, .99))
                                all_lats.extend(lats)

                            if len(queries) >= 100:
                                print("\r" + " " * 78 + "\r", end="")
                            mean_ms = sum(all_lats) / len(all_lats)
                            recall = sum(recs) / len(recs)
                            med = lambda xs: sorted(xs)[len(xs) // 2]
                            p50, p95, p99 = med(rep_p50), med(rep_p95), med(rep_p99)
                            p50_spread = max(rep_p50) - min(rep_p50)
                            p95_spread = max(rep_p95) - min(rep_p95)
                            qps = 1000.0 / mean_ms
                            rows_out.append(
                                f"{filt:>7}{sel*100:>7.2f}{mode:>10}{frac:>7}{eps:>6}{k:>5}"
                                f"{recall:>9.3f}{p50:>8.2f}{p95:>8.2f}{p99:>8.2f}{qps:>8.1f}")
                            print(rows_out[-1])
                            rec = {
                                "filter": filt, "sel_pct": round(sel * 100, 2),
                                "mode": mode, "rung": rung,
                                "probe_fraction": frac, "eps": eps, "k": k,
                                "recall": round(recall, 4),
                                "p50ms": round(p50, 2), "p95ms": round(p95, 2),
                                "p99ms": round(p99, 2), "qps": round(qps, 1),
                                "repeats": max(1, args.repeats),
                                "p50_spread": round(p50_spread, 2),
                                "p95_spread": round(p95_spread, 2),
                            }
                            if probe_lines is not None:
                                agg = aggregate_probe_stats(cell_ps)
                                rec.update(agg)
                                if agg:
                                    print(f"        probe: visited~{agg['ps_visited']:.0f} "
                                          f"filt~{agg['ps_pruned_filter']:.0f} "
                                          f"seen~{agg['ps_pruned_seen']:.0f} "
                                          f"scored~{agg['ps_scored']:.0f}/floor~"
                                          f"{agg['ps_min_candidates']:.0f}  "
                                          f"term[ceil={agg['ps_pct_ceiling']:.0%} "
                                          f"gate={agg['ps_pct_gate']:.0%} "
                                          f"exh={agg['ps_pct_exhausted']:.0%}] "
                                          f"starved={agg['ps_pct_starved']:.0%} "
                                          f"rskips~{agg['ps_radius_skips']:.1f} "
                                          f"heapsat~{agg['ps_heap_saturated']:.1f}")
                            records.append(rec)

        print()
        print("=== full grid ===")
        print(hdr)
        print("-" * len(hdr))
        for r in rows_out:
            print(r)

    # ---- artifacts ----------------------------------------------------------
    print()
    if not args.no_csv:
        write_csv(records, os.path.join(
            args.outdir, f"recall_{ds.family}_{numerize_safe(ds.size)}.csv"))
    if per_query_rows is not None:
        pq_cols = ["rung", "mode", "filter", "eps", "probe_fraction", "k",
                   "rep", "qid", "recall", "probed", "ceiling", "gate",
                   "exhausted"]
        os.makedirs(os.path.dirname(args.per_query_csv) or ".", exist_ok=True)
        with open(args.per_query_csv, "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=pq_cols)
            w.writeheader()
            for r in per_query_rows:
                w.writerow({c: r.get(c, "") for c in pq_cols})
        log(f"wrote per-query CSV -> {args.per_query_csv} ({len(per_query_rows)} rows)")
    if not args.no_plot:
        try:
            plot_recall_curves(records, args.outdir, ds, epsilons, ks, filters)
        except Exception as exc:
            log(f"plotting skipped: {exc} (install matplotlib, or pass --no-plot)")


if __name__ == "__main__":
    main()