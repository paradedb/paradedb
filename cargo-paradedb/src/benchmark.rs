use anyhow::Result;
use async_std::sync::Mutex;
use async_std::task::block_on;
use criterion::async_executor::AsyncStdExecutor;
use criterion::Criterion;
use sqlx::Executor;
use sqlx::{postgres::PgConnectOptions, Connection, PgConnection};
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::debug;

pub struct Benchmark {
    pub group_name: String,
    pub function_name: String,
    pub setup_query: Option<String>,
    pub query: String,
    pub database_url: String,
}

impl Benchmark {
    pub async fn setup_query(&self, conn: &mut PgConnection) -> Result<()> {
        if let Some(query) = &self.setup_query {
            conn.execute(query.as_ref()).await?;
        }

        Ok(())
    }
    pub async fn run(&self) -> Result<()> {
        // One-time setup code goes here.
        debug!(DATABASE_URL = self.database_url);
        let mut criterion = Criterion::default();
        let mut group = criterion.benchmark_group(&self.group_name);

        // Lowered from default sample size to remove Criterion warning.
        // Must be higher than 10, or Criterion will panic.
        group.sample_size(60);
        group.bench_function(&self.function_name, |runner| {
            // Per-sample (note that a sample can be many iterations) setup goes here.
            let conn_opts = &PgConnectOptions::from_str(&self.database_url).unwrap();
            let conn = block_on(async {
                Arc::new(Mutex::new(
                    PgConnection::connect_with(conn_opts).await.unwrap(),
                ))
            });

            // Run setup query.
            block_on(async {
                let local_conn = conn.clone();
                let mut conn = local_conn.lock().await; // Acquire the lock asynchronously.
                self.setup_query(&mut conn).await.unwrap();
            });

            runner.to_async(AsyncStdExecutor).iter(|| {
                // Measured code goes here.
                async {
                    let local_conn = conn.clone();
                    let mut conn = local_conn.lock().await; // Acquire the lock asynchronously.
                    sqlx::query(&self.query).execute(&mut *conn).await.unwrap();
                }
            });
        });

        group.finish();

        Ok(())
    }

    pub async fn run_once(&self) -> Result<()> {
        let conn_opts = &PgConnectOptions::from_str(&self.database_url).unwrap();
        let mut conn = PgConnection::connect_with(conn_opts).await.unwrap();

        // Run setup query if present.
        self.setup_query(conn.as_mut()).await.unwrap();

        // Run actual query to be benchmarked.
        let start_time = SystemTime::now();
        block_on(async {
            sqlx::query(&self.query).execute(&mut conn).await.unwrap();
        });
        let end_time = SystemTime::now();

        Self::print_results(start_time, end_time);

        Ok(())
    }

    pub fn print_results(start_time: SystemTime, end_time: SystemTime) {
        if let Ok(duration) = end_time.duration_since(start_time) {
            println!("Start time: {:?}", start_time);
            println!("End time: {:?}", end_time);

            let milliseconds = duration.as_millis();
            let seconds = duration.as_secs_f64(); // Use floating point for seconds
            let minutes = seconds / 60.0; // Convert seconds to minutes
            let hours = seconds / 3600.0; // Convert seconds to hours

            println!("Duration: {} milliseconds", milliseconds);
            println!("Duration: {:.4} seconds", seconds); // Print with 4 decimal places
            println!("Duration: {:.4} minutes", minutes); // Print with 4 decimal places
            println!("Duration: {:.4} hours", hours); // Print with 4 decimal places
        } else {
            println!("An error occurred while calculating the duration.");
        }
    }
}
