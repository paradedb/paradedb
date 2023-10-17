#!/bin/bash

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}
source "${CRUNCHY_DIR}/bin/common_lib.sh"
NAMESPACE=${HOSTNAME?}

if [[ -v PGDATA_PATH_OVERRIDE ]]
then
    NAMESPACE=${PGDATA_PATH_OVERRIDE?}
fi

# Set default pgbackrest env vars if not explicity provided
set_pgbackrest_env_vars() {

    if [[ ! -v PGBACKREST_STANZA ]]
    then
        export PGBACKREST_STANZA="db"
        default_env_vars+=("PGBACKREST_STANZA=${PGBACKREST_STANZA}")
    fi

    if [[ ! -v PGBACKREST_PG1_PATH ]] && [[ ! -v PGBACKREST_DB_PATH ]] \
      && [[ ! -v PGBACKREST_DB1_PATH ]]
    then
        export PGBACKREST_PG1_PATH="/pgdata/${NAMESPACE}"
        default_env_vars+=("PGBACKREST_PG1_PATH=${PGBACKREST_PG1_PATH}")
    fi

    if [[ ! -v PGBACKREST_REPO1_PATH ]] && [[ ! -v PGBACKREST_REPO_PATH ]]
    then
        export PGBACKREST_REPO1_PATH="/backrestrepo/${NAMESPACE}-backups"
        default_env_vars+=("PGBACKREST_REPO1_PATH=${PGBACKREST_REPO1_PATH}")
    fi

    if [[ ! -v PGBACKREST_LOG_PATH ]]
    then
        export PGBACKREST_LOG_PATH="/tmp"
        default_env_vars+=("PGBACKREST_LOG_PATH=${PGBACKREST_LOG_PATH}")
    fi

    if [[ "${PGBACKREST_ARCHIVE_ASYNC}" == "y" ]]
    then
        if [[ ! -v PGBACKREST_SPOOL_PATH ]] && [[ -v XLOGDIR ]]
        then
            export PGBACKREST_SPOOL_PATH="/pgwal/${NAMESPACE?}-spool"
            default_env_vars+=("PGBACKREST_SPOOL_PATH=${PGBACKREST_SPOOL_PATH}")
        elif [[ ! -v PGBACKREST_SPOOL_PATH ]]
        then
            export PGBACKREST_SPOOL_PATH="/pgdata/${NAMESPACE?}-spool"
            default_env_vars+=("PGBACKREST_SPOOL_PATH=${PGBACKREST_SPOOL_PATH}")
        fi
    fi

    if [[ ! ${#default_env_vars[@]} -eq 0 ]]
    then
        echo_info "pgBackRest: Defaults have been set for the following pgbackrest env vars:"
        echo_info "pgBackRest: [${default_env_vars[*]}]"
    fi
}

# Create default pgbackrest directories if they don't already exist
create_pgbackrest_dirs() {

    if [[ -v PGBACKREST_REPO_PATH ]]
    then
        repo_dir="${PGBACKREST_REPO_PATH}"
    else
        repo_dir="${PGBACKREST_REPO1_PATH}"
    fi
    
    if [[ ! -d "${repo_dir}" ]]
    then
        mkdir -p "${repo_dir}"
        echo_info "pgBackRest: Created pgbackrest repository directory ${repo_dir}"
    fi
    
    if [[ ! -d "${PGBACKREST_LOG_PATH}" ]]
    then
        mkdir -p "${PGBACKREST_LOG_PATH}"
        echo_info "pgBackRest: Created pgbackrest logging directory ${PGBACKREST_LOG_PATH}"
    fi

    # Only create spool directories if async archiving enabled
    if [[ "${PGBACKREST_ARCHIVE_ASYNC}" == "y" ]]
    then
        if [[ ! -d "${PGBACKREST_SPOOL_PATH}" ]]
        then
            mkdir -p "${PGBACKREST_SPOOL_PATH}"
            echo_info "pgBackRest: Created async archive spool directory ${PGBACKREST_SPOOL_PATH}"
        fi
    fi
}

set_pgbackrest_env_vars
create_pgbackrest_dirs

# Check if configuration is valid
if [[ "${BACKREST_SKIP_CREATE_STANZA}" == "true" ]]
then
    echo_info "pgBackRest: BACKREST_SKIP_CREATE_STANZA is 'true'.  Skipping configuration check.."
else
    echo_info "pgBackRest: Checking if configuration is valid.."
    pgbackrest info > /tmp/pgbackrest.stdout 2> /tmp/pgbackrest.stderr
    err=$?
    err_check ${err} "pgBackRest Configuration Check" \
        "Error with pgBackRest configuration: \n$(cat /tmp/pgbackrest.stderr)"
    if [[ ${err} == 0 ]]
    then
        echo_info "pgBackRest: Configuration is valid"
    fi
fi

# Create stanza
if [[ "${BACKREST_SKIP_CREATE_STANZA}" == "true" ]]
then
    echo_info "pgBackRest: BACKREST_SKIP_CREATE_STANZA is 'true'.  Skipping stanza creation.." 
else
    echo_info "pgBackRest: The following pgbackrest env vars have been set:"
    ( set -o posix ; set | grep -oP "^PGBACKREST.*" | sed -e 's/\(KEY\|PASS\|SECRET\)=.*/\1=*********/' )

    echo_info "pgBackRest: Executing 'stanza-create' to create stanza '${PGBACKREST_STANZA}'.."
    pgbackrest stanza-create --no-online --log-level-console=info \
        2> /tmp/pgbackrest.stderr
    err=$?
    err_check ${err} "pgBackRest Stanza Creation" \
        "Could not create a pgBackRest stanza: \n$(cat /tmp/pgbackrest.stderr)"
fi
