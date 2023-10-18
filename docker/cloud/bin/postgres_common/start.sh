#!/bin/bash
# Start script for compacted postgres operator image
# Used to run correct start script based on MODE

# Exit on subcommand errors
set -Eeuo pipefail

CRUNCHY_DIR=${CRUNCHY_DIR:-'/opt/crunchy'}
source "${CRUNCHY_DIR}/bin/common_lib.sh"
enable_debugging

env_check_err "MODE"

echo_info "Image mode found: ${MODE}"
case $MODE in
  postgres)
    echo_info "Starting in 'postgres' mode"
    exec "${CRUNCHY_DIR}/bin/postgres/start.sh"
    ;;
  *)
    echo_err "Invalid Image Mode; Please set the MODE environment variable to a supported mode"
    exit 1
    ;;
esac
