#!/bin/bash

# Copyright 2017 - 2023 Crunchy Data Solutions, Inc.
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

PGDATA_PATH_FULL=/pgdata/"${PGDATA_PATH}"
BACKUP_PATH_FULL=/backup/"${BACKUP_PATH}"

# Validate that the proper env vars have been set as needed to restore from a pg_basebackup backup
validate_pgbasebackup_restore_env_vars()  {
    if [[ ! -v PGDATA_PATH ]]
    then
        echo_err "Env var PGDATA_PATH must be set in order to restore from a pg_basebackup backup"
        exit 1
    fi
}

# Validate that the backup directory provided contains a pg database
validate_backup_dir() {
    if [ ! -f "${BACKUP_PATH_FULL}"/postgresql.conf ]
    then
        echo_err "A PostgreSQL db was not found in backup path '${BACKUP_PATH_FULL}'"
        exit 1
    fi
}

# Create an empty pgdata directory for the restore if it does not already exist
create_restore_pgdata_dir()  {
    if [[ ! -d "${PGDATA_PATH_FULL}" ]]
    then
        mkdir -p "${PGDATA_PATH_FULL}"
        echo_info "Created new pgdata directory ${PGDATA_PATH_FULL} for pg_basebackup restore"
    fi
}

# Use rsync to copy backup files to new pgdata directory
rsync_backup()  {

    if [[ "${RSYNC_SHOW_PROGRESS}" == "true" ]]
    then
        progress="--progress"
    fi
    rsync -a $progress --exclude 'pg_log/*' "${BACKUP_PATH_FULL}"/ "${PGDATA_PATH_FULL}" \
        2> /tmp/rsync.stderr
    err_check "$?" "Restore from pg_basebackup backup" \
        "Unable to rsync pg_basebackup backup: \n$(cat /tmp/rsync.stderr)"

    echo_info "rysnc of backup into restore directory complete"

    chmod -R 0700 "${PGDATA_PATH_FULL}"
}

validate_pgbasebackup_restore_env_vars
validate_backup_dir
create_restore_pgdata_dir

echo_info "Restoring from pg_basebackup backup:"
echo_info "   Backup Path = '${BACKUP_PATH_FULL}'"
echo_info "  Restore Path = '${PGDATA_PATH_FULL}'"
rsync_backup

echo_info "pg_basebackup restore complete"
