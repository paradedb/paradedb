use crate::runner::SuiteRunner;
use crate::MetricsLine;
use postgres::Row;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn get_value(row: &Row, col: &str) -> serde_json::Value {
    let col = col.split(':').next().unwrap();
    if let Ok(v) = row.try_get::<_, Option<i8>>(col) {
        serde_json::Value::from(v)
    } else if let Ok(v) = row.try_get::<_, Option<i16>>(col) {
        serde_json::Value::from(v)
    } else if let Ok(v) = row.try_get::<_, Option<i32>>(col) {
        serde_json::Value::from(v)
    } else if let Ok(v) = row.try_get::<_, Option<i64>>(col) {
        serde_json::Value::from(v)
    } else if let Ok(v) = row.try_get::<_, Option<f32>>(col) {
        serde_json::Value::from(v)
    } else if let Ok(v) = row.try_get::<_, Option<f64>>(col) {
        serde_json::Value::from(v)
    } else if let Ok(v) = row.try_get::<_, Option<Decimal>>(col) {
        serde_json::Value::from(v.and_then(|v| v.to_f64()))
    } else {
        panic!("column `{col}` contains an unrecognized value: {row:?}");
    }
}

pub fn run(
    suite_runner: Arc<SuiteRunner>,
    log_file: Option<PathBuf>,
    log_interval_ms: u64,
    runtime_ms: Option<u128>,
) -> anyhow::Result<()> {
    let mut writer: Box<dyn Write + Send> = match &log_file {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(std::io::stdout()),
    };

    let log_interval = std::time::Duration::from_millis(log_interval_ms);
    let start_time = std::time::Instant::now();
    eprintln!("Running suite...");
    let mut last_progress = 0;
    while suite_runner.alive() {
        let duration = start_time.elapsed();
        for runner in suite_runner.runners().chain(suite_runner.monitor_runners()) {
            if runner.errored() {
                break;
            }

            let stats = runner.runtime_stats();
            let job = runner.job();

            for (conninfo, runtime_stats) in stats {
                let mut columns = serde_json::Map::new();
                for col in &job.log_columns {
                    let Ok(results) = runtime_stats.results.clone() else {
                        break;
                    };

                    if let Some(first_row) = results.first() {
                        let value = get_value(first_row, col);
                        columns.insert(col.to_string(), value);
                    }
                }

                if !runner.is_monitor() {
                    // don't log these stats for monitor jobs
                    // they make the generated graphs super busy for little value
                    if job.log_tps {
                        columns.insert("tps".into(), serde_json::Value::from(runtime_stats.tps()));
                    }
                    columns.insert(
                        "cpu".into(),
                        serde_json::Value::from(runtime_stats.cpu_usage),
                    );
                    columns.insert(
                        "mem".into(),
                        serde_json::Value::from(runtime_stats.mem_usage),
                    );
                }

                let metrics_line = MetricsLine {
                    duration,
                    job_title: job.title(),
                    server_name: conninfo.server().name.clone(),
                    metrics: columns,
                };
                serde_json::to_writer(&mut writer, &metrics_line)?;
                writer.write_all(b"\n")?;
            }
        }

        if let Some(runtime_ms) = runtime_ms {
            let pct_complete =
                (duration.as_millis() as f64 / runtime_ms as f64 * 100.0).floor() as u32;
            if pct_complete > 0 && pct_complete > last_progress {
                eprintln!("{pct_complete}%");
                last_progress = pct_complete;
            }
        }
        std::thread::sleep(log_interval);
        if let Some(runtime_ms) = runtime_ms {
            if duration.as_millis() >= runtime_ms {
                break;
            }
        }
    }
    writer.flush()?;
    drop(writer);

    if suite_runner.errored() {
        for runner in suite_runner.monitor_runners().chain(suite_runner.runners()) {
            for (conninfo, runtime_stats) in runner.runtime_stats() {
                let job = runner.job();

                if let Some(e) = runtime_stats.assert_error {
                    eprintln!(
                        "ASSERTION ERROR: job={}, server={}, {}",
                        job.title(),
                        conninfo.server().name,
                        e
                    );
                } else if let Err(e) = runtime_stats.results {
                    eprintln!(
                        "SQL ERROR: job={}, server={}, {}",
                        job.title(),
                        conninfo.server().name,
                        e
                    );
                }
            }
        }
    }

    suite_runner.terminate();

    let finished = Arc::new(AtomicBool::new(false));
    {
        let finished = finished.clone();
        std::thread::spawn(move || {
            let errors = suite_runner
                .wait_for_finish()
                .expect("wait_for_finish() should not fail");

            let mut reported_errors = 0;
            for error in errors {
                let errstr = error.to_string();
                if errstr.contains("failed to merge: User requested cancel")
                    || errstr.contains("Merge cancelled")
                    || errstr.contains("canceling statement due to conflict with recovery")
                {
                    // hack to detect this error message -- it's harmless and shouldn't cause an unsuccessful exit
                    eprintln!("IGNORING TERMINATION ERROR: {errstr}");
                    continue;
                }

                reported_errors += 1;
                eprintln!("TERMINATION ERROR: {errstr}")
            }

            // only unsuccessfully exit if we reported errors to the user
            if reported_errors > 0 {
                std::process::exit(1);
            }

            finished.store(true, std::sync::atomic::Ordering::Relaxed);
        })
    };

    let terminate_start = std::time::Instant::now();
    while terminate_start.elapsed() < std::time::Duration::from_secs(60) {
        if finished.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if !finished.load(std::sync::atomic::Ordering::Relaxed) {
        eprintln!("suite did not finish after 60 seconds");
        std::process::exit(0);
    }

    Ok(())
}
