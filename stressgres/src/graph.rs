// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// -----------------------------------------------------------------------------
//
// This module adds a "parse-logs" subcommand to Stressgres. It reads the
// line-based logs from headless mode output, parses them into time-series
// records, and generates a single PNG file that includes five sub-charts:
//
//    1) TPS vs Time
//    2) block_count vs Time
//    3) segment_count vs Time
//    4) cpu vs Time
//    5) mem vs Time
//
// We parse any numeric metric encountered (e.g. 'tps=12.34', 'cpu=3.77').
// Lines containing 'ERROR=' are ignored. The final PNG is saved to
// <your-prefix>.png

use crate::cli::GraphArgs;
use crate::metrics::group_by_job;
use crate::{metrics, MetricsLine};
use anyhow::Result;
use full_palette::{BROWN, ORANGE};
use image::imageops::FilterType;
use image::ImageReader;
use plotters::coord::Shift;
use plotters::prelude::full_palette::INDIGO_800;
use plotters::prelude::*;
use plotters::style::full_palette::{
    AMBER, BLUEGREY, LIGHTBLUE, LIME, PINK_300, PURPLE, TEAL_A700,
};

/// Main entry point for the "graph" subcommand.
/// Parses the log file, then generates one PNG with 5 sub-charts side by side.
pub fn run(args: &GraphArgs) -> Result<()> {
    let records = metrics::load_metrics_lines(&args.log_path)?;
    println!(
        "Parsed {} records from {}. Generating combined chart...",
        records.len(),
        args.log_path.display()
    );

    // Write one PNG file with 5 sub-charts side by side
    let out_name = args.output.to_string();
    let metrics = metrics::discover_metric_names(&records);

    // gapfill(&metrics, &mut records);
    generate_graph(&metrics, &records, &out_name)?;

    let mut image = ImageReader::open(&out_name)?;
    image.no_limits();
    let png = image.decode()?;
    let resized = png.resize(3860, 2160, FilterType::CatmullRom);
    resized.save(&out_name)?;

    println!("Done. Wrote chart => {}", out_name);
    Ok(())
}

/// Parse the entire log file line-by-line, ignoring lines with ERROR=.
/// Returns a vector of LogRecord (time-series data).
const BGCOLOR: &RGBAColor = &RGBAColor(0x00, 0x2E, 0x47, 1.0);
const PALETTE: &[RGBColor] = &[
    LIME, LIGHTBLUE, CYAN, PURPLE, ORANGE, PINK_300, AMBER, BLUEGREY, BROWN, INDIGO_800, TEAL_A700,
    GREEN,
];

/// Create one PNG with as many sub-charts are there are individual metrics gathered from the [`LogRecord`]s
fn generate_graph(metrics: &[String], records: &[MetricsLine], output_path: &str) -> Result<()> {
    // The final order of subcharts we want to show (left to right):
    eprintln!("metrics={metrics:?}");

    // Gap size (in pixels) between charts
    let gap = 20;

    // Dimensions for each sub-chart
    let subchart_height = 1024;
    let subchart_width = ((subchart_height * metrics.len()) as f64 * (16.0 / 9.0)).ceil() as usize;

    let total_width = subchart_width;
    let total_height = subchart_height * (metrics.len()) + gap * (metrics.len());

    let root = BitMapBackend::new(output_path, (total_width as u32, total_height as u32))
        .into_drawing_area();
    root.fill(BGCOLOR)?;

    // We'll carve out each sub-chart horizontally with a gap in between
    let mut remainder = root.clone();
    for (i, metric_name) in metrics.iter().enumerate() {
        // 1) Split off the sub-chart area
        let (chart_area, next_remainder) = remainder.split_vertically(subchart_height as i32);
        // 2) If there's still more charts to go, split off a gap
        if i < metrics.len() - 1 {
            let (gap_area, next_next) = next_remainder.split_vertically(gap as i32);
            gap_area.fill(&BLACK.mix(0.1))?;
            remainder = next_next;
        } else {
            remainder = next_remainder; // nothing left
        }

        // Draw that metric on the sub-area
        draw_metric(records, metric_name, chart_area)?;
    }

    Ok(root.present()?)
}

/// Draw a single sub-chart (e.g. "tps") into the provided drawing area.
fn draw_metric(
    metrics_lines: &[MetricsLine],
    metric_name: &str,
    drawing_area: DrawingArea<BitMapBackend, Shift>,
) -> Result<()> {
    // Gather data for that metric across all job names
    let (min_time, max_time, max_val, by_job) = group_by_job(metrics_lines, metric_name);

    drawing_area.fill(BGCOLOR)?;

    // If there's no data for this metric, just print a message
    if by_job.is_empty() {
        let _ = drawing_area.titled(&format!("No data for '{metric_name}'"), ("sans-serif", 20));
        return Ok(());
    }

    // Determine y-axis label
    let y_label = match metric_name {
        "cpu" => "cpu (%)",
        "mem" => "mem (MB)",
        other => other, // e.g. "tps", "block_count", "segment_count"
    };

    // Build a chart with some margin
    let mut chart = ChartBuilder::on(&drawing_area)
        .margin(20)
        .caption(
            pretty(metric_name),
            ("sans-serif", 48, &WHITE).into_text_style(&drawing_area),
        )
        .x_label_area_size(80)
        .y_label_area_size(150)
        .right_y_label_area_size(150)
        .build_cartesian_2d(min_time..max_time, 0.0..(max_val * 1.1))?;

    chart
        .configure_mesh()
        .x_desc("time (sec)")
        .x_label_style(("sans-serif", 40, &WHITE).into_text_style(&drawing_area))
        .y_desc(y_label)
        .y_label_style(("sans-serif", 40, &WHITE).into_text_style(&drawing_area))
        .draw()?;

    // We'll cycle through some colors for different job names
    let mut color_idx = 0;

    // Plot each job's timeseries
    #[allow(clippy::explicit_counter_loop)]
    for ((job_title, server_name), mut series) in by_job {
        let agg = metrics::aggregate_job_series(job_title, server_name, metric_name, &series);

        if !series.iter().all(|(_, val)| val.is_nan()) {
            series.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let color = PALETTE[color_idx % PALETTE.len()].stroke_width(4);

            let style = ShapeStyle {
                color: PALETTE[color_idx % PALETTE.len()].into(),
                filled: true,
                stroke_width: 1,
            };

            let mut line_series = LineSeries::new(series, style);
            line_series = line_series.point_size(2);

            let job_title = format!("{} ({})", agg.job_title, agg.server_name);

            let label = format!("{job_title} ({:.3}/{:.3}/{:.3})", agg.min, agg.avg, agg.max);
            chart
                .draw_series(line_series)?
                .label(label)
                .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 25, y)], color));
        }

        color_idx += 1;
    }

    // draw the legend
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::Coordinate(50, 0))
        .border_style(ShapeStyle {
            color: RGBAColor(0x00, 0x00, 0x00, 0.25),
            filled: true,
            stroke_width: 1,
        })
        .background_style(BGCOLOR)
        .margin(20)
        .label_font(("sans-serif", 38, &WHITE).into_text_style(&drawing_area))
        .draw()?;

    Ok(())
}

fn pretty(s: &str) -> String {
    let has_mb = s.ends_with(":MB");
    let s = s.split(":MB").next().unwrap();

    let mut pretty = s.to_string();
    if pretty.len() == 3 {
        pretty = pretty.to_uppercase();
    }

    {
        let pretty = unsafe { pretty.as_bytes_mut() };
        for i in 0..pretty.len() {
            if i == 0 || pretty[i - 1] == b' ' {
                pretty[i] = pretty[i].to_ascii_uppercase();
            } else if pretty[i] == b'_' {
                pretty[i] = b' ';
            }
        }
    }

    if has_mb {
        pretty += " (in megabytes)";
    }

    pretty
}
