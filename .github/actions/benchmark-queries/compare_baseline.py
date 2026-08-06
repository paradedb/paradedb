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
which is why the baseline is the second-to-last entry).
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
QUANTILES = {"p50": 0.50, "p95": 0.95}
# Charted but never alerted: at ~100 query vectors a p99 estimate rests on the top one or two
# order statistics, which no comparison rule can turn into reliable evidence of a regression.
NEVER_ALERT = {"p99"}
EFFECT_FLOOR = 1.15
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


def paired_ratio_stats(base, cand):
    """Per label, the (median, one-sided lower 95% bound) of the paired percentile ratio.

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
        label: (
            sorted(ratios)[len(ratios) // 2],
            sorted(ratios)[int((1 - CONFIDENCE) * len(ratios))],
        )
        for label, ratios in stats.items()
        if ratios
    }


def load_baseline(data_js_path, suite):
    """Return the newest baseline entry: the one before the just-appended current run.

    The publish step always appends the current run to its local gh-pages clone before
    this script runs, so the last entry is the current run itself, never the baseline.
    Commit ids can't disambiguate instead: a labeled PR run and a dispatch baseline of
    the same branch both record the branch head sha.
    """
    with open(data_js_path, encoding="utf-8") as f:
        raw = f.read()
    data = json.loads(raw[raw.index("=") + 1 :])
    entries = data.get("entries", {}).get(suite, [])
    return entries[-2] if len(entries) >= 2 else None


def pairable(cur_groups, base_groups, group):
    """True when both sides published samples for the same vector set."""
    cur, base = cur_groups.get(group), base_groups.get(group)
    return (
        cur is not None
        and base is not None
        and cur[0] == base[0]
        and len(cur[1]) == len(base[1])
    )


def judge(bench, base, cur_groups, base_groups, stats_cache):
    """Return (regressed, rule, ratio_display) for one series under the strategy ladder.

    ratio_display is the statistic the decision was made on, so tables and logs stay
    consistent with the alert: the bootstrapped ratio for paired series, the raw point
    ratio otherwise.
    """
    ratio = bench["value"] / base["value"]
    group, sep, label = bench["name"].rpartition(" - ")
    if sep and label in NEVER_ALERT:
        return False, f"{label} charted but not alerted", f"{ratio:.2f}x"
    if sep and label in QUANTILES and pairable(cur_groups, base_groups, group):
        if group not in stats_cache:
            stats_cache[group] = paired_ratio_stats(
                base_groups[group][1], cur_groups[group][1]
            )
        stats = stats_cache[group].get(label)
        if stats is not None:
            median, bound = stats
            return (
                bound > EFFECT_FLOOR,
                "paired bootstrap",
                f"{median:.2f}x (>={bound:.2f}x at 95%)",
            )
    cur_ci, base_ci = parse_ci(bench.get("range")), parse_ci(base.get("range"))
    if cur_ci and base_ci:
        return (
            cur_ci[0] > base_ci[1] and ratio > EFFECT_FLOOR,
            "CIs disjoint",
            f"{ratio:.2f}x",
        )
    return ratio > FALLBACK_RATIO, "point ratio", f"{ratio:.2f}x"


def write_alert_report(path, suite, baseline, alerts):
    """Write the alert comment body as a markdown table."""
    lines = [
        f"## :warning: Possible performance regression vs `{baseline['commit']['id'][:7]}`",
        "",
        (
            "Percentile series with paired per-query samples alert when the bootstrapped "
            "one-sided lower 95% bound of the paired percentile ratio exceeds "
            f"{EFFECT_FLOOR:.2f}x. Without samples, a series alerts when its 95% CI sits "
            f"entirely above the baseline's and the value regressed >{EFFECT_FLOOR:.2f}x; "
            f"without CIs, when the value regressed >{FALLBACK_RATIO:.2f}x."
        ),
        "",
        f"### {suite}",
        "",
        "| Benchmark | Baseline | Current | Ratio | Rule |",
        "|-|-|-|-|-|",
    ]
    for bench, base, ratio_display, rule in alerts:
        lines.append(
            f"| `{bench['name']}` "
            f"| {base['value']:.3f} {base['unit']} {base.get('range', '')} "
            f"| {bench['value']:.3f} {bench['unit']} {bench.get('range', '')} "
            f"| {ratio_display} | {rule} |"
        )
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def collect_alerts(current, baseline):
    """Judge every current series against the baseline; return the regressed ones."""
    baseline_by_name = {b["name"]: b for b in baseline["benches"]}
    cur_groups = samples_by_group(current)
    base_groups = samples_by_group(baseline["benches"])
    stats_cache = {}
    alerts = []
    for bench in current:
        base = baseline_by_name.get(bench["name"])
        if base is None or base["value"] <= 0:
            continue
        regressed, rule, ratio_display = judge(
            bench, base, cur_groups, base_groups, stats_cache
        )
        if regressed:
            alerts.append((bench, base, ratio_display, rule))
        print(
            f"{'ALERT' if regressed else 'ok':5} {bench['name']}: "
            f"{base['value']:.3f} -> {bench['value']:.3f} {bench['unit']} "
            f"({ratio_display}, {rule})"
        )
    return alerts


def main():
    """Compare the current results against the baseline and report alerts."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--results", required=True)
    parser.add_argument("--data-js", required=True)
    parser.add_argument("--suite", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    with open(args.results, encoding="utf-8") as f:
        current = json.load(f)

    baseline = load_baseline(args.data_js, args.suite)
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
