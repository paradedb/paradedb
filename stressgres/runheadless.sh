#!/bin/bash

set +x

PGVER=18.1
EXTENSION=pg_search
MANIFEST=~/_work/$1/Cargo.toml
MANIFESTDIR=$(dirname "${MANIFEST}")
PGRX_HOME=~/.pgrx

SUITE="$2"
TIMEOUT="$3"
HERE=$(pwd)

if [ "$2" = "" ] || [ "$3" = "" ]; then
	echo "usage: runheadless.sh <crate-name> <suite.toml> <timeout_ms> [logfile]"
	exit 1
fi

LOGFILE=$(basename -- "${SUITE}")
LOGFILE="${LOGFILE%.*}.log"

if [ "$4" != "" ]; then
  LOGFILE="$4"
fi

set -x
cd "${MANIFESTDIR}" || exit
cargo pgrx install --profile prof --manifest-path "${MANIFEST}" --package ${EXTENSION} --features=icu --pg-config ${PGRX_HOME}/${PGVER}/pgrx-install/bin/pg_config || exit $?

cd "${HERE}" || exit
pwd
cargo run --release headless "${SUITE}" --log-file="${LOGFILE}" --runtime ${TIMEOUT}

cargo run --release -- graph "${LOGFILE}" "${LOGFILE}".png && open "${LOGFILE}".png
