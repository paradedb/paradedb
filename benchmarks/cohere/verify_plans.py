"""Check the parameterized k6 query plans before measuring latency."""

import argparse
import json
import os
import subprocess
from pathlib import Path


def verify(plan, filtered, engine="pg_search", index_name=None):
    """Require the selected index and its engine's expected filtering path."""
    nodes = [plan[0]["Plan"]]
    scans = []
    if engine == "pg_vectorsearch":
        while nodes:
            node = nodes.pop()
            if node.get("Custom Plan Provider"):
                raise ValueError("Expected a PRISM Index Scan, not a custom scan")
            if (
                node.get("Node Type") == "Index Scan"
                and node.get("Index Name") == index_name
                and index_name
            ):
                scans.append(node)
            nodes.extend(node.get("Plans", []))
        if not scans:
            raise ValueError(f"Expected a PRISM Index Scan on {index_name}")
        if filtered and not any(
            "to_tsvector" in scan.get("Filter", "") and "@@" in scan.get("Filter", "")
            for scan in scans
        ):
            raise ValueError(
                "Expected PostgreSQL full-text filtering on the PRISM scan"
            )
        return
    if engine != "pg_search":
        raise ValueError(f"Unknown benchmark engine: {engine}")
    while nodes:
        node = nodes.pop()
        if node.get("Filter"):
            raise ValueError("Filter runs outside the ParadeDB index")
        if "ParadeDB" in node.get("Custom Plan Provider", ""):
            scans.append(node)
        nodes.extend(node.get("Plans", []))
    if not scans or (
        filtered
        and not any("text" in str(scan.get("Tantivy Query", "")) for scan in scans)
    ):
        raise ValueError("Expected a ParadeDB scan with the text predicate pushed down")


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
        session = point["session"] | {point["probe_param"]: point["probe"]}
        settings = "\n".join(
            f"SET {key} = '{value}';" for key, value in session.items()
        )
        sql = f"""{settings}
PREPARE vector_ci(vector(1024)) AS {point["sql"].rstrip(";")};
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
            point["query"] != "knn_top10_unfiltered",
            point["engine"],
            point["index_name"],
        )
        print(f"Verified {point['name']}: {path}")
