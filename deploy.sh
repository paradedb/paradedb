#!/bin/bash

# This script is inspired from PostHog and Airbyte, thanks to them for the inspiration!

set -Eeuo pipefail

RETAKE_API_KEY=$(head -c 28 /dev/urandom | sha224sum -b | head -c 56)
export RETAKE_API_KEY

# Make sure the console is huuuge
if test "$(tput cols)" -ge 64; then
  # Make it green!
  echo -e "\033[32m"
  echo -e "______ _____ _____ ___   _   __ _____ "
  echo -e "| ___ \  ___|_   _/ _ \ | | / /|  ___|"
  echo -e "| |_/ / |__   | |/ /_\ \| |/ / | |__  "
  echo -e "|    /|  __|  | ||  _  ||    \ |  __| "
  echo -e "| |\ \| |___  | || | | || |\  \| |___ "
  echo -e "\_| \_\____/  \_/\_| |_/\_| \_/\____/ "
  echo -e "                      Universal Search"
  # Make it less green
  echo -e "\033[0m"
  sleep 1
fi

# Talk to the user
echo "Welcome to the single instance Retake installer!"
echo ""
echo "Power user or aspiring power user?"
echo "Check out our docs on deploying Retake! https://docs.getretake.com/deployment/local"
echo ""

if [ -f ".env" ]; then
    echo "Environment file .env found, sourcing it."
    # shellcheck disable=SC1091
    source .env
else
    echo "Environment file .env not found. You will be prompted for the following environment variables: RETAKE_APP_TAG, DOMAIN"
    echo ""
fi

if [ -z "${RETAKE_APP_TAG:-}" ]; then
    echo "What version of Retake would you like to install? Browse available versions here: https://hub.docker.com/r/retake/retakesearch/tags"
    read -rp "Please enter a valid tag (i.e.: vX.Y.Z) or press Enter to default to 'latest': " RETAKE_APP_TAG
    if [ -z "$RETAKE_APP_TAG" ]; then
        RETAKE_APP_TAG="${RETAKE_APP_TAG:-latest}"
    fi
    echo "Using tag: $RETAKE_APP_TAG"
fi
echo ""

if [ -z "${DOMAIN:-}" ]; then
    echo "Let's get the exact domain Retake will be installed on. This will be used for TLS 🔐."
    echo "Make sure that you have a Host A DNS record pointing to this instance!"
    read -rp "Please enter your configured domain (i.e.: search.getretake.com): " DOMAIN
    if [ -z "$DOMAIN" ]; then
        echo "Domain not provided. Will default to localhost..."
        DOMAIN="localhost"
    fi
fi
echo ""

echo "Ok! we'll set up certs for https://$DOMAIN 🎉"
echo ""
echo "We will need sudo access so the next question is for you to give us superuser access"
echo "Please enter your sudo password now:"
sudo echo ""
echo "Thanks! 🙏"
echo ""
echo "Ok! We'll take it from here 🚀"
echo ""

# Retake uses basic telemetry to monitor usage (number of deployments, and number
# of search queries per deployment). If you prefer not to be included in our telemetry,
# simply set TELEMETRY=disabled in your .env file.
if [ -n "${TELEMETRY:-}" ]; then
    if [ "${TELEMETRY}" == "disabled" ]; then
        echo "Telemetry successfully disabled -- Retake will not get any usage data from your deployment."
        echo "Retake has very light telemetry (i.e.: is your deploy running, and how many search queries are you running?)."
        echo "We do this to get a sense of how much usage Retake is getting, which helps us prioritize features and support."
        echo "We never collect actual search queries, or PII. If this telemetry is okay with you, please consider re-enabling it."
        echo "Much love <3!"
    fi
fi

echo "Grabbing latest APT caches..."
sudo apt update

echo "Cloning Retake..."
sudo apt install -y git
# try to clone - if folder is already there pull latest for that branch
git clone https://github.com/getretake/retake.git &> /dev/null || true
cd retake

if [[ "$RETAKE_APP_TAG" = "latest" ]]
then
    echo "Pulling latest from current branch: $(git branch --show-current)"
    git pull
else
    releaseTag="${RETAKE_APP_TAG/release-/""}"
    git fetch --tags
    echo "Checking out Retake release: $releaseTag"
    git checkout "$releaseTag"
fi

echo "Making sure any stack that might exist is stopped..."
cd docker
sudo -E docker-compose -f docker-compose.prod.yml stop &> /dev/null || true

# Write Caddyfile
cat << EOF > Caddyfile
$DOMAIN :80 :443 {
    reverse_proxy api:8000
}
EOF

# Write .env file
cat << EOF > .env
API_KEY=$RETAKE_API_KEY
EOF

# Install Docker and Docker Compose
echo "Installing Docker..."
sudo apt install -y ca-certificates curl gnupg
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg

# shellcheck source=/dev/null
echo \
  "deb [arch=\"$(dpkg --print-architecture)\" signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  \"$(. /etc/os-release && echo "$VERSION_CODENAME")\" stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update
sudo apt -y install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Enable non-root docker
sudo usermod -aG docker "${USER}" || true

# Start stack
echo ""
echo "Starting docker compose..."
sudo -E docker compose -f docker-compose.prod.yml --env-file=.env up -d

echo ""
echo "⏳ Waiting for Retake to be ready (this will take a few minutes)"
bash -c 'while [[ "$(curl -s -o /dev/null -H "Authorization: Bearer $RETAKE_API_KEY" -w ''%{http_code}'' localhost/)" != "200" ]]; do sleep 5; done'
echo "⌛️ Retake looks up at https://${DOMAIN}!"
echo ""
echo "🎉🎉🎉 Done! 🎉🎉🎉"
echo ""
echo "To stop the stack, run 'docker compose -f docker-compose.prod.yml down'"
echo "To start the stack again, run 'docker compose -f docker-compose.prod.yml up'"
echo "You can find the Retake API key in the .env file that was generated for you"
echo "If you have any issues at all delete everything in this directory and run the curl command again"
echo "Happy searching!"
echo ""
