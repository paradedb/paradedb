"""Capture prepared-plan choices for the Wikipedia OR workload."""

import collections
import json
import pathlib
import statistics
import subprocess
import sys

queries = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))["queries"]
prefix = pathlib.Path(sys.argv[2])
METADATA_SQL = """
SELECT json_build_object(
  'settings', (SELECT json_object_agg(name, setting) FROM pg_settings),
  'index', (SELECT json_agg(t) FROM paradedb.index_info('documents_body_bm25_idx') t),
  'table', (SELECT row_to_json(t) FROM (SELECT reltuples, relpages FROM pg_class WHERE oid='documents'::regclass) t));
"""
command = [
    "docker",
    "exec",
    "-i",
    "paradedb",
    "psql",
    "-X",
    "-qAt",
    "-U",
    "postgres",
    "-d",
    "benchmark",
    "-v",
    "ON_ERROR_STOP=1",
]
metadata = subprocess.run(
    command, input=METADATA_SQL, text=True, capture_output=True, check=True
)
prefix.with_suffix(".metadata.json").write_text(metadata.stdout, encoding="utf-8")
for mode in ["auto", "force_custom_plan", "force_generic_plan"]:
    sql = (
        f"BEGIN READ ONLY; SET LOCAL plan_cache_mode={mode}; "
        "PREPARE bench(text) AS SELECT id, body, pdb.score(id) AS score "
        "FROM documents WHERE body ||| $1 ORDER BY score DESC LIMIT 10;\n"
    )
    requests = queries * (3 if mode == "auto" else 1)
    for query in requests:
        term = query["engines"]["paradedb"]["match"].replace("'", "''")
        sql += f"EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) EXECUTE bench('{term}');\n"
        sql += (
            "SELECT json_build_object('generic',generic_plans,'custom',custom_plans) "
            "FROM pg_prepared_statements WHERE name='bench';\n"
        )
    sql += "ROLLBACK;"
    result = subprocess.run(
        command, input=sql, text=True, capture_output=True, check=True
    )
    remaining = result.stdout.strip()
    decoder = json.JSONDecoder()
    items = []
    while remaining:
        value, end = decoder.raw_decode(remaining)
        items.append(value)
        remaining = remaining[end:].lstrip()
    rows = []
    for i, query in enumerate(requests):
        plan = items[2 * i][0]
        nodes = [plan["Plan"]]
        workers = 0
        while nodes:
            node = nodes.pop()
            workers += node.get("Workers Launched", 0)
            nodes.extend(node.get("Plans", []))
        rows.append(
            {
                "i": i,
                "query": query["text"],
                "counters": items[2 * i + 1],
                "workers": workers,
                "plan": plan,
            }
        )
    prefix.with_name(prefix.name + f"-{mode}-plans.json").write_text(
        json.dumps(rows, indent=2) + "\n", encoding="utf-8"
    )
    first_generic = next((row["i"] for row in rows if row["counters"]["generic"]), None)
    print(
        json.dumps(
            {
                "phase": prefix.name,
                "mode": mode,
                "first_generic": first_generic,
                "final_counters": rows[-1]["counters"],
                "workers": dict(collections.Counter(row["workers"] for row in rows)),
                "execution_mean_ms": statistics.mean(
                    row["plan"]["Execution Time"] for row in rows
                ),
            }
        ),
        flush=True,
    )
