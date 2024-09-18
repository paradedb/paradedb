use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{Level, Subscriber};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// We only want to initialize the logger once per process. This atomic boolean will be
/// our flag to make sure we are initializing only once.
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
        let mut buffer = self.buffer.lock();
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
        let mut buffer = self.buffer.lock();
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

        let log = format!("{name}:{message}{fields_string}");

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

/// Intialize the tracing subscriber for pgrx/Postgres logging.
/// This function needs to be called in every process for logging to work.
/// It should be called explicitly in background workers, and also in a hook
/// that will automatically intialize it for connection processes.
pub fn init_ereport_logger(crate_name: &str) {
    if INITIALIZED.load(Ordering::SeqCst) {
        return;
    }

    INITIALIZED.store(true, Ordering::SeqCst);

    let default_directive: Directive = format!("{crate_name}=debug")
        .parse()
        .expect("should be able to parse default directive");

    tracing_subscriber::registry()
        .with(EreportLogger::new())
        .with(
            EnvFilter::builder()
                .with_default_directive(default_directive)
                .from_env_lossy(),
        )
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

/// Connection processes don't have an explicit "entry point", so it's difficult
/// to choose a place where the tracing subscriber should be initialized.
/// We're taking an aggressive approach and registering an executor start hook
/// to ensure it's initialized before any query.
///
/// This should have a negligeable cost, as after initialization the only work
/// being performed is checking our atomic boolean flag.
///
/// Background processes will still need to initialize the subscriber explicitly.
pub struct TraceHook;

#[allow(deprecated)]
impl pgrx::PgHooks for TraceHook {
    fn executor_start(
        &mut self,
        query_desc: pgrx::PgBox<pgrx::prelude::pg_sys::QueryDesc>,
        eflags: i32,
        prev_hook: fn(
            query_desc: pgrx::PgBox<pgrx::prelude::pg_sys::QueryDesc>,
            eflags: i32,
        ) -> pgrx::HookResult<()>,
    ) -> pgrx::HookResult<()> {
        init_ereport_logger("pg_search");
        prev_hook(query_desc, eflags)
    }
}
