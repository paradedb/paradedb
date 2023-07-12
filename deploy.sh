#!/usr/bin/env bash
set -e
KAFKA_HOST=localhost
KAFKA_PORT=9094
SCHEMA_REGISTRY_HOST=localhost
SCHEMA_REGISTRY_PORT=8081
SCHEMA_REGISTRY_INTERNAL_HOST=schema-registry
KAFKA_CONNECT_HOST=localhost
KAFKA_CONNECT_PORT=8083
ENV_DIR="$HOME/.config/retake"

# Update
sudo apt update

# Install git
sudo apt install -y git

# Clone repo
echo "Cloning Retake..."
git clone https://github.com/getretake/retake.git
cd retake
git checkout cli # Remove once cli branch is merged into main

# Write .env file
mkdir -p $ENV_DIR
cat <<EOF > $ENV_DIR/.env
KAFKA_HOST=$KAFKA_HOST
KAFKA_PORT=$KAFKA_PORT
SCHEMA_REGISTRY_HOST=$SCHEMA_REGISTRY_HOST
SCHEMA_REGISTRY_PORT=$SCHEMA_REGISTRY_PORT
SCHEMA_REGISTRY_INTERNAL_HOST=$SCHEMA_REGISTRY_INTERNAL_HOST
KAFKA_CONNECT_HOST=$KAFKA_CONNECT_HOST
KAFKA_CONNECT_PORT=$KAFKA_CONNECT_PORT
EOF

# Install Docker and Docker Compose
echo "Setting up Docker..."
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

# Install poetry
# Once the CLI is published, do pip install instead
echo "Installing Retake CLI..."
curl -sSL https://install.python-poetry.org | python3 -
export PATH="$HOME/.local/bin:$PATH"
cd realtime_server
poetry install