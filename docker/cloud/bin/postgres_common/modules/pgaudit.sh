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

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}
if [[ -v PGAUDIT_ANALYZE ]]
then
    source "${CRUNCHY_DIR}/bin/common_lib.sh"
    echo_info "Applyed pgaudit module.."
    pgaudit_analyze ${PATRONI_POSTGRESQL_DATA_DIR:-$PGDATA}/pg_log --user=postgres --log-file /tmp/pgaudit_analyze.log &
fi
