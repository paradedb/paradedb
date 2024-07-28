// Copyright (c) 2023-2024 Retake, Inc.
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

use pgrx::varlena_to_byte_slice;
use supabase_wrappers::prelude::*;

pub fn deparse_qual(qual: &Qual) -> String {
    if qual.use_or {
        match &qual.value {
            Value::Cell(_) => unreachable!(),
            Value::Array(cells) => {
                let conds: Vec<String> = cells
                    .iter()
                    .map(|cell| format!("{} {} {}", qual.field, qual.operator, format_cell(cell)))
                    .collect();
                conds.join(" or ")
            }
        }
    } else {
        match &qual.value {
            Value::Cell(cell) => match qual.operator.as_str() {
                "is" | "is not" => match cell {
                    Cell::String(cell) if cell == "null" => {
                        format!("{} {} null", qual.field, qual.operator)
                    }
                    _ => format!("{} {} {}", qual.field, qual.operator, format_cell(cell)),
                },
                "~~" => format!("{} like {}", qual.field, format_cell(cell)),
                "!~~" => format!("{} not like {}", qual.field, format_cell(cell)),
                _ => format!("{} {} {}", qual.field, qual.operator, format_cell(cell)),
            },
            Value::Array(_) => unreachable!(),
        }
    }
}

pub fn format_cell(cell: &Cell) -> String {
    match cell {
        Cell::Bytea(bytes) => {
            let byte_u8 = unsafe { varlena_to_byte_slice(*bytes) };
            let hex = byte_u8
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<String>>()
                .join("");
            if hex.is_empty() {
                r#"E''"#.to_string()
            } else {
                format!(r#"E'\\x{}'"#, hex)
            }
        }
        Cell::Uuid(uuid) => {
            format!(r#"'{}'"#, uuid)
        }
        Cell::BoolArray(array) => format_array(array),
        Cell::StringArray(array) => format_array(array),
        Cell::I16Array(array) => format_array(array),
        Cell::I32Array(array) => format_array(array),
        Cell::I64Array(array) => format_array(array),
        Cell::F32Array(array) => format_array(array),
        Cell::F64Array(array) => format_array(array),
        _ => format!("{}", cell),
    }
}

fn format_array<T: std::fmt::Display>(array: &[Option<T>]) -> String {
    let res = array
        .iter()
        .map(|e| match e {
            Some(val) => format!("{}", val),
            None => "null".to_owned(),
        })
        .collect::<Vec<String>>()
        .join(",");
    format!("[{}]", res)
}
