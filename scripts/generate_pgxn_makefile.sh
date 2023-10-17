#!/bin/bash

# This script generates a Makefile for bundling when publishing an extension to
# PGXN, so that the extension can be installed. It is intended to be used to make
# PGXN support pgrx-based extensions, which don't come with a standard PGXS Makefile.
# For anything development-related, please follow the instructions in the README of
# the extension and use 'pgrx' instead.

# Exit on subcommand errors
set -Eeuo pipefail

# Check if the correct number of arguments is provided
if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <path_to_directory> <postgres_major_version>"
  exit 1
fi

DIR="$1"
PG_MAJOR_VERSION="$2"

# Ensure the directory exists
if [ ! -d "$DIR" ]; then
  echo "Error: Directory $DIR does not exist."
  exit 1
fi

# Extract necessary fields from Cargo.toml
name=$(grep '^name =' "$DIR"/Cargo.toml | awk -F'\"' '{print $2}')
version=$(grep '^version =' "$DIR"/Cargo.toml | awk -F'\"' '{print $2}')

# Generate the Makefile in the specified directory
cat > "$DIR"/Makefile <<EOL
# This Makefile is used exclusively to install the extension via PGXN. For
# anything development related, please follow the instructions in the README
# and use 'pgrx' instead.

# Variables
EXTENSION = $name
BUILD_DIR = build

# Detect the OS
UNAME_S := \$(shell uname -s)

# Set pg_config path and PostgreSQL directories based on the OS and installation method
ifeq (\$(UNAME_S),Linux)
	PG_CONFIG = pg_config
endif
ifeq (\$(UNAME_S),Darwin) # This is macOS
	# Check if Postgres.app's pg_config is available
	ifeq (\$(wildcard /Applications/Postgres.app/Contents/Versions/*/bin/pg_config),)
		# Fallback to Homebrew's pg_config if Postgres.app's pg_config isn't found
		PG_CONFIG = /usr/local/bin/pg_config
	else
		PG_CONFIG = \$(wildcard /Applications/Postgres.app/Contents/Versions/*/bin/pg_config)
	endif
endif

PG_LIB_DIR = \$(shell \$(PG_CONFIG) --pkglibdir)
PG_EXTENSION_DIR = \$(shell \$(PG_CONFIG) --sharedir)/extension

# Default target
all: install

# Install the extension
install: \$(BUILD_DIR)
	cp \$(BUILD_DIR)/$name--$version/usr/lib/postgresql/$PG_MAJOR_VERSION/lib/\$(EXTENSION).so \$(PG_LIB_DIR)/
	cp \$(BUILD_DIR)/$name--$version/usr/share/postgresql/$PG_MAJOR_VERSION/extension/\$(EXTENSION)--$version.sql \$(PG_EXTENSION_DIR)/
	cp \$(BUILD_DIR)/$name--$version/usr/share/postgresql/$PG_MAJOR_VERSION/extension/\$(EXTENSION).control \$(PG_EXTENSION_DIR)/

# Clean up build artifacts
clean:
	rm -rf \$(BUILD_DIR)

# Phony targets
.PHONY: all install clean
EOL

echo "Makefile generated in $DIR successfully!"
