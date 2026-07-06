# Preparing a Dataset

This document walks through the full process of preparing a new non-synthetic dataset for benchmarking. The high-level steps are:

1. Load source data into S3
2. Create a dataset config
3. Sample the data at each size you need
4. Convert the sampled parquet data to CSV
5. Load the heap into Postgres
6. Snapshot or restore the heap with pgBackRest
7. Run the benchmark

Unless otherwise noted, run commands from the `benchmarks/` directory.

## Step 1: Load Source Data into S3

Upload your source data as partitioned parquet files to S3. Each table should be in its own subdirectory under a `source/parquet/` path (filename doesn't matter):

```text
s3://<dataset-bucket>/datasets/{dataset-name}/source/parquet/
├── {table_a}/
│   ├── part-0001.parquet
│   ├── part-0002.parquet
│   └── ...
├── {table_b}/
│   └── ...
└── {table_c}/
    └── ...
```

For example, for the Stack Overflow dataset:

```text
s3://<dataset-bucket>/datasets/stackoverflow/source/parquet/
├── stackoverflow_posts/
├── comments/
└── users/
```

## Step 2: Create a Config

### Writing the Config

Create a TOML config file at `datasets/{dataset-name}/config.toml` that describes your table relationships. The config specifies which table is the root (the one that gets sampled directly) and how child tables relate to it via joins.

```toml
sampling_seed = 723
s3_base_path = "s3://<dataset-bucket>/datasets/root_dataset"

[root_table]
name = "root_table_name"
primary_key = "id"

[[tables]]
name = "child_table"
parent = "root_table_name"
parent_join_col = "id"
join_col = "parent_id"
```

Fields:

- `sampling_seed`: Seed for deterministic, reproducible sampling.
- `s3_base_path`: Optional default source for `load-heap`. It can be overridden with `--data-source`.
- `[root_table]`: The primary table. The `--rows` argument controls how many rows are sampled from this table.
- `primary_key`: Unique, non-null key used for deterministic root-table sampling.
- `[[tables]]`: One entry per non-root table. Child tables specify `parent`, `parent_join_col`, and `join_col` to define the relationship.

Child tables are not sampled independently. Instead, they are filtered via an inner join with their parent table, so only rows that reference a sampled parent row are kept. This preserves referential integrity across the dataset.

Tables can form a hierarchy (a child can be a parent of another table). They are processed in topological order.

See `datasets/stackoverflow/config.toml` for a real example.

## Step 3: Sample the Source Data

### Running the Sampling Tool

Run the `sample` command once for each dataset size you need. The `--rows` argument sets the target row count for the root table. Output goes to the `sampled/{size}/parquet/` path.

```bash
# Sample to 10k rows
cargo run --release -- sample \
  --input s3://<dataset-bucket>/datasets/stackoverflow/source/parquet/ \
  --output s3://<dataset-bucket>/datasets/stackoverflow/sampled/10k/parquet/ \
  --config ./datasets/stackoverflow/config.toml \
  --rows 10000

# Sample to 100k rows
cargo run --release -- sample \
  --input s3://<dataset-bucket>/datasets/stackoverflow/source/parquet/ \
  --output s3://<dataset-bucket>/datasets/stackoverflow/sampled/100k/parquet/ \
  --config ./datasets/stackoverflow/config.toml \
  --rows 100000

# Sample to 1m rows
cargo run --release -- sample \
  --input s3://<dataset-bucket>/datasets/stackoverflow/source/parquet/ \
  --output s3://<dataset-bucket>/datasets/stackoverflow/sampled/1m/parquet/ \
  --config ./datasets/stackoverflow/config.toml \
  --rows 1000000
```

Notes:

- The output path must be empty (no pre-existing data).
- For small targets (<=100k rows), sampling is exact using reservoir sampling. For larger targets, it uses system sampling and the result will be approximate (within ~3-5%).
- Use `--dry-run` to validate inputs and see planned row counts without writing anything.

## Step 4: Convert Sampled Data to CSV

Run the `convert` command for each sampled size to produce CSV versions. The `--tables` flag takes a comma-separated list of all tables to convert.

```bash
# Convert 10k sampled data
cargo run --release -- convert \
  --input s3://<dataset-bucket>/datasets/stackoverflow/sampled/10k/parquet/ \
  --output s3://<dataset-bucket>/datasets/stackoverflow/sampled/10k/csv/ \
  --tables stackoverflow_posts,comments,users

# Convert 100k sampled data
cargo run --release -- convert \
  --input s3://<dataset-bucket>/datasets/stackoverflow/sampled/100k/parquet/ \
  --output s3://<dataset-bucket>/datasets/stackoverflow/sampled/100k/csv/ \
  --tables stackoverflow_posts,comments,users

# Convert 1m sampled data
cargo run --release -- convert \
  --input s3://<dataset-bucket>/datasets/stackoverflow/sampled/1m/parquet/ \
  --output s3://<dataset-bucket>/datasets/stackoverflow/sampled/1m/csv/ \
  --tables stackoverflow_posts,comments,users
```

You can also convert the full source data:

```bash
cargo run --release -- convert \
  --input s3://<dataset-bucket>/datasets/stackoverflow/source/parquet/ \
  --output s3://<dataset-bucket>/datasets/stackoverflow/source/csv/ \
  --tables stackoverflow_posts,comments,users
```

Notes:

- The output path must be empty.
- Row counts are verified after conversion to ensure no data is lost.
- Use `--dry-run` to validate without writing.
- AWS credentials must be accessible via the standard credential chain (env vars, `~/.aws/credentials`, or instance metadata).

## Step 5: Load the Heap

Load the sampled CSV data into Postgres with `load-heap`:

```bash
POSTGRES_URL="postgresql://localhost:28818/postgres"

cargo run --release -- load-heap \
  --url "${POSTGRES_URL}" \
  --dataset stackoverflow \
  --size 100k
```

`load-heap` reads from `{s3_base_path}/sampled/{size}/csv/{table}/`, creates tables from `datasets/{dataset}/create_tables.sql`, and bulk-loads CSV files. Use `--data-source` to override `s3_base_path` from the dataset config.

## Step 6: Snapshot or Restore the Heap

The snapshot commands shell out to `pgbackrest`, so install and configure pgBackRest before using them. Use `--config` to provide your own pgBackRest config, or pass repository settings to generate one. The examples below use ParadeDB's CI snapshot repository: `s3://paradedb-ci-benchmarks/snapshots/{dataset}/{size}/`.

Stop Postgres before snapshotting a loaded heap:

```bash
PGDATA="/home/runner/.pgrx/data-18"
BACKREST_ARGS=(
  --stanza bench
  --repo-bucket paradedb-ci-benchmarks
  --repo-path-prefix /snapshots
  --repo-region us-east-1
  --repo-endpoint s3.us-east-1.amazonaws.com
  --repo-s3-key-type shared
)

(cd ../pg_search && cargo pgrx stop pg18)
cargo run --release -- snapshot-heap \
  --dataset stackoverflow \
  --size 100k \
  --pgdata "${PGDATA}" \
  "${BACKREST_ARGS[@]}"
```

To restore that heap later, keep Postgres stopped, restore the snapshot, and start Postgres:

```bash
cargo run --release -- restore-heap \
  --dataset stackoverflow \
  --size 100k \
  --pgdata "${PGDATA}" \
  "${BACKREST_ARGS[@]}"
(cd ../pg_search && cargo pgrx start pg18)
```

Useful snapshot options:

- `--repo-bucket`: S3 bucket for the pgBackRest repository.
- `--repo-path-prefix`: Prefix inside the bucket.
- `--repo-s3-key-type`: Use `shared`, `auto`, or `web-id`.
- `--process-max`: Max pgBackRest worker processes. Defaults to available CPUs.

## Step 7: Run Benchmarks

After loading or restoring the heap, run `benchmark`:

```bash
cargo run --release -- benchmark \
  --url "${POSTGRES_URL}" \
  --dataset stackoverflow \
  --index bm25
```

## Final S3 Layout

After preparing data and publishing snapshots, your artifacts will look like this:

```text
s3://<dataset-bucket>/datasets/{dataset-name}/
├── source/
│   ├── parquet/
│   │   ├── {table_a}/
│   │   ├── {table_b}/
│   │   └── {table_c}/
│   └── csv/
│       ├── {table_a}/
│       ├── {table_b}/
│       └── {table_c}/
└── sampled/
    ├── 10k/
    │   ├── parquet/
    │   │   ├── {table_a}/
    │   │   ├── {table_b}/
    │   │   └── {table_c}/
    │   └── csv/
    │       ├── {table_a}/
    │       ├── {table_b}/
    │       └── {table_c}/
    ├── 100k/
    │   └── ...
    └── 1m/
        └── ...

s3://<snapshot-bucket>/snapshots/{dataset-name}/
├── 100k/
├── 1m/
└── 20m/
```

## Preparing the Vector-Update Benchmark (cohere)

The vector-update benchmark (`.github/workflows/benchmark-pg_search-updates.yml`) grows the corpus
from empty to `sampled/1m` one chunk (~100k) at a time, measuring latency and recall between inserts
to see how an incrementally-maintained pgvector index degrades versus a one-shot build. It needs two
prep artifacts in S3, both produced by scripts and consumed by the workflow:

1. **Chunks.** Run `scripts/split_cohere_chunks.sh 1m`, which cuts `sampled/1m` into
   `sampled/1m_chunk0 … sampled/1m_chunk9` by `ntile(10) OVER (ORDER BY _id)`. The union of all ten
   equals `sampled/1m` exactly, so the final step's corpus matches the one-shot benchmark. The
   workflow loads `1m_chunk0` (creating the table + index), then `load-heap --append`s each later
   chunk. (The script takes a size argument, so larger targets can be split the same way if needed.)

2. **Ground truth.** Run `scripts/generate_cohere_ground_truth.sh 1m`, which computes exact top-10
   recall ground truth in DuckDB (brute-force cosine, plus the stemmed text filter for the filtered
   queries) for each cumulative step and writes `queries/ground_truth_{query}_upd_1m_{cum}.parquet`.
   The workflow's `recall --ground-truth-path` loads these, so recall stays on its normal
   precomputed-parquet path. This reproduces the method behind the existing standalone
   `ground_truth_{query}_1m.parquet` (verified id-for-id).

```text
s3://<dataset-bucket>/datasets/cohere/
├── sampled/
│   ├── 1m/                          # split source (unchanged)
│   ├── 1m_chunk0/ … 1m_chunk9/      # from split_cohere_chunks.sh 1m
│   └── ...
└── queries/
    ├── cohere_queries.parquet                          # held-out query vectors (existing)
    └── ground_truth_{query}_upd_1m_{100k…1m}.parquet   # from generate_cohere_ground_truth.sh 1m
```
