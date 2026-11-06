#!/usr/bin/env python3
"""Turns audit.json, perf.json and online.json under --out into REPORT.md, and fails when the
online recall strays from the offline coverage recall by more than TOLERANCE."""
import argparse
import json
import os

TOLERANCE = 0.01
TARGET_NAMES = {0.9: "r90", 0.95: "r95", 0.99: "r99"}


def number(value, digits=1):
    if value is None:
        return "-"
    if isinstance(value, float):
        return f"{value:,.{digits}f}"
    return f"{value:,}"


def table(headers, rows):
    lines = ["| " + " | ".join(headers) + " |", "|" + "---|" * len(headers)]
    lines += ["| " + " | ".join(str(cell) for cell in row) + " |" for row in rows]
    return lines + [""]


def per_level(values):
    return " / ".join(number(value) for value in values) if values else "-"


def name_of(ranking):
    return f"{ranking['name']} aps {'on' if ranking['aps'] else 'off'}"


def median(values):
    """The upper median of the values present, or None."""
    present = sorted(value for value in values if value is not None)
    return present[len(present) // 2] if present else None


def spread_text(spread):
    """`median [min, max]`, with `-` for a bound no build reached."""
    return f"{number(spread['median'])} [{number(spread['min'])}, {number(spread['max'])}]"


def dips(rows):
    return sum(len(row["non_monotone"]) for row in rows)


def flat_section(report):
    lines = ["## A1. Flat ranking", ""]
    for query_set, flat in report["flat"].items():
        lines += [f"### {query_set} ({flat['queries']} queries)", ""]
        lines += table(
            ["target", "m_r", "recall", "rows_r"],
            [
                [
                    TARGET_NAMES[row["target"]],
                    number(row["m_r"]),
                    number(row["recall"], 4),
                    number(row["rows_r"], 0),
                ]
                for row in flat["targets"]
            ],
        )
        lines += table(
            ["need", "p50", "p95", "p99"],
            [
                [name, flat[key]["p50"], flat[key]["p95"], flat[key]["p99"]]
                for name, key in (("9/10", "need_9"), ("10/10", "need_10"))
            ],
        )
    return lines


def stored_row(ranking, row):
    attribution = row["attribution"] or {}
    work = row["work"] or {}
    below, above = row["below"], row["above"]
    return [
        name_of(ranking),
        TARGET_NAMES[row["target"]],
        number(row["m_r"]) if row["m_r"] else f"not reached ({row['best_recall']:.4f})",
        row["flat_m_r"],
        number(row["ratio"], 3),
        f"{below['k']}: {below['recall']:.4f}" if below else "-",
        f"{above['k']}: {above['recall']:.4f}" if above else "-",
        len(row["non_monotone"]),
        f"{attribution.get('unopened', 0):.2f}",
        f"{attribution.get('dropped', 0):.2f}",
        f"{attribution.get('late', 0):.2f}",
        number(attribution.get("missed"), 3),
        number(attribution.get("gained"), 3),
        per_level(work.get("lists_per_level")),
        number(work.get("one_bit_scored"), 0),
        number(work.get("exact_scored"), 0),
        number(work.get("bytes", 0) / 1024, 0),
        number(work.get("reads"), 1),
        number(work.get("units_r"), 1),
    ]


def rebuilt_row(ranking, query_set, index, spread):
    rows = [build["targets"][query_set][index] for build in ranking["builds"]]

    def attribution(key):
        return median((row["attribution"] or {}).get(key) for row in rows)

    def work(key):
        return median((row["work"] or {}).get(key) for row in rows)

    ratio = spread["median"] / spread["flat_m_r"] if spread["median"] else None
    return [
        name_of(ranking),
        TARGET_NAMES[spread["target"]],
        spread_text(spread),
        spread["flat_m_r"],
        number(ratio, 3),
        dips(rows),
        f"{attribution('unopened') or 0:.2f}",
        f"{attribution('dropped') or 0:.2f}",
        f"{attribution('late') or 0:.2f}",
        number(attribution("missed"), 3),
        number(attribution("gained"), 3),
        number(work("one_bit_scored"), 0),
        number(work("exact_scored"), 0),
        number((work("bytes") or 0) / 1024, 0),
        number(work("reads"), 1),
        number(work("units_r"), 1),
    ]


def ranking_section(report):
    lines = ["## A2. Routers against the flat ranking", ""]
    for query_set in report["flat"]:
        lines += [f"### Stored router, {query_set}", ""]
        rows = [
            stored_row(ranking, row)
            for ranking in report["rankings"]
            for row in ranking["targets"][query_set]
        ]
        lines += table(
            [
                "router",
                "target",
                "m_r",
                "flat",
                "ratio",
                "grid below",
                "grid above",
                "dips",
                "unopened",
                "dropped",
                "late",
                "missed/q",
                "gained/q",
                "lists per level",
                "one-bit",
                "exact",
                "KB",
                "reads",
                "units_r",
            ],
            rows,
        )
        lines += [f"### Rebuilt routers, {query_set}: medians over builds", ""]
        rows = [
            rebuilt_row(ranking, query_set, index, spread)
            for ranking in report["rebuilt"]
            for index, spread in enumerate(ranking["spread"][query_set])
        ]
        lines += table(
            [
                "router",
                "target",
                "m_r median [min, max]",
                "flat",
                "median ratio",
                "dips",
                "unopened",
                "dropped",
                "late",
                "missed/q",
                "gained/q",
                "one-bit",
                "exact",
                "KB",
                "reads",
                "units_r",
            ],
            rows,
        )
    return lines


def perf_section(perf, online):
    lines = ["## A3. Cost against posting (online)", ""]
    offline = {row["label"]: row["offline_recall"] for row in online}
    rows = []
    for point in perf:
        stages = point["stages"]
        rows.append(
            [
                point["label"],
                point["max_probe"],
                number(point["p50_ms"], 2),
                number(point["p99_ms"], 2),
                number(point["clusters_mean"]),
                point["clusters_p95"],
                number(stages["routing"]["ns"] / 1e6, 3),
                number(stages["routing_exact"]["ns"] / 1e6, 3),
                number(stages["layer0_scan"]["ns"] / 1e6, 3),
                number(stages["exact_phase"]["ns"] / 1e6, 3),
                number(point["routing_share"], 3),
                number(point["router_bytes_over_layer0"], 3),
                f"{point['recall']:.4f}",
                f"{offline[point['label']]:.4f}",
            ]
        )
    lines += table(
        [
            "point",
            "max_probe",
            "p50 ms",
            "p99 ms",
            "clusters",
            "p95",
            "routing ms",
            "routing_exact ms",
            "layer0 ms",
            "exact ms",
            "routing share",
            "router/layer0 bytes",
            "online recall",
            "offline recall",
        ],
        rows,
    )
    rows = []
    for point in perf:
        for stage, cost in point["stages"].items():
            rows.append(
                [
                    point["label"],
                    stage,
                    number(cost["ns"], 0),
                    number(cost["bytes_read"], 0),
                    number(cost["reads"], 1),
                ]
            )
    lines += table(["point", "stage", "ns", "bytes_read", "reads"], rows)
    lines += table(
        [
            "point",
            "router ns/one-bit",
            "layer0 ns/row",
            "router ns/exact",
            "exact ns/row",
        ],
        [
            [
                point["label"],
                number(point["router_ns_per_one_bit"], 2),
                number(point["layer0_ns_per_row"], 2),
                number(point["router_ns_per_exact"], 2),
                number(point["exact_ns_per_row"], 2),
            ]
            for point in perf
        ],
    )
    return lines


def phase_b_section(report):
    phase_b = report.get("phase_b")
    if not phase_b:
        return []
    lines = ["## B. Structures and list-open rules", ""]
    lines += [f"S(2, 0.25, on) reproduces K1 on every build: {phase_b['s_reproduces_k1']}", ""]
    for structure in phase_b["structures"]:
        lines += [f"### Structure {structure['name']}", ""]
        lines += table(
            ["seed", "top", "level", "lists", "members", "min", "p50", "p99", "max"],
            [
                [
                    seed,
                    build["top"],
                    level,
                    shape["lists"],
                    shape["members"],
                    shape["min"],
                    shape["p50"],
                    shape["p99"],
                    shape["max"],
                ]
                for seed, build in enumerate(structure["builds"])
                for level, shape in enumerate(build["levels"])
            ],
        )
    for query_set in report["flat"]:
        lines += [f"### Rules, {query_set}: medians over builds; ~ marks a range across the line", ""]
        rows = []
        for rule in phase_b["rules"]:
            ranking = rule["ranking"]
            for index, spread in enumerate(ranking["spread"][query_set]):
                builds = [build["targets"][query_set][index] for build in ranking["builds"]]

                def work(key):
                    return median((row["work"] or {}).get(key) for row in builds)

                ratio = spread["median"] / spread["flat_m_r"] if spread["median"] else None
                levels = [row["work"]["lists_per_level"] for row in builds if row["work"]]
                scored = [row["work"]["scored_per_level"] for row in builds if row["work"]]
                rows.append(
                    [
                        rule["structure"],
                        ranking["overscan"],
                        ranking["min_fraction"],
                        "on" if ranking["aps"] else "off",
                        TARGET_NAMES[spread["target"]],
                        spread_text(spread) + (" ~" if spread["straddles"] else ""),
                        number(ratio, 3),
                        dips(builds),
                        per_level([median(v) for v in zip(*levels)]),
                        per_level([median(v) for v in zip(*scored)]),
                        number(spread["median_scored"], 0),
                        number(work("units_at_layer0"), 1),
                    ]
                )
        lines += table(
            [
                "structure",
                "c",
                "f",
                "patience",
                "target",
                "m_r median [min, max]",
                "median ratio",
                "dips",
                "lists per level",
                "scored per level",
                "scored",
                "units_r",
            ],
            rows,
        )

    def describe(index):
        return phase_b["rules"][index]["ranking"]["name"]

    rows = [
        [
            gate["structure"],
            gate["query_set"],
            describe(gate["pick"]) if gate["pick"] is not None else "none",
            "-" if gate["pick"] is not None or gate["closest"] is None else describe(gate["closest"]),
            number(gate["closest_ratio"], 3),
            ", ".join(describe(index) for index in gate["straddling"]) or "-",
        ]
        for gate in phase_b["gates"]
    ]
    lines += ["### Gate: cheapest rule with median m_r within 1.02 of flat at every target", ""]
    lines += table(["structure", "set", "pick", "closest", "worst median ratio", "straddling"], rows)
    return lines


def rebuild_section(report):
    lines = ["## Rebuild check: stored K3 among K3r's builds", ""]
    lines += table(
        [
            "factor",
            "aps",
            "set",
            "target",
            "stored",
            "K3r median",
            "p95",
            "min",
            "max",
            "percentile",
            "above p95",
        ],
        [
            [
                check["rerank_factor"],
                "on" if check["aps"] else "off",
                check["query_set"],
                TARGET_NAMES[check["target"]],
                check["stored"],
                check["median"],
                check["p95"],
                check["min"],
                check["max"],
                f"{check['percentile']:.3f}",
                check["above_p95"],
            ]
            for check in report["rebuild_check"]
        ],
    )
    return lines


def monotone_section(report):
    lines = ["## Non-monotone steps around m_r", ""]
    rows = []
    for ranking in report["rankings"]:
        for query_set, targets in ranking["targets"].items():
            for row in targets:
                for step in row["non_monotone"]:
                    rows.append(
                        [
                            name_of(ranking),
                            query_set,
                            TARGET_NAMES[row["target"]],
                            step["k"],
                            f"{step['recall']:.4f}",
                            f"{step['next_recall']:.4f}",
                        ]
                    )
    lines += ["Stored router:", ""]
    lines += table(["router", "set", "target", "k", "recall(k)", "recall(k+1)"], rows)

    def count(reports):
        total = 0
        for ranking in reports:
            for build in ranking["builds"]:
                total += sum(dips(rows) for rows in build["targets"].values())
        return total

    phase_b = report.get("phase_b") or {"rules": []}
    lines += [
        f"Rebuilt routers: {count(report['rebuilt'])} steps. "
        f"Rule sweep: {count(rule['ranking'] for rule in phase_b['rules'])} steps.",
        "",
    ]
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

    lines = ["# Router audit", ""]
    lines += [
        f"K = {report['clusters']:,}, {report['rows']:,} rows, "
        f"{report['mean_cluster_rows']:.1f} rows per cluster; segment {meta['segment_id']}; "
        f"router {meta['router_kind']}, quantization {meta['quantization']}, spill {meta['spill']}, "
        f"max_leaf_size {meta['max_leaf_size']}, training_sample_ratio "
        f"{meta['training_sample_ratio']}.",
        "",
        f"Held-out vectors sha256 {queries['held_out_sha256']}.",
        "",
    ]
    log = f"{args.out}/ivf_build.log"
    if os.path.exists(log):
        with open(log) as handle:
            lines += ["```", handle.read().strip(), "```", ""]
    lines += table(
        ["level", "lists", "members", "min", "p50", "p99", "max"],
        [
            [
                level,
                shape["lists"],
                shape["members"],
                shape["min"],
                shape["p50"],
                shape["p99"],
                shape["max"],
            ]
            for level, shape in enumerate(report["stored_router"])
        ],
    )
    lines += flat_section(report)
    lines += ranking_section(report)
    lines += perf_section(perf, online)
    lines += phase_b_section(report)
    lines += rebuild_section(report)
    lines += monotone_section(report)
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
