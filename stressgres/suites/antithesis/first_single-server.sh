#!/bin/bash

# Setup for the single-server suite; runs before fault injection begins. See helper_lib.sh.
set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/helper_lib.sh
source "$(dirname "$(readlink -f "$0")")/helper_lib.sh"

setup single-server.toml single
