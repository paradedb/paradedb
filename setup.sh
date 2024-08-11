#!/bin/sh -e

installDocker() {
  # Set default values
  pguser="postgres"
  pgpass="postgres"
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

installSource(){
    echo "[TODO]: Building from source"
}

installStable(){
    echo "[TODO]: Installing stable build"
}


echo -e "============================================================="

echo -e "\t\tWelcome to ParadeDB Setup!"

echo -e "=============================================================\n\n"

OS=$(uname -s)
ARCH=$(uname -m)

OPTIONS=("🐳Latest Docker Image"
         "⬇ Stable Binary"
         "🚀Development Build(Build ParadeDB from Source)")

select opt in "${OPTIONS[@]}" 
do
    case $opt in
        "🐳Latest Docker Image")
            installDocker
            break;;
        "⬇ Stable Binary")
            installStable
            break;;
        "🚀Development Build(Build ParadeDB from Source)")
            installSource
            break;;
        *)
            exit

    esac
done

