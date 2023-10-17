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

# define the default nss_wrapper dir for this container and the ssh nss_wrapper dir
NSS_WRAPPER_DEFAULT_DIR="/tmp/nss_wrapper/postgres"
NSS_WRAPPER_SSH_DIR="/tmp/nss_wrapper/ssh"

# Configures nss_wrapper passwd and group files for SSH connections
function nss_wrapper_ssh() {
    mkdir -p "${NSS_WRAPPER_SSH_DIR}"
    cp "${NSS_WRAPPER_DEFAULT_DIR}/passwd" "${NSS_WRAPPER_SSH_DIR}"
    cp "${NSS_WRAPPER_DEFAULT_DIR}/group" "${NSS_WRAPPER_SSH_DIR}"
}

if [[ ${ENABLE_SSHD} == "true" ]]
then
    echo_info "Applying SSHD.."

    # configure nss_wrapper files for ssh connections
    nss_wrapper_ssh
    echo_info "nss_wrapper: ssh configured"

    echo_info 'Checking for SSH Host Keys in /sshd..'

    if [[ ! -f /sshd/ssh_host_ed25519_key ]]; then
        echo_err 'No ssh_host_ed25519_key found in /sshd.  Exiting..'
        exit 1
    fi

    echo_info 'Checking for authorized_keys in /sshd'

    if [[ ! -f /sshd/authorized_keys ]]; then
        echo_err 'No authorized_keys file found in /sshd  Exiting..'
        exit 1
    fi

    echo_info 'Checking for sshd_config in /sshd'

    if [[ ! -f /sshd/sshd_config ]]; then
        echo_err 'No sshd_config file found in /sshd  Exiting..'
        exit 1
    fi

    echo_info "setting up .ssh directory"
    mkdir ~/.ssh
    cp /sshd/config ~/.ssh/
    cp /sshd/id_ed25519 /tmp
    chmod 400 /tmp/id_ed25519 ~/.ssh/config

    echo_info 'Starting SSHD..'
    /usr/sbin/sshd -f /sshd/sshd_config
fi
