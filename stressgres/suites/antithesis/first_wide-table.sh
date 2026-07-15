#!/bin/bash

# Setup for the wide-table suite; runs before fault injection begins. See suite_setup.sh.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/suite_setup.sh"

setup wide-table.toml single
