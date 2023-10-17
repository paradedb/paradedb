#!/bin/bash

# Copyright 2018 - 2023 Crunchy Data Solutions, Inc.
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

RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
RESET="\033[0m"

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}

function enable_debugging() {
    if [[ ${CRUNCHY_DEBUG:-false} == "true" ]]
    then
        echo_info "Turning debugging on.."
        export PS4='+(${BASH_SOURCE}:${LINENO})> ${FUNCNAME[0]:+${FUNCNAME[0]}(): }'
        set -x
    fi
}

function env_check_err() {
    if [[ -z ${!1} ]]
    then
        echo_err "$1 environment variable is not set, aborting."
        exit 1
    fi
}

function env_check_warn() {
    if [[ -z ${!1} ]]
    then
        echo_warn "$1 environment variable is not set."
    fi
}

function env_check_info() {
    if [[ ! -z ${!1} ]]
    then
        echo_info "$2"
    fi
}

function dir_check_err() {
    if [[ ! -d ${1} ]]
    then
        echo_err "The $1 directory does not exist and is required."
        exit 1
    fi
}

function echo_err() {
    echo -e "${RED?}$(date) ERROR: ${1?}${RESET?}"
}

function echo_info() {
    echo -e "${GREEN?}$(date) INFO: ${1?}${RESET?}"
}

function echo_warn() {
    echo -e "${YELLOW?}$(date) WARN: ${1?}${RESET?}"
}

function pgisready() {
    export PGROOT=$(find /usr/ -type d -name 'pgsql-*')
    local dbname=${1?}
    local dbhost=${2?}
    local dbport=${3?}
    local dbuser=${4?}
    local max_attempts=${5:-5}
    local timeout=${6:-2}
    test_server ${dbname?} ${dbhost?} ${dbport?} ${dbuser?} ${max_attempts?} ${timeout?}
    test_query ${dbname?} ${dbhost?} ${dbport?} ${dbuser?} ${max_attempts?} ${timeout?}
}

# Check if PostgreSQL is ready with exponential backoffs on attempts
function test_server() {
    local dbname=${1?}
    local dbhost=${2?}
    local dbport=${3?}
    local dbuser=${4?}
    local max_attempts=${5:-5}
    local timeout=${6:-2}
    local attempt=0
    local error='false'

    echo_info "Waiting for PostgreSQL to be ready.."
    while [[ ${attempt?} < ${max_attempts?} ]]
    do
        ${PGROOT?}/bin/pg_isready \
            --dbname=${dbname?} --host=${dbhost?} \
            --port=${dbport?} --username=${dbuser?}
        if [[ $? -eq 0 ]]
        then
            error='false'
            break
        fi
        error='true'
        sleep ${timeout?}
        attempt=$(( attempt + 1 ))
        timeout=$(( timeout * 2 ))
    done

    if [[ ${error?} == 'true' ]]
    then
        echo_err "Could not connect to PostgreSQL: Host=${dbhost?}:${dbport?} DB=${dbname?} User=${dbuser?}"
        exit 1
    fi
}

# Check if PostgreSQL is ready with exponential backoffs on attempts
function test_query {
    local dbname=${1?}
    local dbhost=${2?}
    local dbport=${3?}
    local dbuser=${4?}
    local max_attempts=${5:-5}
    local timeout=${6:-2}
    local attempt=0
    local error='false'

    echo_info "Checking if PostgreSQL is accepting queries.."
    while [[ ${attempt?} < ${max_attempts?} ]]
    do
        ${PGROOT?}/bin/psql \
            --dbname=${dbname?} --host=${dbhost?} \
            --port=${dbport?} --username=${dbuser?} \
            --command="SELECT now();"
        if [[ $? -eq 0 ]]
        then
            error='false'
            break
        fi
        error='true'
        sleep ${timeout?}
        attempt=$(( attempt + 1 ))
        timeout=$(( timeout * 2 ))
    done

    if [[ ${error?} == 'true' ]]
    then
        echo_err "Could not run query against PostgreSQL: Host=${dbhost?}:${dbport?} DB=${dbname?} User=${dbuser?}"
        exit 1
    fi
}

function err_check {
    RC=${1?}
    CONTEXT=${2?}
    ERROR=${3?}

    if [[ ${RC?} != 0 ]]
    then
        echo_err "${CONTEXT?}: ${ERROR?}"
        exit ${RC?}
    fi
}
