use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

struct JsonVisitor<'a>(&'a mut HashMap<String, serde_json::Value>);

impl<'a> tracing::field::Visit for JsonVisitor<'a> {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.0.insert(
            field.name().to_string(),
            serde_json::json!(value.to_string()),
        );
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(
            field.name().to_string(),
            serde_json::json!(format!("{:?}", value)),
        );
    }
}

struct EreportLogger;

impl<S: Subscriber> Layer<S> for EreportLogger {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Covert the values into JSON.
        // The tracing library requires you to use a visitor
        // pattern to access the values of the fields.
        let mut fields = HashMap::new();
        let mut visitor = JsonVisitor(&mut fields);
        event.record(&mut visitor);

        let fields_string: String = fields
            .iter()
            // The tracing lib handles the message field separately in its default
            // formatter, so we'll do that too.
            .filter(|(key, _)| *key != "message")
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join(", ");

        let metadata = event.metadata();
        let target = metadata.target();
        let name = metadata.name();
        let message = fields
            // We will be displaying the message separately from other fields, so
            // we implement special handling here.
            .get("message")
            // We serialized everything into JSON above, so we have to un-serialize
            // the message field so that it can be displayed un-quoted.
            .and_then(|m| serde_json::from_value::<String>(m.clone()).ok())
            // Ensure there's only one whitespace on each side of the string.
            .map(|m| format!(" {} ", m.trim()))
            // Default to only a single whitespace.
            .unwrap_or_else(|| " ".into());

        let log = format!("{target}: {name}:{message}{fields_string}");

        match *metadata.level() {
            Level::TRACE => pgrx::debug1!("{log}"),
            Level::DEBUG => pgrx::log!("{log}"),
            Level::INFO => pgrx::info!("{log}"),
            Level::WARN => pgrx::warning!("{log}"),
            Level::ERROR => pgrx::error!("{log}"),
        }
    }
}

struct SqliteLogger {
    conn: Mutex<Connection>,
}

impl<S: Subscriber> Layer<S> for SqliteLogger {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Covert the values into JSON.
        // The tracing library requires you to use a visitor
        // pattern to access the values of the fields.
        let mut fields = HashMap::new();
        let mut visitor = JsonVisitor(&mut fields);
        event.record(&mut visitor);

        let metadata = event.metadata();
        let message = fields
            // We will be displaying the message separately from other fields, so
            // we implement special handling here.
            .get("message")
            // We serialized everything into JSON above, so we have to un-serialize
            // the message field so that it can be displayed un-quoted.
            .and_then(|m| serde_json::from_value::<String>(m.clone()).ok())
            // Ensure there's only one whitespace on each side of the string.
            .map(|m| format!(" {} ", m.trim()))
            // Default to only a single whitespace.
            .unwrap_or_else(|| " ".into());

        // Remove the message field, as we've extracted it separately.
        fields.remove("message");

        let target = metadata.target();
        let millistamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|s| s.as_millis() as i64)
            .ok();
        let level = metadata.level().to_string();
        let module = metadata.module_path();
        let file = metadata.file();
        let line = metadata.line();
        let json = serde_json::to_string(&fields).ok();
        let pid = std::process::id();
        let backtrace = String::default();

        let guard = match self.conn.lock() {
            Ok(guard) => guard,
            Err(err) => return pgrx::warning!("error locking db logger connection: {err}"),
        };
        let result = guard.execute(
            "
                INSERT INTO logs
                (millistamp, target, level, module, file, line, message, json, pid, backtrace)
                VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![millistamp, target, level, module, file, line, message, json, pid, backtrace],
        );

        if let Err(err) = result {
            pgrx::warning!("Error writing logs to logs db: {err}");
        }
    }
}

fn sqlite_logger_connection(path: &Path) -> Result<Connection, Box<dyn Error>> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            millistamp INTEGER,
            target TEXT,
            level TEXT,
            module TEXT,
            file TEXT,
            line INTEGER,
            message TEXT,
            json JSON,
            pid INTEGER,
            backtrace TEXT
        )",
        [],
    )?;
    Ok(conn)
}

#[allow(unused)]
pub fn init_sqlite_logger() {
    let path = match std::env::var("PGDATA") {
        Ok(dir) => PathBuf::from(dir).join("paradedb").join("logs.db"),
        Err(err) => {
            pgrx::log!("error reading data path to initialize sqlite logger: {err}");
            return;
        }
    };

    let conn = match sqlite_logger_connection(&path) {
        Ok(conn) => Some(conn),
        Err(err) => {
            pgrx::warning!("error initializing logging db: {err}");
            None
        }
    };
    if let Some(conn) = conn {
        tracing_subscriber::registry()
            .with(SqliteLogger {
                conn: Mutex::new(conn),
            })
            .with(EnvFilter::from_default_env())
            .init();
    }
}

#[allow(unused)]
pub fn init_ereport_logger() {
    tracing_subscriber::registry()
        .with(EreportLogger)
        .with(EnvFilter::from_default_env())
        .init();
}
