#!/bin/bash

# Exit on subcommand errors
set -Eeuo pipefail

# Default values, these match with the docker-compose.yaml configuration
KAFKA_HOST=localhost
KAFKA_PORT=9094
SCHEMA_REGISTRY_HOST=localhost
SCHEMA_REGISTRY_PORT=8081
SCHEMA_REGISTRY_INTERNAL_HOST=schema-registry
KAFKA_CONNECT_HOST=localhost
KAFKA_CONNECT_PORT=8083
ENV_DIR="$HOME/.config/retake"
GIT_BRANCH="main"

# Parse command line options
VALID_ARGS=$(getopt -o b: --long branch: -- "$@")
if [[ $? -ne 0 ]]; then
  exit 1;
fi

eval set -- "$VALID_ARGS"
while [ : ]; do
  case "$1" in
    -b | --branch)
      GIT_BRANCH="$2"
      shift 2
      ;;
    --) shift;
      break
      ;;
  esac
done

get_external_ip() {
  # Query Google Cloud metadata endpoint
  echo "Trying gcloud..."
  response=$(curl -s -w '%{http_code}\n' "http://metadata.google.internal/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip" -H "Metadata-Flavor: Google")
  ip_address=$(echo "$response" | head -n 1)
  status_code=$(echo "$response" | tail -n 1)

  if [ "$status_code" -eq 200 ] && [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  else
    echo "Failed to retrieve IP address from gcloud"
  fi

  # Query AWS EC2 metadata endpoint
  echo "Trying AWS..."
  response=$(curl -s -w '%{http_code}\n' http://169.254.169.254/latest/meta-data/public-ipv4)
  ip_address=$(echo "$response" | head -n 1)
  status_code=$(echo "$response" | tail -n 1)

  if [ "$status_code" -eq 200 ] && [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  else
    echo "Failed to retrieve IP address from AWS"
  fi

  # Query Azure metadata endpoint
  echo "Trying Azure..."
  response=$(curl -s -w '%{http_code}\n' "http://169.254.169.254/metadata/instance/network/interface/0/ipv4/ipAddress/0/publicIpAddress?api-version=2021-07-01&format=text")
  ip_address=$(echo "$response" | head -n 1)
  status_code=$(echo "$response" | tail -n 1)

  if [ "$status_code" -eq 200 ] && [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  else
    echo "Failed to retrieve IP address from Azure"
  fi

  # Cloud agnostic way
  echo "Trying icanhazip..."
  response=$(curl -4 -s -w '%{http_code}\n' icanhazip.com)
  ip_address=$(echo "$response" | head -n 1)
  status_code=$(echo "$response" | tail -n 1)

  if [ "$status_code" -eq 200 ] && [ -n "$ip_address" ]; then
    echo "$ip_address"
    return
  else
    echo "Failed to retrieve IP address from icanhazip"
  fi

  # If no IP address found, stop the script
  echo "No external IP address found. Exiting."
  exit 1
}

# Start deploy.sh

# Update
sudo apt-get update

# Install git
sudo apt-get install -y git

# Clone repo
echo "Cloning Retake..."
git clone https://github.com/getretake/retake.git
cd retake
git checkout "$GIT_BRANCH"

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

# # Set advertise listener as this machine's ip address
echo "Getting external ip"
get_external_ip
echo $ip_address
sed -i "s/localhost/$ip_address/g" docker-compose.yaml

# Install Docker and Docker Compose
echo "Setting up Docker..."
sudo apt-get install -y ca-certificates curl gnupg
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg
echo "deb [arch="$(dpkg --print-architecture)" signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  "$(. /etc/os-release && echo "$VERSION_CODENAME")" stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt-get update
sudo apt-get -y install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Enable non-root docker
sudo usermod -aG docker "${USER}" || true

# Start stack
echo "Starting docker compose..."
sudo -E docker compose up -d
