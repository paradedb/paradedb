use pgrx::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[allow(dead_code)]
pub const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::INFO;

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

#[macro_export]
macro_rules! plog {
    ($msg:expr) => {
        plog!($msg, serde_json::Value::Null)
    };
    ($msg:expr, $json:expr) => {
        plog!($crate::logs::DEFAULT_LOG_LEVEL, $msg, $json)
    };
    ($level:expr, $msg:expr) => {
        plog!($level, $msg, serde_json::Value::Null)
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
            let log_json_result = serde_json::to_string(&serializable_arg);
            let json = match log_json_result {
                Ok(json_str) => LogJson {
                    data: serde_json::from_str(&json_str).unwrap_or_else(|_| serde_json::Value::Null),
                    error: None,
                },
                Err(e) => LogJson {
                    data: serde_json::Value::Null,
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

// Implement the Display trait for the LogLevel enum
impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Struct to represent the JSON structure in the logs
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
            Err(_) => write!(f, "{}", "{}"), // Fallback to an empty JSON object
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_bool_guc() {
        // Default should be false.
        assert_eq!(
            crate::gucs::PARADEDB_LOGS.get(),
            false,
            "default is not set to false"
        );

        // Setting to on should work.
        Spi::run("SET paradedb.logs = on").expect("SPI failed");
        assert_eq!(
            crate::gucs::PARADEDB_LOGS.get(),
            true,
            "setting parameter to on didn't work"
        );

        // Setting to default should set to off.
        Spi::run("SET paradedb.logs TO DEFAULT;").expect("SPI failed");
        assert_eq!(
            crate::gucs::PARADEDB_LOGS.get(),
            false,
            "setting parameter to default produced wrong value"
        );
    }
}
