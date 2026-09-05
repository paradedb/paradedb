#!/bin/bash

set -Eeuo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)"
# shellcheck source=stressgres/suites/antithesis/helper_suite_setup.sh
source "${SCRIPT_DIR}/helper_suite_setup.sh"

setup logical-replication-mixed-workload-multi-subscriber.toml pub_multi_sub
