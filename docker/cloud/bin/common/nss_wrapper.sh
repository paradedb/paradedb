#!/bin/bash

# Copyright 2021 - 2023 Crunchy Data Solutions, Inc.
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

# The following sets up an nss_wrapper environment in accordance with OpenShift
# guidance for supporting arbitrary user ID's

# Define nss_wrapper directory and passwd & group files that will be utilized by nss_wrapper.  The
# nss_wrapper_env.sh script (which also sets these vars) isn't sourced here since the nss_wrapper
# has not yet been setup, and we therefore don't yet want the nss_wrapper vars in the environment.
mkdir -p /tmp/nss_wrapper
chmod g+rwx /tmp/nss_wrapper

NSS_WRAPPER_DIR="/tmp/nss_wrapper/${NSS_WRAPPER_SUBDIR}"
NSS_WRAPPER_PASSWD="${NSS_WRAPPER_DIR}/passwd"
NSS_WRAPPER_GROUP="${NSS_WRAPPER_DIR}/group"

# create the nss_wrapper directory
mkdir -p "${NSS_WRAPPER_DIR}"

# grab the current user ID and group ID
USER_ID=$(id -u)
export USER_ID
GROUP_ID=$(id -g)
export GROUP_ID

# get copies of the passwd and group files
[[ -f "${NSS_WRAPPER_PASSWD}" ]] || cp "/etc/passwd" "${NSS_WRAPPER_PASSWD}"
[[ -f "${NSS_WRAPPER_GROUP}" ]] || cp "/etc/group" "${NSS_WRAPPER_GROUP}"

# if the username is missing from the passwd file, then add it
if [[ ! $(cat "${NSS_WRAPPER_PASSWD}") =~ ${CRUNCHY_NSS_USERNAME}:x:${USER_ID} ]]; then
    echo "nss_wrapper: adding user"
    passwd_tmp="${NSS_WRAPPER_DIR}/passwd_tmp"
    cp "${NSS_WRAPPER_PASSWD}" "${passwd_tmp}"
    sed -i "/${CRUNCHY_NSS_USERNAME}:x:/d" "${passwd_tmp}"
    # needed for OCP 4.x because crio updates /etc/passwd with an entry for USER_ID
    sed -i "/${USER_ID}:x:/d" "${passwd_tmp}"
    printf '${CRUNCHY_NSS_USERNAME}:x:${USER_ID}:${GROUP_ID}:${CRUNCHY_NSS_USER_DESC}:${HOME}:/bin/bash\n' >> "${passwd_tmp}"
    envsubst < "${passwd_tmp}" > "${NSS_WRAPPER_PASSWD}"
    rm "${passwd_tmp}"
else
    echo "nss_wrapper: user exists"
fi

# if the username (which will be the same as the group name) is missing from group file, then add it
if [[ ! $(cat "${NSS_WRAPPER_GROUP}") =~ ${CRUNCHY_NSS_USERNAME}:x:${USER_ID} ]]; then
    echo "nss_wrapper: adding group"
    group_tmp="${NSS_WRAPPER_DIR}/group_tmp"
    cp "${NSS_WRAPPER_GROUP}" "${group_tmp}"
    sed -i "/${CRUNCHY_NSS_USERNAME}:x:/d" "${group_tmp}"
    printf '${CRUNCHY_NSS_USERNAME}:x:${USER_ID}:${CRUNCHY_NSS_USERNAME}\n' >> "${group_tmp}"
    envsubst < "${group_tmp}" > "${NSS_WRAPPER_GROUP}"
    rm "${group_tmp}"
else
    echo "nss_wrapper: group exists"
fi

# export the nss_wrapper env vars
source "${CRUNCHY_DIR}/bin/nss_wrapper_env.sh"
echo "nss_wrapper: environment configured"
