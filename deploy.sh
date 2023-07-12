#!/usr/bin/env bash
set -e

# Default values, these match with the docker-compose.yaml configuration
KAFKA_HOST=localhost
KAFKA_PORT=9094
SCHEMA_REGISTRY_HOST=localhost
SCHEMA_REGISTRY_PORT=8081
SCHEMA_REGISTRY_INTERNAL_HOST=schema-registry
KAFKA_CONNECT_HOST=localhost
KAFKA_CONNECT_PORT=8083
ENV_DIR="$HOME/.config/retake"

get_external_ip() {
  # Query Google Cloud metadata endpoint
  ip_address=$(curl -s "http://metadata.google.internal/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip" -H "Metadata-Flavor: Google")

  if [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  fi

  # Query AWS EC2 metadata endpoint
  ip_address=$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4)

  if [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  fi

  # Query Azure metadata endpoint
  ip_address=$(curl -s "http://169.254.169.254/metadata/instance/network/interface/0/ipv4/ipAddress/0/publicIpAddress?api-version=2021-07-01&format=text")

  if [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  fi

  # Cloud agnostic way
  ip_address=$(curl -4 icanhazip.com)
  if [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  fi

  # If no IP address found, stop the script
  echo "No external IP address found. Exiting."
  exit 1
}

# Start deploy.sh

# Update
sudo apt update

# Install git
sudo apt install -y git

# Clone repo
echo "Cloning Retake..."
git clone https://github.com/getretake/retake.git
cd retake

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

# Set advertise listener as this machine's ip address
get_external_ip
sed -i "s/placeholder_listener/$ip_address/g" docker-compose.yaml

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

# Install retake-cli
echo "Installing Retake CLI..."
pip install retake-cli