#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Check if a directory argument is provided
if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <path_to_directory>"
  exit 1
fi

DIR="$1"

# Ensure the directory exists
if [ ! -d "$DIR" ]; then
  echo "Error: Directory $DIR does not exist."
  exit 1
fi

# Extract necessary fields from Cargo.toml
name=$(grep '^name =' "$DIR"/Cargo.toml | awk -F'\"' '{print $2}')
version=$(grep '^version =' "$DIR"/Cargo.toml | awk -F'\"' '{print $2}')
description=$(grep '^description =' "$DIR"/Cargo.toml | awk -F'\"' '{print $2}')

# Generate the META.json content in the specified directory
cat > "$DIR"/META.json <<EOL
{
    "name": "$name",
    "version": "$version",
    "abstract": "$description",
    "description": "$description",
    "license": "agpl_3",
    "maintainer": "ParadeDB <support@paradedb.com>",
    "provides": {
        "$name": {
            "file": "sql/$name.sql",
            "version": "$version"
        }
    },
    "prereqs": {
        "runtime": {
            "requires": {
                "PostgreSQL": "11.0.0"
            }
        },
        "test": {
            "requires": {
                "plpgsql": "0"
            }
        }
    },
    "resources": {
        "bugtracker": {
            "web": "https://github.com/paradedb/paradedb/issues"
        },
        "repository": {
            "web": "https://github.com/paradedb/paradedb",
            "url": "git://github.com/paradedb/paradedb.git",
            "type": "git"
        }
    },
    "generated_by": "ParadeDB",
    "meta-spec": {
        "version": "1.0.0",
        "url": "http://pgxn.org/meta/spec.txt"
    },
    "tags": [
        "search"
    ]
}
EOL

echo "META.json generated in $DIR successfully!"
