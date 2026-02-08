// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use crate::cli::CsvArgs;
use crate::metrics;
use std::path::PathBuf;

pub fn run(args: &CsvArgs) -> anyhow::Result<()> {
    let metrics_lines = metrics::load_metrics_lines(&args.log_path)?;
    println!(
        "Parsed {} records from {}. Generating aggregated metrics...",
        metrics_lines.len(),
        args.log_path.display()
    );

    let metric_names = metrics::discover_metric_names(&metrics_lines);

    let output = PathBuf::from(&args.output);
    let mut csv_file = std::fs::File::create(&output)?;
    println!("Writing metrics to {}", output.canonicalize()?.display());
    let mut writer = csv::Writer::from_writer(&mut csv_file);
    for metric_name in metric_names {
        let (_, _, _, series_by_job) = metrics::group_by_job(&metrics_lines, &metric_name);

        for ((job_title, server_name), series) in series_by_job {
            let agg = metrics::aggregate_job_series(job_title, server_name, &metric_name, &series);

            writer.serialize(agg)?;
        }
    }

    writer.flush()?;

    Ok(())
}
