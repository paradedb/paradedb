#!/bin/bash

# Workload only. The paired first_ command built the schema; connect to it with --skip-setup
# and run the workload. No teardown, so the schema survives for the rest of the timeline.

set -Eeuo pipefail

# Short runtime: the fuzzer branches across many short timelines, so a fast run covers more
# fault schedules per budget.
echo ""
echo "Running Stressgres workload for logical-replication.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/logical-replication.toml --skip-setup --runtime 100000 --log-interval-ms 10000 --reconnect-grace 3600000 --reconnect-grace-file /tmp/stressgres-reconnect-grace

echo ""
echo "Stressgres workload complete!"
