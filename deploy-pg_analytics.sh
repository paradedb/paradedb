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
ARCH=$(uname -m)
VERSION="unset"
TELEMETRY="unset"
EMAIL="unset"

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
    echo "Using default and installing latest pg_analytics"
    VERSION="latest"
  else
    echo "Using provided version: $VERSION"
  fi
fi

# Install dependencies
echo ""
echo "Installing dependencies 📦"
if [ "$OS" == "Linux" ]; then
  echo ""
  echo "We will need sudo access to interact install pg_search dependencies, so the next question is for you to give us superuser access."
  echo "Please enter your sudo password now:"
  sudo echo ""
  echo "Thanks! 🙏"
  echo ""
  echo "Ok! We'll take it from here 🚀"
  echo ""
  sudo apt-get install -y curl
elif [ "$OS" == "Darwin" ]; then
  brew install curl
else
  echo "Unsupported OS. Exiting..."
  exit 1
fi
echo "All clear!"

# Download and install the extension
curl -L "https://github.com/paradedb/paradedb/releases/download/$VERSION/pg_analytics-v$VERSION-pg${version}-$ARCH-ubuntu2204.deb" -o /tmp/pg_analytics.deb 
sudo apt-get install -y /tmp/pg_analytics.deb

echo "By default, the pg_analytics extension is installed with anonymous telemetry enabled."
echo "This allows us to collect non-identifiable information about the usage of the extension, which helps us improve it for everyone."
echo "If you prefer to disable telemetry, you can do so by running the following command:"
echo "export PARADEDB_TELEMERY=false"
echo ""
echo "Please consider enabling telemetry to help us improve the extension for everyone. 🙏"

# Ask the user for their email, if they want to share it
if [ "$EMAIL" == "unsert" ]; then
  echo "To help us better understand how you use pg_analytics, we would like to collect your email address."
  echo "We may reach out to you to ask for feedback or to share updates about the extension."
  echo "Your email will not be shared with any third parties and we won't send you any marketing emails."
  echo ""
  echo "Please enter your email address:"
  read -r EMAIL
  if [ -z "$EMAIL" ]; then
    echo "No email provided. Don't hesitate to come say hello in our Slack community: https://paradedb.com/slack"
  else
    echo "Thanks for sharing your email with us! We take your trust very seriously. 🙏"
  fi
fi

# Final instructions
echo "Extension installed. 🚀"
echo "Simply connect to your Postgres database via your tool of choice and run:"
echo "CREATE EXTENSION pg_analytics;"
echo ""
echo "To get started with pg_analytics, check out the docs: https://docs.paradedb.com/analytics/quickstart"
echo "To upgrade the extension, check out: https://docs.paradedb.com/upgrading"
echo ""
echo "🎉🎉🎉 You're all set, enjoy! 🎉🎉🎉"
