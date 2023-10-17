#!/bin/bash

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

set -e -u

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}
source "${CRUNCHY_DIR}/bin/common_lib.sh"
enable_debugging

export PGBENCH_BENCHMARK_OPTS=${PGBENCH_BENCHMARK_OPTS:-}
export PGBENCH_CLIENTS=${PGBENCH_CLIENTS:-1}
export PGBENCH_INIT_OPTS=${PGBENCH_INIT_OPTS:-}
export PGBENCH_JOBS=${PGBENCH_JOBS:-1}
export PGBENCH_SCALE=${PGBENCH_SCALE:-1}
export PGBENCH_TRANSACTIONS=${PGBENCH_TRANSACTIONS:-10}
export PGPASSFILE=/tmp/.pgpass
export PG_PORT=${PG_PORT:-5432}
export TRANSACTION_SCRIPT='/pgconf/transactions.sql'

set +e
env_check_err "PG_DATABASE"
env_check_err "PG_HOSTNAME"
env_check_err "PG_PASSWORD"
env_check_err "PG_USERNAME"
set -e

if [[ -f ${TRANSACTION_SCRIPT?} ]]
then
    PGBENCH_FILENAME="-f ${TRANSACTION_SCRIPT?}"
fi

function create_pgpass() {
    cd /tmp

# escape any instances of ':' or '\' with '\' in the provided password
# before storing the value in the password file
ESCAPED_PASSWORD=$(sed <<< "${PG_PASSWORD?}" 's/[:\\]/\\&/g')

cat >> ".pgpass" <<-EOF
*:*:*:${PG_USERNAME?}:${ESCAPED_PASSWORD?}
EOF
    chmod 0600 .pgpass
}

create_pgpass

${PGROOT?}/bin/pgbench --initialize \
    --host=${PG_HOSTNAME?} \
    --port=${PG_PORT?} \
    --username=${PG_USERNAME?} \
    --scale=${PGBENCH_SCALE?} \
    --quiet \
    ${PGBENCH_INIT_OPTS?} \
    ${PG_DATABASE?}

${PGROOT?}/bin/pgbench \
    --client=${PGBENCH_CLIENTS?} \
    --jobs=${PGBENCH_JOBS?} \
    --scale=${PGBENCH_SCALE?} \
    --transactions=${PGBENCH_TRANSACTIONS?} \
    --host=${PG_HOSTNAME?} \
    --port=${PG_PORT?} \
    --username=${PG_USERNAME?} \
    ${PGBENCH_FILENAME:-} \
    ${PGBENCH_BENCHMARK_OPTS?} \
    ${PG_DATABASE?}
