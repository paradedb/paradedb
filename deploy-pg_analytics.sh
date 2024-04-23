#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),  Display this help message"
  echo " -v (optional),  pg_search version to install. Defaults to 'latest'."
  echo " -t (optional),  Whether to enable telemetry. Defaults to 'true'."
  exit 1
}

# Variables
export DEBIAN_FRONTEND=noninteractive
OS=$(uname -s)
VERSION="unset"
TELEMETRY="unset"

# We don't yet support pre-built macOS binaries, so exit early
if [ "$OS" == "Darwin" ]; then
  echo ""
  echo "This script is not yet supported on macOS."
  echo "To install pg_analytics on macOS, please clone the ParadeDB repository and follow"
  echo "the instructions in the pg_analytics/README.md to compile and install the extension."
  echo ""
  exit 1
fi

# Assign flags to vars and check
while getopts "hv:d:s:t:" flag
do
  case $flag in
    h)
      usage
      ;;
    v)
      VERSION=$OPTARG
      ;;
    t)
      TELEMETRY=$OPTARG
      ;;
    *)
      usage
      ;;
  esac
done

# Talk to the user
echo ""
echo " ██████   ██████           █████  ███    ██  █████  ██      ██    ██ ████████ ██  ██████ ███████ "
echo " ██   ██ ██               ██   ██ ████   ██ ██   ██ ██       ██  ██     ██    ██ ██      ██      "
echo " ██████  ██   ███         ███████ ██ ██  ██ ███████ ██        ████      ██    ██ ██      ███████ "
echo " ██      ██    ██         ██   ██ ██  ██ ██ ██   ██ ██         ██       ██    ██ ██           ██ "
echo " ██       ██████  ███████ ██   ██ ██   ████ ██   ██ ███████    ██       ██    ██  ██████ ███████ "
echo "                                                                                     by ParadeDB "
echo ""
echo "Welcome to the self-hosted Postgres pg_analytics extension installation script! 🐘"
echo ""
echo "Power user or aspiring power user?"
echo "Check out our docs on deplying ParadeDB in production: https://docs.paradedb.com/deploy/"
echo ""

# Retrieve the pg_analytics version to install
if [ "$VERSION" == "unsert" ]; then
  echo "What version of pg_analytics would you like to install? (We default to 'latest')"
  echo "You can check out available versions here: https://hub.docker.com/r/paradedb/paradedb/tags"
  read -r VERSION
  if [ -z "$VERSION" ]; then
    echo "Using default and installing latest ParadeDB"
  else
    echo "Using provided tag: $VERSION"
  fi
fi

# Install dependencies
# There are no dependencies to install for pg_analytics

# TODO: Download + Install the extension .deb

# TODO: Set telemetry, tell user to turn it off

# TODO: Ask for their email

# TODO: Add final instructions on getting started and tell them to CREATE EXTENSION
