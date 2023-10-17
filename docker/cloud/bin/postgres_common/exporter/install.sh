#!/bin/bash

# Copyright 2019 - 2023 Crunchy Data Solutions, Inc.
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

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}

source "${CRUNCHY_DIR}/bin/common_lib.sh"
export PGHOST="/tmp"

test_server "postgres" "${PGHOST?}" "${PGHA_PG_PORT}" "postgres"
VERSION=$(psql --port="${PG_PRIMARY_PORT}" -d postgres -qtAX -c "SELECT current_setting('server_version_num')")

if (( ${VERSION?} >= 90600 )) && (( ${VERSION?} < 100000 ))
then
    function_file="${CRUNCHY_DIR}/bin/modules/pgexporter/pg96/setup.sql"
elif (( ${VERSION?} >= 100000 )) && (( ${VERSION?} < 110000 ))
then
    function_file="${CRUNCHY_DIR}/bin/modules/pgexporter/pg10/setup.sql"
elif (( ${VERSION?} >= 110000 )) && (( ${VERSION?} < 120000 ))
then
    function_file="${CRUNCHY_DIR}/bin/modules/pgexporter/pg11/setup.sql"
elif (( ${VERSION?} >= 120000 )) && (( ${VERSION?} < 130000 ))
then
    function_file="${CRUNCHY_DIR}/bin/modules/pgexporter/pg12/setup.sql"
elif (( ${VERSION?} >= 130000 ))
then
    function_file="${CRUNCHY_DIR}/bin/modules/pgexporter/pg13/setup.sql"
else
    echo_err "Unknown or unsupported version of PostgreSQL.  Exiting.."
    exit 1
fi

echo_info "Using setup file '${function_file}' for pgMonitor"
cp "${function_file}" "/tmp/setup_pg.sql"
sed -i "s,/usr/bin/pgbackrest-info.sh,${CRUNCHY_DIR}/bin/postgres/pgbackrest_info.sh,g" "/tmp/setup_pg.sql"

psql -U postgres --port="${PG_PRIMARY_PORT}" -d postgres \
    < "/tmp/setup_pg.sql" >> /tmp/pgmonitor-setup.stdout 2>> /tmp/pgmonitor-setup.stderr

psql -U postgres --port="${PG_PRIMARY_PORT}" -d postgres \
    -c "CREATE EXTENSION IF NOT EXISTS pgnodemx WITH SCHEMA monitor;" >> /tmp/pgmonitor-setup.stdout 2>> /tmp/pgmonitor-setup.stderr
