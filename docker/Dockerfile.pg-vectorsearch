FROM postgres:18 AS base

RUN apt-get update && apt-get install -y --no-install-recommends \
    postgresql-18-pgvector libopenblas0 && \
    rm -rf /var/lib/apt/lists/*

FROM base AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    postgresql-server-dev-18 build-essential meson ninja-build pkg-config \
    libopenblas-dev git && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN meson setup builddir --buildtype=release \
    -Dnative=true -Dpostgresql=enabled -Dblas=enabled -Dtools=disabled && \
    DESTDIR=/artifacts meson install -C builddir

FROM base AS runtime

COPY --from=builder /artifacts/ /
RUN printf 'CREATE EXTENSION IF NOT EXISTS vector;\nCREATE EXTENSION IF NOT EXISTS pg_vectorsearch;\n' \
    > /docker-entrypoint-initdb.d/10-extensions.sql
