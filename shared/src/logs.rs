#![allow(clippy::crate_in_macro_def)]
use pgrx::{extension_sql, IntoDatum};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::INFO;
pub static mut PARADE_LOGS_TABLE_INITIALIZED: bool = false;

// Logs will live in the table created below.
// The schema must already exist when this code is executed.
extension_sql!(
    r#"
    DO $$
    BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_tables
                   WHERE schemaname = 'paradedb' AND tablename = 'logs') THEN
        CREATE SCHEMA IF NOT EXISTS paradedb;
        CREATE TABLE paradedb.logs (
            id SERIAL PRIMARY KEY,
            timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            level TEXT NOT NULL,
            module TEXT NOT NULL,
            file TEXT NOT NULL,
            line INTEGER NOT NULL,
            message TEXT NOT NULL,
            json JSON,
            pid INTEGER NOT NULL,
            backtrace TEXT
        );
        ELSE
            RAISE WARNING 'The table paradedb.logs already exists, skipping.';
        END IF;
    END $$;
    "#
    name = "create_parade_logs_table",
    bootstrap
);

/// A logging macro designed for use within the ParadeDB system. It facilitates logging
/// messages at various levels, optionally including additional JSON data. This macro supports
/// three forms of invocation, allowing for flexibility in log detail.
///
/// # Forms
///
/// 1. Basic Logging: `plog!($msg:expr)`
///    Logs a message using the default log level.
/// 2. Logging with Additional JSON Data: `plog!($msg:expr, $json:expr)`
///    Logs a message along with additional JSON data using the default log level.
/// 3. Logging with Specified Level and JSON Data: `plog!($level:expr, $msg:expr, $json:expr)`
///    Logs a message with a specified log level and additional JSON data.
///    Accepts any type that implements Serialize.
///
/// # Examples
///
/// Basic Logging:
/// ```
/// plog!("Starting the application");
/// ```
///
/// Logging with Additional JSON Data:
/// ```
/// plog!("User login", serde_json::json!({"username": "johndoe", "status": "success"}));
/// plog!("User active sessions", vec!["4b84b15", "a3c65c2"]);
/// ```
///
/// Logging with Specified Level and JSON Data:
/// ```
/// plog!($crate::logs::LogLevel::INFO, "Application started successfully", serde_json::Value::Null);
/// plog!($crate::logs::LogLevel::ERROR, "Failed to connect to database", serde_json::json!({"error_code": 500}));
/// plog!($crate::logs::LogLevel::DEBUG, "Received data packet", serde_json::json!({"packet_id": 123, "size": 1024}));
/// ```
///
/// # Log Levels
///
/// The `LogLevel` is an enumeration defined within the crate's `logs` module. It typically
/// contains levels such as `DEBUG`, `INFO`, `WARN`, `ERROR`, etc. The chosen log level determines
/// the significance of the log message and can also control whether a backtrace is captured.
///
/// # Inner Workings
///
/// The macro captures several pieces of contextual information including the file, line, module,
/// process ID, and optionally a backtrace. It then serializes the provided JSON argument and
/// constructs an SQL statement to insert the log entry into the `paradedb.logs` table. If the
/// `PARADEDB_LOGS` flag is enabled, it executes the SQL statement using the `Spi::run_with_args`
/// function.
///
/// # Error Handling
///
/// If any errors occur during the logging process, such as a failure to serialize JSON data or to
/// insert the log entry into the database, the macro handles them gracefully. It logs any errors
/// related to writing logs to the `paradedb.logs` table using the `info!` macro from the `pgrx`
/// crate.
#[macro_export]
macro_rules! plog {
    ($msg:expr) => {
        plog!($crate::logs::DEFAULT_LOG_LEVEL, $msg, $crate::serde_json::Value::Null)
    };
    ($msg:expr, $json:expr) => {
        plog!($crate::logs::DEFAULT_LOG_LEVEL, $msg, $json)
    };
    ($level:expr, $msg:expr, $json:expr) => {
        if crate::GUCS.telemetry_enabled() {
            use pgrx::*;
            use $crate::logs::*;

            let message: &str = $msg;
            let level: LogLevel = $level;
            let serializable_arg = $json;

            let file = file!();
            let line = line!();
            let module = module_path!();
            let pid = std::process::id();
            let backtrace = match level {
                LogLevel::ERROR | LogLevel::DEBUG => {
                    Some(format!("{:#?}", std::backtrace::Backtrace::force_capture()))
                },
                _ => None
            };

            // Serialize the provided JSON and handle any serialization errors
            let log_json_result = $crate::serde_json::to_string(&serializable_arg);
            let json = match log_json_result {
                Ok(json_str) => LogJson {
                    data: $crate::serde_json::from_str(&json_str).unwrap_or_else(|_| $crate::serde_json::Value::Null),
                    error: None,
                },
                Err(e) => LogJson {
                    data: $crate::serde_json::Value::Null,
                    error: Some(e.to_string()),
                },
            };

            #[cfg(any(not(test)))] // Don't compile this during unit tests.
            Spi::run_with_args(
                "INSERT INTO paradedb.logs (level, module, file, line, message, json, pid, backtrace) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                Some(vec![
                    (PgBuiltInOids::TEXTOID.oid(), level.into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), module.into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), file.into_datum()),
                    (PgBuiltInOids::INT8OID.oid(), line.into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), message.into_datum()),
                    (PgBuiltInOids::JSONOID.oid(), json.into_datum()),
                    (PgBuiltInOids::INT8OID.oid(), pid.into_datum()),
                    (PgBuiltInOids::TEXTOID.oid(), backtrace.into_datum()),
                ])
            ).unwrap_or_else(|e| info!("Error writing logs to paradedb.logs: {e}"));
        }
    };
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LogLevel {
    INFO,
    WARN,
    ERROR,
    DEBUG,
    TRACE,
}

impl IntoDatum for LogLevel {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let self_string = &self.to_string();
        self_string.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::prelude::pg_sys::TEXTOID
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize)]
pub struct LogJson {
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl IntoDatum for LogJson {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let string = serde_json::to_string(&self).expect("failed to serialize Json value");
        string.into_datum()
    }

    fn type_oid() -> pgrx::prelude::pg_sys::Oid {
        pgrx::prelude::pg_sys::TEXTOID
    }
}

impl Display for LogJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json_str) => write!(f, "{}", json_str),
            Err(_) => write!(f, "{{}}"), // Fallback to an empty JSON object
        }
    }
}

/// Tests for `plog!` are written here, but each extension must run the tests
/// on its own, as the shared crate is not itself a Postgres extension.
/// Example test run:
/// ```
/// #[cfg(any(test, feature = "pg_test"))]
/// #[pgrx::pg_schema]
/// mod tests {
///     #[pgrx::pg_test]
///     fn test_parade_logs() {
///         shared::test_plog!("<extension_name>");
///     }
/// }
/// ```
#[macro_export]
macro_rules! test_plog {
    ($extension_name:expr) => {
        use $crate::logs::{LogJson, LogLevel};
        use $crate::pgrx::{JsonString, Spi};
        use $crate::plog;

        let guc_name: &str = &format!("paradedb.{}.logs", $extension_name);
        let guc_setting = &crate::GUCS;
        // Default should be false.
        assert!(
            !guc_setting.telemetry_enabled(),
            "default is not set to false"
        );

        // Setting to on should work.
        Spi::run(&format!("SET {guc_name} = on",)).expect("SPI failed");
        assert!(
            guc_setting.telemetry_enabled(),
            "setting parameter to on didn't work"
        );

        // Setting to default should set to off.
        Spi::run(&format!("SET {guc_name} TO DEFAULT;")).expect("SPI failed");
        assert!(
            !guc_setting.get(),
            "setting parameter to default produced wrong value"
        );

        // We'll log a few things in each of the valid forms of plog!.
        // The expectation here is that the call is skipped entirely,
        // and nothing is inserted into the database.
        plog!("message only");
        plog!("message and data", vec![1, 2, 3]);
        plog!(LogLevel::DEBUG, "message and data and enum", vec![1, 2, 3]);

        let log_table_exists = Spi::get_one(
            r#"
            SELECT EXISTS (
                SELECT FROM information_schema.tables
                WHERE table_schema = 'paradedb'
                AND table_name = 'logs'
            );
            "#,
        );
        assert_eq!(
            log_table_exists,
            Ok(Some(true)),
            "The paradedb.logs table should exist"
        );

        let row_count = Spi::get_one("SELECT count(*) from paradedb.logs");
        assert_eq!(
            row_count,
            Ok(Some(0i64)), // counts must be i64
            "should be no rows before paradedb.logs is set to true"
        );

        // Now we'll set paradedb.<extension_name>.logs to on, and we expect rows to be written.
        Spi::run(&format!("SET {guc_name} = on")).expect("error setting logs parameter to on");

        // Test just message
        plog!("message only");
        let message = Spi::get_one("SELECT message from paradedb.logs where ID = 1");
        assert_eq!(
            message,
            Ok(Some("message only")),
            "incorrect message in message only query"
        );

        // Test message and data
        plog!("message and data", vec![1, 2, 3]);
        let message = Spi::get_one("SELECT message FROM paradedb.logs WHERE ID = 2");
        let json = Spi::get_one("SELECT json FROM paradedb.logs WHERE ID = 2");
        assert_eq!(
            message,
            Ok(Some("message and data")),
            "incorrect message in messsage and data query"
        );
        match json {
            Ok(Some(JsonString(s))) => assert_eq!(
                s, "{\"data\":[1,2,3]}",
                "incorrect message in message and data query"
            ),
            _ => panic!("Unable to retrieve json data from message and data query"),
        }

        // Test level and message and data
        plog!(LogLevel::ERROR, "level and message and data", vec![1, 2, 3]);
        let message = Spi::get_one("SELECT message FROM paradedb.logs WHERE ID = 3");
        let level = Spi::get_one("SELECT level FROM paradedb.logs WHERE ID = 3");
        let json = Spi::get_one("SELECT json FROM paradedb.logs WHERE ID = 3");
        assert_eq!(
            message,
            Ok(Some("level and message and data")),
            "incorrect message in level and message and data query"
        );
        assert_eq!(
            level,
            Ok(Some(format!("{}", LogLevel::ERROR))),
            "incorrect level in level and message and data query"
        );
        match json {
            Ok(Some(JsonString(s))) => assert_eq!(
                s, "{\"data\":[1,2,3]}",
                "incorrect message in level and message and data query"
            ),
            _ => panic!("Unable to retrieve json data from message and data query"),
        }

        // Confirm that only 3 rows were written.
        let row_count = Spi::get_one("SELECT count(*) from paradedb.logs");
        assert_eq!(
            row_count,
            Ok(Some(3i64)), // counts must be i64
            "wrong number of rows written during plog! test"
        );
    };
}
