#!/bin/bash

# Workload only: first_ built the schema. --skip-setup, no teardown.

set -Eeuo pipefail

# Short runtime: more short timelines cover more fault schedules.
echo ""
echo "Running Stressgres workload for background-merge.toml..."
/home/app/target/release/stressgres headless /home/app/stressgres/suites/background-merge.toml --skip-setup --runtime 100000 --log-interval-ms 10000 --reconnect-grace 3600000 --reconnect-grace-file /tmp/stressgres-reconnect-grace

echo ""
echo "Stressgres workload complete!"
