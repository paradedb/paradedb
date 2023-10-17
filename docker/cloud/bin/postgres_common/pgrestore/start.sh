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

set -e

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}
source "${CRUNCHY_DIR}/bin/common_lib.sh"
enable_debugging

export PGROOT=$(find /usr/ -type d -name 'pgsql-*')
export PGPASSFILE=/tmp/pgpass

env_check_err "PGRESTORE_DB"
env_check_err "PGRESTORE_HOST"
env_check_err "PGRESTORE_PASS"
env_check_err "PGRESTORE_USER"

PGRESTORE_PORT=${PGRESTORE_PORT:-5432}
PGRESTORE_CUSTOM_OPTS=${PGRESTORE_CUSTOM_OPTS:-}
PGRESTORE_BACKUP_TIMESTAMP=${PGRESTORE_BACKUP_TIMESTAMP:-}
PGDUMP_BACKUP_HOST="${PGDUMP_BACKUP_HOST:-${PGRESTORE_HOST?}}"

# Todo: Add SSL support
conn_opts="-h ${PGRESTORE_HOST?} -p ${PGRESTORE_PORT?} -U ${PGRESTORE_USER?} -d ${PGRESTORE_DB?}"

# escape any instances of ':' or '\' with '\' in the provided password
# before storing the value in the password file
ESCAPED_PASSWORD=$(sed <<< "${PGRESTORE_PASS?}" 's/[:\\]/\\&/g')

cat >> "${PGPASSFILE?}" <<-EOF
*:*:*:${PGRESTORE_USER?}:${ESCAPED_PASSWORD?}
EOF

chmod 600 ${PGPASSFILE?}
# chown postgres:postgres ${PGPASSFILE?}

set +e
pgisready ${PGRESTORE_DB?} ${PGRESTORE_HOST?} ${PGRESTORE_PORT?} ${PGRESTORE_USER?}
set -e

PGRESTORE_BASE=/pgdata/${PGDUMP_BACKUP_HOST?}-backups
if [[ -z ${PGRESTORE_BACKUP_TIMESTAMP?} ]]
then
    echo_info "Backup timestamp not set.  Defaulting to latest backup found.."
    PGRESTORE_BACKUP_TIMESTAMP=$(ls -t ${PGRESTORE_BASE?} | head -1)
fi

BACKUP_DIR="${PGRESTORE_BASE?}/${PGRESTORE_BACKUP_TIMESTAMP?}"
if [[ ! -d ${BACKUP_DIR?} ]]
then
    echo_err "Backup directory does not exist: ${BACKUP_DIR?}"
    exit 1
fi

BACKUP_FILE="${BACKUP_DIR?}/$(ls ${BACKUP_DIR?} | head -n 1)"
if [[ ! -f ${BACKUP_FILE?} ]] && [[ ! -d ${BACKUP_FILE?} ]]
then
    echo_err "Backup file does not exist: ${BACKUP_FILE?}"
    exit 1
fi

echo_info "Restore will be attempted using backup ${BACKUP_FILE?}"

# Plain pg_dump backups are restored via psql - any kind of custom backup
# (tar, directory, custom) are restored via pg_restore
BACKUP_TYPE=$(file ${BACKUP_FILE?})
set +e
if [[ ${BACKUP_TYPE?} = *"text"* ]]
then
    echo_info "Restoring from SQL backup via psql.."
    ${PGROOT?}/bin/psql ${conn_opts?} -f ${BACKUP_FILE?}
else
    echo_info "Restoring from backup via pg_restore.."
    ${PGROOT?}/bin/pg_restore ${conn_opts?} ${PGRESTORE_CUSTOM_OPTS?} ${BACKUP_FILE?}
fi
set -e

echo_info "Logical restore completed.  Exiting.."

exit 0
