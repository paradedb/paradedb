#!/bin/bash

# Setup for the logical-replication suite; runs before fault injection begins. See helper_suite_setup.sh.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/helper_suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/helper_suite_setup.sh"

setup logical-replication.toml pub_sub
