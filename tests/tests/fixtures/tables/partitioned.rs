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

use chrono::{NaiveDate, NaiveDateTime};
use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct PartitionedTable {
    pub id: i32,
    pub sale_date: NaiveDateTime,
    pub amount: f32,
    pub description: String,
}

impl PartitionedTable {
    pub fn setup() -> String {
        Self::setup_with_indexed_columns(["description", "sale_date", "amount"])
    }

    pub fn setup_with_indexed_columns<'a>(
        indexed_columns: impl IntoIterator<Item = &'a str>,
    ) -> String {
        let formatted_indexed_columns = indexed_columns.into_iter().collect::<Vec<_>>().join(", ");
        format!(
            r#"
            BEGIN;
                CREATE TABLE sales (
                    id SERIAL,
                    sale_date DATE NOT NULL,
                    amount REAL NOT NULL,
                    description TEXT,
                    PRIMARY KEY (id, sale_date)
                ) PARTITION BY RANGE (sale_date);

                CREATE TABLE sales_2023_q1 PARTITION OF sales
                FOR VALUES FROM ('2023-01-01') TO ('2023-03-31');

                CREATE TABLE sales_2023_q2 PARTITION OF sales
                FOR VALUES FROM ('2023-04-01') TO ('2023-06-30');

                CREATE INDEX sales_index ON sales
                USING bm25 (id, {formatted_indexed_columns})
                WITH (key_field='id', numeric_fields='{{"amount": {{"fast": true}}}}');
            COMMIT;
            "#
        )
    }
}
