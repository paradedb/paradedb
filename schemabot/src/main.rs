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

mod migration;
mod schema;
mod sql;

fn main() {
    if let Err(error) = run() {
        eprintln!("SchemaBot: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, before, after] if command == "diff" => {
            let before = sql::read(before)?;
            let after = sql::read(after)?;
            print!("{}", schema::diff(&before, &after)?);
            Ok(())
        }
        [command, expected, fragments @ ..] if command == "check" && !fragments.is_empty() => {
            let expected = std::fs::read_to_string(expected).map_err(|e| e.to_string())?;
            let fragments = fragments.iter().map(|path| {
                std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
            }).collect::<Result<Vec<_>, _>>()?;
            migration::check(&expected, &fragments)
        }
        _ => Err("usage: schemabot diff <base.sql> <head.sql> | schemabot check <diff.sql> <fragment.sql>...".into()),
    }
}
