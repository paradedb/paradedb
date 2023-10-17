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
source "${CRUNCHY_DIR}/bin/common_lib.sh"
enable_debugging

function trap_sigterm() {
    echo_warn "Signal trap triggered, beginning shutdown.." >> $PGDATA/trap.output
    echo_warn "Signal trap triggered, beginning shutdown.."

    # Clean shutdowns begin here (force fast mode in case of PostgreSQL < 9.5)
    echo_info "Cleanly shutting down PostgreSQL in force fast mode.."
    PGCTLTIMEOUT=${PG_CTL_STOP_TIMEOUT} pg_ctl -w -D $PGDATA -m fast stop

    # Unclean shutdowns begin here (if all else fails)
    if [ -f $PGDATA/postmaster.pid ]; then
            kill -SIGINT $(head -1 $PGDATA/postmaster.pid) >> $PGDATA/trap.output
    fi
    if [[ ${ENABLE_SSHD} == "true" ]]; then
        echo_info "killing SSHD.."
        pkill sshd
    fi
}

trap 'trap_sigterm' SIGINT SIGTERM

source "${CRUNCHY_DIR}/bin/postgres/setenv.sh"
source check-for-secrets.sh

env_check_err "PG_MODE"

if [ "$PG_MODE" = "replica" ]; then
    env_check_err "PG_PRIMARY_HOST"
fi

env_check_err "PG_PRIMARY_USER"
env_check_err "PG_PRIMARY_PASSWORD"
env_check_err "PG_USER"
env_check_err "PG_PASSWORD"
env_check_err "PG_DATABASE"
env_check_err "PG_ROOT_PASSWORD"
env_check_err "PG_PRIMARY_PORT"

export PG_MODE=$PG_MODE
export PG_PRIMARY_HOST=$PG_PRIMARY_HOST
export PG_REPLICA_HOST=$PG_REPLICA_HOST
export PG_PRIMARY_PORT=$PG_PRIMARY_PORT
export PG_PRIMARY_USER=$PG_PRIMARY_USER
export PG_PRIMARY_PASSWORD=$PG_PRIMARY_PASSWORD
export PG_USER=$PG_USER
export PG_PASSWORD=$PG_PASSWORD
export PG_DATABASE=$PG_DATABASE
export PG_ROOT_PASSWORD=$PG_ROOT_PASSWORD

# allow for a custom timeout value for the "pg_ctl start" command
export PG_CTL_START_TIMEOUT="${PG_CTL_START_TIMEOUT:-60}"
# allow for a custom timeout value for the "pg_ctl stop" command
export PG_CTL_STOP_TIMEOUT="${PG_CTL_STOP_TIMEOUT:-60}"
# allow for a custom timeout value for the "pg_ctl promote" command
export PG_CTL_PROMOTE_TIMEOUT="${PG_CTL_PROMOTE_TIMEOUT:-60}"

echo_info "PG_CTL_START_TIMEOUT set at: ${PG_CTL_START_TIMEOUT}"
echo_info "PG_CTL_STOP_TIMEOUT set at: ${PG_CTL_STOP_TIMEOUT}"
echo_info "PG_CTL_PROMOTE_TIMEOUT set at: ${PG_CTL_PROMOTE_TIMEOUT}"

mkdir -p $PGDATA
chmod 0700 $PGDATA

if [[ -v ARCHIVE_MODE ]]; then
    if [ $ARCHIVE_MODE == "on" ]; then
        mkdir -p $PGWAL
        chmod 0700 $PGWAL
        echo_info "Creating wal directory in ${PGWAL?}.."
    fi
fi


function initdb_logic() {
    echo_info "Starting initdb.."

    cmd="initdb -D $PGDATA "
    if [[ -v PG_LOCALE ]]; then
        cmd+=" --locale="$PG_LOCALE
    fi

    if [[ -v XLOGDIR ]] && [[ ${XLOGDIR?} == "true" ]]
    then
        echo_info "XLOGDIR enabled.  Setting initdb to use ${PGWAL?}.."
        mkdir ${PGWAL?}

        if [[ -d "${PGWAL?}" ]]
        then
            cmd+=" -X "$PGWAL
        fi
    else
        echo_info "XLOGDIR not found. Using default pg_wal directory.."
    fi

    if [[ ${CHECKSUMS?} == 'true' ]]
    then
        echo_info "Data checksums enabled.  Setting initdb to use data checksums.."
        cmd+=" --data-checksums"
    fi
    cmd+=" > /tmp/initdb.stdout 2> /tmp/initdb.stderr"

    echo_info "Running initdb command: ${cmd?}"
    eval $cmd
    err_check "$?" "Initializing the database (initdb)" \
        "Unable to initialize the database: \n$(cat /tmp/initdb.stderr)"

    echo_info "Overlaying PostgreSQL's default configuration with customized settings.."
    cp /tmp/postgresql.conf $PGDATA

    cp "${CRUNCHY_DIR}/conf/postgres/pg_hba.conf" /tmp
    sed -i "s/PG_PRIMARY_USER/$PG_PRIMARY_USER/g" /tmp/pg_hba.conf
    cp /tmp/pg_hba.conf $PGDATA
}

function fill_conf_file() {
    env_check_info "TEMP_BUFFERS" "Setting TEMP_BUFFERS to ${TEMP_BUFFERS:-8MB}."
    env_check_info "LOG_MIN_DURATION_STATEMENT" "Setting LOG_MIN_DURATION_STATEMENT to ${LOG_MIN_DURATION_STATEMENT:-60000}."
    env_check_info "LOG_STATEMENT" "Setting LOG_STATEMENT to ${LOG_STATEMENT:-none}."
    env_check_info "MAX_CONNECTIONS" "Setting MAX_CONNECTIONS to ${MAX_CONNECTIONS:-100}."
    env_check_info "SHARED_BUFFERS" "Setting SHARED_BUFFERS to ${SHARED_BUFFERS:-128MB}."
    env_check_info "WORK_MEM" "Setting WORK_MEM to ${WORK_MEM:-4MB}."
    env_check_info "MAX_WAL_SENDERS" "Setting MAX_WAL_SENDERS to ${MAX_WAL_SENDERS:-6}."

    cp "${CRUNCHY_DIR}/conf/postgres/postgresql.conf.template" /tmp/postgresql.conf

    sed -i "s/TEMP_BUFFERS/${TEMP_BUFFERS:-8MB}/g" /tmp/postgresql.conf
    sed -i "s/LOG_MIN_DURATION_STATEMENT/${LOG_MIN_DURATION_STATEMENT:-60000}/g" /tmp/postgresql.conf
    sed -i "s/LOG_STATEMENT/${LOG_STATEMENT:-none}/g" /tmp/postgresql.conf
    sed -i "s/MAX_CONNECTIONS/${MAX_CONNECTIONS:-100}/g" /tmp/postgresql.conf
    sed -i "s/SHARED_BUFFERS/${SHARED_BUFFERS:-128MB}/g" /tmp/postgresql.conf
    sed -i "s/WORK_MEM/${WORK_MEM:-4MB}/g" /tmp/postgresql.conf
    sed -i "s/MAX_WAL_SENDERS/${MAX_WAL_SENDERS:-6}/g" /tmp/postgresql.conf
    sed -i "s/PG_PRIMARY_PORT/${PG_PRIMARY_PORT}/g" /tmp/postgresql.conf
}

function create_pgpass() {
    cd /tmp
cat >> ".pgpass" <<-EOF
*:*:*:*:${PG_PRIMARY_PASSWORD}
EOF
    chmod 0600 .pgpass
}

function waitforpg() {
    export PGPASSFILE=/tmp/.pgpass
    CONNECTED=false
    while true; do
        pg_isready --dbname=$PG_DATABASE --host=$PG_PRIMARY_HOST \
        --port=$PG_PRIMARY_PORT \
        --username=$PG_PRIMARY_USER --timeout=2
        if [ $? -eq 0 ]; then
            echo_info "The database is ready."
            break
        fi
        sleep 2
    done

    while true; do
        psql -h $PG_PRIMARY_HOST -p $PG_PRIMARY_PORT -U $PG_PRIMARY_USER $PG_DATABASE -f "${CRUNCHY_DIR}/bin/postgres/readiness.sql"
        if [ $? -eq 0 ]; then
            echo_info "The database is ready."
            CONNECTED=true
            break
        fi

        echo_info "Attempting pg_isready on primary.."
        sleep 2
    done

}

function initialize_replica() {
    echo_info "Initializing the replica."
    rm -rf $PGDATA/*
    chmod 0700 $PGDATA

    echo_info "Waiting to allow the primary database time to successfully start before performing the initial backup.."
    waitforpg

    pg_basebackup -X fetch --no-password --pgdata $PGDATA --host=$PG_PRIMARY_HOST \
        --port=$PG_PRIMARY_PORT -U $PG_PRIMARY_USER > /tmp/pgbasebackup.stdout 2> /tmp/pgbasebackup.stderr
    err_check "$?" "Initialize Replica" "Could not run pg_basebackup: \n$(cat /tmp/pgbasebackup.stderr)"

    # PostgreSQL recovery configuration.
    if [[ -v SYNC_REPLICA ]]; then
        echo_info "SYNC_REPLICA environment variable is set."
        APPLICATION_NAME=$SYNC_REPLICA
    else
        APPLICATION_NAME=$HOSTNAME
        echo_info "SYNC_REPLICA environment variable is not set."
    fi
    echo_info "${APPLICATION_NAME} is the APPLICATION_NAME being used."

    # PostgreSQL 12 changed how a replica/standby is set up. There is no longer
    # a recovery.conf file that indicates a PostgreSQL instance is a standby,
    # but rather one of two "signal" files that are available. The settings
    # for the PostgreSQL recovery/standby are kept in the main PostgreSQL.conf
    # file. As such we will fork off here, and allow each more to be st up.
    PG_VERSION=`cat $PGDATA/PG_VERSION`
    if [[ $PG_VERSION -ge 12 ]]; then
        initialize_replica_post12
    else
        initialize_replica_pre12
    fi
}

# PostgreSQL 12 moved all settings that affect a replica instance into the
# main "postgresql.conf" file and removed the "replica.conf" file that would
# put a PostgreSQL instance into recovery/standby mode. Now, in addition to the
# recovery settings being in the "postgresql.conf" file, one must add one of the
# following files in the $PGDATA directory to put the server into one of these
# modes:
#
# - "standby.signal": used for a replica that you want in a current, read-only
#                     state
# - "recovery.signal": used for a "targeted recovery", e.g. a
#                      point-in-time-recovery (PITR), which will stop replaying
#                      logs once the targeted recovery point is reached.
#
# For more info:
# https://www.postgresql.org/docs/current/runtime-config-wal.html#RUNTIME-CONFIG-WAL-ARCHIVE-RECOVERY
function initialize_replica_post12() {
    echo_info "Setting up recovery using methodology for PostgreSQL 12 and above."
    # set up a temporary file with recovery settings that we output to
    RECOVERY_FILE_TMP='/tmp/pgrepl-recovery.conf'
    echo_info "Preparing recovery settings in ${RECOVERY_FILE_TMP}"
    # first, create a temporary file to build up the recovery file separately.
    # use the legacy name to help
    touch $RECOVERY_FILE_TMP
    # As of PostgreSQL 12, the recovery_target_timeline defaults to "latest",
    # so we will not add that in.
    #
    # Also as of PostgreSQL 12, the "standby_mode" parameter is dropped, as that
    # is now dictated by which signal file is used.
    #
    # In PostgreSQL 12, "trigger_file" => "promote_trigger_file"
    echo "promote_trigger_file = '/tmp/pg-failover-trigger'" > $RECOVERY_FILE_TMP
    # the primary_conninfo string stays mostly the same
    PGCONF_PRIMARY_CONNINFO="application_name=${APPLICATION_NAME} host=${PG_PRIMARY_HOST} port=${PG_PRIMARY_PORT} user=${PG_PRIMARY_USER}"
    echo "primary_conninfo = '${PGCONF_PRIMARY_CONNINFO}'" >> $RECOVERY_FILE_TMP
    # append the contents of $RECOVERY_FILE_TMP to the postgresql.conf file
    cat $RECOVERY_FILE_TMP >> $PGDATA/postgresql.conf
    # and put the server into standby mode
    touch $PGDATA/standby.signal
}

# Before PostgreSQL 12, the way to set up a replica instance was to use a file
# called recovery.conf. The recovery.conf file serves as a way to indicate to
# PostgreSQL to boot up in "recovery" mode, and based on the settings, one could
# create a "hot standby" where read-only queries could be routed. This is the
# basis of streaming replication, etc. All the settings for setting up the file
# are documented here:
# https://www.postgresql.org/docs/11/recovery-config.html
function initialize_replica_pre12() {
    echo_info "Setting up recovery using methodology for PostgreSQL 11 and below."
    # Basically, we have a preconfigured recovery file with some settings in it,
    # And we substitute out some of the settings
    cp "${CRUNCHY_DIR}/conf/postgres/pgrepl-recovery.conf" /tmp
    sed -i "s/PG_PRIMARY_USER/$PG_PRIMARY_USER/g" /tmp/pgrepl-recovery.conf
    sed -i "s/PG_PRIMARY_HOST/$PG_PRIMARY_HOST/g" /tmp/pgrepl-recovery.conf
    sed -i "s/PG_PRIMARY_PORT/$PG_PRIMARY_PORT/g" /tmp/pgrepl-recovery.conf
    sed -i "s/APPLICATION_NAME/$APPLICATION_NAME/g" /tmp/pgrepl-recovery.conf
    cp /tmp/pgrepl-recovery.conf $PGDATA/recovery.conf
}

# Function to create the database if the PGDATA folder is empty, or do nothing if PGDATA
# is not empty.
function initialize_primary() {
    echo_info "Initializing the primary database.."
    if [ ! -f ${PGDATA?}/postgresql.conf ]; then
        ID="$(id)"
        echo_info "PGDATA is empty. ID is ${ID}. Creating the PGDATA directory.."
        mkdir -p ${PGDATA?}

        initdb_logic

        echo "Starting database.." >> /tmp/start-db.log

        echo_info "Temporarily starting database to run setup.sql.."
        PGCTLTIMEOUT=${PG_CTL_START_TIMEOUT} pg_ctl -D ${PGDATA?} \
            -o "-c listen_addresses='' ${PG_CTL_OPTS:-}" start \
            2> /tmp/pgctl.stderr
        err_check "$?" "Temporarily Starting PostgreSQL (primary)" \
            "Unable to start PostgreSQL: \n$(cat /tmp/pgctl.stderr)"

        echo_info "Waiting for PostgreSQL to start.."
        while true; do
            pg_isready \
            --host=/tmp \
            --port=${PG_PRIMARY_PORT} \
            --username=${PG_PRIMARY_USER?} \
            --timeout=2
            if [ $? -eq 0 ]; then
                echo_info "The database is ready for setup.sql."
                break
            fi
            sleep 2
        done


        echo_info "Loading setup.sql.." >> /tmp/start-db.log
        cp "${CRUNCHY_DIR}/bin/postgres/setup.sql" /tmp
        if [ -f /pgconf/setup.sql ]; then
            echo_info "Using setup.sql from /pgconf.."
            cp /pgconf/setup.sql /tmp
        fi
        sed -i "s/PG_PRIMARY_USER/$PG_PRIMARY_USER/g" /tmp/setup.sql
        sed -i "s/PG_PRIMARY_PASSWORD/$PG_PRIMARY_PASSWORD/g" /tmp/setup.sql
        sed -i "s/PG_USER/$PG_USER/g" /tmp/setup.sql
        sed -i "s/PG_PASSWORD/$PG_PASSWORD/g" /tmp/setup.sql
        sed -i "s/PG_DATABASE/$PG_DATABASE/g" /tmp/setup.sql
        sed -i "s/PG_ROOT_PASSWORD/$PG_ROOT_PASSWORD/g" /tmp/setup.sql

        # Set PGHOST to use the socket in /tmp. unix_socket_directory is changed
        # to use /tmp instead of /var/run.
        export PGHOST=/tmp
        psql -U postgres -p "${PG_PRIMARY_PORT}" < /tmp/setup.sql
        if [ -f /pgconf/audit.sql ]; then
            echo_info "Using pgaudit_analyze audit.sql from /pgconf.."
            psql -U postgres < /pgconf/audit.sql
        fi

        echo_info "Stopping database after primary initialization.."
        PGCTLTIMEOUT=${PG_CTL_STOP_TIMEOUT} pg_ctl -D $PGDATA --mode=fast stop

        if [[ -v SYNC_REPLICA ]]; then
            echo "Synchronous_standby_names = '"$SYNC_REPLICA"'" >> $PGDATA/postgresql.conf
        fi
    else
        echo_info "PGDATA already contains a database."
    fi
}

configure_archiving() {
    printf "\n# Archive Configuration:\n" >> /"${PGDATA?}"/postgresql.conf

    export ARCHIVE_MODE=${ARCHIVE_MODE:-off}
    export ARCHIVE_TIMEOUT=${ARCHIVE_TIMEOUT:-0}

    if [[ "${PGBACKREST}" == "true" ]]
    then
        export ARCHIVE_MODE=on
        echo_info "Setting pgbackrest archive command.."
        if [[ "${BACKREST_LOCAL_AND_S3_STORAGE}" == "true" ]]
        then
            cat "${CRUNCHY_DIR}/conf/postgres/backrest-archive-command-local-and-s3" >> /"${PGDATA?}"/postgresql.conf
        elif [[ "${BACKREST_LOCAL_AND_GCS_STORAGE}" == "true" ]]
        then
            cat "${CRUNCHY_DIR}/conf/postgres/backrest-archive-command-local-and-gcs" >> /"${PGDATA?}"/postgresql.conf
        else
            cat "${CRUNCHY_DIR}/conf/postgres/backrest-archive-command" >> /"${PGDATA?}"/postgresql.conf
        fi
    elif [[ "${ARCHIVE_MODE}" == "on" ]] && [[ ! "${PGBACKREST}" == "true" ]]
    then
        echo_info "Setting standard archive command.."
        cat "${CRUNCHY_DIR}/conf/postgres/archive-command" >> /"${PGDATA?}"/postgresql.conf
    fi

    echo_info "Setting ARCHIVE_MODE to ${ARCHIVE_MODE?}."
    echo "archive_mode = ${ARCHIVE_MODE?}" >> "${PGDATA?}"/postgresql.conf

    echo_info "Setting ARCHIVE_TIMEOUT to ${ARCHIVE_TIMEOUT?}."
    echo "archive_timeout = ${ARCHIVE_TIMEOUT?}" >> "${PGDATA?}"/postgresql.conf
}

# Clean up any old pid file that might have remained
# during a bad shutdown of the container/postgres
echo_info "Cleaning up the old postmaster.pid file.."
if [[ -f $PGDATA/postmaster.pid ]]
then
    rm $PGDATA/postmaster.pid
fi

ID="$(id)"
echo_info "User ID is set to ${ID}."

fill_conf_file

case "$PG_MODE" in
    "replica")
    echo_info "Working on replica.."
    create_pgpass
    export PGPASSFILE=/tmp/.pgpass
    if [ ! -f $PGDATA/postgresql.conf ]; then
        initialize_replica
    fi
    ;;
    "primary")
    echo_info "Working on primary.."
    initialize_primary
    ;;
    *)
    echo_err "PG_MODE is not an accepted value. Check that the PG_MODE environment variable is set to one of the two valid values (primary, replica)."
    ;;
esac

# Configure pgbackrest if enabled
if [[ ${PGBACKREST} == "true" ]]
then
    echo_info "pgBackRest: Enabling pgbackrest.."
    source "${CRUNCHY_DIR}/bin/postgres/pgbackrest.sh"
fi

configure_archiving

source "${CRUNCHY_DIR}/bin/postgres/custom-configs.sh"

# Run pre-start hook if it exists
if [ -f /pgconf/pre-start-hook.sh ]
then
	source /pgconf/pre-start-hook.sh
fi

# Start SSHD if necessary prior to starting PG
source "${CRUNCHY_DIR}/bin/postgres/sshd.sh"

echo_info "Starting PostgreSQL.."
postgres -D $PGDATA &

# Apply enhancement modules
for module in "${CRUNCHY_DIR}"/bin/modules/*.sh
do
    source ${module?}
done


# Run post start hook if it exists
if [ -f /pgconf/post-start-hook.sh ]
then
	source /pgconf/post-start-hook.sh
fi


# We will wait indefinitely until "docker stop [container_id]"
# When that happens, we route to the "trap_sigterm" function above
wait

echo_info "PostgreSQL is shutting down. Exiting.."
