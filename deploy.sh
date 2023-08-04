#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Domain not provided."
    echo "Usage: $0 <domain>"
    echo "Please ensure that the server running this script has a DNS A record pointing to it from the provided domain."
    exit 1
fi

DOMAIN=$1

if [ ! -z "$2" ]; then
    echo "Using 'dev' branch as default."
    GIT_BRANCH="dev"
else
    GIT_BRANCH=$2
fi

# Clone repo
echo "Cloning Retake..."
git clone https://github.com/getretake/retake.git
cd retake
git checkout "$GIT_BRANCH"

# Write Caddyfile
cat << EOF > Caddyfile
$DOMAIN {
    reverse_proxy api:8000
}
EOF

# Install Docker and Docker Compose
echo "Installing Docker..."
sudo apt-get install -y ca-certificates curl gnupg
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg
echo \
  "deb [arch="$(dpkg --print-architecture)" signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  "$(. /etc/os-release && echo "$VERSION_CODENAME")" stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update
sudo apt-get -y install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Enable non-root docker
sudo usermod -aG docker "${USER}" || true

# Start stack
echo "Starting docker compose..."
sudo -E docker compose up -d
