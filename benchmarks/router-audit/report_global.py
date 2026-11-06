#!/usr/bin/env python3
"""Turns a global audit's audit.json, perf.json and online.json under --out into REPORT.md, and
fails when the online recall strays from the offline coverage recall by more than TOLERANCE."""
import argparse
import json
import os

from report import TARGET_NAMES, TOLERANCE, number, per_level, perf_section, table

# The single-segment 10M run this layout is read against (run 42247953555).
SINGLE_SEGMENT = {
    "FLAT": {0.9: (65, 26370), 0.95: (177, 71371), 0.99: (687, 273756)},
    "K3-4": {0.9: 65, 0.95: 175, 0.99: 777},
}


def segment_section(report):
    lines = ["## Segments", ""]
    lines += table(
        ["segment", "rows", "K", "rows per cluster min / p50 / max", "partition norm", "router lists"],
        [
            [
                segment["segment_id"],
                number(segment["rows"]),
                number(segment["clusters"]),
                f"{segment['min']} / {segment['p50']} / {segment['max']}",
                number(segment["partition_norm"], 4),
                " + ".join(str(level["lists"]) for level in segment["router"]),
            ]
            for segment in report["segments"]
        ],
    )
    return lines


def flat_section(report):
    lines = ["## A1. FLAT-G against the single-segment flat ranking", ""]
    for query_set, flat in report["flat"].items():
        rows = []
        for row in flat["targets"]:
            clusters, single_rows = SINGLE_SEGMENT["FLAT"][row["target"]]
            rows.append(
                [
                    TARGET_NAMES[row["target"]],
                    number(row["m_r"]),
                    clusters,
                    number(row["m_r"] / clusters, 3),
                    number(row["rows_r"], 0),
                    number(single_rows),
                    number(row["rows_r"] / single_rows, 3),
                ]
            )
        lines += [f"### {query_set} ({flat['queries']} queries)", ""]
        lines += table(
            ["target", "FLAT-G m_r", "single", "ratio", "FLAT-G rows_r", "single", "ratio"], rows
        )
        lines += table(
            ["need", "p50", "p95", "p99"],
            [
                [name, flat[key]["p50"], flat[key]["p95"], flat[key]["p99"]]
                for name, key in (("9/10", "need_9"), ("10/10", "need_10"))
            ],
        )
    return lines


def ranking_section(report):
    lines = ["## A2. Rankings against FLAT-G", ""]
    for query_set in report["flat"]:
        lines += [f"### {query_set}", ""]
        rows = []
        for ranking in report["rankings"]:
            for row in ranking["targets"][query_set]:
                attribution = row["attribution"]
                work = row["work"] or {}
                segments = row["segments"] or {}
                below, above = row["below"], row["above"]
                rows.append(
                    [
                        f"{ranking['name']} aps {'on' if ranking['aps'] else 'off'}",
                        TARGET_NAMES[row["target"]],
                        number(row["m_r"]) if row["m_r"] else f"not reached ({row['best_recall']:.4f})",
                        row["flat_m_r"],
                        number(row["ratio"], 3),
                        f"{below['k']}: {below['recall']:.4f}" if below else "-",
                        f"{above['k']}: {above['recall']:.4f}" if above else "-",
                        len(row["non_monotone"]),
                        f"{attribution['partition']:.2f}",
                        f"{attribution['unopened']:.2f}",
                        f"{attribution['dropped']:.2f}",
                        f"{attribution['late']:.2f}",
                        number(attribution["missed"], 3),
                        number(attribution["gained"], 3),
                        number(segments.get("opened")),
                        number(segments.get("holding_gt")),
                        per_level(work.get("lists_per_level")),
                        number(work.get("rows_r"), 0),
                        number(work.get("one_bit_scored"), 0),
                        number(work.get("exact_scored"), 0),
                        number(work.get("bytes", 0) / 1024, 0),
                        number(work.get("reads"), 1),
                        number(work.get("units_r"), 1),
                    ]
                )
        lines += table(
            [
                "ranking",
                "target",
                "m_r",
                "flat",
                "ratio",
                "grid below",
                "grid above",
                "dips",
                "partition",
                "unopened",
                "dropped",
                "late",
                "missed/q",
                "gained/q",
                "segments opened",
                "segments with GT",
                "lists per level",
                "rows_r",
                "one-bit",
                "exact",
                "KB",
                "reads",
                "units_r",
            ],
            rows,
        )
    return lines


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    with open(f"{args.out}/audit.json") as handle:
        document = json.load(handle)
    report, meta = document["report"], document["meta"]
    with open(f"{args.out}/perf.json") as handle:
        perf = json.load(handle)
    with open(f"{args.out}/online.json") as handle:
        online = json.load(handle)
    with open(f"{args.out}/queries.meta.json") as handle:
        queries = json.load(handle)

    lines = ["# Global router audit", ""]
    lines += [
        f"{len(report['segments'])} segments, K = {report['clusters']:,}, {report['rows']:,} rows, "
        f"{report['mean_cluster_rows']:.1f} rows per cluster; router {meta['router_kind']}, "
        f"quantization {meta['quantization']}, spill {meta['spill']}, partition_by "
        f"{meta['partition_by']}.",
        "",
        f"Held-out vectors sha256 {queries['held_out_sha256']}.",
        "",
        f"FLAT-S equals FLAT-G on every query: {report['flat_s_matches_flat_g']}.",
        "",
    ]
    log = f"{args.out}/ivf_build.log"
    if os.path.exists(log):
        with open(log) as handle:
            lines += ["```", handle.read().strip(), "```", ""]
    lines += segment_section(report)
    lines += flat_section(report)
    lines += ranking_section(report)
    lines += perf_section(perf, online)
    with open(f"{args.out}/REPORT.md", "w") as handle:
        handle.write("\n".join(lines))

    offline = {row["label"]: row["offline_recall"] for row in online}
    strayed = [
        f"{point['label']}: online {point['recall']:.4f}, offline {offline[point['label']]:.4f}"
        for point in perf
        if abs(point["recall"] - offline[point["label"]]) > TOLERANCE
    ]
    if strayed:
        raise SystemExit("online recall strays from coverage recall:\n" + "\n".join(strayed))


if __name__ == "__main__":
    main()
