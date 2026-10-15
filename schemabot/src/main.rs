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

use std::io::Write;

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("SchemaBot: {error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32, String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let manual_path = if args.get(1).is_some_and(|arg| arg == "--manual") {
        if args.len() < 3 {
            return Err("--manual requires a path".into());
        }
        let path = args.remove(2);
        args.remove(1);
        Some(path)
    } else {
        None
    };
    match args.as_slice() {
        [command, before, after] if command == "diff" => {
            let before = sql::read(before)?;
            let after = sql::read(after)?;
            let diff = schema::diff(&before, &after)?;
            if let Some(path) = manual_path {
                std::fs::write(path, &diff.manual_sql).map_err(|error| error.to_string())?;
            }
            let mut stdout = std::io::stdout().lock();
            stdout.write_all(diff.sql.as_bytes()).map_err(|error| error.to_string())?;
            stdout.flush().map_err(|error| error.to_string())?;
            if diff.manual.is_empty() { Ok(0) } else {
                eprintln!("SchemaBot: {}", diff.manual.join("\n"));
                Ok(2)
            }
        }
        [command, expected, fragments @ ..] if command == "check" && !fragments.is_empty() => {
            let expected = std::fs::read_to_string(expected).map_err(|e| e.to_string())?;
            let fragments = fragments.iter().map(|path| {
                std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
            }).collect::<Result<Vec<_>, _>>()?;
            migration::check(&expected, &fragments)?;
            if let Some(path) = manual_path {
                let manual = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
                migration::check_manual(&manual, &fragments)?;
            }
            Ok(0)
        }
        _ => Err("usage: schemabot diff [--manual manual.sql] <base.sql> <head.sql> | schemabot check [--manual manual.sql] <diff.sql> <fragment.sql>...".into()),
    }
}
