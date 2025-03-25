#!/bin/bash

# This script prompts the user for installation options and installs ParadeDB on their system. It
# is *not* intended for production use. The script is included in our website and can be curl'd.

set -Eeuo pipefail

########################################
# Variables
########################################

ARCH=$(uname -m)
LATEST_RELEASE_TAG=$(curl -s "https://api.github.com/repos/paradedb/paradedb/releases/latest" | jq -r .tag_name)
LATEST_RELEASE_VERSION="${LATEST_RELEASE_TAG#v}"
PG_MAJOR_VERSION=""

########################################
# Helper Functions
########################################

function isRHEL() {
  cat /etc/redhat-release >/dev/null 2>&1
}

function selectPostgresVersion() {
  echo -e ""
  echo -e "Please select your desired Postgres major version:"
  PG_VERSION_OPTIONS=("17" "16" "15" "14")
  select opt in "${PG_VERSION_OPTIONS[@]}"; do
    if [[ "$REPLY" == "1" || "$REPLY" == "17" ]]; then
      echo -e "Selected Postgres major version: 17"
      PG_MAJOR_VERSION="17"
      break
    elif [[ "$REPLY" == "2" || "$REPLY" == "16" ]]; then
      echo -e "Selected Postgres major version: 16"
      PG_MAJOR_VERSION="16"
      break
    elif [[ "$REPLY" == "3" || "$REPLY" == "15" ]]; then
      echo -e "Selected Postgres major version: 15"
      PG_MAJOR_VERSION="15"
      break
    elif [[ "$REPLY" == "4" || "$REPLY" == "14" ]]; then
      echo -e "Selected Postgres major version: 14"
      PG_MAJOR_VERSION="14"
      break
    else
      echo "Invalid option. You must select one of '14', '15', '16' or '17'. Exiting..."
      exit 9
      break
    fi
    echo -e ""
  done
}


# TODO: Add support for installing ParadeDB with a local Kubernetes cluster here
function installKubernetes() {
  echo -e ""
  echo -e "To install ParadeDB on Kubernetes, please refer to our documentation: https://docs.paradedb.com/deploy/self-hosted/kubernetes"
}


function installDocker() {
  # Set default values
  pguser="myuser"
  pgpass="mypassword"
  dbname="paradedb"

  # Check that Docker is installed
  if ! command -v docker >/dev/null 2>&1; then
    echo -e ""
    echo -e "Docker not found. Please install Docker before continuing."
    exit 2
  fi

  if ! docker info >/dev/null 2>&1; then
    echo -e ""
    echo -e "Docker daemon is not running. Please start Docker before continuing."
    exit 3
  fi

  # Check for existing ParadeDB Docker container(s)
  if docker inspect "paradedb" > /dev/null 2>&1; then
    echo -e ""
    echo -e "An existing ParadeDB Docker container was found on your system. To avoid conflicts, please remove it before continuing."
    exit 4
  fi

  # Prompt the user for their desired Postgres major version 
  selectPostgresVersion

  # Prompt the user for their desired database credentials
  echo -e ""
  read -r -p "ParadeDB database user name (default: myuser): " input
  pguser="${input:-myuser}"
  read -r -p "ParadeDB database user password (default: mypassword): " input
  pgpass="${input:-mypassword}"
  read -r -p "ParadeDB database name (default: paradedb): " input
  dbname="${input:-paradedb}"

  # Prompt the user for whether they want to use a Docker volume
  echo -e ""
  echo -e "Would you like to add a Docker volume to your ParadeDB container? The volume will be named 'paradedb_data' and will ensure that your database persists across Docker restarts."
  VOLUME_OPTIONS=("Yes" "No")
  select opt in "${VOLUME_OPTIONS[@]}"; do
    extra_opts=""
    if [[ "$REPLY" == "1" || "$REPLY" == "Yes" ]]; then
      echo -e ""
      echo -e "Adding volume 'paradedb_data'"
      extra_opts="-v paradedb_data:/var/lib/postgresql/data/"
    fi

    # TODO: Replace `latest` by pg-major-version and latest tag
    echo -e ""
    echo -e "Running paradedb/paradedb:latest Docker container (${LATEST_RELEASE_TAG})..."
    docker run \
      --name paradedb \
      -e POSTGRES_USER="$pguser" \
      -e POSTGRES_PASSWORD="$pgpass" \
      -e POSTGRES_DB="$dbname" \
      $extra_opts \
      -p 5432:5432 \
      -d paradedb/paradedb:latest || { echo -e "Failed to start Docker container. Please check if an existing container is active or not."; exit 4; }
    break
  done
  echo -e ""
  echo -e "🚀 ParadeDB Docker Container started! To connect to your ParadeDB database, execute: docker exec -it paradedb psql $dbname -U $pguser"
}


installMacOS() {
  # Confirm architecture
  if [ "$ARCH" != "arm64" ]; then
    echo -e ""
    echo -e "ParadeDB macOS prebuilt binaries are only available for Apple Silicon (M-series) Macs. Exiting..."
    exit 6
  fi

  # Prompt the user for their desired Postgres major version 
  selectPostgresVersion

  # Retrieve the OS version
  OS_VERSION=$(sw_vers -productVersion)
  OS_NAME=""
  if [[ "$OS_VERSION" == 15.* ]]; then
    OS_NAME="sequoia"
  elif [[ "$OS_VERSION" == 14.* ]]; then
    OS_NAME="sonoma"
  else
    echo -e ""
    echo -e "Unsupported macOS version: $OS_VERSION. Only macOS Sequoia and macOS Sonoma are supported. Exiting..."
    exit 7
  fi

  # TODO: Mention to the user they will be prompted for sudo
  echo -e ""
  echo -e "Downloading and installing ParadeDB pg_search Postgres extension version $LATEST_RELEASE_VERSION for Postgres $PG_MAJOR_VERSION on macOS $OS_NAME..."
  filename="pg_search@${PG_MAJOR_VERSION}--${LATEST_RELEASE_VERSION}.arm64_${OS_NAME}.pkg"
  url="https://github.com/paradedb/paradedb/releases/download/${LATEST_RELEASE_TAG}/"
  curl -L "${url}${filename}" -o "$HOME/Downloads/$filename" || false
  sudo installer -pkg "$HOME/Downloads/$filename" -target /
}


installLinux() {
  # Red Hat-based
  if isRHEL; then
    # Retrieve the arch
    if [ "$ARCH" = "x86_64" ]; then
      ARCH="amd64"
    else
      ARCH="aarch64"
    fi
    # Retrieve the OS version
    RHEL_VERSION=$(awk -F'[. ]' '/release/{print $6}' /etc/redhat-release)
  # Debian-based
  else
    # Retrieve the arch
    if [ "$ARCH" = "x86_64" ]; then
      ARCH="amd64"
    else
      ARCH="arm64"
    fi
    # Retrieve the OS version
    DEB_DISTRO_NAME=$(awk -F'[= ]' '/VERSION_CODENAME=/{print $2}'  /etc/os-release)
  fi

  # Prompt the user for their desired Postgres major version 
  selectPostgresVersion

  # Red Hat-based
  if isRHEL; then
    # TODO: Remove the code duplication here
    echo -e ""
    echo -e "Downloading and installing ParadeDB pg_search Postgres extension version $LATEST_RELEASE_VERSION for Postgres $PG_MAJOR_VERSION on RHEL $RHEL_VERSION..."
    filename="pg_search_${PG_MAJOR_VERSION}-$LATEST_RELEASE_VERSION-1PARADEDB.el${RHEL_VERSION}.${ARCH}.rpm"
    url="https://github.com/paradedb/paradedb/releases/download/${LATEST_RELEASE_TAG}/"
    curl -L "${url}${filename}" -o "$HOME/Downloads/$filename" || false
    sudo rpm -i "$filename" || false
  # Debian-based
  else
    # TODO: Remove the code duplication here
    echo -e ""
    echo -e "Downloading and installing ParadeDB pg_search Postgres extension version $LATEST_RELEASE_VERSION for Postgres $PG_MAJOR_VERSION on $DEB_DISTRO_NAME..."
    filename="postgresql-${PG_MAJOR_VERSION}-pg-search_${LATEST_RELEASE_VERSION}-1PARADEDB-${DEB_DISTRO_NAME}_${ARCH}.deb"
    url="https://github.com/paradedb/paradedb/releases/download/${LATEST_RELEASE_TAG}/"
    curl -L "${url}${filename}" -o "$HOME/Downloads/$filename" || false
    sudo apt-get install ./"$filename" -y || false
  fi
}

########################################
# Main Loop
########################################

echo -e ""
echo -e "  _____                    _      _____  ____     "
echo -e " |  __ \                  | |    |  __ \|  _ \    "
echo -e " | |__) |_ _ _ __ __ _  __| | ___| |  | | |_) |   "
echo -e " |  ___/ _\` | '__/ _\` |/ _\` |/ _ \ |  | |  _ < "
echo -e " | |  | (_| | | | (_| | (_| |  __/ |__| | |_) |   "
echo -e " |_|   \__,_|_|  \__,_|\__,_|\___|_____/|____/    "
echo -e ""
echo -e ""
echo -e "🚀 Welcome to the ParadeDB Installation Script 🚀"
echo -e ""
echo -e "ParadeDB is an alternative to Elasticsearch built on Postgres."
echo -e "It is available as a Helm chart for Kubernetes, as a Docker image, and as prebuilt binaries for self-hosted Postgres instances."
echo -e ""
echo -e "For more information on deploying ParadeDB, including purchasing access to ParadeDB Bring-Your-Own-Cloud, please"
echo -e "visit our documentation: https://docs.paradedb.com/deploy/overview"
echo -e ""

# Build the list of installation options based on the user's OS
OPTIONS=("Kubernetes" "Docker")
if [[ "$OSTYPE" = "msys" ]] || [[ "$OSTYPE" = "cygwin" ]]; then
  echo -e "Operating system detected: Windows"
elif [[ "$OSTYPE" = "darwin"* ]]; then
  echo -e "Operating system detected: macOS"
  OPTIONS+=("Prebuilt Binaries")
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
  if isRHEL; then
    echo -e "Operating system detected: Red Hat-based Linux"
  else
    echo -e "Operating system detected: Debian-based Linux"
  fi
  OPTIONS+=("Prebuilt Binaries")
else
  echo -e "Operating system not supported, exiting..."
  exit 0
fi

# Prompt the user for their preferred installation method
echo -e ""
echo -e "Please select your preferred installation method from the following options available for your system:"
select opt in "${OPTIONS[@]}"; do
  if [[ "$REPLY" == "1" || "$REPLY" == "Kubernetes" ]]; then
    installKubernetes
    break
  elif [[ "$REPLY" == "2" || "$REPLY" == "Docker" ]]; then
    installDocker
    break
  elif [[ "$REPLY" == "3" || "$REPLY" == "Prebuilt Binaries" ]]; then
    if [[ "$OSTYPE" = "darwin"* ]]; then
      installMacOS
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
      installLinux
    fi

    # Catch errors in install commands
    if [[ $? -ne 0 ]]; then
      # TODO: Provide a better error message here
      echo -e ""
      echo -e "Failed to install ParadeDB pg_search. Exiting..."
      exit 8
    fi
    echo -e ""
    echo -e "🚀 ParadeDB pg_search installed! To use ParadeDB, connect to your local Postgres database and execute: 'CREATE EXTENSION pg_search;'"
    break
  else
    echo "Invalid option. You must select one of 'Kubernetes', 'Docker' or 'Prebuilt Binaries'. Exiting..."
    exit 1
    break
  fi
  echo -e ""
done

# TODO: Add support for asking the user for their email

# Exit message
echo -e ""
echo -e ""
echo -e "🎉 Thank you for installing ParadeDB! 🎉"
echo -e ""
echo -e "To get started, we recommend our quickstart tutorial: https://docs.paradedb.com/documentation/getting-started/quickstart"
echo -e ""
echo -e "To stay up to date and get help, please join our Slack community: https://join.slack.com/t/paradedbcommunity/shared_invite/zt-32abtyjg4-yoYoi~RPh9MSW8tDbl0BQw"
echo -e "And of course, don't forget to star us on GitHub: https://github.com/paradedb/paradedb"
echo -e ""
echo -e "Here's a happy elephant to celebrate your installation: 🐘"
echo -e ""
