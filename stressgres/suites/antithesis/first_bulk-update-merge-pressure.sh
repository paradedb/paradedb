#!/bin/bash

# Setup for the bulk-update merge-pressure suite; runs before fault injection begins.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/helper_suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/helper_suite_setup.sh"

setup bulk-update-merge-pressure.toml single
