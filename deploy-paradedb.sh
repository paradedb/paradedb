#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Handle params
usage() {
  echo "Usage: $0 [OPTIONS]"
  echo "Options:"
  echo " -h (optional),  Display this help message"
  echo " -v (optional),  ParadeDB version to install. Defaults to 'latest'."
  echo " -d (optional),  Whether to run Docker Compose in detached mode. Defaults to 'false'."
  echo " -s (optional),  Whether to persist the Docker volume for ParadeDB. Defaults to 'false'."
  echo " -t (optional),  Whether to enable telemetry. Defaults to 'true'."
  exit 1
}

# Variables
export DEBIAN_FRONTEND=noninteractive
OS=$(uname -s)
ARCH=$(uname -m)
PERSIST_VOLUME="unset"
RUN_DETACHED="unset"
VERSION="unset"
TELEMETRY="unset"

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
    d)
      RUN_DETACHED=$OPTARG
      ;;
    s)
      PERSIST_VOLUME=$OPTARG
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
echo " ██████   █████  ██████   █████  ██████  ███████ ██████  ██████  "
echo " ██   ██ ██   ██ ██   ██ ██   ██ ██   ██ ██      ██   ██ ██   ██ "
echo " ██████  ███████ ██████  ███████ ██   ██ █████   ██   ██ ██████  "
echo " ██      ██   ██ ██   ██ ██   ██ ██   ██ ██      ██   ██ ██   ██ "
echo " ██      ██   ██ ██   ██ ██   ██ ██████  ███████ ██████  ██████  "
echo "                               Postgres for Search and Analytics "
echo ""
echo "Welcome to the single instance ParadeDB installation script! 🐘"
echo ""
echo "⚠️ We recommend at least 2 GBs of memory and 10 GBs of disk space to run this stack ⚠️"
echo ""
echo "Power user or aspiring power user?"
echo "Check out our docs on deplying ParadeDB in production: https://docs.paradedb.com/deploy/"
echo ""

# Retrieve the ParadeDB version to install
if [ "$VERSION" == "unsert" ]; then
  echo "What version of ParadeDB would you like to install? (We default to 'latest')"
  echo "You can check out available versions here: https://hub.docker.com/r/paradedb/paradedb/tags"
  read -r VERSION
  if [ -z "$VERSION" ]; then
    echo "Using default and installing latest ParadeDB"
  else
    echo "Using provided tag: $VERSION"
  fi
fi

# Ensure any existing stack is stopped
if [ "$OS" == "Linux" ];
  echo ""
  echo "We will need sudo access to interact with Docker Compose, so the next question is for you to give us superuser access."
  echo "Please enter your sudo password now:"
  sudo echo ""
  echo "Thanks! 🙏"
  echo ""
  echo "Ok! We'll take it from here 🚀"
  echo ""
  echo "Making sure any previous ParadeDB stack that might exist is stopped..."
  sudo -E docker-compose -f docker-compose.yml stop &> /dev/null || true
elif [ "$OS" == "Darwin" ]; then
  echo "Making sure any previous ParadeDB stack that might exist is stopped..."
  docker-compose -f docker-compose.yml stop &> /dev/null || true
else
  echo "Unsupported OS. Exiting..."
  exit 1
fi
echo "All clear!"

# On Linux, make sure we have the latest APT caches
if [ "$OS" == "Linux" ]; then
  echo ""
  echo "Grabbing the latest APT caches..."
  sudo apt-get update
  echo "All done!"
fi



# Install dependencies
echo ""
echo "Installing dependencies 📦"
if [ "$OS" == "Linux" ]; then
  sudo apt-get install -y git
elif [ "$OS" == "Darwin" ]; then
  brew install git
else
  echo "Unsupported OS. Exiting..."
  exit 1
fi



# Check if Docker is already installed
if ! command -v docker &> /dev/null; then
    echo "Docker is not installed. Setting up Docker."

    # Setup Docker
    sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo -E apt-key add -
    sudo add-apt-repository -y "deb [arch=amd64] https://download.docker.com/linux/ubuntu bionic stable"
    sudo apt update
    sudo apt-cache policy docker-ce
    sudo apt install -y docker-ce
else
    echo "Docker is already installed. Skipping installation."
fi

# setup docker-compose
echo "Setting up Docker Compose"
sudo curl -L "https://github.com/docker/compose/releases/download/v2.13.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose || true
sudo chmod +x /usr/local/bin/docker-compose

# enable docker without sudo
sudo usermod -aG docker "${USER}" || true






# Install ParadeDB
echo ""
echo "Installing ParadeDB 🐘 from Github"
git clone https://github.com/paradedb/paradedb.git &> /dev/null || true
cd paradedb/









# clone posthog
echo "Installing PostHog 🦔 from Github"
# try to clone - if folder is already there pull latest for that branch
git clone https://github.com/PostHog/posthog.git &> /dev/null || true
cd posthog

if [[ "$POSTHOG_APP_TAG" = "latest-release" ]]
then
    git fetch --tags
    latestReleaseTag=$(git describe --tags `git rev-list --tags --max-count=1`)
    echo "Checking out latest PostHog release: $latestReleaseTag"
    git checkout $latestReleaseTag
elif [[ "$POSTHOG_APP_TAG" = "latest" ]]
then
    echo "Pulling latest from current branch: $(git branch --show-current)"
    git pull
elif [[ "$POSTHOG_APP_TAG" =~ ^[0-9a-f]{40}$ ]]
then
    echo "Checking out specific commit hash: $POSTHOG_APP_TAG"
    git checkout $POSTHOG_APP_TAG
else
    releaseTag="${POSTHOG_APP_TAG/release-/""}"
    git fetch --tags
    echo "Checking out PostHog release: $releaseTag"
    git checkout $releaseTag
fi

cd ..



# Set telemetry, and tell the user they can turn it off


# Ask for their email to see if we can help

# Add another step to configure the SSL on the Bitnami container
# 


# Add another step about whether to add a volume to persist ParadeDB


# Start with docker compose, tell them they can update teh values
# add information about logical replication and other eventually

# start up the stack
echo "Configuring Docker Compose...."
echo "Starting the stack!" 
sudo -E docker-compose -f docker-compose.yml up -d




# Need to wait 10 seconds for Docker compose and Postgres to get going


# send telemetry


# add instructions on how to stop it


echo "We will need to wait ~5-10 minutes for things to settle down, migrations to finish, and TLS certs to be issued"
echo ""
echo "⏳ Waiting for PostHog web to boot (this will take a few minutes)"
bash -c 'while [[ "$(curl -s -o /dev/null -w ''%{http_code}'' localhost/_health)" != "200" ]]; do sleep 5; done'
echo "⌛️ PostHog looks up!"
echo ""
echo "🎉🎉🎉  Done! 🎉🎉🎉"
# send log of this install for continued support!
curl -o /dev/null -L --header "Content-Type: application/json" -d "{
    \"api_key\": \"sTMFPsFhdP1Ssg\",
    \"distinct_id\": \"${DOMAIN}\",
    \"properties\": {\"domain\": \"${DOMAIN}\"},
    \"type\": \"capture\",
    \"event\": \"magic_curl_install_complete\"
}" https://us.i.posthog.com/batch/ &> /dev/null
echo ""
echo "To stop the stack run 'docker-compose stop'"
echo "To start the stack again run 'docker-compose start'"
echo "If you have any issues at all delete everything in this directory and run the curl command again"
echo ""
echo 'To upgrade: run /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/posthog/posthog/HEAD/bin/upgrade-hobby)"'
echo ""
echo "PostHog will be up at the location you provided!"
echo "https://${DOMAIN}"
echo ""
echo "It's dangerous to go alone! Take this: 🦔"


