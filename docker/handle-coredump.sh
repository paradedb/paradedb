#!/bin/bash
set -Eeuo pipefail

# This script is used by the kernel to handle core dumps.
# It writes the core dump to the dedicated coredumps volume.

COREDUMPS_VOLUME="/var/lib/postgresql/tablespaces/coredumps"

if [ ! -d "$COREDUMPS_VOLUME" ]; then
  echo "Error: Coredumps volume ($COREDUMPS_VOLUME) not found. Exiting." >&2
  exit 1
fi

# The core dump file will be created by root, so we chown it to the postgres user.
# The filename includes the PID of the crashing process.
CORE_FILE="$COREDUMPS_VOLUME/core.$1"
cat > "$CORE_FILE"
chown postgres:postgres "$CORE_FILE"
