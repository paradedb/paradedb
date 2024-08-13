use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{Level, Subscriber};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

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

struct EreportLogger {
    buffer: Arc<Mutex<VecDeque<(Level, String)>>>,
}

impl EreportLogger {
    fn new() -> Self {
        EreportLogger {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn flush_logs(&self) {
        let mut buffer = self.buffer.lock().unwrap();
        while let Some((level, log)) = buffer.pop_front() {
            match level {
                Level::TRACE => pgrx::debug1!("{log}"),
                Level::DEBUG => pgrx::log!("{log}"),
                Level::INFO => pgrx::info!("{log}"),
                Level::WARN => pgrx::warning!("{log}"),
                Level::ERROR => pgrx::error!("{log}"),
            }
        }
    }

    fn buffer_log(&self, level: Level, log: String) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.push_back((level, log));
    }
}

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

        // It's important to remember that, based on the tracing filter, we could be
        // processing logging calls from our dependencies here... which may be running
        // code in a non-main thread. Because this Layer is calling pgrx::* functions...
        // we cannot run any non-main thread code.
        //
        // Because of this, we build a buffer of log items, and we'll only "flush" (call
        // pgrx functions) if we are sure we are in the main thread.

        self.buffer_log(*metadata.level(), log);

        if is_os_main_thread().unwrap_or(false) {
            self.flush_logs();
        }
    }
}

pub fn init_ereport_logger() {
    if INITIALIZED.load(Ordering::SeqCst) {
        return;
    }

    INITIALIZED.store(true, Ordering::SeqCst);

    tracing_subscriber::registry()
        .with(EreportLogger::new())
        .with(EnvFilter::from_default_env())
        .init();
}

// Used in PGRX to detect non-main-thread FFI use.
// Returns None if "unsure" about main thread.
pub fn is_os_main_thread() -> Option<bool> {
    #[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd"))]
    return unsafe {
        match libc::pthread_main_np() {
            1 => Some(true),
            0 => Some(false),
            // Note that this returns `-1` in some error conditions.
            //
            // In these cases we are almost certainly not the main thread, but
            // we don't know -- it's better for this function to return `None`
            // in cases of uncertainty.
            _ => None,
        }
    };
    #[cfg(target_os = "linux")]
    return unsafe {
        // Use the raw syscall, which is available in all versions of linux that Rust supports.
        let tid = libc::syscall(libc::SYS_gettid) as core::ffi::c_long;
        let pid = libc::getpid() as core::ffi::c_long;
        Some(tid == pid)
    };
    #[allow(unreachable_code)]
    {
        None
    }
}
