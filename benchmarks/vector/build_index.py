"""Run pdbench-load while retaining observations of its parallel index workers."""

import argparse
import json
import os
import subprocess
import time
from pathlib import Path

WORKERS_SQL = """
SELECT count(*) FROM pg_stat_activity
WHERE backend_type = 'parallel worker' AND leader_pid IN (
    SELECT pid FROM pg_stat_progress_create_index
    WHERE relid = :'table'::regclass AND datname = current_database()
);
"""


def build(config_path):
    """Preserve the loader's exit status and flush worker samples as they arrive."""
    config = json.loads(config_path.read_text())
    table = config["datastore"]["table"]
    url = os.path.expandvars(config["datastore"]["url"])
    started = time.monotonic()
    with (
        (config_path.parent / "workers.jsonl").open("w", buffering=1) as samples,
        (config_path.parent / "worker-sampler.log").open("w") as errors,
        subprocess.Popen(["pdbench-load", "finalize", str(config_path)]) as loader,
    ):
        try:
            while loader.poll() is None:
                try:
                    result = subprocess.run(
                        [
                            "psql",
                            url,
                            "-XqAt",
                            "-v",
                            "ON_ERROR_STOP=1",
                            "-v",
                            f"table={table}",
                        ],
                        input=WORKERS_SQL,
                        text=True,
                        stdout=subprocess.PIPE,
                        stderr=errors,
                        timeout=5,
                        check=True,
                    )
                    samples.write(
                        json.dumps(
                            {
                                "elapsed_seconds": time.monotonic() - started,
                                "workers": int(result.stdout.strip()),
                            }
                        )
                        + "\n"
                    )
                except (subprocess.SubprocessError, ValueError) as error:
                    print(error, file=errors, flush=True)
                try:
                    return loader.wait(timeout=0.5)
                except subprocess.TimeoutExpired:
                    pass
            return loader.returncode
        finally:
            if loader.poll() is None:
                loader.terminate()
                try:
                    loader.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    loader.kill()
                    loader.wait()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("config", type=Path)
    raise SystemExit(build(parser.parse_args().config))
