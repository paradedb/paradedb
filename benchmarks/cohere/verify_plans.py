"""Check the parameterized k6 query plans before measuring latency."""

import argparse
import json
import os
import subprocess
from pathlib import Path


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
        if isinstance(query, str):
            query = json.loads(query)
        pending = [query]
        indexed_fields = set()
        while pending:
            part = pending.pop()
            if isinstance(part, dict):
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


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    points = json.loads((args.directory / "operating_points.json").read_text())
    vector = json.loads((args.directory / "query_vectors.json").read_text())[0]
    output = args.directory / "plans"
    output.mkdir(exist_ok=True)
    for point in points:
        if not point["reached"]:
            continue
        session = point["session"] | {
            "paradedb.vector_cluster_max_probe": point["probe"]
        }
        settings = "\n".join(
            f"SET {key} = '{value}';" for key, value in session.items()
        )
        sql = f"""{settings}
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
        path = output / f"{point['name']}.json"
        path.write_text(result.stdout)
        verify(
            json.loads(result.stdout),
            not point["query"].endswith("_unfiltered"),
            point["neighbors"],
            point["filter_fields"],
        )
        print(f"Verified {point['name']}: {path}")
