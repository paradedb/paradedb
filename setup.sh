#!/bin/bash

ARCH=$(uname -m)
LATEST_RELEASE_TAG=$(curl -s "https://api.github.com/repos/paradedb/paradedb/releases/latest" | jq -r .tag_name)
LATEST_RELEASE_VERSION="${LATEST_RELEASE_TAG#v}"
OSTYPE="darwin"

EXIT_MSG="\n\nIf you'd like to stay upto date with everything about ParadeDB\nJoin our slack channel: https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ \nGitHub: https://github.com/paradedb/paradedb"

set -Eeuo pipefail

installDockerDepsLinux(){
  echo "Which type of distro are you using?(Required to install dependencies)"
  OPTIONS=("Debian Based" "RHEL Based" "Arch Based")

  select opt in "${OPTIONS[@]}"
  do
    case $opt in
      "Debian Base")
        sudo apt-get install docker -y || false
        break ;;
      "RHEL Based")
        sudo dnf install docker -y || false
        break ;;
      "Arch Based")
        sudo pacman -Su docker || false
        break ;;
      *)
        break ;;
    esac
  done

}

installDocker() {
  # Set default values
  pguser="myuser"
  pgpass="mypassword"
  dbname="paradedb"

  echo "Installing docker..."

  if [[ "$OSTYPE" =  "darwin" ]]; then
    brew install docker
  elif [[ "$OSTYPE" = "linux-gnu" ]]; then
    installDockerDepsLinux
  elif [[ "$OSTYPE" = "msys" ]] || [[ "$OSTYPE" = "cygwin" ]]; then # Detects Git bash/ cygwin environments
    read -p "Do you have docker installed(y/n)? " -n 1 -r
    if [[ $REPLY =~ ^[Nn]$ ]]
    then
      echo -e "\nPlease install docker first and get back to the setup!"
      echo -e "$EXIT_MSG"
      exit 1
    fi
  fi

  echo "Successfully Installed Docker✅"


  # Prompt for user input
  read -r -p "Username for Database (default: myuser): " tmp_pguser
  if [[ -n "$tmp_pguser" ]]; then
    pguser="$tmp_pguser"
  fi

  read -r -p "Password for Database (default: mypassword): " tmp_pgpass
  if [[ -n "$tmp_pgpass" ]]; then
    pgpass="$tmp_pgpass"
  fi

  read -r -p "Name for your database (default: paradedb): " tmp_dbname
  if [[ -n "$tmp_dbname" ]]; then
    dbname="$tmp_dbname"
  fi


  # Pull Docker image
  echo "Pulling Docker Image for Parade DB: docker pull paradedb/paradedb"
  docker pull paradedb/paradedb || { echo "Failed to pull Docker image"; exit 1; }
  echo -e "Pulled Successfully ✅\n"

  echo -e "Removing any existing containers\n"
  docker stop paradedb > /dev/null || true
  docker rm paradedb > /dev/null || true
  echo -e "\n"

  echo -e "Would you like to add a Docker volume to your database?\nA docker volume will ensure that your ParadeDB Postgres database is stored across Docker restarts.\nNote that you will need to manually update ParadeDB versions on your volume via: https://docs.paradedb.com/upgrading.\nIf you're only testing, we do not recommend adding a volume."

  volume_opts=("Yes" "No(Default)")

  select vopt in "${volume_opts[@]}"
  do
    case $vopt in
      "Yes")
        echo "Adding volume at: /var/lib/postgresql/data"
        docker run \
          --name paradedb \
          -e POSTGRES_USER="$pguser" \
          -e POSTGRES_PASSWORD="$pgpass" \
          -e POSTGRES_DB="$dbname" \
          -v paradedb_data:/var/lib/postgresql/data/ \
          -p 5432:5432 \
          -d \
          paradedb/paradedb:latest || { echo "Failed to start Docker container. Please check if an existing container is active or not."; exit 1; }
        break ;;
      *)
        docker run \
          --name paradedb \
          -e POSTGRES_USER="$pguser" \
          -e POSTGRES_PASSWORD="$pgpass" \
          -e POSTGRES_DB="$dbname" \
          -p 5432:5432 \
          -d \
          paradedb/paradedb:latest || { echo "Failed to start Docker container. Please check if an existing container is active or not."; exit 1; }
        break ;;
    esac
  done
  echo "Docker Container started ✅"

  # Provide usage information
  echo -e "\n\nTo use paradedb execute the command: docker exec -it paradedb psql $dbname -U $pguser"
}

installDeb(){
  echo "Select your distribution"

  echo "Installing dependencies...."
  echo "Installing cURL"

  sudo apt-get update -y || false
  sudo apt-get install curl -y || false

  echo "Successfully Installed cURL✅"

  distros=("bookworm(Debian 12.0)" "jammy(Ubuntu 22.04)" "noble(Ubuntu 24.04)")
  distro=
  select op in "${distros[@]}"
  do
    case $op in
      "bookworm(Debian 12.0)")
        distro="bookworm"
        break ;;
      "jammy(Ubuntu 22.04)")
        distro="jammy"
        break ;;
      "noble(Ubuntu 24.04)")
        distro="noble"
        break ;;
    esac
  done

  if [ "$ARCH" = "x86_64" ]; then
    ARCH="amd64"
  fi

  filename="postgresql-$1-pg-search_${LATEST_RELEASE_VERSION}-1PARADEDB-${distro}_${ARCH}.deb"
  url="https://github.com/paradedb/paradedb/releases/latest/download/${filename}"

  echo "Downloading ${url}"

  curl -L "$url" > "$filename" || false

  sudo apt install ./"$filename" -y || false
}

# Installs latest RPM package
installRPM(){
  filename="pg_search_$1-$LATEST_RELEASE_VERSION-1PARADEDB.el9.${ARCH}.rpm"
  url="https://github.com/paradedb/paradedb/releases/latest/download/${filename}"
  echo -e "Installing cURL"
  sudo dnf install curl || false
  echo "Successfully Installed cURL✅"

  echo "Downloading ${url}"
  curl -l "$url" > "$filename" || false

  sudo rpm -i "$filename" || false
  echo "ParadeDB installed successfully!"
}

# Installs latest binary for ParadeDB
installBinary(){

  if [[ "$OSTYPE" = "msys" ]] || [[ "$OSTYPE" = "cygwin" ]]; then
    echo "Sorry but we do not support windows yet!"
    echo -e "$EXIT_MSG"
    exit 1
  elif [[ "$OSTYPE" = "darwin" ]]; then
    echo -e "\nOops! We don't have any prebuilt binaries for MacOS right now.\nYou can follow our comprehensive guide to compile ParadeDB from source at: https://github.com/paradedb/paradedb/blob/dev/README.md!"
    echo -e "$EXIT_MSG"
    exit 1
  fi

  # Select postgres version
  pg_version=
  echo "Select postgres version(Note: ParadeDB is supported on PG12-16. For other postgres versions, you will need to compile from source.)"
  versions=("14" "15" "16")

  select vers in "${versions[@]}"
  do
    case $vers in
      "14")
        pg_version="14"
        break ;;
      "15")
        pg_version="15"
        break ;;
      "16")
        pg_version="16"
        break ;;
    esac
  done

  # Select Base type
  echo "Select supported file type: "
  opts=(".deb" ".rpm")

  select opt in "${opts[@]}"
  do
    case $opt in
      ".deb")
        installDeb $pg_version
        break ;;
      ".rpm")
        installRPM $pg_version
        break ;;
    esac
  done
}


echo -e "=========================================================\n"

echo -e "Hi there!

Welcome to ParadeDB, an open-source alternative to Elasticsearch built on Postgres.\nThis script will guide you through installing ParadeDB.

ParadeDB is available as a Kubernetes Helm chart, a Docker image, and as prebuilt binaries for Debian-based and Red Hat-based Linux distributions.\nHow would you like to install ParadeDB?"


echo -e "=========================================================\n"




OPTIONS=("🐳Latest Docker Image" "⬇️ Stable Binary")


select opt in "${OPTIONS[@]}"
do
  case $opt in
    "🐳Latest Docker Image")
      installDocker
      echo -e "Installation Successfull!\n"
      break ;;
    "⬇️ Stable Binary")
      echo "Stable"
      installBinary
      echo -e "Installation Successfull!\n"
      break ;;
    *)
      echo -e "No option selected, exiting setup.\n"
      break ;;
  esac
done


echo -e "$EXIT_MSG"
