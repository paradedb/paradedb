// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
use crate::fixtures::io_s3_obj_store::LocalStackS3Client;
use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use criterion::{BenchmarkId, Criterion};
use datafusion::dataframe::DataFrame;
use rand::Rng;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::fixtures::postgres_test_client::{ConnectionType, PostgresTestClient};
use crate::tables::benchlogs_extn_hsp_pq::{
    EsLogBenchManager, EsLogParquetForeignTableManager, EsLogParquetManager,
};
use camino::Utf8PathBuf;

// Constants for benchmark configuration
const SAMPLE_SIZE: usize = 60;
const MEASUREMENT_TIME_SECS: u64 = 200;
const WARM_UP_TIME_SECS: u64 = 2;
const DS_ESLOG_TOTAL_EVENTS: u64 = 10_000;
const DS_ESLOG_CHUNK_SIZE: u64 = 1_000;
const TOTAL_RECORDS: usize = 10_000;

const PG_POOL_CONN_MAX: usize = 2;

const S3_BUCKET_ID: &str = "demo-mlp-eslogs";
const S3_PREFIX: &str = "eslogs_dataset";

const RUN_BENCH_WITH_PG_CONN_TYPE: ConnectionType = ConnectionType::Exclusive;

#[derive(Clone)]
struct BenchResource {
    df: Arc<DataFrame>,
    s3_storage: Arc<LocalStackS3Client>,
    db: Arc<PostgresTestClient>,
}

impl BenchResource {
    async fn new(postgres_url: &str) -> Result<Self> {
        let (df, s3_storage, db) = Self::setup_benchmark(postgres_url)
            .await
            .with_context(|| "Failed to setup benchmark".to_string())?;

        Ok(Self {
            df: Arc::new(df),
            s3_storage: Arc::new(s3_storage),
            db: Arc::new(db),
        })
    }

    async fn setup_benchmark(
        postgres_url: &str,
    ) -> Result<(DataFrame, LocalStackS3Client, PostgresTestClient)> {
        // Initialize database
        let db = PostgresTestClient::new(postgres_url, PG_POOL_CONN_MAX).await?;

        db.execute_with_connection(ConnectionType::Exclusive, |pg_conn| {
            Box::pin(async move {
                sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_analytics;")
                    .execute(pg_conn)
                    .await?;
                Ok(())
            })
        })
        .await?;

        // Generate and load data
        let parquet_dataset_path = Self::parquet_path();
        tracing::warn!("parquet_path :: {:#?}", parquet_dataset_path);

        // Check if the Parquet file already exists at the specified path.
        if !parquet_dataset_path.exists() || !Self::dataset_exists(&parquet_dataset_path) {
            let seed = Self::generate_seed();
            tracing::info!("Generating ESLogs dataset with seed: {}", seed);
            EsLogParquetManager::create_hive_partitioned_parquet(
                DS_ESLOG_TOTAL_EVENTS,
                seed,
                DS_ESLOG_CHUNK_SIZE,
                &parquet_dataset_path,
            )
            .context("Failed to generate and save ESLogs dataset as local Parquet")?;
        } else {
            tracing::warn!(
                "Path Status :: exists: {}, contains files: {}",
                parquet_dataset_path.exists(),
                Self::dataset_exists(&parquet_dataset_path)
            );
        }

        // Verify that the Parquet files were created
        if !Self::dataset_exists(&parquet_dataset_path) {
            tracing::error!(
                "Parquet dataset was not created at {:?}",
                parquet_dataset_path
            );
            return Err(anyhow::anyhow!("Parquet dataset was not created"));
        }

        // List the contents of the dataset directory
        tracing::debug!("Listing contents of dataset directory:");
        for entry in fs::read_dir(&parquet_dataset_path)? {
            let entry = entry?;
            tracing::debug!("{:?}", entry.path());
        }

        // Create DataFrame from Parquet file
        let eslogs_df = EsLogParquetManager::read_parquet_dataset(&parquet_dataset_path)
            .await
            .context("Failed to read ESLogs dataset from local Parquet")?;

        // Set up S3
        let s3_storage = LocalStackS3Client::new()?;

        EsLogParquetManager::upload_parquet_dataset_to_s3(
            &s3_storage,
            S3_BUCKET_ID,
            S3_PREFIX,
            parquet_dataset_path,
        )
        .await
        .context("Failed to upload Parquet dataset to S3")?;

        s3_storage.print_object_list(S3_BUCKET_ID, S3_PREFIX)?;

        Ok((eslogs_df, s3_storage, db))
    }

    fn parquet_path() -> PathBuf {
        let target_dir = MetadataCommand::new()
            .no_deps()
            .exec()
            .map(|metadata| metadata.workspace_root)
            .unwrap_or_else(|err| {
                tracing::warn!(
                    "Failed to get workspace root: {}. Using 'target' as fallback.",
                    err
                );
                Utf8PathBuf::from("target")
            });

        let parquet_path = target_dir
            .join("target")
            .join("tmp_dataset")
            .join("eslogs_dataset");

        // Check if the file exists; if not, create the necessary directories
        if !parquet_path.exists() {
            if let Some(parent_dir) = parquet_path.parent() {
                fs::create_dir_all(parent_dir)
                    .with_context(|| format!("Failed to create directory: {:#?}", parent_dir))
                    .unwrap_or_else(|err| {
                        tracing::error!("{}", err);
                        panic!("Critical error: {}", err);
                    });
            }
        }

        parquet_path.into()
    }

    /// Generates a random seed for reproducible data generation.
    fn generate_seed() -> u64 {
        let mut rng = rand::thread_rng();
        let seed: u32 = rng.gen();
        tracing::debug!("Generated random seed: {}", seed);
        u64::from(seed)
    }

    /// Checks if the given path is a directory and is empty.
    fn dataset_exists(base_path: &Path) -> bool {
        let exists = base_path.exists()
            && base_path
                .read_dir()
                .map(|mut i| i.next().is_some())
                .unwrap_or(false);
        tracing::debug!("Dataset exists at {:?}: {}", base_path, exists);
        exists
    }

    #[allow(clippy::await_holding_lock)]
    async fn setup_tables(
        &self,
        foreign_table_id: String,
        with_disk_cache: bool,
        with_mem_cache: bool,
    ) -> Result<()> {
        // Clone Arc to avoid holding the lock across await points

        let db = Arc::clone(&self.db);
        let s3_storage = Arc::clone(&self.s3_storage);

        db.execute_with_connection(RUN_BENCH_WITH_PG_CONN_TYPE, |pg_conn| {
            Box::pin(async move {
                EsLogParquetForeignTableManager::setup_tables(
                    pg_conn,
                    &s3_storage,
                    S3_BUCKET_ID,
                    S3_PREFIX,
                    &foreign_table_id,
                    with_disk_cache,
                )
                .await?;

                let with_mem_cache_cfg = if with_mem_cache { "true" } else { "false" };
                let query = format!(
                    "SELECT duckdb_execute($$SET enable_object_cache={}$$)",
                    with_mem_cache_cfg
                );
                sqlx::query(&query).execute(pg_conn).await?;

                Ok(())
            })
        })
        .await
    }

    #[allow(clippy::await_holding_lock)]
    async fn bench_total_sales(&self, foreign_table_id: String) -> Result<()> {
        let db = Arc::clone(&self.db);
        let df = Arc::clone(&self.df);

        db.execute_with_connection(RUN_BENCH_WITH_PG_CONN_TYPE, |pg_conn| {
            Box::pin(async move {
                EsLogBenchManager::bench_time_range_query(pg_conn, &df, &foreign_table_id, 10).await
            })
        })
        .await
    }
}

async fn eslog_pq_disk_cache_bench(postgres_url: &str, c: &mut Criterion) {
    tracing::info!("Starting ESLog PQ disk cache benchmark");

    let bench_resource = match BenchResource::new(postgres_url).await {
        Ok(resource) => resource,
        Err(e) => {
            tracing::error!("Failed to initialize BenchResource: {}", e);
            return;
        }
    };

    let foreign_table_id: String = String::from("eslog_pq_disk_cache");

    let mut group = c.benchmark_group("Eslog PQ Disk Cache Benchmarks");
    group.sample_size(10); // Adjust sample size if necessary

    // Setup tables for the benchmark
    if let Err(e) = bench_resource
        .setup_tables(foreign_table_id.clone(), true, false)
        .await
    {
        tracing::error!("Table setup failed: {}", e);
    }

    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .throughput(criterion::Throughput::Elements(TOTAL_RECORDS as u64))
        .bench_function(BenchmarkId::new("ESLog PQ", "Disk Cache"), |runner| {
            runner.iter(|| {
                let _ = async_std::task::block_on(
                    bench_resource.bench_total_sales(foreign_table_id.clone()),
                )
                .context("Benchmark execution failed");
            });
        });

    tracing::info!("Mem cache benchmark completed");
    group.finish();
}

async fn eslog_pq_mem_cache_bench(postgres_url: &str, c: &mut Criterion) {
    tracing::info!("Starting ESLog PQ mem cache benchmark");

    let bench_resource = match BenchResource::new(postgres_url).await {
        Ok(resource) => resource,
        Err(e) => {
            tracing::error!("Failed to initialize BenchResource: {}", e);
            return;
        }
    };

    let foreign_table_id: String = String::from("eslog_pq_mem_cache");

    let mut group = c.benchmark_group("Eslog PQ Mem Cache Benchmarks");
    group.sample_size(10); // Adjust sample size if necessary

    // Setup tables for the benchmark
    if let Err(e) = bench_resource
        .setup_tables(foreign_table_id.clone(), false, true)
        .await
    {
        tracing::error!("Table setup failed: {}", e);
    }

    bench_resource
        .bench_total_sales(foreign_table_id.clone())
        .await
        .unwrap();

    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .throughput(criterion::Throughput::Elements(TOTAL_RECORDS as u64))
        .bench_function(BenchmarkId::new("ESLog PQ", "Mem Cache"), |runner| {
            runner.iter(|| {
                let _ = async_std::task::block_on(
                    bench_resource.bench_total_sales(foreign_table_id.clone()),
                )
                .context("Benchmark execution failed");
            });
        });

    tracing::info!("Mem cache benchmark completed");
    group.finish();
}

async fn eslog_pq_full_cache_bench(postgres_url: &str, c: &mut Criterion) {
    tracing::info!("Starting ESLog PQ Full cache benchmark");

    let bench_resource = match BenchResource::new(postgres_url).await {
        Ok(resource) => resource,
        Err(e) => {
            tracing::error!("Failed to initialize BenchResource: {}", e);
            return;
        }
    };

    let foreign_table_id: String = String::from("eslog_pq_full_cache");

    let mut group = c.benchmark_group("Eslog PQ Full Cache Benchmarks");
    group.sample_size(10); // Adjust sample size if necessary

    // Setup tables for the benchmark
    if let Err(e) = bench_resource
        .setup_tables(foreign_table_id.clone(), true, true)
        .await
    {
        tracing::error!("Table setup failed: {}", e);
    }

    bench_resource
        .bench_total_sales(foreign_table_id.clone())
        .await
        .unwrap();

    // Run the benchmark with no cache
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .throughput(criterion::Throughput::Elements(TOTAL_RECORDS as u64))
        .bench_function(BenchmarkId::new("ESLog PQ", "Full Cache"), |runner| {
            runner.iter(|| {
                let _ = async_std::task::block_on(
                    bench_resource.bench_total_sales(foreign_table_id.clone()),
                )
                .context("Benchmark execution failed");
            });
        });

    tracing::info!("Full cache benchmark completed");
    group.finish();
}

async fn eslog_pq_no_cache_bench(postgres_url: &str, c: &mut Criterion) {
    tracing::info!("Starting ESLog PQ no cache benchmark");

    let bench_resource = match BenchResource::new(postgres_url).await {
        Ok(resource) => resource,
        Err(e) => {
            tracing::error!("Failed to initialize BenchResource: {}", e);
            return;
        }
    };

    let foreign_table_id: String = String::from("eslog_no_cache");

    let mut group = c.benchmark_group("Eslog PQ No Cache Benchmarks");
    group.sample_size(10); // Adjust sample size if necessary

    // Setup tables for the benchmark
    if let Err(e) = bench_resource
        .setup_tables(foreign_table_id.clone(), false, false)
        .await
    {
        tracing::error!("Table setup failed: {}", e);
    }

    bench_resource
        .bench_total_sales(foreign_table_id.clone())
        .await
        .unwrap();

    // Run the benchmark with no cache
    group
        .sample_size(SAMPLE_SIZE)
        .measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS))
        .warm_up_time(Duration::from_secs(WARM_UP_TIME_SECS))
        .throughput(criterion::Throughput::Elements(TOTAL_RECORDS as u64))
        .bench_function(BenchmarkId::new("ESLog PQ", "No Cache"), |runner| {
            runner.iter(|| {
                let _ = async_std::task::block_on(
                    bench_resource.bench_total_sales(foreign_table_id.clone()),
                )
                .context("Benchmark execution failed");
            });
        });

    tracing::info!("No cache benchmark completed");
    group.finish();
}

async fn pga_benchlogs_hsp_pq(postgres_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize criterion
    let mut criterion = Criterion::default();

    eslog_pq_disk_cache_bench(postgres_url, &mut criterion).await;
    eslog_pq_mem_cache_bench(postgres_url, &mut criterion).await;
    eslog_pq_full_cache_bench(postgres_url, &mut criterion).await;
    eslog_pq_no_cache_bench(postgres_url, &mut criterion).await;

    Ok(())
}

pub async fn run_all_bench(postgres_url: &str) -> Result<()> {
    if let Err(err) = pga_benchlogs_hsp_pq(postgres_url).await {
        tracing::error!("TODO:: {}", err);
    }
    Ok(())
}
