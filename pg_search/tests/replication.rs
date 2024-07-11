#![allow(unused)]
use anyhow::Result;
use cmd_lib::run_cmd;
use rstest::*;
use shared::fixtures::db::Query;
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, PgConnection};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;
use tempfile::TempDir;

// Static variables for initializing port assignment and ensuring one-time setup
static INIT: Once = Once::new();
static LAST_PORT: AtomicUsize = AtomicUsize::new(49152);

// Function to check if a port can be bound (i.e., is available)
fn can_bind(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

// Function to get a free port in the dynamic port range
fn get_free_port() -> u16 {
    let port_upper_bound = 65535;
    let port_lower_bound = 49152;

    INIT.call_once(|| {
        LAST_PORT.store(port_lower_bound, Ordering::SeqCst);
    });

    loop {
        let port = LAST_PORT.fetch_add(1, Ordering::SeqCst);
        if port > port_upper_bound {
            LAST_PORT.store(port_lower_bound, Ordering::SeqCst);
            continue;
        }

        if can_bind(port as u16) {
            return port as u16;
        }
    }
}

// Struct to manage an ephemeral PostgreSQL instance
struct EphemeralPostgres {
    pub tempdir: TempDir,
    pub tempdir_path: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
}

// Implement Drop trait to ensure the PostgreSQL instance is properly stopped
impl Drop for EphemeralPostgres {
    fn drop(&mut self) {
        let path = &self.tempdir_path;
        run_cmd!(pg_ctl -D $path stop &> /dev/null).unwrap();
    }
}

// Implementation of EphemeralPostgres
impl EphemeralPostgres {
    fn new() -> Self {
        let port = get_free_port();
        let tempdir = TempDir::new().expect("Failed to create temp dir");
        let tempdir_path = tempdir.path().to_str().unwrap().to_string();
        let timestamp = chrono::Utc::now().timestamp_millis();
        let logfile = format!("/tmp/ephemeral_postgres_logs/{}.log", timestamp);

        // Initialize PostgreSQL data directory
        run_cmd!(initdb -D $tempdir_path &> /dev/null)
            .expect("Failed to initialize Postgres data directory");

        // Write to postgresql.conf
        let config_content = format!(
            "
            port = {}
            wal_level = logical
            max_replication_slots = 4
            max_wal_senders = 4
            shared_preload_libraries = 'pg_search'
            ",
            port
        );
        let config_path = format!("{}/postgresql.conf", tempdir_path);
        std::fs::write(config_path, config_content).expect("Failed to write to postgresql.conf");

        // Create log directory
        std::fs::create_dir_all(Path::new(&logfile).parent().unwrap())
            .expect("Failed to create log directory");

        // Start PostgreSQL
        run_cmd!(pg_ctl -D $tempdir_path -l $logfile start &> /dev/null)
            .expect("Failed to start Postgres");

        EphemeralPostgres {
            tempdir,
            tempdir_path,
            host: "localhost".to_string(),
            port,
            dbname: "postgres".to_string(),
        }
    }

    // Method to establish a connection to the PostgreSQL instance
    async fn connection(&self) -> Result<PgConnection> {
        Ok(PgConnection::connect(&format!(
            "postgresql://{}:{}/{}",
            self.host, self.port, self.dbname
        ))
        .await?)
    }
}

// Test function to test the ephemeral PostgreSQL setup
#[rstest]
async fn test_ephemeral_postgres() -> Result<()> {
    let source_postgres = EphemeralPostgres::new();
    let target_postgres = EphemeralPostgres::new();

    let mut source_conn = source_postgres.connection().await?;
    let mut target_conn = target_postgres.connection().await?;

    // Create pg_search extension on both source and target databases
    "CREATE EXTENSION pg_search".execute(&mut source_conn);
    "CREATE EXTENSION pg_search".execute(&mut target_conn);

    // Create the mock_items table schema
    let schema = "
        CREATE TABLE mock_items (
          id SERIAL PRIMARY KEY,
          description TEXT,
          rating INTEGER CHECK (rating BETWEEN 1 AND 5),
          category VARCHAR(255),
          in_stock BOOLEAN,
          metadata JSONB,
          created_at TIMESTAMP,
          last_updated_date DATE,
          latest_available_time TIME
        )
    ";
    schema.execute(&mut source_conn);
    schema.execute(&mut target_conn);

    // Create the bm25 index on the description field
    "CALL paradedb.create_bm25(
        table_name => 'mock_items',
        index_name => 'mock_items',
        schema_name => 'public',
        key_field => 'id',
        text_fields => '{description: {}}'
    )"
    .execute(&mut source_conn);
    "CALL paradedb.create_bm25(
        table_name => 'mock_items',
        index_name => 'mock_items',
        schema_name => 'public',
        key_field => 'id',
        text_fields => '{description: {}}'
    )"
    .execute(&mut target_conn);

    // Create publication and subscription for replication
    "CREATE PUBLICATION mock_items_pub FOR TABLE mock_items".execute(&mut source_conn);
    format!(
        "CREATE SUBSCRIPTION mock_items_sub
         CONNECTION 'host={} port={} dbname={}'
         PUBLICATION mock_items_pub;",
        source_postgres.host, source_postgres.port, source_postgres.dbname
    )
    .execute(&mut target_conn);

    // Verify initial state of the search results
    let source_results: Vec<(String,)> =
        "SELECT * FROM mock_items.search('description:shoes')".fetch(&mut source_conn);
    let target_results: Vec<(String,)> =
        "SELECT * FROM mock_items.search('description:shoes')".fetch(&mut target_conn);

    assert_eq!(source_results.len(), 0);
    assert_eq!(target_results.len(), 0);

    // Insert a new item into the source database
    "INSERT INTO mock_items (description, category, in_stock, latest_available_time, last_updated_date, metadata, created_at, rating)
    VALUES ('Red sports shoes', 'Footwear', true, '12:00:00', '2024-07-10', '{}', '2024-07-10 12:00:00', 1)".execute(&mut source_conn);

    // Verify the insert is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items.search('description:shoes')".fetch(&mut source_conn);

    // Wait for the replication to complete
    std::thread::sleep(std::time::Duration::from_secs(1));
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items.search('description:shoes')".fetch(&mut target_conn);

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);

    // Additional insert test
    "INSERT INTO mock_items (description, category, in_stock, latest_available_time, last_updated_date, metadata, created_at, rating)
    VALUES ('Blue running shoes', 'Footwear', true, '14:00:00', '2024-07-10', '{}', '2024-07-10 14:00:00', 2)".execute(&mut source_conn);

    // Verify the additional insert is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items.search('description:\"running shoes\"')"
            .fetch(&mut source_conn);

    // Wait for the replication to complete
    std::thread::sleep(std::time::Duration::from_secs(1));
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items.search('description:\"running shoes\"')"
            .fetch(&mut target_conn);

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);

    // Update test
    "UPDATE mock_items SET rating = 5 WHERE description = 'Red sports shoes'"
        .execute(&mut source_conn);

    // Verify the update is replicated to the target database
    let source_results: Vec<(i32,)> =
        "SELECT rating FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut source_conn);

    std::thread::sleep(std::time::Duration::from_secs(1));
    let target_results: Vec<(i32,)> =
        "SELECT rating FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut target_conn);

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);
    assert_eq!(source_results[0], target_results[0]);

    // Delete test
    "DELETE FROM mock_items WHERE description = 'Red sports shoes'".execute(&mut source_conn);

    // Verify the delete is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut source_conn);

    std::thread::sleep(std::time::Duration::from_secs(1));
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut target_conn);

    assert_eq!(source_results.len(), 0);
    assert_eq!(target_results.len(), 0);

    Ok(())
}
