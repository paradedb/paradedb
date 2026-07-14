#!/bin/bash

set -Eeuo pipefail

# Workload only. The paired first_ command has already rewritten both connection strings and
# built the schema fault-free, so we connect with --skip-setup and run the workload under
# active faults. No teardown: the schema is left in place for the rest of the timeline.
echo ""
echo "Running Stressgres workload for logical-replication-merge.toml..."
# Keep the runtime short: Antithesis explores by branching across many short timelines, so a
# fast, self-contained run covers far more fault schedules per budget than one long scenario.
/home/app/target/release/stressgres headless /home/app/stressgres/suites/logical-replication-merge.toml --skip-setup --runtime 100000 --log-interval-ms 10000 --reconnect-grace 3600000 --reconnect-grace-file /tmp/stressgres-reconnect-grace

echo ""
echo "Stressgres workload completed!"
