#!/bin/bash

# Setup for the logical-replication FSM merge-race suite; runs before fault injection begins.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/helper_suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/helper_suite_setup.sh"

setup logical-replication-fsm-merge-race.toml pub_sub
