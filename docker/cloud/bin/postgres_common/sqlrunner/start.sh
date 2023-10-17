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

set -e -u

for sql in /pgconf/*.sql
do
    PGPASSWORD="${PG_PASSWORD?}" psql -d ${PG_DATABASE?} -U ${PG_USER?} \
         -p ${PG_PORT?} -h ${PG_HOST?} \
         -f ${sql?}
done

exit 0
