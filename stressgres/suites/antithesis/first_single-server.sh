#!/bin/bash

# Setup for the single-server suite; runs before fault injection begins. See suite_setup.sh.

set -Eeuo pipefail

# shellcheck source=stressgres/suites/antithesis/suite_setup.sh
source "$(dirname "$(readlink -f "$0")")/suite_setup.sh"

setup single-server.toml single
