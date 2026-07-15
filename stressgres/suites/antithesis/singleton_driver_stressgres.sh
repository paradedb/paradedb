#!/bin/bash
#
# Workload only. Whichever first_ command ran this timeline built its suite's schema
# fault-free and published it at WORKLOAD_LINK, so we connect with --skip-setup and run that
# workload under active faults. No teardown: the schema is left in place for the rest of the
# timeline.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/suite_setup.sh"

# Short runtime: the fuzzer branches across many short timelines, so a fast run covers more
# fault schedules per budget. reconnect-grace > runtime, so connectivity faults never fail
# the run; the liveness assertions judge recovery.
echo ""
echo "Running Stressgres workload from ${WORKLOAD_LINK}..."
"${STRESSGRES}" headless "${WORKLOAD_LINK}" --skip-setup --runtime 100000 --log-interval-ms 10000 --reconnect-grace 200000 --reconnect-grace-file /tmp/stressgres-reconnect-grace

echo ""
echo "Stressgres workload complete!"
