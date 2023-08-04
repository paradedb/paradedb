#!/bin/bash
set -e

if [ $# -eq 0 ]; then
    echo "Please provide a domain as a positional argument."
    exit 1
fi

DOMAIN=$1

# Write Caddyfile
cat << EOF > Caddyfile
$DOMAIN {
    handle_path /retake/* {
        reverse_proxy api:8000
    }
}
EOF

# Start cluster
docker compose up -d
