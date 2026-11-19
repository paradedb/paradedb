"""Check the parameterized k6 query plans before measuring latency."""

import argparse
import json
import os
import re
import subprocess
from collections import Counter
from pathlib import Path

# A text predicate whose query string comes from a session setting stays an expression until
# execution. It is accepted only in this exact form, which names the field it searches.
SETTING_DRIVEN_FIELD = re.compile(
    r"paradedb\.with_index\('[^']+'::regclass, "
    r"paradedb\.parse_with_field\('(?P<field>[^']+)'::paradedb\.fieldname, "
    r"current_setting\('[^']+'::text\), NULL::boolean, NULL::boolean\)\)"
)


def setting_driven_field(expression, required):
    """The required field a `postgres_expression` searches, or None for any other expression."""
    match = SETTING_DRIVEN_FIELD.fullmatch(
        str(expression.get("expr", {}).get("expr_desc", ""))
    )
    return match["field"] if match and match["field"] in required else None


def verify_ann(plan, index_name, fields, access_method, prefilter=False):
    """Require a cosine ANN scan and every PostgreSQL filter predicate."""
    pending = [plan[0]["Plan"]]
    nodes = []
    while pending:
        node = pending.pop()
        nodes.append(node)
        pending.extend(node.get("Plans", []))
    if any(node.get("Custom Plan Provider") for node in nodes):
        raise ValueError("Expected a PostgreSQL index scan, found a custom scan")
    predicates = " ".join(
        str(node.get(key, ""))
        for node in nodes
        for key in ("Filter", "Index Cond", "Recheck Cond")
    )
    if any(not re.search(rf"\b{re.escape(field)}\b", predicates) for field in fields):
        raise ValueError("Expected every filter predicate in the PostgreSQL plan")
    scans = [
        node
        for node in nodes
        if node.get("Node Type") == "Index Scan"
        and node.get("Index Name") == index_name
        and "<=>" in node.get("Order By", "")
    ]
    if scans:
        if prefilter and access_method != "vchordrq":
            raise ValueError("Prefilter mode requires VectorChord")
        filter_mode = "prefilter" if prefilter else "postfilter"
        return f"{access_method}-{filter_mode}" if fields else access_method
    raise ValueError(f"Expected a cosine {access_method} scan on {index_name}")


def verify(plan, filtered, neighbors=None, fields=None):
    """Require cosine TopK and all scalar predicates inside the index."""
    required = set(fields if fields is not None else (["text"] if filtered else []))
    nodes = [plan[0]["Plan"]]
    scans = []
    while nodes:
        node = {key.strip(): value for key, value in nodes.pop().items()}
        if any(node.get(key) for key in ("Filter", "Index Cond", "Recheck Cond")):
            raise ValueError("Filter runs outside the ParadeDB index")
        if "ParadeDB" in node.get("Custom Plan Provider", ""):
            scans.append(node)
        nodes.extend(node.get("Plans", []))
    matching = []
    for scan in scans:
        query = scan.get("Tantivy Query", '"all"')
        pending = [json.loads(query) if isinstance(query, str) else query]
        indexed_fields = set()
        while pending:
            part = pending.pop()
            if isinstance(part, dict):
                field = (
                    setting_driven_field(part["postgres_expression"], required)
                    if isinstance(part.get("postgres_expression"), dict)
                    and len(part) == 1
                    else None
                )
                if field is not None:
                    indexed_fields.add(field)
                    continue
                if {"heap_filter", "postgres_expression"} & part.keys():
                    raise ValueError(
                        "Predicate uses heap filtering or an unresolved expression"
                    )
                indexed_fields.update(
                    part[key] for key in ("field", "field_name") if key in part
                )
                pending.extend(part.values())
            elif isinstance(part, list):
                pending.extend(part)
        if required <= indexed_fields:
            matching.append(scan)
    if not matching:
        raise ValueError(
            f"Expected a ParadeDB scan with every predicate pushed down: {sorted(required)}"
        )
    if neighbors is not None and not any(
        scan.get("TopK Limit") == neighbors
        and "<=> vector" in str(scan.get("TopK Order By", ""))
        for scan in matching
    ):
        raise ValueError(
            f"Expected cosine vector TopK with limit {neighbors} inside ParadeDB"
        )


def execution_path(plan):
    """Router path per segment, worker launch and vector filter stage of one executed
    pg_search plan. Needs `EXPLAIN (ANALYZE, VERBOSE)` with `paradedb.vector_stats` on; the
    leader's Segment Info already holds the segments its parallel workers scanned."""
    paths = Counter()
    planned = launched = 0
    filtering = None
    pending = [plan[0]["Plan"]]
    while pending:
        node = {key.strip(): value for key, value in pending.pop().items()}
        planned += node.get("Workers Planned", 0)
        launched += node.get("Workers Launched", 0)
        filtering = filtering or node.get("Vector Filtering")
        info = node.get("Segment Info")
        if info is not None:
            info = json.loads(info) if isinstance(info, str) else info
            paths.update(stats.get("access_path", "unknown") for stats in info.values())
        pending.extend(node.get("Plans", []))
    segments = sum(paths.values())
    return {
        "router": ", ".join(
            f"{path.lower()} {count}/{segments}"
            for path, count in sorted(paths.items())
        )
        or "—",
        "execution": f"parallel {launched}/{planned}" if planned else "serial",
        "filter": filtering or "—",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    points = json.loads((args.directory / "operating_points.json").read_text())
    vector = json.loads((args.directory / "query_vectors.json").read_text())[0]
    output = args.directory / "plans"
    output.mkdir(exist_ok=True)
    plan_modes = ["| Operating point | Execution path |", "|---|---|"]
    pg_paths = [
        "| Operating point | Router | Execution | Vector filter |",
        "|---|---|---|---|",
    ]
    for point in points:
        if not point["reached"]:
            continue
        session = point["session"] | {
            point.get("param", "paradedb.vector_cluster_max_probe"): point["probe"]
        }
        if point.get("engine", "pg_search") == "pg_search":
            session |= {"paradedb.vector_stats": "on"}
        settings = "\n".join(
            f"SET {key} = '{value}';" for key, value in session.items()
        )
        capture = (
            "SELECT json_build_object('vchordrq.prefilter', "
            "current_setting('vchordrq.prefilter'));"
            if point.get("engine") == "vectorchord"
            else ""
        )
        sql = f"""{settings}
{capture}
PREPARE vector_ci(vector({point["dimensions"]})) AS {point["sql"].rstrip(";")};
EXPLAIN (ANALYZE, VERBOSE, BUFFERS, FORMAT JSON) EXECUTE vector_ci('{vector}');
"""
        result = subprocess.run(
            [
                "psql",
                os.environ["PARADEDB_URL"],
                "-XqAt",
                "-v",
                "ON_ERROR_STOP=1",
            ],
            input=sql,
            capture_output=True,
            text=True,
            check=True,
        )
        plan_text = result.stdout
        prefilter_enabled = False
        if point.get("engine") == "vectorchord":
            settings_text, plan_text = plan_text.split("\n", 1)
            actual = json.loads(settings_text)
            expected = session.get("vchordrq.prefilter")
            if (
                expected not in ("on", "off")
                or actual.get("vchordrq.prefilter") != expected
            ):
                raise ValueError(
                    "VectorChord prefilter setting differs from the recall sweep"
                )
            (output / f"{point['name']}-settings.json").write_text(settings_text + "\n")
            prefilter_enabled = expected == "on"
        path = output / f"{point['name']}.json"
        path.write_text(plan_text)
        executed_plan = json.loads(plan_text)
        if point.get("engine") in ("pg_vectorsearch", "vectorchord"):
            mode = verify_ann(
                executed_plan,
                point["index_name"],
                point["filter_fields"],
                "prism" if point["engine"] == "pg_vectorsearch" else "vchordrq",
                prefilter=prefilter_enabled,
            )
            point["plan_mode"] = mode
            plan_modes.append(f"| `{point['name']}` | `{mode}` |")
            if point["filter_fields"]:
                point["name"] += f"-{mode}"
        else:
            verify(
                executed_plan,
                not point["query"].endswith("_unfiltered"),
                point["neighbors"],
                point["filter_fields"],
            )
            point["execution_path"] = execution_path(executed_plan)
            pg_paths.append(
                f"| `{point['name']}` | "
                + " | ".join(point["execution_path"].values())
                + " |"
            )
        print(f"Verified {point['name']}: {path}")
    (args.directory / "operating_points.json").write_text(
        json.dumps(points, indent=2) + "\n"
    )
    report = None
    if len(plan_modes) > 2:
        report = "\n" + "\n".join(plan_modes) + "\n"
    elif len(pg_paths) > 2:
        # GitHub renders Markdown inside <details> only with a blank line after the summary.
        report = "\n".join(
            [
                "<details><summary>Execution paths</summary>",
                "",
                *pg_paths,
                "",
                (
                    "One `EXPLAIN (ANALYZE, VERBOSE)` per operating point on the first query "
                    "vector, with `paradedb.vector_stats` on. Router counts segments that routed "
                    "or located their filter's matches (router bypass); execution is "
                    "launched/planned workers."
                ),
                "</details>",
                "",
            ]
        )
    if report:
        (args.directory / "plan-modes.md").write_text(report)
        for destination in (
            args.directory / "result-context.md",
            os.environ.get("GITHUB_STEP_SUMMARY"),
        ):
            if destination:
                with Path(destination).open("a", encoding="utf-8") as stream:
                    stream.write(report)
