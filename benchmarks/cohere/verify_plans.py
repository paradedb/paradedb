"""Check the parameterized k6 query plans before measuring latency."""

import argparse
import json
import os
import subprocess
from pathlib import Path


def verify(plan, filtered, neighbors=None):
    """Require ParadeDB execution and index-level text filtering."""
    nodes = [plan[0]["Plan"]]
    scans = []
    while nodes:
        node = nodes.pop()
        if node.get("Filter"):
            raise ValueError("Filter runs outside the ParadeDB index")
        if "ParadeDB" in node.get("Custom Plan Provider", ""):
            scans.append({key.strip(): value for key, value in node.items()})
        nodes.extend(node.get("Plans", []))
    if not scans or (
        filtered
        and not any("text" in str(scan.get("Tantivy Query", "")) for scan in scans)
    ):
        raise ValueError("Expected a ParadeDB scan with the text predicate pushed down")
    if neighbors is not None and not any(
        scan.get("TopK Limit") == neighbors
        and "<=> vector" in str(scan.get("TopK Order By", ""))
        for scan in scans
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
        verify(json.loads(result.stdout), point["filtered"], point["neighbors"])
        print(f"Verified {point['name']}: {path}")
