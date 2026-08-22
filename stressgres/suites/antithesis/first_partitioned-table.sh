#!/bin/bash

# Setup partitioned-table coverage before fault injection begins.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/helper_suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/helper_suite_setup.sh"

setup partitioned-table.toml single
