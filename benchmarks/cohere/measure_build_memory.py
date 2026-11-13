"""Run an index build while sampling its database container's memory."""

import argparse
import http.client
import json
import socket
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import quote


class DockerConnection(http.client.HTTPConnection):
    """Read the local Docker daemon over its Unix socket."""

    def __init__(self):
        super().__init__("localhost", timeout=5)

    def connect(self):
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.settimeout(self.timeout)
        self.sock.connect("/var/run/docker.sock")


def read_stats(container):
    """Fetch one raw Docker stats sample."""
    connection = DockerConnection()
    try:
        connection.request(
            "GET",
            f"/containers/{quote(container, safe='')}/stats?stream=false&one-shot=true",
        )
        response = connection.getresponse()
        if response.status != 200:
            raise RuntimeError(f"Docker stats returned HTTP {response.status}")
        return json.load(response)
    finally:
        connection.close()


def memory_values(stats):
    """Extract both build and query memory definitions from a Docker sample."""
    memory = stats["memory_stats"]
    usage = memory["usage"]
    details = memory["stats"]
    # Match pdbench's build metric on cgroup v2 and v1.
    inactive = details.get("inactive_file", details.get("total_inactive_file"))
    if inactive is None or usage <= 0:
        raise ValueError("Docker stats lacks usable memory usage/inactive-file data")
    working = usage - inactive if inactive < usage else usage
    cache = details.get("cache", 0)
    # Match the pinned Benchmarker query collector as a separate series.
    query_usage = usage - cache if 0 < cache < usage else usage
    return {
        "container_memory": working,
        "container_memory_usage": query_usage,
        "container_memory_raw_usage": usage,
        "container_memory_inactive_file": inactive,
    }


def measure(command, container, output, interval=1, reader=read_stats):
    """Preserve the build exit status and flush samples while the build runs."""
    output.mkdir(parents=True, exist_ok=True)
    errors = []
    peak = None
    count = 0
    started = time.monotonic()
    with (output / "points.jsonl").open("w") as stream:

        def sample():
            nonlocal peak, count
            values = memory_values(reader(container))
            timestamp = datetime.now(timezone.utc).isoformat()
            for name, value in values.items():
                if count == 0:
                    stream.write(
                        json.dumps(
                            {
                                "type": "Metric",
                                "metric": name,
                                "kind": "gauge",
                                "unit": "bytes",
                            }
                        )
                        + "\n"
                    )
                stream.write(
                    json.dumps(
                        {
                            "type": "Point",
                            "metric": name,
                            "time": timestamp,
                            "value": value,
                            "tags": {"phase": "build"},
                        }
                    )
                    + "\n"
                )
            stream.flush()
            peak = max(peak or 0, values["container_memory"])
            count += 1

        # Verify that the container is observable before starting an expensive build.
        sample()
        with subprocess.Popen(command) as process:
            try:
                while True:
                    try:
                        code = process.wait(timeout=interval)
                    except subprocess.TimeoutExpired:
                        code = None
                    try:
                        sample()
                    except (
                        OSError,
                        RuntimeError,
                        ValueError,
                        KeyError,
                        http.client.HTTPException,
                    ) as error:
                        errors.append(str(error))
                        print(f"Memory sampling failed: {error}", file=sys.stderr)
                    if code is not None:
                        break
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
                (output / "summary.json").write_text(
                    json.dumps(
                        {
                            "container": container,
                            "command": command,
                            "elapsed_seconds": time.monotonic() - started,
                            "exit_code": process.returncode,
                            "sample_count": count,
                            "peak_container_memory_bytes": peak,
                            "sampling_errors": errors,
                        },
                        indent=2,
                    )
                    + "\n"
                )
    # Preserve a build failure; do not publish an incomplete memory trace as success.
    return code if code else int(bool(errors))


def main():
    """Parse the container, artifact destination, and build command."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--container", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a build command is required after --")
    return measure(command, args.container, args.output)


if __name__ == "__main__":
    sys.exit(main())
