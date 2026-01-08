use crate::{metrics, MetricsLine};
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub job_title: String,
    pub server_name: String,
    pub metric_name: String,
    pub min: f64,
    pub avg: f64,
    pub median: f64,
    pub max: f64,
    pub count: u64,
}

pub fn load_metrics_lines(log_path: &std::path::Path) -> anyhow::Result<Vec<MetricsLine>> {
    let records = metrics::parse_log_file(log_path)
        .with_context(|| format!("Failed to parse log file: {}", log_path.display()))?;

    if records.is_empty() {
        return Err(anyhow!(
            "No valid lines found in log file: {}",
            log_path.display()
        ));
    }
    Ok(records)
}

fn parse_log_file(log_path: &std::path::Path) -> anyhow::Result<Vec<MetricsLine>> {
    let file = File::open(log_path)?;
    let reader = BufReader::new(file);

    let mut records = Vec::new();
    for line_res in reader.lines() {
        let metrics_line = serde_json::from_str::<MetricsLine>(&line_res?)?;
        records.push(metrics_line);
    }
    Ok(records)
}

pub fn discover_metric_names(metrics_lines: &[MetricsLine]) -> Vec<String> {
    let mut metrics = metrics_lines
        .iter()
        .flat_map(|line| line.metrics.keys())
        .map(|key| key.to_string())
        .collect::<HashSet<_>>();

    let have_tps = metrics.remove("tps");
    metrics.remove("cpu");
    metrics.remove("mem");

    let mut metrics = metrics.into_iter().collect::<Vec<_>>();
    metrics.sort();
    if have_tps {
        metrics.insert(0, "tps".into());
    }
    metrics.push("cpu".into());
    metrics.push("mem".into());
    metrics
}

pub type TimeValue = f64;
pub type MinTimeValue = f64;
pub type MaxTimeValue = f64;

pub type Metric = f64;
pub type MaxMetric = f64;

pub type JobTitle = String;
pub type ServerName = String;

pub fn group_by_job(
    metrics_lines: &[MetricsLine],
    metric_name: &str,
) -> (
    MinTimeValue,
    MaxTimeValue,
    MaxMetric,
    BTreeMap<(JobTitle, ServerName), Vec<(TimeValue, Metric)>>,
) {
    let mut by_job: BTreeMap<(JobTitle, ServerName), Vec<(TimeValue, Metric)>> = Default::default();
    let mut min_time = f64::MAX;
    let mut max_time = 0.0;
    let mut max_val = 0.0;

    for line in metrics_lines {
        if let Some(raw) = line.metrics.get(metric_name) {
            if raw.is_null() {
                continue;
            }

            let raw = raw.as_f64().unwrap_or_else(|| {
                panic!("`{raw:?}` cannot be represented as a f64 for metric '{metric_name}' from line: {line:?}");
            });

            let val = if metric_name == "mem" || metric_name.ends_with(":MB") {
                // convert things measured in megabytes from bytes to megabytes
                raw / (1024.0 * 1024.0) // convert to MB
            } else {
                raw
            };

            let time_secs = line.duration.as_secs_f64();
            by_job
                .entry((line.job_title.clone(), line.server_name.clone()))
                .or_default()
                .push((time_secs, val));
            if time_secs < min_time {
                min_time = time_secs;
            }
            if time_secs > max_time {
                max_time = time_secs;
            }
            if val > max_val {
                max_val = val;
            }
        }
    }

    (min_time, max_time, max_val, by_job)
}

pub fn aggregate_job_series(
    job_title: String,
    server_name: String,
    metric_name: &str,
    job_series: &Vec<(TimeValue, Metric)>,
) -> AggregatedMetric {
    let (min, avg, median, max, count) = {
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        let mut total = 0.0;
        let mut count = 0;
        for (_, val) in job_series {
            let val = *val;
            if val < min {
                min = val;
            }
            if val > max {
                max = val;
            }
            total += val;
            count += 1;
        }
        let avg = total / count as f64;

        let median = median(job_series.iter().map(|(_, val)| *val));

        (min, avg, median, max, count)
    };

    AggregatedMetric {
        job_title,
        server_name,
        metric_name: metric_name.to_string(),
        min,
        avg,
        median,
        max,
        count,
    }
}

fn median(data: impl Iterator<Item = f64>) -> f64 {
    let mut sorted = data.collect::<Vec<_>>();
    sorted.sort_unstable_by(|a, b| a.total_cmp(b));
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}
