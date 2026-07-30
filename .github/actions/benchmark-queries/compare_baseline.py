#!/usr/bin/env python3
"""Alert on regressions using paired per-query statistics rather than point values.

Baseline and candidate runs measure the same held-out query vectors, so their samples
pair up index-by-index. For each operating point that published raw samples on both
sides (carried in the p50 entry's `extra`), query indices are bootstrapped and each
percentile series alerts when the one-sided lower 95% bound of the paired percentile
ratio exceeds a practical floor. Pairing cancels cross-query latency variance, which
dominates the width of the single-run percentile CIs but is irrelevant to whether the
code got slower.

Fallbacks, in order, for series where pairing isn't possible: disjoint 95% CIs plus
the same floor (percentile entries without baseline samples), then the plain point
ratio github-action-benchmark applies (build time/size, pre-CI baselines).

Reads the current run's results.json and the gh-pages data.js that the
github-action-benchmark step has already cloned (and appended the current run to,
which is why the baseline is the newest entry from a *different* commit).
"""

import argparse
import json
import math
import os
import random
import re
import sys

CI_RANGE = re.compile(r"95% CI \[([0-9.]+), ([0-9.]+)\]")
SAMPLES = re.compile(r"samples_fnv=([0-9a-f]{16}); samples=\[([0-9.,eE+-]*)\]")
QUANTILES = {"p50": 0.50, "p95": 0.95, "p99": 0.99}
EFFECT_FLOOR = 1.05
FALLBACK_RATIO = 1.15
BOOTSTRAP_RESAMPLES = 10_000
CONFIDENCE = 0.95


def parse_ci(range_str):
    """Extract (lo, hi) from a '95% CI [lo, hi]' range string, or None."""
    m = CI_RANGE.fullmatch(range_str or "")
    return (float(m.group(1)), float(m.group(2))) if m else None


def parse_samples(extra):
    """Extract (vector_hash, [latencies]) from an entry's extra, or None."""
    m = SAMPLES.search(extra or "")
    if not m:
        return None
    values = [float(v) for v in m.group(2).split(",") if v]
    return (m.group(1), values) if values else None


def samples_by_group(benches):
    """Map operating point -> (vector_hash, samples) from its p50 carrier entry."""
    groups = {}
    for bench in benches:
        group, sep, label = bench["name"].rpartition(" - ")
        if sep and label == "p50":
            parsed = parse_samples(bench.get("extra"))
            if parsed:
                groups[group] = parsed
    return groups


def percentile_of(sorted_values, q):
    """Linearly interpolated quantile, matching the Rust percentile()."""
    rank = q * (len(sorted_values) - 1)
    lo, hi = math.floor(rank), math.ceil(rank)
    if lo == hi:
        return sorted_values[lo]
    return sorted_values[lo] + (sorted_values[hi] - sorted_values[lo]) * (rank - lo)


def paired_lower_bounds(base, cand):
    """One-sided lower 95% bounds of the paired percentile ratios, per label.

    Bootstraps query indices: each resample recomputes both sides' percentile over
    the same indices, so the statistic is the ratio of paired percentile estimates.
    """
    n = len(base)
    rng = random.Random(0)
    stats = {label: [] for label in QUANTILES}
    for _ in range(BOOTSTRAP_RESAMPLES):
        ids = rng.choices(range(n), k=n)
        base_sorted = sorted(base[i] for i in ids)
        cand_sorted = sorted(cand[i] for i in ids)
        for label, q in QUANTILES.items():
            base_q = percentile_of(base_sorted, q)
            if base_q > 0:
                stats[label].append(percentile_of(cand_sorted, q) / base_q)
    return {
        label: sorted(ratios)[int((1 - CONFIDENCE) * len(ratios))]
        for label, ratios in stats.items()
        if ratios
    }


def load_baseline(data_js_path, suite, current_sha):
    """Return the suite's newest entry not authored by the current commit."""
    with open(data_js_path, encoding="utf-8") as f:
        raw = f.read()
    data = json.loads(raw[raw.index("=") + 1 :])
    entries = data.get("entries", {}).get(suite, [])
    for entry in reversed(entries):
        if entry["commit"]["id"] != current_sha:
            return entry
    return None


def pairable(cur_groups, base_groups, group):
    """True when both sides published samples for the same vector set."""
    cur, base = cur_groups.get(group), base_groups.get(group)
    return (
        cur is not None
        and base is not None
        and cur[0] == base[0]
        and len(cur[1]) == len(base[1])
    )


def judge(bench, base, cur_groups, base_groups, bounds_cache):
    """Return (regressed, rule) for one series under the strategy ladder."""
    ratio = bench["value"] / base["value"]
    group, sep, label = bench["name"].rpartition(" - ")
    if sep and label in QUANTILES and pairable(cur_groups, base_groups, group):
        if group not in bounds_cache:
            bounds_cache[group] = paired_lower_bounds(
                base_groups[group][1], cur_groups[group][1]
            )
        bound = bounds_cache[group].get(label)
        if bound is not None:
            return bound > EFFECT_FLOOR, f"paired bootstrap (lower bound {bound:.2f}x)"
    cur_ci, base_ci = parse_ci(bench.get("range")), parse_ci(base.get("range"))
    if cur_ci and base_ci:
        return cur_ci[0] > base_ci[1] and ratio > EFFECT_FLOOR, "CIs disjoint"
    return ratio > FALLBACK_RATIO, "point ratio"


def write_alert_report(path, suite, baseline, alerts):
    """Write the alert comment body as a markdown table."""
    lines = [
        f"## :warning: Possible performance regression vs `{baseline['commit']['id'][:7]}`",
        "",
        "Percentile series with paired per-query samples alert when the bootstrapped "
        "one-sided lower 95% bound of the paired percentile ratio exceeds "
        f"{EFFECT_FLOOR:.2f}x. Without samples, a series alerts when its 95% CI sits "
        f"entirely above the baseline's and the value regressed >{EFFECT_FLOOR:.2f}x; "
        f"without CIs, when the value regressed >{FALLBACK_RATIO:.2f}x.",
        "",
        f"### {suite}",
        "",
        "| Benchmark | Baseline | Current | Ratio | Rule |",
        "|-|-|-|-|-|",
    ]
    for bench, base, ratio, rule in alerts:
        lines.append(
            f"| `{bench['name']}` "
            f"| {base['value']:.3f} {base['unit']} {base.get('range', '')} "
            f"| {bench['value']:.3f} {bench['unit']} {bench.get('range', '')} "
            f"| {ratio:.2f}x | {rule} |"
        )
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def collect_alerts(current, baseline):
    """Judge every current series against the baseline; return the regressed ones."""
    baseline_by_name = {b["name"]: b for b in baseline["benches"]}
    cur_groups = samples_by_group(current)
    base_groups = samples_by_group(baseline["benches"])
    bounds_cache = {}
    alerts = []
    for bench in current:
        base = baseline_by_name.get(bench["name"])
        if base is None or base["value"] <= 0:
            continue
        regressed, rule = judge(bench, base, cur_groups, base_groups, bounds_cache)
        ratio = bench["value"] / base["value"]
        if regressed:
            alerts.append((bench, base, ratio, rule))
        print(
            f"{'ALERT' if regressed else 'ok':5} {bench['name']}: "
            f"{base['value']:.3f} -> {bench['value']:.3f} {bench['unit']} "
            f"({ratio:.2f}x, {rule})"
        )
    return alerts


def main():
    """Compare the current results against the baseline and report alerts."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", required=True)
    parser.add_argument("--data-js", required=True)
    parser.add_argument("--suite", required=True)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    with open(args.results, encoding="utf-8") as f:
        current = json.load(f)

    baseline = load_baseline(args.data_js, args.suite, args.sha)
    if baseline is None:
        print(f"No baseline entry for suite '{args.suite}'; nothing to compare.")
        return

    alerts = collect_alerts(current, baseline)
    if alerts:
        write_alert_report(args.out, args.suite, baseline, alerts)

    if github_output := os.environ.get("GITHUB_OUTPUT"):
        with open(github_output, "a", encoding="utf-8") as f:
            f.write(f"alert={'true' if alerts else 'false'}\n")
    print(f"\n{len(alerts)} alert(s) out of {len(current)} series.")


if __name__ == "__main__":
    sys.exit(main())
