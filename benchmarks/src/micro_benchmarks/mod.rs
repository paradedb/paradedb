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

mod benchmark_data_setup;
mod fast_field_benchmarks;

pub use benchmark_data_setup::{create_bm25_index, setup_benchmark_database};
pub use fast_field_benchmarks::{
    benchmark_mixed_fast_fields, check_execution_plan_metrics, collect_json_field_values,
    detect_exec_method, display_results, run_benchmark, run_benchmarks_with_methods,
    set_execution_method, BenchmarkConfig, BenchmarkResult,
};
