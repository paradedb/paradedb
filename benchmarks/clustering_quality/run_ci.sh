#!/usr/bin/env bash
set -euo pipefail

repo="$(pwd)"
task_root="${RUNNER_TEMP:?}/cohere-cluster-comparison"
artifacts="$repo/benchmark-artifacts"
pg_bin=/usr/lib/postgresql/18/bin
pg_data="$task_root/data"
mkdir -p "$task_root/socket" "$artifacts"
exec > >(tee "$artifacts/run.log") 2>&1
trap '"$pg_bin/pg_ctl" -D "$pg_data" -m fast -w stop >/dev/null 2>&1 || true' EXIT

CARGO_BUILD_JOBS=16 cargo pgrx install -p pg_search --sudo --release --no-default-features --features pg18 \
    --pg-config "$pg_bin/pg_config"
CC="$($pg_bin/pg_config --cc)" meson setup .cluster-export-deps/pg_vectorsearch/build \
    .cluster-export-deps/pg_vectorsearch --buildtype=release -Dnative=true \
    -Dpostgresql=enabled -Dpg_config="$pg_bin/pg_config" -Dtools=enabled \
    -Dhdf5=disabled -Dtap_tests=disabled
ninja -C .cluster-export-deps/pg_vectorsearch/build -j 16
sudo meson install -C .cluster-export-deps/pg_vectorsearch/build

python3 -m venv "$task_root/python"
py="$task_root/python/bin/python"
"$py" -m pip install 'numpy<3' 'psycopg[binary]<4' 'pyarrow<24' 'matplotlib<4'
"$py" -m pip freeze > "$artifacts/python-versions.txt"
(cd benchmarks/clustering_quality && "$py" -m unittest -v test_compare.py)

for source in "$repo" "$repo/.cluster-export-deps/tantivy" "$repo/.cluster-export-deps/pg_vectorsearch"; do
    git -C "$source" log -1 --format='%H %s'
done > "$artifacts/source-commits.txt"
sha256sum "$($pg_bin/pg_config --pkglibdir)/pg_search.so" \
    "$($pg_bin/pg_config --pkglibdir)/pg_vectorsearch-0.2.0-dev.so" > "$artifacts/library-sha256.txt"
cp benchmarks/clustering_quality/{compare.py,prepare_cohere.py,plot.py,README.md,run_ci.sh} "$artifacts/"

cargo run --manifest-path benchmarks/Cargo.toml --release -- restore-heap \
    --dataset cohere --size 10m --pgdata "$pg_data" --stanza bench \
    --repo-bucket paradedb-benchmark --repo-path-prefix /snapshots \
    --repo-region us-east-1 --repo-endpoint s3.us-east-1.amazonaws.com \
    --repo-s3-key-type shared --process-max 16
cat > "$pg_data/postgresql.auto.conf" <<'CONF'
CONF
cat > "$task_root/postgresql.conf" <<CONF
listen_addresses = ''
port = 29926
unix_socket_directories = '$task_root/socket'
shared_preload_libraries = 'pg_search'
shared_buffers = '12GB'
maintenance_work_mem = '12GB'
work_mem = '64MB'
max_worker_processes = 32
max_parallel_workers = 8
max_parallel_maintenance_workers = 8
max_parallel_workers_per_gather = 8
autovacuum = off
jit = off
max_wal_size = '100GB'
wal_level = minimal
max_wal_senders = 0
huge_pages = off
CONF
cp "$task_root/postgresql.conf" "$artifacts/postgresql.conf"
"$pg_bin/pg_ctl" -D "$pg_data" -l "$artifacts/postgres.log" \
    -o "-c config_file=$task_root/postgresql.conf" -w start
url="host=$task_root/socket port=29926 dbname=postgres"
"$pg_bin/psql" "$url" -X -v ON_ERROR_STOP=1 --single-transaction \
    -c 'DROP EXTENSION IF EXISTS pg_search;' -c 'CREATE EXTENSION pg_search;'
aws s3 cp s3://paradedb-benchmark/datasets/cohere/v1/queries/cohere_queries.parquet \
    "$task_root/cohere_queries.parquet"
"$py" benchmarks/clustering_quality/prepare_cohere.py --url "$url" \
    --queries "$task_root/cohere_queries.parquet" --output "$task_root/dataset" --expected-rows 10000000
cp "$task_root/dataset/source.json" "$artifacts/dataset-source.json"
"$py" benchmarks/clustering_quality/compare.py --url "$url" \
    --dataset "$task_root/dataset" --dataset-name 'Cohere 10m' --output "$artifacts/results"
"$py" benchmarks/clustering_quality/plot.py "$artifacts/results"
df -h "$task_root" > "$artifacts/disk-final.txt"
