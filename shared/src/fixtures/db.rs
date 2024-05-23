use super::arrow::schema_to_batch;
use async_std::prelude::Stream;
use async_std::stream::StreamExt;
use async_std::task::block_on;
use bytes::Bytes;
use datafusion::arrow::{datatypes::SchemaRef, record_batch::RecordBatch};
use sqlx::{
    postgres::PgRow,
    testing::{TestArgs, TestContext, TestSupport},
    ConnectOptions, Decode, Executor, FromRow, PgConnection, Postgres, Type,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Db {
    context: TestContext<Postgres>,
}

impl Db {
    pub async fn new() -> Self {
        // Use a timestamp as a unique identifier.
        let path = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_string();

        let args = TestArgs::new(Box::leak(path.into_boxed_str()));
        let context = Postgres::test_context(&args)
            .await
            .unwrap_or_else(|err| panic!("could not create test database: {err:#?}"));

        Self { context }
    }

    pub async fn connection(&self) -> PgConnection {
        self.context
            .connect_opts
            .connect()
            .await
            .unwrap_or_else(|err| panic!("failed to connect to test database: {err:#?}"))
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        let db_name = self.context.db_name.to_string();
        async_std::task::spawn(async move {
            Postgres::cleanup_test(db_name.as_str()).await.unwrap();
        });
    }
}

pub trait Query
where
    Self: AsRef<str> + Sized,
{
    fn execute(self, connection: &mut PgConnection) {
        block_on(async {
            connection.execute(self.as_ref()).await.unwrap();
        })
    }

    fn execute_result(self, connection: &mut PgConnection) -> Result<(), sqlx::Error> {
        block_on(async { connection.execute(self.as_ref()).await })?;
        Ok(())
    }

    fn fetch<T>(self, connection: &mut PgConnection) -> Vec<T>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(self.as_ref())
                .fetch_all(connection)
                .await
                .unwrap()
        })
    }

    fn fetch_dynamic(self, connection: &mut PgConnection) -> Vec<PgRow> {
        block_on(async {
            sqlx::query(self.as_ref())
                .fetch_all(connection)
                .await
                .unwrap()
        })
    }

    /// A convenient helper for processing PgRow results from Postgres into a DataFusion RecordBatch.
    /// It's important to note that the retrieved RecordBatch may not necessarily have the same
    /// column order as your Postgres table, or parquet file in a foreign table.
    /// You shouldn't expect to be able to test two RecordBatches directly for equality.
    /// Instead, just test the column equality for each column, like so:
    ///
    /// assert_eq!(stored_batch.num_columns(), retrieved_batch.num_columns());
    /// for field in stored_batch.schema().fields() {
    ///     assert_eq!(
    ///         stored_batch.column_by_name(field.name()),
    ///         retrieved_batch.column_by_name(field.name())
    ///     )
    /// }
    ///
    fn fetch_recordbatch(self, connection: &mut PgConnection, schema: &SchemaRef) -> RecordBatch {
        block_on(async {
            let rows = sqlx::query(self.as_ref())
                .fetch_all(connection)
                .await
                .unwrap();
            schema_to_batch(schema, &rows).unwrap()
        })
    }

    fn fetch_scalar<T>(self, connection: &mut PgConnection) -> Vec<T>
    where
        T: Type<Postgres> + for<'a> Decode<'a, sqlx::Postgres> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_scalar(self.as_ref())
                .fetch_all(connection)
                .await
                .unwrap()
        })
    }

    fn fetch_one<T>(self, connection: &mut PgConnection) -> T
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(self.as_ref())
                .fetch_one(connection)
                .await
                .unwrap()
        })
    }

    fn fetch_result<T>(self, connection: &mut PgConnection) -> Result<Vec<T>, sqlx::Error>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(self.as_ref())
                .fetch_all(connection)
                .await
        })
    }

    fn fetch_collect<T, B>(self, connection: &mut PgConnection) -> B
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
        B: FromIterator<T>,
    {
        self.fetch(connection).into_iter().collect::<B>()
    }
}

impl Query for String {}
impl Query for &String {}
impl Query for &str {}

pub trait DisplayAsync: Stream<Item = Result<Bytes, sqlx::Error>> + Sized {
    fn to_csv(self) -> String {
        let mut csv_str = String::new();
        let mut stream = Box::pin(self);

        while let Some(chunk) = block_on(stream.as_mut().next()) {
            let chunk = chunk.unwrap();
            csv_str.push_str(&String::from_utf8_lossy(&chunk));
        }

        csv_str
    }
}

impl<T> DisplayAsync for T where T: Stream<Item = Result<Bytes, sqlx::Error>> + Send + Sized {}
