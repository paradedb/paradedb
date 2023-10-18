#!/bin/bash
# shellcheck source=/dev/null

# Copyright 2016 - 2023 Crunchy Data Solutions, Inc.
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Exit on subcommand errors
set -Eeuo pipefail

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}
source "${CRUNCHY_DIR}/bin/common_lib.sh"
enable_debugging

if [ -d "/pguser" ]; then
  echo_info "The PGUSER secret exists."
  PG_USER=$(cat /pguser/username)
  export PG_USER
  PG_PASSWORD=$(cat /pguser/password)
  export PG_PASSWORD
fi
if [ -d "/pgroot" ]; then
  echo_info "The PGROOT secret exists."
  PG_ROOT_PASSWORD=$(cat /pgroot/password)
  export PG_ROOT_PASSWORD
fi
if [ -d "/pgprimary" ]; then
  echo_info "The PGPRIMARY secret exists."
  PG_PRIMARY_USER=$(cat /pgprimary/username)
  export PG_PRIMARY_USER
  PG_PRIMARY_PASSWORD=$(cat /pgprimary/password)
  export PG_PRIMARY_PASSWORD
fi
