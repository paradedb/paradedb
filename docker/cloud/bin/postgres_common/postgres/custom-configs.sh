#!/bin/bash

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}
source "${CRUNCHY_DIR}/bin/common_lib.sh"
enable_debugging

function custom_config() {
    src=${1?}
    dest=${2?}
    mode=${3?}
    if [[ -f ${src?} ]]
    then
        echo_info "Custom ${src?} detected.  Applying custom configuration.."

        cp ${src?} ${dest?}
        err_check "$?" "Applying custom configuration" "Could not copy ${src?} to ${dest?}"

        chmod ${mode?} ${dest?}
        err_check "$?" "Applying custom configuration" "Could not set mode ${mode?} on ${dest?}"
    fi
}

custom_config "/pgconf/postgresql.conf" "${PGDATA?}/postgresql.conf" 600
custom_config "/pgconf/pg_hba.conf" "${PGDATA?}/pg_hba.conf" 600
custom_config "/pgconf/pg_ident.conf" "${PGDATA?}/pg_ident.conf" 600
custom_config "/pgconf/server.key" "${PGDATA?}/server.key" 600
custom_config "/pgconf/server.crt" "${PGDATA?}/server.crt" 600
custom_config "/pgconf/ca.crt" "${PGDATA?}/ca.crt" 600
custom_config "/pgconf/ca.crl" "${PGDATA?}/ca.crl" 600
