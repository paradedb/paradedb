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
use anyhow::{Context, Result};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::Object;
use bytes::Bytes;
use datafusion::{
    arrow::record_batch::RecordBatch,
    parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder, parquet::arrow::ArrowWriter,
    parquet::file::properties::WriterProperties,
};
use futures::future::BoxFuture;
use std::io::Cursor;
use std::sync::Arc;
use std::{
    fs::{self},
    path::Path,
};
use testcontainers::runners::AsyncRunner;
use testcontainers::ContainerAsync;
use testcontainers_modules::{localstack::LocalStack, testcontainers::ImageExt};
use tokio::runtime::Runtime;

/// A wrapper type to own both the testcontainers container for localstack
/// and the S3 client. It's important that they be owned together, because
/// testcontainers will stop the Docker container is stopped once the variable
/// is dropped.
#[allow(unused)]
pub struct LocalStackS3Client {
    container: Arc<ContainerAsync<LocalStack>>,
    pub client: aws_sdk_s3::Client,
    pub url: String,
    runtime: Runtime,
}

impl LocalStackS3Client {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        let (container, client, url) = runtime.block_on(async {
            let request = LocalStack::default().with_env_var("SERVICES", "s3");
            let container = request
                .start()
                .await
                .context("Failed to start the container")?;

            let host_ip = container.get_host().await.expect("failed to get Host IP");
            let host_port = container
                .get_host_port_ipv4(4566)
                .await
                .context("Failed to get Host Port")?;
            let url = format!("{host_ip}:{host_port}");
            let creds = aws_sdk_s3::config::Credentials::new("fake", "fake", None, None, "test");

            let config = aws_sdk_s3::config::Builder::default()
                .behavior_version(BehaviorVersion::v2024_03_28())
                .region(Region::new("us-east-1"))
                .credentials_provider(creds)
                .endpoint_url(format!("http://{}", url.clone()))
                .force_path_style(true)
                .build();

            let client = aws_sdk_s3::Client::from_conf(config);
            Ok::<_, anyhow::Error>((container, client, url))
        })?;

        Ok(Self {
            container: Arc::new(container),
            client,
            url,
            runtime,
        })
    }

    #[allow(unused)]
    pub fn create_bucket(&self, bucket: &str) -> Result<()> {
        self.runtime.block_on(async {
            let _ = self
                .client
                .create_bucket()
                .bucket(bucket)
                .send()
                .await
                .context("Failed to create bucket");
            Ok(())
        })
    }

    #[allow(unused)]
    pub fn put_batch(&self, bucket: &str, key: &str, batch: &RecordBatch) -> Result<()> {
        self.runtime.block_on(async {
            // Create a cursor to write the batch data
            let mut cursor = Cursor::new(Vec::new());

            // Create ArrowWriter with default properties
            let props = WriterProperties::builder().build();
            let mut writer = ArrowWriter::try_new(&mut cursor, batch.schema(), Some(props))
                .context("Failed to create ArrowWriter")?;

            // Write the batch
            writer.write(batch).context("Failed to write batch")?;

            // Finish writing and flush the data
            writer.close().context("Failed to close ArrowWriter")?;

            // Get the written data
            let data = cursor.into_inner();

            // Upload the data to S3
            let _ = self
                .client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(ByteStream::from(data))
                .send()
                .await
                .context("Failed to put batch");
            Ok(())
        })
    }

    #[allow(unused)]
    pub fn get_batch(&self, bucket: &str, key: &str) -> Result<RecordBatch> {
        self.runtime.block_on(async {
            // Retrieve the object from S3
            let get_object_output = self
                .client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .context("Failed to get object from S3")?;

            // Read the body of the object
            let body = get_object_output.body.collect().await?;
            let bytes: Bytes = body.into_bytes();

            // Create a Parquet reader
            let builder = ParquetRecordBatchReaderBuilder::try_new(bytes)
                .context("Failed to create Parquet reader builder")?;

            // Create the reader
            let mut reader = builder.build().context("Failed to build Parquet reader")?;

            // Read the first batch
            reader
                .next()
                .context("No batches found in Parquet file")?
                .context("Failed to read batch")
        })
    }

    // #[allow(dead_code)]
    pub fn put_directory(&self, bucket: &str, s3_prefix: &str, local_dir: &Path) -> Result<()> {
        self.runtime.block_on(async {
            self.put_directory_recursive(bucket, s3_prefix, local_dir)
                .await
        })
    }

    fn put_directory_recursive<'a>(
        &'a self,
        bucket: &'a str,
        s3_prefix: &'a str,
        local_dir: &'a Path,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            let base_path = local_dir.to_path_buf();
            let entries = fs::read_dir(local_dir).context("Failed to read directory")?;

            for entry in entries {
                let entry = entry.context("Failed to read directory entry")?;
                let path = entry.path();
                let relative_path = path.strip_prefix(&base_path)?;
                let s3_key = if s3_prefix.is_empty() {
                    relative_path.to_string_lossy().to_string()
                } else {
                    format!("{}/{}", s3_prefix, relative_path.to_string_lossy())
                };

                if path.is_file() {
                    tracing::info!("Uploading file: local_path={:?}, s3_key={}", path, s3_key);

                    let content = fs::read(&path).context("Failed to read file")?;
                    self.client
                        .put_object()
                        .bucket(bucket)
                        .key(&s3_key)
                        .body(ByteStream::from(content))
                        .send()
                        .await
                        .context("Failed to upload file to S3")?;

                    tracing::info!("Uploaded file: s3_key={}", s3_key);
                } else if path.is_dir() {
                    self.put_directory_recursive(bucket, &s3_key, &path).await?;
                }
            }

            Ok(())
        })
    }

    pub fn print_object_list(&self, bucket: &str, prefix: &str) -> Result<()> {
        self.runtime.block_on(async {
            let objects = self
                .list_objects(bucket, prefix)
                .await
                .context("Failed to list bucket objects")?;

            tracing::info!("Objects in s3://{}/{}", bucket, prefix);
            for object in objects {
                tracing::info!(
                    "  {} ({})",
                    object.key().unwrap_or(""),
                    object.size().unwrap_or(0)
                );
            }

            Ok(())
        })
    }

    async fn list_objects(&self, bucket: &str, prefix: &str) -> Result<Vec<Object>> {
        let mut objects = Vec::new();

        let mut paginator = self
            .client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(prefix)
            .into_paginator()
            .send();

        while let Some(page) = paginator.next().await {
            match page {
                Ok(output) => {
                    if let Some(contents) = output.contents {
                        objects.extend(contents);
                    }
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Failed to list objects: {:?}", err));
                }
            }
        }

        Ok(objects)
    }
}

impl Drop for LocalStackS3Client {
    fn drop(&mut self) {
        tracing::info!("S3 resource drop initiated");

        // Clone the container to avoid borrowing issues
        let container = Arc::clone(&self.container);

        // Create a future that doesn't capture `self`
        let future = async move {
            if let Err(e) = container.stop().await {
                tracing::error!("Failed to stop container: {:?}", e);
            } else {
                tracing::info!("Container stopped successfully");
            }
        };

        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                tracing::debug!("Existing Tokio runtime found, executing in current context");
                tokio::task::block_in_place(move || {
                    handle.block_on(future);
                });
            }
            Err(e) => {
                tracing::warn!("No Tokio runtime found, spawning new thread: {:?}", e);
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
                    rt.block_on(future);
                });
            }
        }

        tracing::info!("S3 resource drop completed");
    }
}
