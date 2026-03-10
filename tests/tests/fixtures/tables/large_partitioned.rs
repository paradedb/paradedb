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

use chrono::NaiveDateTime;
use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct LargePartitionedTable {
    pub id: i32,
    pub sale_date: NaiveDateTime,
    pub amount: f32,
    pub description: String,
}

impl LargePartitionedTable {
    pub fn setup() -> String {
        LARGE_PARTITIONED_TABLE_SETUP.into()
    }
}

static LARGE_PARTITIONED_TABLE_SETUP: &str = r#"
BEGIN;
    CREATE TABLE sales_large (
        id SERIAL,
        sale_date DATE NOT NULL,
        amount REAL NOT NULL,
        description TEXT,
        PRIMARY KEY (id, sale_date)
    ) PARTITION BY RANGE (sale_date);

    CREATE TABLE sales_large_2022_q1 PARTITION OF sales_large
      FOR VALUES FROM ('2022-01-01') TO ('2022-04-01');
    CREATE TABLE sales_large_2022_q2 PARTITION OF sales_large
      FOR VALUES FROM ('2022-04-01') TO ('2022-07-01');
    CREATE TABLE sales_large_2022_q3 PARTITION OF sales_large
      FOR VALUES FROM ('2022-07-01') TO ('2022-10-01');
    CREATE TABLE sales_large_2022_q4 PARTITION OF sales_large
      FOR VALUES FROM ('2022-10-01') TO ('2023-01-01');
    CREATE TABLE sales_large_2023_q1 PARTITION OF sales_large
      FOR VALUES FROM ('2023-01-01') TO ('2023-04-01');
    CREATE TABLE sales_large_2023_q2 PARTITION OF sales_large
      FOR VALUES FROM ('2023-04-01') TO ('2023-07-01');
    CREATE TABLE sales_large_2023_q3 PARTITION OF sales_large
      FOR VALUES FROM ('2023-07-01') TO ('2023-10-01');
    CREATE TABLE sales_large_2023_q4 PARTITION OF sales_large
      FOR VALUES FROM ('2023-10-01') TO ('2024-01-01');

    CREATE INDEX sales_large_idx ON sales_large
      USING bm25 (id, description, sale_date, amount)
      WITH (
        key_field='id',
        numeric_fields='{"amount": {"fast": true}}',
        datetime_fields='{"sale_date": {"fast": true}}'
      );

    INSERT INTO sales_large (sale_date, amount, description)
    SELECT
        (DATE '2022-01-01' + (random() * 729)::integer) AS sale_date,
        (random() * 1000)::real AS amount,
        ('wine '::text || md5(random()::text)) AS description
    FROM generate_series(1, 100000);

    ANALYZE sales_large;
COMMIT;
"#;
