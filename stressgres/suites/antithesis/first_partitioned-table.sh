#!/bin/bash
# Setup partitioned-table coverage before fault injection begins.

set -Eeuo pipefail
source /home/app/stressgres/suites/antithesis/helper_suite_setup.sh

setup partitioned-table.toml single
