#!/bin/bash

set -Eeuo pipefail

PGVER=18.6
EXTENSION=pg_search
PGRX_HOME=~/.pgrx

if (( $# < 3 )); then
  echo "usage: runheadless.sh <crate-name> <suite.toml> <timeout_ms> [logfile]"
  exit 1
fi

MANIFEST=~/_work/$1/Cargo.toml
MANIFESTDIR=$(dirname "${MANIFEST}")
SUITE="$2"
TIMEOUT="$3"
HERE=$(pwd)

LOGFILE=$(basename -- "${SUITE}")
LOGFILE="${LOGFILE%.*}.log"

if [ "${4:-""}" != "" ]; then
  LOGFILE="$4"
fi

cd "${MANIFESTDIR}" || exit
cargo pgrx install --profile prof --manifest-path "${MANIFEST}" --package "${EXTENSION}" --pg-config "${PGRX_HOME}/${PGVER}/pgrx-install/bin/pg_config"

cd "${HERE}" || exit
pwd
cargo run --release -- headless "${SUITE}" --log-file="${LOGFILE}" --runtime "${TIMEOUT}"

cargo run --release -- graph "${LOGFILE}" "${LOGFILE}".png && open "${LOGFILE}".png
