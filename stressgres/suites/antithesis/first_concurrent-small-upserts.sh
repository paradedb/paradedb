#!/bin/bash

# Setup for the concurrent-small-upserts suite; runs before fault injection begins. See helper_suite_setup.sh.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/helper_suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/helper_suite_setup.sh"

setup concurrent-small-upserts.toml single
