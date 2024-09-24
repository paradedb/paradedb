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
use super::benchlogs::EsLog;
use crate::fixtures::io_s3_obj_store::LocalStackS3Client;
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Utc};
use datafusion::arrow::array::*;
use datafusion::arrow::array::{
    Int32Builder, ListBuilder, StringBuilder, StructBuilder, TimestampMillisecondBuilder,
};
use datafusion::arrow::datatypes::{DataType, Field, Fields, Schema, TimeUnit};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::parquet::arrow::ArrowWriter;
use datafusion::prelude::*;
use rayon::prelude::*;
use sqlx::Executor;
use sqlx::PgConnection;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tempfile::tempdir;

pub struct EsLogParquetManager;

impl EsLogParquetManager {
    fn download_file(url: &str, path: &Path) -> Result<()> {
        let response = ureq::get(url).call()?;
        let mut file = std::fs::File::create(path)?;
        std::io::copy(&mut response.into_reader(), &mut file)?;
        Ok(())
    }

    pub async fn read_parquet_dataset(base_path: &Path) -> Result<DataFrame> {
        tracing::debug!("Reading Parquet dataset from {:?}", base_path);
        let ctx = SessionContext::new();
        let df = ctx
            .read_parquet(
                base_path.to_str().unwrap(),
                ParquetReadOptions::default().table_partition_cols(vec![
                    ("year".to_string(), DataType::Utf8),
                    ("month".to_string(), DataType::Utf8),
                ]),
            )
            .await
            .with_context(|| format!("Failed to read Parquet dataset from {:?}", base_path))?;
        tracing::info!("Successfully read Parquet dataset from {:?}", base_path);
        Ok(df)
    }

    fn check_golang_installation() -> Result<()> {
        Command::new("go")
            .arg("version")
            .output()
            .context("Golang is not installed")?;
        Ok(())
    }

    fn install_generator_tool() -> Result<()> {
        tracing::debug!("Installing elastic-integration-corpus-generator-tool");
        Command::new("go")
            .args([
                "install",
                "github.com/elastic/elastic-integration-corpus-generator-tool@latest",
            ])
            .output()
            .context("Failed to install generator tool")?;
        Ok(())
    }

    fn download_config_files(config_tempdir: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
        let template_file = config_tempdir.join("template.tpl");
        let fields_file = config_tempdir.join("fields.yml");
        let config_file = config_tempdir.join("config-1.yml");

        let opensearch_repo_url =
            "https://raw.githubusercontent.com/elastic/elasticsearch-opensearch-benchmark/main";

        let files = [
            ("template.tpl", &template_file),
            ("fields.yml", &fields_file),
            ("config-1.yml", &config_file),
        ];

        files.iter().try_for_each(|(filename, path)| {
            Self::download_file(
                &format!("{}/dataset/{}", opensearch_repo_url, filename),
                path,
            )
        })?;

        Ok((template_file, fields_file, config_file))
    }

    fn get_generator_exe_path() -> Result<String> {
        let go_path = String::from_utf8(
            Command::new("go")
                .args(["env", "GOPATH"])
                .output()
                .context("Failed to get GOPATH")?
                .stdout,
        )?;
        Ok(format!(
            "{}/bin/elastic-integration-corpus-generator-tool",
            go_path.trim()
        ))
    }

    fn create_output_directory() -> Result<PathBuf> {
        let generated_tempdir =
            tempdir().context("Failed to create temporary directory for generated files")?;
        let generated_dir = generated_tempdir.path().join("generated");
        fs::create_dir_all(&generated_dir)
            .context("Failed to create directory for generated files")?;
        std::env::set_var("DATA_DIR", &generated_dir);
        Ok(generated_dir)
    }

    fn run_generator_tool(
        generator_exe: &str,
        template_file: &Path,
        fields_file: &Path,
        config_file: &Path,
        events: u64,
        seed: u64,
    ) -> Result<std::process::Output> {
        tracing::debug!("Running generator tool");
        let output = Command::new(generator_exe)
            .args([
                "generate-with-template",
                template_file.to_str().unwrap(),
                fields_file.to_str().unwrap(),
                "--tot-events",
                &events.to_string(),
                "--config-file",
                config_file.to_str().unwrap(),
                "--template-type",
                "gotext",
                "--seed",
                &seed.to_string(),
            ])
            .output()
            .context("Failed to run generator tool")?;

        if !output.status.success() {
            tracing::error!(
                "Generator tool failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            anyhow::bail!("Generator tool failed");
        }

        tracing::info!(
            "Generator tool stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        tracing::debug!(
            "Generator tool stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        Ok(output)
    }

    fn extract_output_file_path(stdout: &[u8]) -> Result<PathBuf> {
        let stdout = String::from_utf8_lossy(stdout);
        let file_path = stdout
            .lines()
            .find(|line| line.starts_with("File generated:"))
            .and_then(|line| line.split(": ").nth(1))
            .context("Could not find generated file path in output")?;

        let output_file_path = PathBuf::from(file_path);
        tracing::debug!("Extracted output file path: {:?}", output_file_path);
        Ok(output_file_path)
    }

    fn validate_output_file(output_file_path: &Path) -> Result<()> {
        if !output_file_path.exists() {
            tracing::error!("Output file does not exist: {:?}", output_file_path);
            anyhow::bail!("Output file does not exist");
        }
        Ok(())
    }

    fn read_and_parse_logs(output_file_path: &Path) -> Result<impl Iterator<Item = EsLog>> {
        let file = fs::File::open(output_file_path)?;
        let reader = BufReader::new(file);

        let logs: Vec<EsLog> = reader
            .lines()
            .enumerate()
            .par_bridge() // Use Rayon for parallel processing
            .filter_map(|(i, line)| match line {
                Ok(json_str) => match serde_json::from_str::<EsLog>(&json_str) {
                    Ok(log) => {
                        if i % 1000 == 0 {
                            tracing::debug!("Generated log {}", i);
                        }
                        Some(log)
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse log entry {}: {}", i, e);
                        None
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read line {} from file: {}", i, e);
                    None
                }
            })
            .collect();

        Ok(logs.into_iter())
    }

    fn verify_files(files: &[&Path]) -> Result<()> {
        for file in files {
            if !file.exists() {
                tracing::error!("File does not exist: {:?}", file);
                anyhow::bail!("Required file does not exist: {:?}", file);
            }

            if let Err(e) = fs::File::open(file) {
                tracing::error!("Unable to open file {:?}: {}", file, e);
                anyhow::bail!("Unable to open required file: {:?}", file);
            }

            tracing::debug!("Verified file exists and is readable: {:?}", file);
        }

        tracing::info!("All required files verified successfully");
        Ok(())
    }

    pub fn generate_dataset(events: u64, seed: u64) -> Result<impl Iterator<Item = EsLog>> {
        tracing::info!(
            "Generating dataset with {} events and seed {}",
            events,
            seed
        );

        Self::check_golang_installation()?;
        Self::install_generator_tool()?;

        let config_tempdir = tempdir().context("Failed to create temporary directory")?;

        let (template_file, fields_file, config_file) =
            Self::download_config_files(config_tempdir.path())?;

        // Verify that all files exist and are readable
        Self::verify_files(&[&template_file, &fields_file, &config_file])?;

        let generator_exe = Self::get_generator_exe_path()?;
        let _generated_dir = Self::create_output_directory()?;

        let output = Self::run_generator_tool(
            &generator_exe,
            &template_file,
            &fields_file,
            &config_file,
            events,
            seed,
        )?;
        let output_file_path = Self::extract_output_file_path(&output.stdout)?;

        Self::validate_output_file(&output_file_path)?;

        let logs = Self::read_and_parse_logs(&output_file_path)?;

        tracing::info!("Successfully generated dataset");
        Ok(logs)
    }

    pub fn create_hive_partitioned_parquet(
        events: u64,
        seed: u64,
        chunk_size: usize,
        base_path: &Path,
    ) -> Result<()> {
        tracing::info!(
            "Creating Hive-partitioned Parquet with {} events, seed {}, chunk size {}",
            events,
            seed,
            chunk_size
        );
        tracing::debug!("Base path for Parquet files: {:?}", base_path);

        let logs = Self::generate_dataset(events, seed)?;
        let mut log_cache: HashMap<(i32, u32), Vec<EsLog>> = HashMap::new();
        let mut log_count = 0;

        tracing::debug!("Starting to process logs");
        for log in logs {
            log_count += 1;
            if log_count % 1000 == 0 {
                tracing::debug!("Processed {} logs", log_count);
            }

            let year = log.timestamp.year();
            let month = log.timestamp.month();
            let key = (year, month);

            log_cache.entry(key).or_default().push(log);

            if log_cache.get(&key).unwrap().len() >= chunk_size {
                let partition_path = base_path.join(format!("year={}/month={:02}", year, month));
                tracing::debug!("Creating partition directory: {:?}", partition_path);
                fs::create_dir_all(&partition_path).with_context(|| {
                    format!("Failed to create partition directory: {:?}", partition_path)
                })?;

                let file_path =
                    partition_path.join(format!("data_{:x}.parquet", Utc::now().timestamp()));
                tracing::debug!("Saving Parquet file: {:?}", file_path);

                if let Some(logs_to_write) = log_cache.remove(&key) {
                    Self::save_to_parquet(logs_to_write, &file_path)
                        .with_context(|| format!("Failed to save Parquet file: {:?}", file_path))?;
                }
            }
        }
        tracing::debug!(
            "Finished processing logs. Total logs processed: {}",
            log_count
        );

        // Write any remaining logs in the cache
        for ((year, month), logs) in log_cache {
            let partition_path = base_path.join(format!("year={}/month={:02}", year, month));
            tracing::debug!("Creating partition directory: {:?}", partition_path);
            fs::create_dir_all(&partition_path).with_context(|| {
                format!("Failed to create partition directory: {:?}", partition_path)
            })?;

            let file_path = partition_path.join(format!("data_{}.parquet", Utc::now().timestamp()));
            tracing::debug!("Saving Parquet file: {:?}", file_path);
            Self::save_to_parquet(logs, &file_path)
                .with_context(|| format!("Failed to save Parquet file: {:?}", file_path))?;
        }

        tracing::info!(
            "Successfully created Hive-partitioned Parquet at {:?}",
            base_path
        );
        Ok(())
    }

    pub fn save_to_parquet(logs: Vec<EsLog>, path: &Path) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new(
                "timestamp",
                DataType::Timestamp(TimeUnit::Millisecond, None),
                false,
            ),
            Field::new(
                "aws_cloudwatch",
                DataType::Struct(Fields::from(vec![
                    Field::new("log_stream", DataType::Utf8, true),
                    Field::new("ingestion_time", DataType::Utf8, true),
                    Field::new("log_group", DataType::Utf8, true),
                ])),
                true,
            ),
            Field::new(
                "cloud",
                DataType::Struct(Fields::from(vec![Field::new(
                    "region",
                    DataType::Utf8,
                    true,
                )])),
                true,
            ),
            Field::new("log_file_path", DataType::Utf8, false),
            Field::new(
                "input",
                DataType::Struct(Fields::from(vec![Field::new("type", DataType::Utf8, true)])),
                true,
            ),
            Field::new(
                "data_stream",
                DataType::Struct(Fields::from(vec![
                    Field::new("namespace", DataType::Utf8, true),
                    Field::new("type", DataType::Utf8, true),
                    Field::new("dataset", DataType::Utf8, true),
                ])),
                true,
            ),
            Field::new(
                "process",
                DataType::Struct(Fields::from(vec![Field::new("name", DataType::Utf8, true)])),
                true,
            ),
            Field::new("message", DataType::Utf8, false),
            Field::new(
                "event",
                DataType::Struct(Fields::from(vec![
                    Field::new("id", DataType::Utf8, true),
                    Field::new("ingested", DataType::Utf8, true),
                    Field::new("dataset", DataType::Utf8, true),
                ])),
                true,
            ),
            Field::new(
                "host",
                DataType::Struct(Fields::from(vec![Field::new("name", DataType::Utf8, true)])),
                true,
            ),
            Field::new("metrics_size", DataType::Int32, false),
            Field::new(
                "agent",
                DataType::Struct(Fields::from(vec![
                    Field::new("id", DataType::Utf8, true),
                    Field::new("name", DataType::Utf8, true),
                    Field::new("type", DataType::Utf8, true),
                    Field::new("version", DataType::Utf8, true),
                    Field::new("ephemeral_id", DataType::Utf8, true),
                ])),
                true,
            ),
            Field::new(
                "tags",
                DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                true,
            ),
        ]));

        let file = fs::File::create(path).context("Failed to create Parquet file")?;
        let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;

        let mut timestamp_builder = TimestampMillisecondBuilder::new();
        let mut aws_cloudwatch_builder = StructBuilder::from_fields(
            Fields::from(vec![
                Field::new("log_stream", DataType::Utf8, true),
                Field::new("ingestion_time", DataType::Utf8, true),
                Field::new("log_group", DataType::Utf8, true),
            ]),
            logs.len(),
        );
        let mut cloud_builder = StructBuilder::from_fields(
            Fields::from(vec![Field::new("region", DataType::Utf8, true)]),
            logs.len(),
        );
        let mut log_file_path_builder = StringBuilder::new();
        let mut input_builder = StructBuilder::from_fields(
            Fields::from(vec![Field::new("type", DataType::Utf8, true)]),
            logs.len(),
        );
        let mut data_stream_builder = StructBuilder::from_fields(
            Fields::from(vec![
                Field::new("namespace", DataType::Utf8, true),
                Field::new("type", DataType::Utf8, true),
                Field::new("dataset", DataType::Utf8, true),
            ]),
            logs.len(),
        );
        let mut process_builder = StructBuilder::from_fields(
            Fields::from(vec![Field::new("name", DataType::Utf8, true)]),
            logs.len(),
        );
        let mut message_builder = StringBuilder::new();
        let mut event_builder = StructBuilder::from_fields(
            Fields::from(vec![
                Field::new("id", DataType::Utf8, true),
                Field::new("ingested", DataType::Utf8, true),
                Field::new("dataset", DataType::Utf8, true),
            ]),
            logs.len(),
        );
        let mut host_builder = StructBuilder::from_fields(
            Fields::from(vec![Field::new("name", DataType::Utf8, true)]),
            logs.len(),
        );
        let mut metrics_size_builder = Int32Builder::new();
        let mut agent_builder = StructBuilder::from_fields(
            Fields::from(vec![
                Field::new("id", DataType::Utf8, true),
                Field::new("name", DataType::Utf8, true),
                Field::new("type", DataType::Utf8, true),
                Field::new("version", DataType::Utf8, true),
                Field::new("ephemeral_id", DataType::Utf8, true),
            ]),
            logs.len(),
        );
        let mut tags_builder = ListBuilder::new(StringBuilder::new());

        for log in &logs {
            timestamp_builder.append_value(log.timestamp.timestamp_millis());

            // AWS Cloudwatch
            aws_cloudwatch_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(
                    log.aws_cloudwatch
                        .get("log_stream")
                        .and_then(|v| v.as_str()),
                );
            aws_cloudwatch_builder
                .field_builder::<StringBuilder>(1)
                .unwrap()
                .append_option(
                    log.aws_cloudwatch
                        .get("ingestion_time")
                        .and_then(|v| v.as_str()),
                );
            aws_cloudwatch_builder
                .field_builder::<StringBuilder>(2)
                .unwrap()
                .append_option(log.aws_cloudwatch.get("log_group").and_then(|v| v.as_str()));
            aws_cloudwatch_builder.append(true);

            // Cloud
            cloud_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(log.cloud.get("region").and_then(|v| v.as_str()));
            cloud_builder.append(true);

            log_file_path_builder.append_value(&log.log_file_path);

            // Input
            input_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(log.input.get("type").and_then(|v| v.as_str()));
            input_builder.append(true);

            // Data Stream
            data_stream_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(log.data_stream.get("namespace").and_then(|v| v.as_str()));
            data_stream_builder
                .field_builder::<StringBuilder>(1)
                .unwrap()
                .append_option(log.data_stream.get("type").and_then(|v| v.as_str()));
            data_stream_builder
                .field_builder::<StringBuilder>(2)
                .unwrap()
                .append_option(log.data_stream.get("dataset").and_then(|v| v.as_str()));
            data_stream_builder.append(true);

            // Process
            process_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(log.process.get("name").and_then(|v| v.as_str()));
            process_builder.append(true);

            message_builder.append_value(&log.message);

            // Event
            event_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(log.event.get("id").and_then(|v| v.as_str()));
            event_builder
                .field_builder::<StringBuilder>(1)
                .unwrap()
                .append_option(log.event.get("ingested").and_then(|v| v.as_str()));
            event_builder
                .field_builder::<StringBuilder>(2)
                .unwrap()
                .append_option(log.event.get("dataset").and_then(|v| v.as_str()));
            event_builder.append(true);

            // Host
            host_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(log.host.get("name").and_then(|v| v.as_str()));
            host_builder.append(true);

            metrics_size_builder.append_value(log.metrics_size);

            // Agent
            agent_builder
                .field_builder::<StringBuilder>(0)
                .unwrap()
                .append_option(log.agent.get("id").and_then(|v| v.as_str()));
            agent_builder
                .field_builder::<StringBuilder>(1)
                .unwrap()
                .append_option(log.agent.get("name").and_then(|v| v.as_str()));
            agent_builder
                .field_builder::<StringBuilder>(2)
                .unwrap()
                .append_option(log.agent.get("type").and_then(|v| v.as_str()));
            agent_builder
                .field_builder::<StringBuilder>(3)
                .unwrap()
                .append_option(log.agent.get("version").and_then(|v| v.as_str()));
            agent_builder
                .field_builder::<StringBuilder>(4)
                .unwrap()
                .append_option(log.agent.get("ephemeral_id").and_then(|v| v.as_str()));
            agent_builder.append(true);

            // Tags
            let tag_builder = tags_builder.values();
            for tag in &log.tags {
                tag_builder.append_value(tag);
            }
            tags_builder.append(true);
        }

        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(timestamp_builder.finish()),
                Arc::new(aws_cloudwatch_builder.finish()),
                Arc::new(cloud_builder.finish()),
                Arc::new(log_file_path_builder.finish()),
                Arc::new(input_builder.finish()),
                Arc::new(data_stream_builder.finish()),
                Arc::new(process_builder.finish()),
                Arc::new(message_builder.finish()),
                Arc::new(event_builder.finish()),
                Arc::new(host_builder.finish()),
                Arc::new(metrics_size_builder.finish()),
                Arc::new(agent_builder.finish()),
                Arc::new(tags_builder.finish()),
            ],
        )?;

        writer.write(&batch)?;
        writer.close()?;

        Ok(())
    }

    #[allow(unused)]
    pub async fn upload_parquet_dataset_to_s3(
        s3: &LocalStackS3Client,
        s3_bucket: &str,
        s3_prefix: &str,
        parquet_dataset_path: PathBuf,
    ) -> Result<()> {
        tracing::info!("Starting upload of Parquet dataset to S3");

        // Ensure the S3 bucket exists
        s3.create_bucket(s3_bucket)
            .context("Failed to create S3 bucket")?;

        // Use the put_directory method to upload the entire dataset
        s3.put_directory(s3_bucket, s3_prefix, &parquet_dataset_path)
            .context("Failed to upload Parquet dataset to S3")?;

        tracing::info!("Completed upload of Parquet dataset to S3");
        Ok(())
    }
}

pub struct EsLogParquetForeignTableManager;

impl EsLogParquetForeignTableManager {
    #[allow(unused)]
    pub async fn teardown_tables(pg_conn: &mut PgConnection, foreign_table_id: &str) -> Result<()> {
        let sql_statements = [
            format!("DROP FOREIGN TABLE IF EXISTS {foreign_table_id} CASCADE"),
            "DROP SERVER IF EXISTS eslogs_ftw_server CASCADE".to_string(),
            "DROP FOREIGN DATA WRAPPER IF EXISTS parquet_wrapper CASCADE".to_string(),
            "DROP USER MAPPING IF EXISTS FOR public SERVER eslogs_ftw_server".to_string(),
        ];

        for statement in sql_statements {
            if let Err(err) = pg_conn.execute(&*statement).await {
                tracing::warn!("Failed to execute: {} :: Err: {}", statement, err);
            }
        }

        Ok(())
    }

    #[allow(unused)]
    pub async fn setup_tables(
        pg_conn: &mut PgConnection,
        s3: &LocalStackS3Client,
        s3_bucket: &str,
        s3_prefix: &str,
        foreign_table_id: &str,
        use_disk_cache: bool,
    ) -> Result<()> {
        tracing::info!(
            "Starting setup_tables for foreign_table_id: {}",
            foreign_table_id
        );

        // First, tear down any existing tables
        if let Err(e) = Self::teardown_tables(pg_conn, foreign_table_id).await {
            tracing::warn!(
                "Error during teardown_tables: {:?}. Continuing with setup.",
                e
            );
        }

        // Setup S3 Foreign Data Wrapper commands
        let s3_fdw_setup = Self::setup_s3_fdw(&s3.url);
        for (i, command) in s3_fdw_setup.split(';').enumerate() {
            let trimmed_command = command.trim();
            if !trimmed_command.is_empty() {
                match pg_conn.execute(trimmed_command).await {
                    Ok(_) => tracing::info!("Successfully executed S3 FDW setup command {}", i + 1),
                    Err(e) => {
                        tracing::error!("Error executing S3 FDW setup command {}: {:?}", i + 1, e);
                        return Err(e.into());
                    }
                }
            }
        }

        // Create partitioned foreign table
        let create_ftw_command = Self::create_partitioned_foreign_table(
            s3_bucket,
            s3_prefix,
            foreign_table_id,
            use_disk_cache,
        );
        match pg_conn.execute(&*create_ftw_command).await {
            Ok(_) => tracing::info!("Successfully created partitioned foreign table"),
            Err(e) => {
                tracing::error!("Error creating partitioned foreign table: {:?}", e);
                return Err(e.into());
            }
        }

        tracing::info!(
            "Completed setup_tables for foreign_table_id: {}",
            foreign_table_id
        );
        Ok(())
    }

    fn setup_s3_fdw(s3_endpoint: &str) -> String {
        format!(
            r#"
            CREATE FOREIGN DATA WRAPPER parquet_wrapper
                HANDLER parquet_fdw_handler
                VALIDATOR parquet_fdw_validator;
    
            CREATE SERVER eslogs_ftw_server
                FOREIGN DATA WRAPPER parquet_wrapper;
    
            CREATE USER MAPPING FOR public
                SERVER eslogs_ftw_server
                OPTIONS (
                    type 'S3',
                    region 'us-east-1',
                    endpoint '{s3_endpoint}',
                    use_ssl 'false',
                    url_style 'path'
                );
            "#
        )
    }

    fn create_partitioned_foreign_table(
        s3_bucket: &str,
        s3_prefix: &str,
        foreign_table_id: &str,
        use_disk_cache: bool,
    ) -> String {
        format!(
            r#"
            CREATE FOREIGN TABLE {foreign_table_id} (
                timestamp               TIMESTAMP WITH TIME ZONE,
                aws_cloudwatch          JSONB,
                cloud                   JSONB,
                log_file_path           TEXT,
                input                   JSONB,
                data_stream             JSONB,
                process                 JSONB,
                message                 TEXT,
                event                   JSONB,
                host                    JSONB,
                metrics_size            INT,
                agent                   JSONB,
                tags                    TEXT[]
            )
            SERVER eslogs_ftw_server
            OPTIONS (
                files 's3://{s3_bucket}/{s3_prefix}/year=*/month=*/data_*.parquet',
                hive_partitioning '1',
                cache '{}'
            );
            "#,
            use_disk_cache
        )
    }
}

pub struct EsLogBenchManager;

impl EsLogBenchManager {
    pub async fn bench_time_range_query(
        pg_conn: &mut PgConnection,
        eslogs_local_df: &DataFrame,
        foreign_table_id: &str,
    ) -> Result<()> {
        // Calculate min and max timestamps
        let min_max_df = eslogs_local_df
            .clone()
            .aggregate(
                vec![],
                vec![
                    min(col("timestamp")).alias("min_timestamp"),
                    max(col("timestamp")).alias("max_timestamp"),
                ],
            )?
            .collect()
            .await?;

        let min_max_batch = &min_max_df[0];

        let min_ts = min_max_batch
            .column_by_name("min_timestamp")
            .ok_or_else(|| anyhow::anyhow!("min_timestamp column not found"))?
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to downcast min_timestamp to TimestampMillisecondArray")
            })?
            .value(0);

        let max_ts = min_max_batch
            .column_by_name("max_timestamp")
            .ok_or_else(|| anyhow::anyhow!("max_timestamp column not found"))?
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to downcast max_timestamp to TimestampMillisecondArray")
            })?
            .value(0);

        let min_ts_str = DateTime::from_timestamp_millis(min_ts)
            .ok_or_else(|| anyhow::anyhow!("Invalid min timestamp"))?
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();
        let max_ts_str = DateTime::from_timestamp_millis(max_ts)
            .ok_or_else(|| anyhow::anyhow!("Invalid max timestamp"))?
            .format("%Y-%m-%d %H:%M:%S%.3f")
            .to_string();

        let query = format!(
            "SELECT * FROM {} WHERE timestamp >= '{}' AND timestamp < '{}'",
            foreign_table_id, min_ts_str, max_ts_str
        );

        let _pg_result = sqlx::query(&query).fetch_all(pg_conn).await?;

        Ok(())
    }
}
