use anyhow::Result;
use async_std::task::block_on;
use aws_config::{BehaviorVersion, Region};

use datafusion::{
    arrow::{datatypes::FieldRef, record_batch::RecordBatch},
    parquet::arrow::ArrowWriter,
};
use rstest::*;
use serde::Serialize;
use serde_arrow::schema::{SchemaLike, TracingOptions};
use sqlx::PgConnection;
use testcontainers::ContainerAsync;
use testcontainers_modules::{
    localstack::LocalStack,
    testcontainers::{runners::AsyncRunner, RunnableImage},
};

pub use shared::fixtures::db::*;
#[allow(unused_imports)]
pub use shared::fixtures::tables::*;
#[allow(unused_imports)]
pub use shared::fixtures::utils::*;

#[fixture]
pub fn database() -> Db {
    block_on(async { Db::new().await })
}

#[fixture]
pub fn conn(database: Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;
        sqlx::query("CREATE EXTENSION pg_lakehouse;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_lakehouse");
        conn
    })
}

#[fixture]
pub fn conn_with_pg_search(database: Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;
        sqlx::query("CREATE EXTENSION pg_lakehouse;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_lakehouse");
        conn
    })
}

/// A wrapper type to own both the testcontainers container for localstack
/// and the S3 client. It's important that they be owned together, because
/// testcontainers will stop the Docker container is stopped once the variable
/// is dropped.
pub struct S3 {
    #[allow(unused)]
    container: ContainerAsync<LocalStack>,
    pub client: aws_sdk_s3::Client,
    pub url: String,
}

impl S3 {
    async fn new() -> Self {
        let image: RunnableImage<LocalStack> =
            RunnableImage::from(LocalStack).with_env_var(("SERVICES", "s3"));
        let container = image.start().await;

        let host_ip = container.get_host().await;
        let host_port = container.get_host_port_ipv4(4566).await;
        let url = format!("http://{host_ip}:{host_port}");
        let creds = aws_sdk_s3::config::Credentials::new("fake", "fake", None, None, "test");

        let config = aws_sdk_s3::config::Builder::default()
            .behavior_version(BehaviorVersion::v2024_03_28())
            .region(Region::new("us-east-1"))
            .credentials_provider(creds)
            .endpoint_url(url.clone())
            .force_path_style(true)
            .build();

        let client = aws_sdk_s3::Client::from_conf(config);
        Self {
            container,
            client,
            url,
        }
    }

    pub async fn create_bucket(&self, bucket: &str) -> Result<()> {
        self.client.create_bucket().bucket(bucket).send().await?;
        Ok(())
    }

    pub async fn put_batch(&self, bucket: &str, key: &str, batch: &RecordBatch) -> Result<()> {
        let mut buf = vec![];
        let mut writer = ArrowWriter::try_new(&mut buf, batch.schema(), None)?;
        writer.write(&batch)?;
        writer.close()?;

        self.client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(buf.into())
            .send()
            .await?;
        Ok(())
    }

    pub async fn put_rows<T: Serialize>(&self, bucket: &str, key: &str, rows: &[T]) -> Result<()> {
        let fields = Vec::<FieldRef>::from_type::<NycTripsTable>(TracingOptions::default())?;
        let batch = serde_arrow::to_record_batch(&fields, &rows)?;

        self.put_batch(bucket, key, &batch).await
    }
}

#[fixture]
pub async fn s3() -> S3 {
    S3::new().await
}
