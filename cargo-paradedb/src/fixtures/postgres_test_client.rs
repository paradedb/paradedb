#![allow(dead_code)]

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

// TECH DEBT: This file is a copy of the `db.rs` file from https://github.com/paradedb/paradedb/blob/dev/shared/src/fixtures/db.rs
// We duplicated because the paradedb repo may use a different version of pgrx than pg_analytics, but eventually we should
// move this into a separate crate without any dependencies on pgrx.
use anyhow::{Context, Result};
use async_lock::{Semaphore, SemaphoreGuardArc};
use rand::Rng;
use sqlx::{
    postgres::PgConnectOptions, postgres::PgPoolOptions, ConnectOptions, Connection, PgConnection,
    PgPool,
};
// use sqlx::Connection;
use futures::future::BoxFuture;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use url::Url;

pub enum ConnectionType {
    Parallel,
    Exclusive,
}

pub struct ActivePoolConnGuard {
    pub inner_conn: PgConnection,
    guard: SemaphoreGuardArc,
}

impl ActivePoolConnGuard {
    pub fn new(pg_conn: PgConnection, guard: SemaphoreGuardArc) -> Self {
        Self {
            inner_conn: pg_conn,
            guard,
        }
    }

    pub async fn release(self) {
        let _ = self.inner_conn.close().await;
    }
}

pub struct PostgresTestClient {
    admin_pool: PgPool,
    test_pool: PgPool,
    test_db_name: String,
    parallel_connection_semaphore: Arc<Semaphore>,
    exclusive_connection: Arc<Mutex<PgConnection>>,
}

impl PostgresTestClient {
    pub async fn new(database_url: &str, max_connections: usize) -> Result<Self> {
        let admin_pool = PgPoolOptions::new()
            .max_connections(2) // Limit connections for admin tasks
            .connect(database_url)
            .await
            .context("Failed to create admin connection pool")?;

        let test_db_name = format!("test_db_{}", rand::thread_rng().gen::<u32>());
        let create_db_query = format!("CREATE DATABASE {}", test_db_name);

        sqlx::query(&create_db_query)
            .execute(&admin_pool)
            .await
            .context("Failed to create test database")?;

        let mut test_url = Url::parse(database_url).context("Failed to parse database URL")?;
        test_url.set_path(&test_db_name);

        let test_pool = PgPoolOptions::new()
            .max_connections(max_connections as u32) // Adjust based on your needs
            .connect(test_url.as_str())
            .await
            .context("Failed to create test database connection pool")?;

        tracing::info!(
            "Created test database: {} with connection pool",
            test_db_name
        );

        let connection_semaphore = Arc::new(Semaphore::new(max_connections));

        let conn_opts = &PgConnectOptions::from_str(test_url.as_str()).unwrap();

        let exclusive_connection = Arc::new(Mutex::new(
            PgConnection::connect_with(conn_opts).await.unwrap(),
        ));

        Ok(Self {
            admin_pool,
            test_pool,
            test_db_name,
            parallel_connection_semaphore: connection_semaphore,
            exclusive_connection,
        })
    }

    pub async fn acquire_connection(&self) -> Result<ActivePoolConnGuard> {
        let guard = self.parallel_connection_semaphore.acquire_arc().await;

        let connect_options = self.test_pool.connect_options().clone();

        match connect_options.connect().await {
            Ok(conn) => Ok(ActivePoolConnGuard::new(conn, guard)),

            Err(e) => Err(e).context("Failed to acquire a connection from the test database pool"),
        }
    }

    #[allow(clippy::await_holding_lock)]
    pub async fn execute_with_connection<F, R>(
        &self,
        connection_type: ConnectionType,
        f: F,
    ) -> Result<R>
    where
        F: for<'a> FnOnce(&'a mut PgConnection) -> BoxFuture<'a, Result<R>>,
    {
        match connection_type {
            ConnectionType::Parallel => {
                let mut conn_guard = self.acquire_connection().await?;
                let rval = f(&mut conn_guard.inner_conn)
                    .await
                    .context("Error executing function with parallel connection")?;
                conn_guard.release().await;
                Ok(rval)
            }
            ConnectionType::Exclusive => {
                let mut conn = self
                    .exclusive_connection
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire database lock: {}", e))?;
                f(&mut conn)
                    .await
                    .context("Error executing function with exclusive connection")
            }
        }
    }

    async fn execute_admin_query(&self, query: &str) -> Result<()> {
        sqlx::query(query)
            .execute(&self.admin_pool)
            .await
            .context("Failed to execute admin query")?;
        Ok(())
    }

    async fn terminate_connections(&self) -> Result<()> {
        let terminate_query = format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            self.test_db_name
        );
        self.execute_admin_query(&terminate_query).await?;
        tracing::info!(
            "Terminated connections to test database: {}",
            self.test_db_name
        );
        Ok(())
    }

    async fn drop_database(&self) -> Result<()> {
        let drop_db_query = format!("DROP DATABASE IF EXISTS {}", self.test_db_name);
        self.execute_admin_query(&drop_db_query).await?;
        tracing::info!("Dropped test database: {}", self.test_db_name);
        Ok(())
    }
}

impl Drop for PostgresTestClient {
    fn drop(&mut self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

        runtime.block_on(async {
            // Close all connections in the test pool
            self.test_pool.close().await;

            if let Err(e) = self.terminate_connections().await {
                tracing::error!("Failed to terminate connections: {}", e);
            }

            // Wait a moment to ensure connections are fully closed
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            if let Err(e) = self.drop_database().await {
                tracing::error!("Failed to drop test database: {}", e);
            }

            // Close the admin pool
            self.admin_pool.close().await;
        });
    }
}
