"""Exercise perf event loss and the profiler's command/cleanup lifecycle."""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("profile-postgres.sh")
TOOLS = r"""#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

tool = Path(sys.argv[0]).name
args = sys.argv[1:]
if tool == "sudo":
    if args[0] == "-n":
        args.pop(0)
    if args[0] != "chown":
        os.execvp(args[0], args)
elif tool == "uname":
    print(os.environ["TEST_ARCH"] if args == ["-m"] else "Test Linux")
elif tool == "docker":
    if args[0] == "top":
        print("PID COMMAND\n123 postgres")
    elif ".State.Pid" in args[2]:
        print("123")
    else:
        print("container=/paradedb image=test")
elif tool == "perf":
    with open(os.environ["TEST_CALLS"], "a") as calls:
        calls.write(json.dumps(args) + "\n")
    if args[0] == "version":
        print("perf version test")
    elif args[0] == "record":
        output = Path(args[args.index("--output") + 1])
        output.write_text("perf data")
        control_arg = next((a for a in args if a.startswith("--control=fifo:")), None)
        if control_arg:
            control, ack = control_arg[len("--control=fifo:"):].split(",")
            with open(control) as commands, open(ack, "w") as replies:
                for command in commands:
                    if command.strip() == "enable":
                        replies.write("ack\n")
                        replies.flush()
                    if command.strip() == "stop":
                        break
        elif os.environ.get("TEST_RECORD_FAILURE"):
            sys.exit(5)
    elif args[0] == "report":
        events = os.environ.get("TEST_EVENTS", "cpu-clock:uH,branch-misses:u")
        for event in events.split(","):
            samples = 0 if event.startswith("cpu-clock") and os.environ.get("TEST_ZERO_CPU") else 12
            print(f"# Samples: {samples} of event '{event}'")
"""


class ProfileTests(unittest.TestCase):
    """Check event validation and lifecycle without requiring a Linux host."""

    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))  # pylint: disable=consider-using-with
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for name in ["sudo", "perf", "docker", "uname"]:
            path = self.bin / name
            path.write_text(TOOLS)
            path.chmod(0o755)
        self.calls = self.root / "calls.jsonl"
        self.env = os.environ | {
            "PATH": str(self.bin) + os.pathsep + os.environ["PATH"],
            "TEST_ARCH": "aarch64",
            "TEST_CALLS": str(self.calls),
        }
        self.output = self.root / "profiles"

    def run_profile(self, *args, **env):
        """Execute the production wrapper with fake host tools."""
        return subprocess.run(
            ["bash", str(SCRIPT), *map(str, args)],
            env=self.env | env,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def perf_calls(self):
        """Read the perf invocations recorded by the fixture."""
        return [json.loads(line) for line in self.calls.read_text().splitlines()]

    def test_arm_clock_preflight_keeps_general_registers_without_sve(self):
        """Retain ordinary DWARF registers without the unsupported SVE register."""
        result = self.run_profile("--check", self.output)
        self.assertEqual(result.returncode, 0, result.stderr)
        record = next(c for c in self.perf_calls() if c[0] == "record")
        registers = record[record.index("--user-regs") + 1].split(",")
        self.assertEqual(registers, [f"x{i}" for i in range(30)] + ["lr", "sp", "pc"])
        self.assertEqual(
            record[record.index("--event") + 1], "cpu-clock:u,branch-misses:u"
        )
        self.assertEqual(record[record.index("--call-graph") + 1], "dwarf,16384")
        self.assertIn("--user-regs", (self.output / "metadata.txt").read_text())

    def test_x86_uses_default_dwarf_registers(self):
        """Keep perf's register selection on other architectures."""
        result = self.run_profile("--check", self.output, TEST_ARCH="x86_64")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = next(c for c in self.perf_calls() if c[0] == "record")
        self.assertNotIn("--user-regs", record)

    def test_preflight_rejects_silently_dropped_events(self):
        """A successful perf exit cannot conceal a dropped event."""
        for events in ["branch-misses:u", "cpu-clock:uH"]:
            with self.subTest(events=events):
                output = self.root / events.replace(":", "-")
                result = self.run_profile("--check", output, TEST_EVENTS=events)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("nonzero cpu-clock and branch-misses", result.stderr)

    def test_preflight_rejects_clock_with_no_samples(self):
        """An event header alone is insufficient evidence of CPU samples."""
        result = self.run_profile("--check", self.output, TEST_ZERO_CPU="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("nonzero cpu-clock and branch-misses", result.stderr)

    def test_preflight_preserves_record_failure(self):
        """Stop before reporting when the recording command fails."""
        result = self.run_profile("--check", self.output, TEST_RECORD_FAILURE="1")
        self.assertEqual(result.returncode, 5)
        self.assertFalse(any(c[0] == "report" for c in self.perf_calls()))

    def test_profile_runs_command_and_generates_both_reports(self):
        """Exercise startup acknowledgement, command execution and recorder shutdown."""
        marker = self.root / "command-ran"
        result = self.run_profile(
            "paradedb",
            self.output,
            "--",
            sys.executable,
            "-c",
            "from pathlib import Path; import sys; Path(sys.argv[1]).touch()",
            marker,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(marker.exists())
        reports = [c for c in self.perf_calls() if c[0] == "report"]
        self.assertEqual(len(reports), 2)
        self.assertTrue((self.output / "report-callgraph.txt").exists())

    def test_profile_rejects_missing_cpu_samples(self):
        """Validate the actual benchmark capture as well as the preflight."""
        result = self.run_profile(
            "paradedb", self.output, "--", "true", TEST_EVENTS="branch-misses:u"
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("nonzero cpu-clock and branch-misses", result.stderr)

    def test_profile_preserves_benchmark_failure(self):
        """Keep the benchmark's error when the profile is also invalid."""
        result = self.run_profile(
            "paradedb",
            self.output,
            "--",
            "bash",
            "-c",
            "exit 7",
            TEST_EVENTS="branch-misses:u",
        )
        self.assertEqual(result.returncode, 7)


if __name__ == "__main__":
    unittest.main()
