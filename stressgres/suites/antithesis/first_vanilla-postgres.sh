#!/bin/bash

# Setup for the vanilla-postgres suite; runs before fault injection begins. See helper_lib.sh.
set -Eeuo pipefail
source "$(dirname "$(readlink -f "$0")")/helper_lib.sh"

setup vanilla-postgres.toml vanilla
