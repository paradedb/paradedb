use pgrx::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[allow(dead_code)]
pub const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::INFO;

// Logs will live in the table created below.
// The schema must already exist when this code is executed.
extension_sql!(
    r#"
    CREATE TABLE IF NOT EXISTS paradedb.logs (
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
    "#
    name = "create_paradedb_logs_table"
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
/// ```no_run
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
        if $crate::gucs::PARADEDB_LOGS.get() {
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
