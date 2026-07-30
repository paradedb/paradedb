#!/usr/bin/env python3
"""Alert on regressions by comparing confidence intervals rather than point values.

The `ci-overlap` alert strategy: a series alerts only when its current confidence
interval is entirely above the baseline's (the difference exceeds sampling noise)
AND the point value regressed by more than a practical floor. Series without a
parseable CI on both sides (index build time/size, pre-CI baselines) fall back to
the same point-ratio rule github-action-benchmark applies.

Reads the current run's results.json and the gh-pages data.js that the
github-action-benchmark step has already cloned (and appended the current run to,
which is why the baseline is the newest entry from a *different* commit).
"""

import argparse
import json
import os
import re
import sys

CI_RANGE = re.compile(r"95% CI \[([0-9.]+), ([0-9.]+)\]")
EFFECT_FLOOR = 1.05
FALLBACK_RATIO = 1.15


def parse_ci(range_str):
    """Extract (lo, hi) from a '95% CI [lo, hi]' range string, or None."""
    m = CI_RANGE.fullmatch(range_str or "")
    return (float(m.group(1)), float(m.group(2))) if m else None


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


def write_alert_report(path, suite, baseline, alerts):
    """Write the alert comment body as a markdown table."""
    lines = [
        f"## :warning: Possible performance regression vs `{baseline['commit']['id'][:7]}`",
        "",
        "A series alerts when its 95% CI sits entirely above the baseline's and the "
        f"value regressed >{(EFFECT_FLOOR - 1) * 100:.0f}% (or, without CIs, when the "
        f"value regressed >{(FALLBACK_RATIO - 1) * 100:.0f}%).",
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

    baseline_by_name = {b["name"]: b for b in baseline["benches"]}
    alerts = []
    for bench in current:
        base = baseline_by_name.get(bench["name"])
        if base is None or base["value"] <= 0:
            continue
        ratio = bench["value"] / base["value"]
        cur_ci, base_ci = parse_ci(bench.get("range")), parse_ci(base.get("range"))
        if cur_ci and base_ci:
            regressed = cur_ci[0] > base_ci[1] and ratio > EFFECT_FLOOR
            rule = "CIs disjoint"
        else:
            regressed = ratio > FALLBACK_RATIO
            rule = "point ratio"
        if regressed:
            alerts.append((bench, base, ratio, rule))
        print(
            f"{'ALERT' if regressed else 'ok':5} {bench['name']}: "
            f"{base['value']:.3f} -> {bench['value']:.3f} {bench['unit']} "
            f"({ratio:.2f}x, {rule})"
        )

    if alerts:
        write_alert_report(args.out, args.suite, baseline, alerts)

    if github_output := os.environ.get("GITHUB_OUTPUT"):
        with open(github_output, "a", encoding="utf-8") as f:
            f.write(f"alert={'true' if alerts else 'false'}\n")
    print(f"\n{len(alerts)} alert(s) out of {len(current)} series.")


if __name__ == "__main__":
    sys.exit(main())
