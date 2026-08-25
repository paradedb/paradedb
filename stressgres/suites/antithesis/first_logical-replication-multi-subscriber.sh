#!/bin/bash

# Setup for the multi-subscriber logical-replication suite; runs before fault injection begins.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/helper_suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/helper_suite_setup.sh"

setup logical-replication-multi-subscriber.toml pub_multi_sub
