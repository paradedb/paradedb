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

export BACKUP_LABEL=${BACKUP_LABEL:-crunchybackup}
export PGPASSFILE=/tmp/pgpass

env_check_err "BACKUP_HOST"
env_check_err "BACKUP_USER"
env_check_err "BACKUP_PASS"

BACKUPBASE=/pgdata/${BACKUP_HOST?}-backups
if [[ ! -d "${BACKUPBASE?}" ]]
then
    echo_info "Creating BACKUPBASE directory as ${BACKUPBASE?}.."
    mkdir -p ${BACKUPBASE?}
fi

TS=`date +%Y-%m-%d-%H-%M-%S`
BACKUP_PATH=${BACKUPBASE?}/${TS?}
mkdir ${BACKUP_PATH?}

env_check_info "BACKUP_LABEL" "BACKUP_LABEL is set to ${BACKUP_LABEL}."
env_check_info "BACKUP_PATH" "BACKUP_PATH is set to ${BACKUP_PATH}."
env_check_info "BACKUP_OPTS" "BACKUP_OPTS is set to ${BACKUP_OPTS}."

# escape any instances of ':' or '\' with '\' in the provided password
# before storing the value in the password file
ESCAPED_PASSWORD=$(sed <<< "${BACKUP_PASS?}" 's/[:\\]/\\&/g')

echo "*:*:*:${BACKUP_USER?}:${ESCAPED_PASSWORD?}" >> ${PGPASSFILE?}
chmod 600 ${PGPASSFILE?}

pg_basebackup --label=${BACKUP_LABEL?} -X fetch \
    --pgdata ${BACKUP_PATH?} --host=${BACKUP_HOST?} \
    --port=${BACKUP_PORT?} -U ${BACKUP_USER?} ${BACKUP_OPTS}

# Open up permissions for the OSE Dedicated random UID scenario
chmod -R o+rx ${BACKUP_PATH?}

echo_info "Backup has completed."
