# Preparing a Dataset

This document walks through the full process of preparing a new non-synthetic dataset for benchmarking. The high-level steps are:

1. Load source data into S3
2. Create a config and sample the data at each size you need
3. Convert the sampled parquet data to CSV

## Step 1: Load Source Data into S3

Upload your source data as partitioned parquet files to S3. Each table should be in its own subdirectory under a `source/parquet/` path (filename doesn't matter):

```text
s3://paradedb-benchmarks/datasets/{dataset-name}/source/parquet/
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
s3://paradedb-benchmarks/datasets/stackoverflow/source/parquet/
├── stackoverflow_posts/
├── comments/
└── users/
```

## Step 2: Create a Config and Sample

### Writing the Config

Create a TOML config file at `datasets/{dataset-name}/config.toml` that describes your table relationships. The config specifies which table is the root (the one that gets sampled directly) and how child tables relate to it via joins.

```toml
root_table = "root_table_name"
sampling_seed = 723               # Fixed seed for deterministic results

[[tables]]
name = "root_table_name"

[[tables]]
name = "child_table"
parent = "root_table_name"
parent_join_col = "id"            # Column in the parent table
join_col = "parent_id"            # Corresponding column in the child table
```

Fields:

- `root_table` -- The primary table. The `--rows` argument controls how many rows are sampled from this table.
- `sampling_seed` -- Seed for deterministic, reproducible sampling.
- `[[tables]]` -- One entry per table. The root table has no `parent`. Child tables specify `parent`, `parent_join_col`, and `join_col` to define the relationship.

Child tables are not sampled independently. Instead, they are filtered via an inner join with their parent table, so only rows that reference a sampled parent row are kept. This preserves referential integrity across the dataset.

Tables can form a hierarchy (a child can be a parent of another table). They are processed in topological order.

See `datasets/stackoverflow/config.toml` for a real example.

### Running the Sampling Tool

Run the `sample` command once for each dataset size you need. The `--rows` argument sets the target row count for the root table. Output goes to the `sampled/{size}/parquet/` path.

```bash
# Sample to 10k rows
cargo run --release -- sample \
  --input s3://paradedb-benchmarks/datasets/stackoverflow/source/parquet/ \
  --output s3://paradedb-benchmarks/datasets/stackoverflow/sampled/10k/parquet/ \
  --config ./datasets/stackoverflow/config.toml \
  --rows 10000

# Sample to 100k rows
cargo run --release -- sample \
  --input s3://paradedb-benchmarks/datasets/stackoverflow/source/parquet/ \
  --output s3://paradedb-benchmarks/datasets/stackoverflow/sampled/100k/parquet/ \
  --config ./datasets/stackoverflow/config.toml \
  --rows 100000

# Sample to 1m rows
cargo run --release -- sample \
  --input s3://paradedb-benchmarks/datasets/stackoverflow/source/parquet/ \
  --output s3://paradedb-benchmarks/datasets/stackoverflow/sampled/1m/parquet/ \
  --config ./datasets/stackoverflow/config.toml \
  --rows 1000000
```

Notes:

- The output path must be empty (no pre-existing data).
- For small targets (<=100k rows), sampling is exact using reservoir sampling. For larger targets, it uses system sampling and the result will be approximate (within ~3-5%).
- Use `--dry-run` to validate inputs and see planned row counts without writing anything.

## Step 3: Convert Sampled Data to CSV

Run the `convert` command for each sampled size to produce CSV versions. The `--tables` flag takes a comma-separated list of all tables to convert.

```bash
# Convert 10k sampled data
cargo run --release -- convert \
  --input s3://paradedb-benchmarks/datasets/stackoverflow/sampled/10k/parquet/ \
  --output s3://paradedb-benchmarks/datasets/stackoverflow/sampled/10k/csv/ \
  --tables stackoverflow_posts,comments,users

# Convert 100k sampled data
cargo run --release -- convert \
  --input s3://paradedb-benchmarks/datasets/stackoverflow/sampled/100k/parquet/ \
  --output s3://paradedb-benchmarks/datasets/stackoverflow/sampled/100k/csv/ \
  --tables stackoverflow_posts,comments,users

# Convert 1m sampled data
cargo run --release -- convert \
  --input s3://paradedb-benchmarks/datasets/stackoverflow/sampled/1m/parquet/ \
  --output s3://paradedb-benchmarks/datasets/stackoverflow/sampled/1m/csv/ \
  --tables stackoverflow_posts,comments,users
```

You can also convert the full source data:

```bash
cargo run --release -- convert \
  --input s3://paradedb-benchmarks/datasets/stackoverflow/source/parquet/ \
  --output s3://paradedb-benchmarks/datasets/stackoverflow/source/csv/ \
  --tables stackoverflow_posts,comments,users
```

Notes:

- The output path must be empty.
- Row counts are verified after conversion to ensure no data is lost.
- Use `--dry-run` to validate without writing.
- AWS credentials must be accessible via the standard credential chain (env vars, `~/.aws/credentials`, or instance metadata).

## Final S3 Layout

After completing all three steps, your dataset will look like this:

```text
s3://paradedb-benchmarks/datasets/{dataset-name}/
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
```
