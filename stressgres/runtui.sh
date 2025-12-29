#!/bin/bash

set +x

PGVER=18.1
EXTENSION=pg_search
MANIFEST=~/_work/$1/Cargo.toml
MANIFESTDIR=$(dirname "${MANIFEST}")
PGRX_HOME=~/.pgrx

SUITE="$2"
HERE=$(pwd)

if [ "$2" = "" ]; then
	echo "usage: runtui.sh <crate-name> <suite.toml>"
	exit 1
fi

set -x
cd "${MANIFESTDIR}" || exit
cargo pgrx install --profile prof --manifest-path "${MANIFEST}" --package ${EXTENSION} --features=icu --pg-config ${PGRX_HOME}/${PGVER}/pgrx-install/bin/pg_config || exit $?

cd "${HERE}" || exit
pwd
cargo run --release ui "${SUITE}" --paused
