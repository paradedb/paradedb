#
# This docker-compose.antithesis.yml file is used to test ParadeDB with deterministic simulation
# testing (DST) via Antithesis. It creates a single node ParadeDB instance and runs Stressgres against it.
#
# This Docker Compose file requires authenticating to the ParadeDB Docker Hub registry in order to pull the
# Stressgres image.

services:
  paradedb-1:
    image: us-central1-docker.pkg.dev/molten-verve-216720/paradedb-repository/paradedb:${COMMIT_SHA}
    container_name: paradedb-1
    environment:
      POSTGRES_USER: myuser
      POSTGRES_PASSWORD: mypassword
      POSTGRES_DB: mydatabase
    ports:
      - "5432:5432"
    volumes:
      - paradedb_data:/var/lib/postgresql/

  stressgres:
    image: us-central1-docker.pkg.dev/molten-verve-216720/paradedb-repository/stressgres:latest
    container_name: stressgres

volumes:
  paradedb_data:
