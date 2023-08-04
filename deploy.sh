#!/bin/bash
set -e

if [ $# -eq 0 ]; then
    # If no argument is provided, set domain to "localhost"
    domain="localhost"
else
    domain=$1
fi

# Write Caddyfile
cat << EOF > Caddyfile
$domain {
    handle_path /retake/* {
        reverse_proxy api:8000
    }
}
EOF

# Start cluster
docker compose up -d
