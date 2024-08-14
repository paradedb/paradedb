#!/bin/sh -e

OS=$(uname -s)
ARCH=$(uname -m)
LATEST_RELEASE_VERSION="0.9.1"

installDocker() {
  # Set default values
  pguser="myuser"
  pgpass="mypassword"
  dbname="paradedb"

  # Prompt for user input
  read -p "Username for Database (default: postgres): " tmp_pguser
  if [[ ! -z "$tmp_pguser" ]]; then
    pguser="$tmp_pguser"
  fi

  read -p "Password for Database (default: postgres): " tmp_pgpass
  if [[ ! -z "$tmp_pgpass" ]]; then
    pgpass="$tmp_pgpass"
  fi

  read -p "Name for your database (default: paradedb): " tmp_dbname
  if [[ ! -z "$tmp_dbname" ]]; then
    dbname="$tmp_dbname"
  fi


  # Pull Docker image
  echo "Pulling Docker Image for Parade DB: docker pull paradedb/paradedb"
  docker pull paradedb/paradedb || { echo "Failed to pull Docker image"; exit 1; }
  echo "Pulled Successfully ✅"

  # Create Docker container
  echo "Processing..."
  docker run \
    --name paradedb \
    -e POSTGRES_USER="$pguser" \
    -e POSTGRES_PASSWORD="$pgpass" \
    -e POSTGRES_DB="$dbname" \
    -v paradedb_data:/var/lib/postgresql/ \
    -p 5432:5432 \
    -d \
    paradedb/paradedb:latest || { echo "Failed to start Docker container. Please check if an existing container is active or not."; exit 1; }
  echo "Docker Container started ✅"

  # Provide usage information
  echo "To use paradedb use the command: docker exec -it paradedb psql $dbname -U $pguser"
}

# Please update the debian file with the lastest version here
installDeb(){
    echo "Select your distribution"

    distros=("bookworm" "jammy" "noble")
    distro=
    select op in ${distros[@]}
    do
        case $op in 
            "bookworm")
                distro="bookworm"
                break;;
            "jammy")
                distro="jammy"
                break;;
            "noble")
                distro="noble"
                break;;
        esac
    done
    
    if [ "$ARCH" = "x86_64" ]; then
        ARCH="amd64"
    fi

    filename="postgresql-$1-pg-search_${LATEST_RELEASE_VERSION}-1PARADEDB-${distro}_${ARCH}.deb"
    url="https://github.com/paradedb/paradedb/releases/download/v${LATEST_RELEASE_VERSION}/${filename}"

    echo "Downloading ${filename}"

    curl -L $url > $filename 

    sudo apt install ./$filename
    echo "ParadeDB installed successfully!"
    exit
}

# Please update the RPM file with the latest version here
installRPM(){
    filename="pg_search_$1-$LATEST_RELEASE_VERSION-1PARADEDB.el9.${ARCH}.rpm"
    url="https://github.com/paradedb/paradedb/releases/download/v${LATEST_RELEASE_VERSION}/$filename"

    echo "Downloading ${filename}"
    curl -l $url > $filename

    sudo rpm -i $filename
    echo "ParadeDB installed successfully!"
    exit
}

installStable(){

    # Select postgres version
    pg_version=
    echo "Select postgres version"
    versions=("14" "15" "16")

    select vers in "${versions[@]}"
    do
        case $vers in
            "14")
                echo $vers
                pg_version="14"
                break;;
            "15")
                pg_version="15"
                break;;
            "16")
                pg_version="16"
                break;;
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
                break;;
            ".rpm")
                installRPM $pg_version
                break;;
        esac
    done
}


echo -e "============================================================="

echo -e "\t\tWelcome to ParadeDB Setup!"

echo -e "=============================================================\n\n"



OPTIONS=("🐳Latest Docker Image" "⬇️ Stable Binary")


select opt in "${OPTIONS[@]}" 
do
    case $opt in
        "🐳Latest Docker Image")
            installDocker
            ;;
        "⬇️ Stable Binary")
            echo "Stable"
            installStable
            break;;
        *)
            exit

    esac
done

