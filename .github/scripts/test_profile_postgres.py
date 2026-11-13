"""Exercise host profiler lifecycle and error handling with fake external tools."""

import json
import os
import signal
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("profile-postgres.sh")
WAIT_SECONDS = 10
POLL_SECONDS = 0.01
FAKE_TOOL = r"""#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys
import time

tool = Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ["EVENTS"], "a") as log:
    log.write(json.dumps([tool, *args]) + "\n")
if tool == "sudo":
    os.execvp(args[1], args[1:])
elif tool == "docker":
    if args[0] == "top":
        if os.environ.get("DOCKER_FAILURE"):
            sys.exit(1)
        print("PID COMMAND")
        if not os.environ.get("NO_POSTGRES"):
            print("101 postgres")
            print("102 postgres")
            print("103 postmaster")
        print("104 unrelated")
    elif ".State.Pid" in args[2]:
        print("101")
    else:
        print("container=/database image=postgres:18 image_id=sha256:example")
elif tool == "perf":
    if args[0] == "version":
        print("perf version test")
    elif args[0] == "record":
        output = Path(args[args.index("--output") + 1])
        output.write_text("profile")
        control_arg = next(arg for arg in args if arg.startswith("--control="))
        control_path, ack_path = control_arg.split("fifo:", 1)[1].split(",")
        with open(control_path) as control, open(ack_path, "w") as ack:
            for command in control:
                with open(os.environ["EVENTS"], "a") as log:
                    log.write(json.dumps(["control", command.strip()]) + "\n")
                if command == "enable\n":
                    ack.write(os.environ.get("STARTUP_ACK", "ack") + "\n")
                    ack.flush()
                elif command == "stop\n":
                    break
        sys.exit(int(os.environ.get("RECORD_STATUS", "0")))
    elif args[0] == "report":
        print("PostgreSQL function samples")
        sys.exit(int(os.environ.get("REPORT_STATUS", "0")))
elif tool == "workload":
    if os.environ.get("WORKLOAD_SLEEP"):
        time.sleep(60)
    sys.exit(int(os.environ.get("WORKLOAD_STATUS", "0")))
"""


class ProfilePostgresTest(unittest.TestCase):
    """Run the shell wrapper against controllable Docker, perf, and workloads."""

    def setUp(self):
        # pylint: disable=consider-using-with
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.output = self.root / "profiles" / "backend" / "point"
        self.events = self.root / "events.jsonl"
        for name in ("sudo", "docker", "perf", "workload"):
            executable = self.root / name
            executable.write_text(FAKE_TOOL)
            executable.chmod(0o755)
        self.env = {
            **os.environ,
            "PATH": f"{self.root}{os.pathsep}{os.environ['PATH']}",
            "EVENTS": str(self.events),
        }

    def run_profile(self, **env):
        """Run one workload with optional failure injection."""
        return subprocess.run(
            ["bash", str(SCRIPT), "database", str(self.output), "--", "workload"],
            env={**self.env, **env},
            check=False,
            capture_output=True,
            text=True,
            timeout=WAIT_SECONDS,
        )

    def read_events(self):
        """Read the ordered tool and control messages."""
        return [json.loads(line) for line in self.events.read_text().splitlines()]

    def test_attaches_before_workload_and_saves_reports(self):
        """Wait for attachment and preserve both symbolized views."""
        result = self.run_profile()
        self.assertEqual(result.returncode, 0, result.stderr)
        events = self.read_events()
        record = next(event for event in events if event[:2] == ["perf", "record"])
        self.assertEqual(record[record.index("--pid") + 1], "101,102,103")
        self.assertIn("--inherit", record)
        self.assertIn("dwarf,16384", record)
        self.assertLess(events.index(["control", "enable"]), events.index(["workload"]))
        self.assertLess(events.index(["workload"]), events.index(["control", "stop"]))
        reports = [event for event in events if event[:2] == ["perf", "report"]]
        self.assertEqual(len(reports), 2)
        for report in reports:
            self.assertIn("/proc/101/root", report)
        for name in ("perf.data", "report.txt", "report-callgraph.txt", "metadata.txt"):
            self.assertTrue((self.output / name).stat().st_size, name)
        self.assertIn("PostgreSQL function samples", result.stdout)

    def test_k6_failure_still_reports_and_preserves_status(self):
        """Keep the workload failure even when reporting also fails."""
        result = self.run_profile(WORKLOAD_STATUS="42", REPORT_STATUS="7")
        self.assertEqual(result.returncode, 42, result.stderr)
        self.assertTrue((self.output / "report.txt").exists())
        self.assertTrue((self.output / "report-callgraph.txt").exists())
        self.assertIn(["control", "stop"], self.read_events())

    def test_record_failure_fails_successful_workload(self):
        """Surface a recording failure after a successful workload."""
        self.assertEqual(self.run_profile(RECORD_STATUS="9").returncode, 9)

    def test_report_failure_fails_successful_workload(self):
        """Surface a report failure after a successful workload."""
        self.assertEqual(self.run_profile(REPORT_STATUS="7").returncode, 7)

    def test_existing_profile_is_not_overwritten(self):
        """Protect a previous operating point from accidental reuse."""
        self.output.mkdir(parents=True)
        (self.output / "perf.data").write_text("previous profile")
        result = self.run_profile()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.output / "perf.data").read_text(), "previous profile")

    def test_docker_failure_does_not_start_workload(self):
        """Stop before measurement when process discovery fails."""
        result = self.run_profile(DOCKER_FAILURE="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(event[0] == "workload" for event in self.read_events()))

    def test_no_postgres_does_not_start_workload(self):
        """Reject containers without PostgreSQL processes."""
        result = self.run_profile(NO_POSTGRES="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("No PostgreSQL processes", result.stderr)
        self.assertFalse(any(event[0] == "workload" for event in self.read_events()))

    def test_startup_failure_does_not_start_workload(self):
        """Stop recording when readiness cannot be confirmed."""
        result = self.run_profile(STARTUP_ACK="error")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("benchmark was not started", result.stderr)
        self.assertFalse(any(event[0] == "workload" for event in self.read_events()))
        self.assertIn(["control", "stop"], self.read_events())

    def test_cancellation_stops_recording_and_reports(self):
        """Flush the profile and report after termination."""
        with subprocess.Popen(
            ["bash", str(SCRIPT), "database", str(self.output), "--", "workload"],
            env={**self.env, "WORKLOAD_SLEEP": "1"},
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        ) as process:
            deadline = time.monotonic() + WAIT_SECONDS
            while time.monotonic() < deadline:
                if self.events.exists() and ["workload"] in self.read_events():
                    break
                time.sleep(POLL_SECONDS)
            else:
                process.kill()
                self.fail("workload did not start")
            process.send_signal(signal.SIGTERM)
            _, stderr = process.communicate(timeout=WAIT_SECONDS)
            self.assertEqual(process.returncode, 143, stderr)
        self.assertIn(["control", "stop"], self.read_events())
        self.assertTrue((self.output / "report.txt").exists())


if __name__ == "__main__":
    unittest.main()
