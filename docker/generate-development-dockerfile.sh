#!/usr/bin/env bash
# Render the normal runtime with a locally built pg_search.deb instead of a release download.
# Keep runtime dependencies, configuration, and bootstrap behavior in Dockerfile.template.
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
awk '
  BEGIN { include = 1; standard = 0 }
  /^# %%ANTITHESIS_BEGIN%%$/ { include = 0; next }
  /^# %%OFFICIAL_BEGIN%%$/ { include = 0; next }
  /^# %%BARMAN_BEGIN%%$/ { include = 1; next }
  /^# %%STANDARD_BEGIN%%$/ {
    standard++
    print "# Install the development package built by this workflow."
    print "COPY pg_search.deb /tmp/pg_search.deb"
    print "RUN apt-get update && apt-get install -y --no-install-recommends /tmp/pg_search.deb && rm /tmp/pg_search.deb && rm -rf /var/lib/apt/lists/*"
    include = 0
    next
  }
  /^# %%(ANTITHESIS|BARMAN|OFFICIAL|STANDARD)_END%%$/ { include = 1; next }
  !include { next }
  { gsub(/@@PG_VERSION_MAJOR@@/, "18"); print }
  END { if (standard != 1) exit 1 }
' "${script_dir}/Dockerfile.template"
