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

source /tmp/pgbackrest_env.sh > /tmp/pgbackrest_env.stdout 2> /tmp/pgbackrest_env.stderr

cmd_args=()
# if TLS verification is disabled, pass in the appropriate flag
# otherwise, leave the default behavior and verify TLS
if [[ "${PGHA_PGBACKREST_S3_VERIFY_TLS}" == "false" ]]
then
    cmd_args+=("--no-repo1-s3-verify-tls")
fi

echo $(echo -n "$conf|" | tr '/' '_'; pgbackrest --output=json ${cmd_args[*]} info | tr -d '\n')
