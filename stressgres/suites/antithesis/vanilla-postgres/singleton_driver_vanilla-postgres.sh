#!/bin/bash

# Workload only. The paired first_ command built the schema; connect to it with --skip-setup
# and run the workload. No teardown, so the schema survives for the rest of the timeline.

set -Eeuo pipefail

# Short runtime: more short timelines cover more fault schedules.
# reconnect-grace > runtime, so connectivity faults never fail the run; the liveness assertions judge recovery.
echo ""
echo "Running Stressgres workload for vanilla-postgres.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/vanilla-postgres.toml --skip-setup --runtime 100000 --log-interval-ms 10000 --reconnect-grace 200000 --reconnect-grace-file /tmp/stressgres-reconnect-grace

echo ""
echo "Stressgres workload complete!"
