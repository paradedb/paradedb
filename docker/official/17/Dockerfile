# Note: Debian Trixie = Debian 13
FROM postgres:17-trixie AS paradedb

LABEL maintainer="ParadeDB - https://paradedb.com" \
    org.opencontainers.image.description="Simple, Elastic-quality search for Postgres" \
    org.opencontainers.image.source="https://github.com/paradedb/paradedb"

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# Install pg_search from Cloudsmith
RUN apt-get update && \
    apt-get install -y --no-install-recommends apt-transport-https ca-certificates curl debian-archive-keyring debian-keyring gnupg \
    && keyring_location=/usr/share/keyrings/paradedb-paradedb-archive-keyring.gpg && \
    curl -1sLf "https://dl.cloudsmith.io/public/paradedb/paradedb/gpg.B82501FB00B5422B.key" | gpg --dearmor -o "${keyring_location}" && \
    curl -1sLf "https://dl.cloudsmith.io/public/paradedb/paradedb/config.deb.txt?distro=debian&codename=trixie&component=main" > /etc/apt/sources.list.d/paradedb-paradedb.list && \
    chmod 644 "${keyring_location}" /etc/apt/sources.list.d/paradedb-paradedb.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends postgresql-17-pg-search=0.23.4 && \
    rm -rf /var/lib/apt/lists/*

# Install core extensions
# Use the PGDG archive so these pinned extension versions remain available after newer
# versions replace them in the live Debian and PGDG apt repositories.
ENV POSTGIS_VERSION_MAJOR=3
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && echo "deb [ signed-by=/usr/local/share/keyrings/postgres.gpg.asc ] https://apt-archive.postgresql.org/pub/repos/apt trixie-pgdg-archive main" > /etc/apt/sources.list.d/pgdg-archive.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    postgresql-17-pgvector=0.8.1-2.pgdg13+1 \
    postgresql-17-cron=1.6.7-2.pgdg13+1 \
    postgresql-17-pg-ivm=1.13-1.pgdg13+1 \
    postgresql-17-postgis-$POSTGIS_VERSION_MAJOR \
    postgresql-17-postgis-$POSTGIS_VERSION_MAJOR-scripts \
    && rm /etc/apt/sources.list.d/pgdg-archive.list \
    && rm -rf /var/lib/apt/lists/* && \
    update-ca-certificates

# The postgresql.conf.sample file is used as a template for the postgresql.conf file, which
# does not exist until the first time the container is started. By adding our settings to the
# postgresql.conf.sample file, we ensure that our settings are applied onto the postgresql.conf file.
#
# The `postgres` database is the default database that exists in every Postgres installation. The pg_cron
# extension requires a database to store its metadata tables. By using `postgres`, we ensure that it has a
# stable, always-available database for its operations, no matter what other databases are created or deleted.
RUN sed -i "s/^#shared_preload_libraries = ''/shared_preload_libraries = 'pg_search,pg_cron,pg_stat_statements'/" /usr/share/postgresql/postgresql.conf.sample && \
    grep "shared_preload_libraries = 'pg_search,pg_cron,pg_stat_statements'" /usr/share/postgresql/postgresql.conf.sample && \
    echo "cron.database_name = 'postgres'" >> /usr/share/postgresql/postgresql.conf.sample && \
    echo "pg_stat_statements.track = 'top'" >> /usr/share/postgresql/postgresql.conf.sample

# Reset the working directory to the root directory
WORKDIR /

# Copy ParadeDB bootstrap script to install extensions and configure postgresql.conf
COPY ./docker/bootstrap.sh /docker-entrypoint-initdb.d/10_bootstrap_paradedb.sh

# The upstream `postgres` Docker image comes with its own `entrypoint.sh` script which
# starts as `root` and then switches to the `postgres` user after running chown and chmod
# on the PostgreSQL data directory. To maintain compatibility with the upstream image and
# ensure that the `postgres` user has the correct permissions on the data directory, we let
# the upstream `entrypoint.sh` script run as the entrypoint and don't specify a custom user.
