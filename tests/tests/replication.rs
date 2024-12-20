mod fixtures;

use anyhow::Result;
use cmd_lib::{run_cmd, run_fun};
use dotenvy::dotenv;
use fixtures::db::Query;
use rstest::*;
use sqlx::{Connection, PgConnection};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// Static variables for initializing port assignment and ensuring one-time setup
static INIT: Once = Once::new();
static LAST_PORT: AtomicUsize = AtomicUsize::new(49152);

const RETRIES: u32 = 60;
const RETRY_DELAY: u64 = 1000; // measured in milliseconds

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
    pub tempdir_path: String,
    pub host: String,
    pub port: u16,
    pub dbname: String,
    pub pg_ctl_path: PathBuf,
}

// Implement Drop trait to ensure the PostgreSQL instance is properly stopped
impl Drop for EphemeralPostgres {
    fn drop(&mut self) {
        let path = &self.tempdir_path;
        let pg_ctl_path = &self.pg_ctl_path;
        run_cmd!($pg_ctl_path -D $path stop &> /dev/null).unwrap();
        std::fs::remove_dir_all(self.tempdir_path.clone()).unwrap();
    }
}

// Implementation of EphemeralPostgres
impl EphemeralPostgres {
    fn pg_bin_path() -> PathBuf {
        let pg_config_path = std::env::var("PG_CONFIG").expect(
            "PG_CONFIG variable must be set to enable creating ephemeral Postgres instances",
        );
        if !PathBuf::from(&pg_config_path).exists() {
            panic!("PG_CONFIG variable must a valid path to enable creating ephemeral Postgres instances, received {pg_config_path}");
        }
        match run_fun!($pg_config_path --bindir) {
            Ok(path) => PathBuf::from(path.trim().to_string()),
            Err(err) => panic!("could run pg_config --bindir to get Postgres bin folder: {err}"),
        }
    }

    fn pg_basebackup_path() -> PathBuf {
        Self::pg_bin_path().join("pg_basebackup")
    }

    fn initdb_path() -> PathBuf {
        Self::pg_bin_path().join("initdb")
    }

    fn pg_ctl_path() -> PathBuf {
        Self::pg_bin_path().join("pg_ctl")
    }

    fn new_from_initialized(
        tempdir_path: &Path,
        postgresql_conf: Option<&str>,
        pg_hba_conf: Option<&str>,
    ) -> Self {
        let tempdir_path = tempdir_path.to_str().unwrap().to_string();
        let port = get_free_port();
        let pg_ctl_path = Self::pg_ctl_path();

        // Write to postgresql.conf
        let config_content = match postgresql_conf {
            Some(config) => format!("port = {}\n{}", port, config.trim()),
            None => format!("port = {}", port),
        };
        let config_path = format!("{}/postgresql.conf", tempdir_path);
        std::fs::write(config_path, config_content).expect("Failed to write to postgresql.conf");

        // Write to pg_hba.conf
        if let Some(config_content) = pg_hba_conf {
            let config_path = format!("{}/pg_hba.conf", tempdir_path);

            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(config_path)
                .expect("Failed to open pg_hba.conf");

            writeln!(file, "{}", config_content).expect("Failed to append to pg_hba.conf");
        }

        // Create log directory
        let timestamp = chrono::Utc::now().timestamp_millis();
        let logfile = format!("/tmp/ephemeral_postgres_logs/{}.log", timestamp);
        std::fs::create_dir_all(Path::new(&logfile).parent().unwrap())
            .expect("Failed to create log directory");

        // Start PostgreSQL
        run_cmd!($pg_ctl_path -D $tempdir_path -l $logfile start)
            .expect("Failed to start Postgres");

        Self {
            // TempDir needs to be stored on the struct to avoid being dropped, otherwise the
            // temp folder will be deleted before the test finishes.
            tempdir_path,
            host: "localhost".to_string(),
            port,
            dbname: "postgres".to_string(),
            pg_ctl_path,
        }
    }

    fn new(postgresql_conf: Option<&str>, pg_hba_conf: Option<&str>) -> Self {
        // Make sure .env files are loaded before reading env vars.
        dotenv().ok();

        let init_db_path = Self::initdb_path();
        let tempdir = TempDir::new().expect("Failed to create temp dir");
        let tempdir_path = tempdir.into_path();

        // Initialize PostgreSQL data directory
        run_cmd!($init_db_path -D $tempdir_path &> /dev/null)
            .expect("Failed to initialize Postgres data directory");

        Self::new_from_initialized(tempdir_path.as_path(), postgresql_conf, pg_hba_conf)
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
    let config = "
        wal_level = logical
        max_replication_slots = 4
        max_wal_senders = 4
        shared_preload_libraries = 'pg_search'
    ";

    let source_postgres = EphemeralPostgres::new(Some(config), None);
    let target_postgres = EphemeralPostgres::new(Some(config), None);

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
    "
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');

    "
    .execute(&mut source_conn);
    "
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "
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
        "SELECT * FROM mock_items WHERE mock_items @@@ 'description:shoes' ORDER BY id"
            .fetch(&mut source_conn);
    let target_results: Vec<(String,)> =
        "SELECT * FROM mock_items WHERE mock_items @@@ 'description:shoes' ORDER BY id"
            .fetch(&mut target_conn);

    assert_eq!(source_results.len(), 0);
    assert_eq!(target_results.len(), 0);

    // Insert a new item into the source database
    "INSERT INTO mock_items (description, category, in_stock, latest_available_time, last_updated_date, metadata, created_at, rating)
    VALUES ('Red sports shoes', 'Footwear', true, '12:00:00', '2024-07-10', '{}', '2024-07-10 12:00:00', 1)".execute(&mut source_conn);

    // Verify the insert is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE mock_items @@@ 'description:shoes' ORDER BY id"
            .fetch(&mut source_conn);

    // Wait for the replication to complete
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE mock_items @@@ 'description:shoes' ORDER BY id"
            .fetch_retry(&mut target_conn, RETRIES, RETRY_DELAY, |result| {
                !result.is_empty()
            });

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);

    // Additional insert test
    "INSERT INTO mock_items (description, category, in_stock, latest_available_time, last_updated_date, metadata, created_at, rating)
    VALUES ('Blue running shoes', 'Footwear', true, '14:00:00', '2024-07-10', '{}', '2024-07-10 14:00:00', 2)".execute(&mut source_conn);

    // Verify the additional insert is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE mock_items @@@ 'description:\"running shoes\"' ORDER BY id"
            .fetch(&mut source_conn);

    // Wait for the replication to complete
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE mock_items @@@ 'description:\"running shoes\"' ORDER BY id".fetch_retry(
            &mut target_conn,
            60,
            1000,
            |result| !result.is_empty(),
        );

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);

    // Update test
    "UPDATE mock_items SET rating = 5 WHERE description = 'Red sports shoes'"
        .execute(&mut source_conn);

    // Verify the update is replicated to the target database
    let source_results: Vec<(i32,)> =
        "SELECT rating FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut source_conn);

    let target_results: Vec<(i32,)> =
        "SELECT rating FROM mock_items WHERE description = 'Red sports shoes'".fetch_retry(
            &mut target_conn,
            60,
            1000,
            |result| !result.is_empty(),
        );

    assert_eq!(source_results.len(), 1);
    assert_eq!(target_results.len(), 1);
    assert_eq!(source_results[0], target_results[0]);

    // Delete test
    "DELETE FROM mock_items WHERE description = 'Red sports shoes'".execute(&mut source_conn);

    // Verify the delete is replicated to the target database
    let source_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description = 'Red sports shoes'"
            .fetch(&mut source_conn);

    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE description = 'Red sports shoes'".fetch_retry(
            &mut target_conn,
            60,
            1000,
            |result| !result.is_empty(),
        );

    assert_eq!(source_results.len(), 0);
    assert_eq!(target_results.len(), 0);

    Ok(())
}

#[rstest]
async fn test_ephemeral_postgres_with_pg_basebackup() -> Result<()> {
    let config = "
        wal_level = logical
        max_replication_slots = 4
        max_wal_senders = 4
        shared_preload_libraries = 'pg_search'
    ";

    let source_postgres = EphemeralPostgres::new(Some(config), None);
    let mut source_conn = source_postgres.connection().await?;
    let source_port = source_postgres.port;
    let source_username = "SELECT CURRENT_USER"
        .fetch_one::<(String,)>(&mut source_conn)
        .0;

    "CREATE TABLE text_array_table (
            id SERIAL PRIMARY KEY,
            text_array TEXT[]
        )"
    .execute(&mut source_conn);

    "INSERT INTO text_array_table (text_array) VALUES
        (ARRAY['apple', 'banana', 'cherry']),
        (ARRAY['dog', 'elephant', 'fox']),
        (ARRAY['grape', 'honeydew', 'kiwi']),
        (ARRAY['lion', 'monkey', 'newt']),
        (ARRAY['octopus', 'penguin', 'quail']),
        (ARRAY['rabbit', 'snake', 'tiger']),
        (ARRAY['umbrella', 'vulture', 'wolf']),
        (ARRAY['x-ray', 'yak', 'zebra']),
        (ARRAY['alpha', 'bravo', 'charlie']),
        (ARRAY['delta', 'echo', 'foxtrot'])"
        .execute(&mut source_conn);

    // Create pg_search extension and bm25 index
    "CREATE EXTENSION pg_search".execute(&mut source_conn);

    "
    CREATE INDEX text_array_table_idx ON text_array_table
    USING bm25 (id, text_array)
    WITH (key_field = 'id');
    "
    .execute(&mut source_conn);

    // Verify search results before pg_basebackup
    let source_results: Vec<(i32,)> = sqlx::query_as(
        "SELECT id FROM text_array_table WHERE text_array_table @@@ 'text_array:dog' ORDER BY id",
    )
    .fetch_all(&mut source_conn)
    .await?;
    assert_eq!(source_results.len(), 1);

    let target_tempdir = TempDir::new().expect("Failed to create temp dir");
    let target_tempdir_path = target_tempdir.into_path();

    // Permissions for the --pgdata directory passed to pg_basebackup
    // should be u=rwx (0700) or u=rwx,g=rx (0750)
    std::fs::set_permissions(
        target_tempdir_path.as_path(),
        std::fs::Permissions::from_mode(0o700),
    )
    .expect("couldn't set permissions on target_tempdir path");

    // Run pg_basebackup
    let pg_basebackup = EphemeralPostgres::pg_basebackup_path();
    run_cmd!($pg_basebackup --pgdata $target_tempdir_path --host localhost --port $source_port --username $source_username)
    .expect("Failed to run pg_basebackup");

    let target_postgres =
        EphemeralPostgres::new_from_initialized(target_tempdir_path.as_path(), Some(config), None);
    let mut target_conn = target_postgres.connection().await?;

    // Verify the content in the target database
    let target_results: Vec<(i32,)> = sqlx::query_as(
        "SELECT id FROM text_array_table WHERE text_array_table @@@ 'text_array:dog'",
    )
    .fetch_all(&mut target_conn)
    .await?;

    assert_eq!(source_results.len(), target_results.len());

    // Verify the table content
    let source_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM text_array_table")
        .fetch_one(&mut source_conn)
        .await?;
    let target_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM text_array_table")
        .fetch_one(&mut target_conn)
        .await?;

    assert_eq!(source_count, target_count);

    Ok(())
}

#[rstest] // Segfaults on PG13.
async fn test_replication_with_pg_search_only_on_replica() -> Result<()> {
    let config = "
        wal_level = logical
        max_replication_slots = 4
        max_wal_senders = 4
        shared_preload_libraries = 'pg_search'
    ";

    let source_postgres = EphemeralPostgres::new(Some(config), None);
    let target_postgres = EphemeralPostgres::new(Some(config), None);

    let mut source_conn = source_postgres.connection().await?;
    let mut target_conn = target_postgres.connection().await?;

    // Do not install pg_search on the source database

    // Create the mock_items table schema on the source
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

    // Create publication for replication
    "CREATE PUBLICATION mock_items_pub FOR TABLE mock_items".execute(&mut source_conn);

    // Install pg_search on the replica and create the same table schema
    "CREATE EXTENSION pg_search".execute(&mut target_conn);
    schema.execute(&mut target_conn);

    // Create the bm25 index on the description field on the replica
    "
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "
    .execute(&mut target_conn);

    // Create subscription on the replica
    format!(
        "CREATE SUBSCRIPTION mock_items_sub
         CONNECTION 'host={} port={} dbname={}'
         PUBLICATION mock_items_pub;",
        source_postgres.host, source_postgres.port, source_postgres.dbname
    )
    .execute(&mut target_conn);

    // Insert a new item into the source database
    "INSERT INTO mock_items (description, category, in_stock, latest_available_time, last_updated_date, metadata, created_at, rating)
    VALUES ('Green hiking shoes', 'Footwear', true, '16:00:00', '2024-07-11', '{}', '2024-07-11 16:00:00', 3)"
    .execute(&mut source_conn);

    // Verify the insert is replicated to the target database and can be searched using pg_search
    let target_results: Vec<(String,)> =
        "SELECT description FROM mock_items WHERE mock_items @@@ 'description:shoes' ORDER BY id"
            .fetch_retry(&mut target_conn, RETRIES, RETRY_DELAY, |result| {
                !result.is_empty()
            });

    assert_eq!(target_results.len(), 1);
    assert_eq!(target_results[0].0, "Green hiking shoes");

    Ok(())
}

#[rstest]
async fn test_wal_streaming_replication() -> Result<()> {
    // Primary Postgres setup + insert data
    let postgresql_conf = "
        listen_addresses = 'localhost'
        wal_level = replica
        max_wal_senders = 4
        shared_preload_libraries = 'pg_search'
    ";
    let pg_hba_conf = "
        host replication all 127.0.0.1/32 md5
        host replication all ::1/128 md5
    ";
    let source_postgres = EphemeralPostgres::new(Some(postgresql_conf), Some(pg_hba_conf));
    let mut source_conn = source_postgres.connection().await?;
    let source_port = source_postgres.port;
    let source_username = "replicator";

    "CREATE USER replicator WITH REPLICATION ENCRYPTED PASSWORD 'replicator_pass'"
        .execute(&mut source_conn);

    // Standby Postgres setup
    let postgresql_conf = "
        shared_preload_libraries = 'pg_search'
    ";
    let target_tempdir = TempDir::new().expect("Failed to create temp dir");
    let target_tempdir_path = target_tempdir.into_path();

    // Permissions for the --pgdata directory passed to pg_basebackup
    // should be u=rwx (0700) or u=rwx,g=rx (0750)
    std::fs::set_permissions(
        target_tempdir_path.as_path(),
        std::fs::Permissions::from_mode(0o700),
    )
    .expect("couldn't set permissions on target_tempdir path");

    // Run pg_basebackup
    let pg_basebackup = EphemeralPostgres::pg_basebackup_path();
    run_cmd!($pg_basebackup -D $target_tempdir_path -Fp -Xs -P -R -h localhost -U $source_username --port $source_port)
        .expect("Failed to run pg_basebackup");

    let target_postgres = EphemeralPostgres::new_from_initialized(
        target_tempdir_path.as_path(),
        Some(postgresql_conf),
        None,
    );

    // Create the mock_items table schema on the source
    let schema = "
        CREATE EXTENSION pg_search;
        CALL paradedb.create_bm25_test_table(
            schema_name => 'public',
            table_name => 'mock_items'
        )
    ";
    schema.execute(&mut source_conn);

    thread::sleep(Duration::from_millis(1000));

    let mut target_conn = target_postgres.connection().await?;
    let target_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM mock_items")
        .fetch_one(&mut target_conn)
        .await?;

    assert_eq!(target_count, (41,));

    "
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (key_field = 'id');
    "
    .execute(&mut source_conn);

    thread::sleep(Duration::from_millis(1000));

    let rows: Vec<(i32,)> =
        "SELECT id FROM mock_items WHERE mock_items @@@ 'description:shoes' ORDER BY id"
            .fetch(&mut target_conn);

    assert_eq!(rows.len(), 3);

    Ok(())
}
