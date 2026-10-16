"""Collect segment and buffer evidence from the failed Stressgres workload."""

import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path


def main():  # pylint: disable=too-many-locals
    """Run one writer-count variant and preserve diagnostic samples."""
    writers = int(sys.argv[1])
    runtime_ms = int(sys.argv[2]) if len(sys.argv) > 2 else 600000
    output = Path("diagnostics") / str(writers)
    output.mkdir(parents=True, exist_ok=True)
    suite = Path("stressgres/suites/bulk-update-merge-pressure.toml").read_text(
        encoding="utf-8"
    )
    suite = suite.replace(
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;",
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;\n"
        "CREATE EXTENSION IF NOT EXISTS pg_buffercache;",
    )
    suite = suite.replace(
        "    RETURN merged_away;",
        "    RAISE LOG 'SEGMENT_SAMPLE immutable=% previous_60s=% merged_away_60s=%',"
        " segments_now, segments_then, merged_away;\n    RETURN merged_away;",
    )
    parts = re.split(r"(?=^\[\[jobs\]\])", suite, flags=re.MULTILINE)
    suite = "".join(
        part
        for part in parts
        if not (match := re.search(r'^title = "Upsert (\d+)"', part, re.MULTILINE))
        or int(match[1]) <= writers
    )
    suite_path = output / "suite.toml"
    suite_path.write_text(suite)
    sql = Path(".github/scripts/diagnose-stressgres.sql").read_text(encoding="utf-8")
    pin_sql = """
    WITH buffers AS MATERIALIZED (SELECT * FROM pg_buffercache)
    SELECT json_build_object(
        'sampled_at', clock_timestamp(),
        'total_buffers', (SELECT count(*) FROM buffers),
        'pinned_buffers', (SELECT count(*) FROM buffers WHERE pinning_backends > 0),
        'pinning_backends_sum', (SELECT sum(pinning_backends) FROM buffers),
        'relations', (SELECT json_agg(r) FROM (
            SELECT relfilenode, reldatabase, count(*) AS cached,
                   count(*) FILTER (WHERE pinning_backends > 0) AS pinned
            FROM buffers GROUP BY relfilenode, reldatabase
            HAVING count(*) FILTER (WHERE pinning_backends > 0) > 0
        ) r)
    );
    """
    connection = (
        "host=localhost port=55518 dbname=stressgres connect_timeout=2 "
        "application_name=segment_diagnostic"
    )
    env = os.environ | {"PGOPTIONS": "-c statement_timeout=4000"}
    started = time.monotonic()
    with (
        (output / "console.log").open("w", encoding="utf-8") as console,
        (output / "samples.jsonl").open("w", encoding="utf-8") as samples,
        subprocess.Popen(
            [
                "stressgres",
                "headless",
                str(suite_path),
                "--pgversion",
                "pg18",
                "--runtime",
                str(runtime_ms),
                "--log-file",
                str(output / "metrics.jsonl"),
            ],
            stdout=console,
            stderr=subprocess.STDOUT,
        ) as process,
    ):
        configured = False
        iteration = 0
        while process.poll() is None:
            sample = {"elapsed": round(time.monotonic() - started, 3)}
            for key, query in [("pins", pin_sql), ("index", sql)]:
                result = subprocess.run(
                    ["psql", connection, "-XAt", "-v", "ON_ERROR_STOP=1", "-c", query],
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                try:
                    sample[key] = (
                        json.loads(result.stdout)
                        if result.returncode == 0
                        else {"error": result.stderr.strip()}
                    )
                except json.JSONDecodeError:
                    sample[key] = {"error": result.stdout + result.stderr}
            if not configured and "error" not in sample["index"]:
                result = subprocess.run(
                    [
                        "psql",
                        connection,
                        "-XAt",
                        "-c",
                        """
                    SELECT json_build_object(
                        'version', version(),
                        'extension', (SELECT extversion FROM pg_extension
                                  WHERE extname='pg_search'),
                        'shared_buffers', current_setting('shared_buffers'),
                        'max_worker_processes', current_setting('max_worker_processes'),
                        'max_wal_size', current_setting('max_wal_size'),
                        'relations', (SELECT json_agg(r) FROM (
                            SELECT oid, relname, pg_relation_filenode(oid) AS relfilenode
                            FROM pg_class WHERE relname IN ('test','idxtest','upserts','idxupserts')
                        ) r));
                """,
                    ],
                    env=env,
                    capture_output=True,
                    text=True,
                    timeout=10,
                    check=False,
                )
                (output / "configuration.json").write_text(result.stdout)
                configured = result.returncode == 0
            samples.write(json.dumps(sample) + "\n")
            samples.flush()
            if iteration % 6 == 0:
                print(
                    json.dumps(
                        {
                            "elapsed": sample["elapsed"],
                            "counts": sample["index"].get("counts"),
                            "pinned": sample["pins"].get("pinned_buffers"),
                            "mergers": sample["index"].get("mergers"),
                        }
                    ),
                    flush=True,
                )
            iteration += 1
            if time.monotonic() - started > runtime_ms / 1000 + 300:
                process.terminate()
                break
            time.sleep(5)
        status = process.wait(timeout=75)
    (output / "exit.json").write_text(
        json.dumps({"exit_code": status, "elapsed": time.monotonic() - started})
    )
    for source in Path("/tmp/stressgres").glob("*.log"):
        shutil.copy2(source, output / source.name)
    print(f"{writers} writers: Stressgres exited {status}", flush=True)
    if status != 0:
        raise SystemExit(status)


if __name__ == "__main__":
    main()
