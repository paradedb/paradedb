#!/bin/bash

set -Eeuo pipefail

PGVER=18.6
EXTENSION=pg_search
PGRX_HOME=~/.pgrx

if (( $# < 2 )); then
  echo "usage: runtui.sh <crate-name> <suite.toml>"
  exit 1
fi

MANIFEST=~/_work/$1/Cargo.toml
MANIFESTDIR=$(dirname "${MANIFEST}")
SUITE="$2"
HERE=$(pwd)

cd "${MANIFESTDIR}" || exit
cargo pgrx install --profile prof --manifest-path "${MANIFEST}" --package "${EXTENSION}" --pg-config "${PGRX_HOME}/${PGVER}/pgrx-install/bin/pg_config"

cd "${HERE}" || exit
pwd
cargo run --release -- ui "${SUITE}" --paused
