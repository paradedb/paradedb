#!/usr/bin/env python3
"""
End-to-end flamegraph sweep for the pg_search IVF query path, over a
(filter x probe-ceiling) grid. Requires the fractional-ceiling engine
(paradedb.vector_cluster_max_probe): the cap is a FRACTION of each
segment's cluster count, resolved per segment.

For each (filter, probes) cell: pins the session (no parallel workers, GUCs),
verifies TopK pushdown for THAT predicate, warms the cache, attaches samply via
sudo, drives the query loop on the SAME backend for the capture window, then
stops samply itself (sudo kill -INT) and validates the capture.

This is the companion to ivf_recall_sweep.py, not a replacement: the sweep
produces trustworthy numbers (default parallel execution, no profiler overhead),
this produces the explanation (serial, sampled). Don't quote latencies from here.

NOTE: --save-only profiles are often written BEFORE symbolication (samply
symbolicates lazily at `samply load`). Symbol columns showing '?' are expected;
open the profile and search the call tree. Zero samples IS conclusive (failed attach).

Usage:
  python profile_flamegraph_sweep.py                                  # cohere/100k
  python profile_flamegraph_sweep.py --size 1000000 --probes 256,2048,65536
  python profile_flamegraph_sweep.py --config configs/profile_rank_centroids.json
  python profile_flamegraph_sweep.py --filters none,sel1 --probes 2048
  # afterwards:  samply load flamegraphs_cohere_100k/sel1_probes_2048.json

A flamegraph is a single operating point, so K is a scalar here (--k), not a
grid -- unlike the recall sweep which sweeps --ks.
"""

import argparse
import gzip
import json
import os
import shutil
import statistics
import subprocess
import sys
import threading
import time

import psycopg

from bench_datasets import (resolve_dataset, load_config_into, numerize,
                            parse_float_list, parse_int_list, parse_str_list)

FILTER_PREDS = {
    "none":  "id @@@ paradedb.all()",
    "sel50": "id @@@ paradedb.range('category', int4range(0, 50, '[)'))",
    "sel10": "id @@@ paradedb.range('category', int4range(0, 10, '[)'))",
    "sel1":  "id @@@ paradedb.term('category', 5)",
}


def log(msg):
    print(f"[profile] {msg}", flush=True)


def die(msg):
    sys.exit(f"[profile] FATAL: {msg}")


# ---------------------------------------------------------------- helpers

def check_build_mode(conn):
    try:
        ver, mode = conn.execute(
            "SELECT version, build_mode FROM paradedb.version_info()").fetchone()
        log(f"pg_search {ver} ({mode} build)")
        if mode != "release":
            log("!!! NOT a release-codegen build -- use: "
                "cargo pgrx run --profile release-with-debug pg18")
    except Exception:
        log("could not read paradedb.version_info() -- continuing")


def fetch_query_vectors(conn, table, n=32):
    rows = conn.execute(
        f"SELECT embedding FROM {table} ORDER BY id LIMIT {n}").fetchall()
    if not rows:
        die(f"table {table} is empty -- run setup_bench.py first")
    return [r[0] for r in rows]


def verify_pushdown(conn, sql, qvec, label):
    plan = "\n".join(r[0] for r in conn.execute(
        "EXPLAIN (COSTS OFF) " + sql, {"q": qvec}).fetchall())
    if "TopKScanExecState" not in plan or "NormalScanExecState" in plan:
        print(plan)
        die(f"[{label}] ORDER BY is NOT pushed to the IVF TopK scan -- a capture "
            "would profile brute force and contain neither phase frame.")
    if "Gather" in plan or "Workers Planned" in plan:
        print(plan)
        die(f"[{label}] plan is still parallel despite "
            "max_parallel_workers_per_gather=0 -- workers would carry the samples.")
    log(f"[{label}] pushdown OK: serial Custom Scan, TopKScanExecState")


def stop_samply(proc, label):
    """Stop the root-owned samply. Signal the samply process ITSELF via pkill
    (sudo's signal relay to its child is unreliable on macOS), with a relay
    attempt through the wrapper as belt-and-braces. All kills use sudo -n so an
    expired credential can never silently block on a password prompt -- the
    keepalive thread should make that impossible anyway."""
    # EXACTLY ONE SIGINT. A second interrupt while samply is saving means
    # "abort now" and the profile never gets written (the empty-capture bug:
    # pkill + wrapper-relay landed two INTs back to back). Each further signal
    # is a fallback that only fires if the previous one didn't take.
    subprocess.run(["sudo", "-n", "pkill", "-INT", "-x", "samply"],
                   capture_output=True)
    try:
        return proc.wait(timeout=45)  # symbol-dump + write can take a while
    except subprocess.TimeoutExpired:
        pass
    log(f"[{label}] samply still up after direct SIGINT -- trying the sudo wrapper relay")
    subprocess.run(["sudo", "-n", "kill", "-INT", str(proc.pid)],
                   capture_output=True)
    try:
        return proc.wait(timeout=30)
    except subprocess.TimeoutExpired:
        log(f"[{label}] samply didn't exit after SIGINT -- force killing "
            "(this capture is lost)")
        subprocess.run(["sudo", "-n", "pkill", "-KILL", "-x", "samply"],
                       capture_output=True)
        return proc.wait()


def inspect_profile(path):
    try:
        raw = open(path, "rb").read()
    except FileNotFoundError:
        return None
    if raw[:2] == b"\x1f\x8b":
        raw = gzip.decompress(raw)
    # The probe path is split into #[inline(never)] frames so each shows up in
    # the call tree. `build_filter_bitset` (filter materialization) and
    # `scan_one_cluster` (per-cluster gate loop) were added alongside the
    # original rank_centroids / scan_clusters; seeing all four confirms the
    # split-debuginfo / local-tantivy patch resolved symbols.
    frames = {
        "rank_centroids":     b"rank_centroids" in raw,
        "scan_clusters":      b"scan_clusters" in raw,
        "scan_one_cluster":   b"scan_one_cluster" in raw,
        "build_filter_bitset": b"build_filter_bitset" in raw,
    }
    try:
        prof = json.loads(raw)
        samples = sum((t.get("samples") or {}).get("length", 0)
                      for t in prof.get("threads", []))
    except Exception:
        samples = -1
    return samples, frames


# ---------------------------------------------------------------- main

def main():
    ap = argparse.ArgumentParser(description="samply flamegraph sweep over (filter x probe cap)")
    ap.add_argument("--config", default=None,
                    help="JSON config file; CLI flags override it, it overrides defaults")
    ap.add_argument("--dataset", default="cohere", help="cohere (only supported dataset)")
    ap.add_argument("--size", type=int, default=100_000,
                    help="dataset size variant; derives the table name + default outdir")
    ap.add_argument("--dsn", default="postgresql://localhost:28818/pg_search")
    ap.add_argument("--table", default=None, help="override table (default <dataset>_<size>)")
    ap.add_argument("--filters", default="none,sel50,sel10,sel1",
                    help=f"comma-separated from {list(FILTER_PREDS)} (default: full matrix)")
    ap.add_argument("--probes", default="1.0,0.2,0.05",
                    help="comma-separated ABSOLUTE probe caps, profiled high->low so "
                         "the cache is hot (65536 clamps to 'all clusters')")
    ap.add_argument("--fanouts", default=None, help=argparse.SUPPRESS)  # removed
    ap.add_argument("--epsilon", default="7.0",
                    help="SPANN ratio coefficient (paper 0.6-7.0; new per-metric "
                         "semantics -- old tunings don't transfer)")
    ap.add_argument("--duration", type=int, default=30, help="seconds per capture")
    ap.add_argument("--k", type=int, default=10,
                    help="single top-K for the profiled query (scalar, not a grid)")
    ap.add_argument("--outdir", default=None,
                    help="capture directory (default flamegraphs_<dataset>_<size>)")
    ap.add_argument("--redo", action="store_true",
                    help="re-capture cells whose output file already exists (default: skip them)")
    ap.add_argument("--open", action="store_true")
    args = ap.parse_args()

    defaults = {"dataset": "cohere", "size": 100_000,
                "dsn": "postgresql://localhost:28818/pg_search", "table": None,
                "filters": "none,sel50,sel10,sel1", "probes": "65536,2048,256",
                "fanouts": None, "epsilon": "7.0", "duration": 30, "k": 10,
                "outdir": None, "redo": False, "open": False}
    if args.config:
        cfg_raw, applied = load_config_into(args, args.config, defaults)
        log(f"config {args.config}: applied {applied or '(nothing -- all overridden on CLI)'}")
        if "fanouts" in cfg_raw:
            die("config sets 'fanouts', which is REMOVED -- use \"probes\" "
                "(absolute caps; old fanout f ~= probes f*num_centroids)")
    if args.fanouts is not None:
        die("--fanouts is REMOVED (the fanout GUC no longer exists); use --probes "
            "with absolute caps (old fanout f ~= probes f*num_centroids)")

    ds = resolve_dataset(args.dataset, args.size, args.table)
    if args.outdir is None:
        args.outdir = f"flamegraphs_{ds.family}_{numerize(ds.size)}"
    log(f"dataset: {ds} outdir={args.outdir}")

    filters = parse_str_list(args.filters)
    for f in filters:
        if f not in FILTER_PREDS:
            die(f"unknown filter '{f}' (choose from {list(FILTER_PREDS)})")
    probes = parse_float_list(args.probes)
    epsilons = parse_float_list(args.epsilon)
    os.makedirs(args.outdir, exist_ok=True)

    n_cells = len(filters) * len(probes) * len(epsilons)
    est_min = n_cells * (args.duration + 8) / 60  # +warmup/teardown slack per cell
    log(f"grid: {len(filters)} filters x {len(probes)} probes x {len(epsilons)} eps "
        f"= {n_cells} cells, "
        f"~{est_min:.0f} min (already-captured cells are skipped; --redo to force)")

    if shutil.which("samply") is None:
        die("samply not on PATH -- cargo install samply")

    log("caching sudo credentials (samply attach + stop need root on macOS)")
    if subprocess.run(["sudo", "-v"]).returncode != 0:
        die("sudo authentication failed")

    # A full grid can outlive sudo's ~5min credential cache; a mid-run expiry
    # makes the next sudo block on a hidden password prompt, which looks like a
    # hang. Refresh the timestamp every 60s for the life of the script.
    def _sudo_keepalive():
        while True:
            time.sleep(60)
            subprocess.run(["sudo", "-n", "-v"], capture_output=True)
    threading.Thread(target=_sudo_keepalive, daemon=True).start()

    results = []
    with psycopg.connect(args.dsn, autocommit=True) as conn:
        check_build_mode(conn)

        conn.execute("SET max_parallel_workers_per_gather = 0")
        pid = conn.execute("SELECT pg_backend_pid()").fetchone()[0]
        log(f"attached session backend pid = {pid}")

        qvecs = fetch_query_vectors(conn, ds.table)

        for filt in filters:
            sql = (f"SELECT id FROM {ds.table} WHERE {FILTER_PREDS[filt]} "
                   f"ORDER BY embedding {ds.dist_op} %(q)s::vector LIMIT {args.k}")
            try:
                conn.execute(sql, {"q": qvecs[0]}).fetchall()
            except Exception as exc:
                die(f"[{filt}] predicate failed to execute: {exc}")
            verify_pushdown(conn, sql, qvecs[0], filt)

            for np_ in probes:
              for eps in epsilons:
                # eps suffix only when it isn't the pathology-valve default, so
                # existing single-eps capture files keep their names/skip logic.
                eps_tag = "" if eps == 7.0 else f"_eps{eps:g}"
                label = f"{filt}@{np_}{eps_tag}"
                out = os.path.join(args.outdir,
                                   f"{filt}_probes_{np_}{eps_tag}.json")

                if not args.redo:
                    prior = inspect_profile(out)
                    if prior is not None and prior[0] != 0:
                        samples, frames = prior
                        results.append((label, out, samples, frames, 0))
                        log(f"[{label}] exists with {samples} samples -- skipping "
                            "(--redo to re-capture)")
                        continue

                conn.execute(f"SET paradedb.vector_cluster_max_probe = {np_}")
                conn.execute(f"SET paradedb.vector_cluster_probe_epsilon = {eps}")

                lats = []
                for v in qvecs:
                    t0 = time.perf_counter()
                    conn.execute(sql, {"q": v}).fetchall()
                    lats.append((time.perf_counter() - t0) * 1000)
                log(f"[{label}] warm mean latency {statistics.mean(lats):.2f} ms")

                log(f"[{label}] recording {args.duration}s -> {out}")
                proc = subprocess.Popen(
                    ["sudo", "samply", "record", "--save-only",
                     "-o", out, "-p", str(pid)])
                time.sleep(1.5)

                n_queries, i = 0, 0
                t_end = time.time() + args.duration
                while time.time() < t_end:
                    conn.execute(sql, {"q": qvecs[i % len(qvecs)]}).fetchall()
                    i += 1
                    n_queries += 1

                rc = stop_samply(proc, label)
                info = inspect_profile(out)
                if info is None:
                    log(f"[{label}] NO OUTPUT FILE -- samply attach failed outright")
                    results.append((label, out, 0, {}, n_queries))
                    continue
                samples, frames = info
                results.append((label, out, samples, frames, n_queries))
                log(f"[{label}] {samples} samples, {n_queries} queries driven")

    # ---------------------------------------------------------- summary
    print()
    # one column per #[inline(never)] probe frame, abbreviated to keep the row
    # narrow: rank / scan / one (scan_one_cluster) / filt (build_filter_bitset)
    hdr = (f"{'cell':>12}{'samples':>9}{'queries':>8}"
           f"{'rank':>6}{'scan':>6}{'one':>6}{'filt':>6}")
    print(hdr)
    print("-" * len(hdr))
    captures_ok = True
    symbols_seen = False
    FRAME_ORDER = ["rank_centroids", "scan_clusters", "scan_one_cluster",
                   "build_filter_bitset"]
    for label, out, samples, frames, nq in results:
        cells = "".join(f"{('YES' if frames.get(fr) else '?'):>6}"
                        for fr in FRAME_ORDER)
        print(f"{label:>12}{samples:>9}{nq:>8}{cells}")
        captures_ok &= samples > 0
        symbols_seen |= all(frames.get(fr) for fr in FRAME_ORDER)

    print()
    if not captures_ok:
        log("empty capture(s): isolate samply with the built-in sampler while the "
            "loop runs:  sudo sample <pid> 10 -f /tmp/s.txt")
        return

    log("all captures have samples. Open them with:")
    for _, out, *_ in results:
        print(f"    samply load {out}")
    if not symbols_seen:
        log("'?' symbol columns are expected with --save-only (symbolication happens "
            "at load time). Verify in the viewer; only if the VIEWER lacks the frames "
            "check the local-tantivy patch / split-debuginfo.")
    log("reading guide: compare none@X vs sel1@X at the same probe cap. sel1 shows "
        "scan_clusters (and the scan_one_cluster children) stretched across many "
        "near-empty clusters -- the 4*K survivor floor probing deep -- with the "
        "filter BitSet checks inside scan_one_cluster. build_filter_bitset is its "
        "own frame now: under sel1 the filter materialization itself can be a "
        "visible slice. rank_centroids is the flat-scan navigation floor, ~constant "
        "across filters at a given centroid count.")

    if args.open:
        for _, out, *_ in results:
            subprocess.run(["samply", "load", out])


if __name__ == "__main__":
    main()