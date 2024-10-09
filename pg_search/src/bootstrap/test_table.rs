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

use std::str::FromStr;

use anyhow::Result;
use pgrx::pg_sys::BuiltinOid;
use pgrx::spi::Spi;
use pgrx::{prelude::*, JsonB};
use serde::Serialize;
use serde_json::json;
use std::fmt::{Display, Formatter};

#[derive(PostgresEnum, Serialize)]
pub enum TestTable {
    Items,
    Orders,
    Parts,
}

impl Display for TestTable {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            TestTable::Items => write!(f, "Items"),
            TestTable::Orders => write!(f, "Orders"),
            TestTable::Parts => write!(f, "Parts"),
        }
    }
}

#[pg_extern(sql = "
CREATE OR REPLACE PROCEDURE paradedb.create_bm25_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb', table_type paradedb.TestTable DEFAULT 'Items')
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
", requires = [ TestTable ])]
fn create_bm25_test_table(
    table_name: Option<&str>,
    schema_name: Option<&str>,
    table_type: Option<TestTable>,
) -> Result<()> {
    let table_name = table_name.unwrap_or("bm25_test_table");
    let schema_name = schema_name.unwrap_or("paradedb");
    let table_type = table_type.unwrap_or(TestTable::Items);
    let full_table_name = format!("{}.{}", schema_name, table_name);

    Spi::connect(|mut client| {
        let original_client_min_messages: String = client
            .select("SELECT current_setting('client_min_messages')", None, None)?
            .first()
            .get(1)? // 1-based indexing
            .unwrap_or("NOTICE".into()); // Postgres default

        client.update("SET client_min_messages TO WARNING", None, None)?;

        let table_not_found = client
            .select(
                &format!(
                    "SELECT FROM pg_catalog.pg_tables WHERE schemaname = '{}' AND tablename = '{}'",
                    schema_name, table_name
                ),
                None,
                None,
            )?
            .is_empty();

        if table_not_found {
            match table_type {
                TestTable::Items => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                                id SERIAL PRIMARY KEY,
                                description TEXT,
                                rating INTEGER CHECK (rating BETWEEN 1 AND 5),
                                category VARCHAR(255),
                                in_stock BOOLEAN,
                                metadata JSONB,
                                created_at TIMESTAMP,
                                last_updated_date DATE,
                                latest_available_time TIME
                            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_items_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (PgOid::BuiltIn(BuiltinOid::TEXTOID), record.0.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.1.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::VARCHAROID), record.2.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::BOOLOID), record.3.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::JSONBOID), JsonB(record.4).into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::TIMESTAMPOID), record.5.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::DATEOID), record.6.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::TIMEOID), record.7.into_datum()),
                            ]),
                        )?;
                    }
                }
                TestTable::Orders => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                                order_id SERIAL PRIMARY KEY,
                                product_id INTEGER NOT NULL,
                                order_quantity INTEGER NOT NULL,
                                order_total DECIMAL(10, 2) NOT NULL,
                                customer_name VARCHAR(255) NOT NULL
                            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_orders_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (product_id, order_quantity, order_total, customer_name) VALUES ($1, $2, $3, $4)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.0.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.1.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::FLOAT4OID), record.2.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::VARCHAROID), record.3.into_datum()),
                            ]),
                        )?;
                    }
                }
                TestTable::Parts => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                                part_id SERIAL PRIMARY KEY,
                                parent_part_id INTEGER NOT NULL,
                                description VARCHAR(255) NOT NULL
                            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_parts_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (part_id, parent_part_id, description) VALUES ($1, $2, $3)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.0.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.1.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::VARCHAROID), record.2.into_datum()),
                            ]),
                        )?;
                    }
                }
            }
        } else {
            pgrx::warning!("The table {} already exists, skipping.", full_table_name);
        }

        client.update(
            &format!(
                "SET client_min_messages TO '{}'",
                original_client_min_messages
            ),
            None,
            None,
        )?;

        Ok(())
    })
}

#[inline]
#[allow(clippy::type_complexity)]
fn mock_items_data() -> Vec<(
    &'static str,
    i32,
    &'static str,
    bool,
    serde_json::Value,
    Timestamp,
    Date,
    Time,
)> {
    vec![
        (
            "Ergonomic metal keyboard",
            4,
            "Electronics",
            true,
            json!({"color": "Silver", "location": "United States"}),
            Timestamp::from_str("2023-05-01 09:12:34").unwrap(),
            Date::from_str("2023-05-03").unwrap(),
            Time::from_str("09:12:34").unwrap(),
        ),
        (
            "Plastic Keyboard",
            4,
            "Electronics",
            false,
            json!({"color": "Black", "location": "Canada"}),
            Timestamp::from_str("2023-04-15 13:27:09").unwrap(),
            Date::from_str("2023-04-16").unwrap(),
            Time::from_str("13:27:09").unwrap(),
        ),
        (
            "Sleek running shoes",
            5,
            "Footwear",
            true,
            json!({"color": "Blue", "location": "China"}),
            Timestamp::from_str("2023-04-28 10:55:43").unwrap(),
            Date::from_str("2023-04-29").unwrap(),
            Time::from_str("10:55:43").unwrap(),
        ),
        (
            "White jogging shoes",
            3,
            "Footwear",
            false,
            json!({"color": "White", "location": "United States"}),
            Timestamp::from_str("2023-04-20 16:38:02").unwrap(),
            Date::from_str("2023-04-22").unwrap(),
            Time::from_str("16:38:02").unwrap(),
        ),
        (
            "Generic shoes",
            4,
            "Footwear",
            true,
            json!({"color": "Brown", "location": "Canada"}),
            Timestamp::from_str("2023-05-02 08:45:11").unwrap(),
            Date::from_str("2023-05-03").unwrap(),
            Time::from_str("08:45:11").unwrap(),
        ),
        (
            "Compact digital camera",
            5,
            "Photography",
            false,
            json!({"color": "Black", "location": "China"}),
            Timestamp::from_str("2023-04-25 11:20:35").unwrap(),
            Date::from_str("2023-04-26").unwrap(),
            Time::from_str("11:20:35").unwrap(),
        ),
        (
            "Hardcover book on history",
            2,
            "Books",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-04-18 14:59:27").unwrap(),
            Date::from_str("2023-04-19").unwrap(),
            Time::from_str("14:59:27").unwrap(),
        ),
        (
            "Organic green tea",
            3,
            "Groceries",
            true,
            json!({"color": "Green", "location": "Canada"}),
            Timestamp::from_str("2023-04-30 09:18:45").unwrap(),
            Date::from_str("2023-05-01").unwrap(),
            Time::from_str("09:18:45").unwrap(),
        ),
        (
            "Modern wall clock",
            4,
            "Home Decor",
            false,
            json!({"color": "Silver", "location": "China"}),
            Timestamp::from_str("2023-04-24 12:37:52").unwrap(),
            Date::from_str("2023-04-25").unwrap(),
            Time::from_str("12:37:52").unwrap(),
        ),
        (
            "Colorful kids toy",
            1,
            "Toys",
            true,
            json!({"color": "Multicolor", "location": "United States"}),
            Timestamp::from_str("2023-05-04 15:29:12").unwrap(),
            Date::from_str("2023-05-06").unwrap(),
            Time::from_str("15:29:12").unwrap(),
        ),
        (
            "Soft cotton shirt",
            5,
            "Apparel",
            true,
            json!({"color": "Blue", "location": "Canada"}),
            Timestamp::from_str("2023-04-29 08:10:17").unwrap(),
            Date::from_str("2023-04-30").unwrap(),
            Time::from_str("08:10:17").unwrap(),
        ),
        (
            "Innovative wireless earbuds",
            5,
            "Electronics",
            true,
            json!({"color": "Black", "location": "China"}),
            Timestamp::from_str("2023-04-22 10:05:39").unwrap(),
            Date::from_str("2023-04-23").unwrap(),
            Time::from_str("10:05:39").unwrap(),
        ),
        (
            "Sturdy hiking boots",
            4,
            "Footwear",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-05-05 13:45:22").unwrap(),
            Date::from_str("2023-05-07").unwrap(),
            Time::from_str("13:45:22").unwrap(),
        ),
        (
            "Elegant glass table",
            3,
            "Furniture",
            true,
            json!({"color": "Clear", "location": "Canada"}),
            Timestamp::from_str("2023-04-26 17:22:58").unwrap(),
            Date::from_str("2023-04-28").unwrap(),
            Time::from_str("17:22:58").unwrap(),
        ),
        (
            "Refreshing face wash",
            2,
            "Beauty",
            false,
            json!({"color": "White", "location": "China"}),
            Timestamp::from_str("2023-04-27 09:52:04").unwrap(),
            Date::from_str("2023-04-29").unwrap(),
            Time::from_str("09:52:04").unwrap(),
        ),
        (
            "High-resolution DSLR",
            4,
            "Photography",
            true,
            json!({"color": "Black", "location": "United States"}),
            Timestamp::from_str("2023-04-21 14:30:19").unwrap(),
            Date::from_str("2023-04-23").unwrap(),
            Time::from_str("14:30:19").unwrap(),
        ),
        (
            "Paperback romantic novel",
            3,
            "Books",
            true,
            json!({"color": "Multicolor", "location": "Canada"}),
            Timestamp::from_str("2023-05-03 10:08:57").unwrap(),
            Date::from_str("2023-05-04").unwrap(),
            Time::from_str("10:08:57").unwrap(),
        ),
        (
            "Freshly ground coffee beans",
            5,
            "Groceries",
            true,
            json!({"color": "Brown", "location": "China"}),
            Timestamp::from_str("2023-04-23 08:40:15").unwrap(),
            Date::from_str("2023-04-25").unwrap(),
            Time::from_str("08:40:15").unwrap(),
        ),
        (
            "Artistic ceramic vase",
            4,
            "Home Decor",
            false,
            json!({"color": "Multicolor", "location": "United States"}),
            Timestamp::from_str("2023-04-19 15:17:29").unwrap(),
            Date::from_str("2023-04-21").unwrap(),
            Time::from_str("15:17:29").unwrap(),
        ),
        (
            "Interactive board game",
            3,
            "Toys",
            true,
            json!({"color": "Multicolor", "location": "Canada"}),
            Timestamp::from_str("2023-05-01 12:25:06").unwrap(),
            Date::from_str("2023-05-02").unwrap(),
            Time::from_str("12:25:06").unwrap(),
        ),
        (
            "Slim-fit denim jeans",
            5,
            "Apparel",
            false,
            json!({"color": "Blue", "location": "China"}),
            Timestamp::from_str("2023-04-28 16:54:33").unwrap(),
            Date::from_str("2023-04-30").unwrap(),
            Time::from_str("16:54:33").unwrap(),
        ),
        (
            "Fast charging power bank",
            4,
            "Electronics",
            true,
            json!({"color": "Black", "location": "United States"}),
            Timestamp::from_str("2023-04-17 11:35:52").unwrap(),
            Date::from_str("2023-04-19").unwrap(),
            Time::from_str("11:35:52").unwrap(),
        ),
        (
            "Comfortable slippers",
            3,
            "Footwear",
            true,
            json!({"color": "Brown", "location": "Canada"}),
            Timestamp::from_str("2023-04-16 09:20:37").unwrap(),
            Date::from_str("2023-04-17").unwrap(),
            Time::from_str("09:20:37").unwrap(),
        ),
        (
            "Classic leather sofa",
            5,
            "Furniture",
            false,
            json!({"color": "Brown", "location": "China"}),
            Timestamp::from_str("2023-05-06 14:45:27").unwrap(),
            Date::from_str("2023-05-08").unwrap(),
            Time::from_str("14:45:27").unwrap(),
        ),
        (
            "Anti-aging serum",
            4,
            "Beauty",
            true,
            json!({"color": "White", "location": "United States"}),
            Timestamp::from_str("2023-05-09 10:30:15").unwrap(),
            Date::from_str("2023-05-10").unwrap(),
            Time::from_str("10:30:15").unwrap(),
        ),
        (
            "Portable tripod stand",
            4,
            "Photography",
            true,
            json!({"color": "Black", "location": "Canada"}),
            Timestamp::from_str("2023-05-07 15:20:48").unwrap(),
            Date::from_str("2023-05-09").unwrap(),
            Time::from_str("15:20:48").unwrap(),
        ),
        (
            "Mystery detective novel",
            2,
            "Books",
            false,
            json!({"color": "Multicolor", "location": "China"}),
            Timestamp::from_str("2023-05-04 11:55:23").unwrap(),
            Date::from_str("2023-05-05").unwrap(),
            Time::from_str("11:55:23").unwrap(),
        ),
        (
            "Organic breakfast cereal",
            5,
            "Groceries",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-05-02 07:40:59").unwrap(),
            Date::from_str("2023-05-03").unwrap(),
            Time::from_str("07:40:59").unwrap(),
        ),
        (
            "Designer wall paintings",
            5,
            "Home Decor",
            true,
            json!({"color": "Multicolor", "location": "Canada"}),
            Timestamp::from_str("2023-04-30 14:18:37").unwrap(),
            Date::from_str("2023-05-01").unwrap(),
            Time::from_str("14:18:37").unwrap(),
        ),
        (
            "Robot building kit",
            4,
            "Toys",
            true,
            json!({"color": "Multicolor", "location": "China"}),
            Timestamp::from_str("2023-04-29 16:25:42").unwrap(),
            Date::from_str("2023-05-01").unwrap(),
            Time::from_str("16:25:42").unwrap(),
        ),
        (
            "Sporty tank top",
            4,
            "Apparel",
            true,
            json!({"color": "Blue", "location": "United States"}),
            Timestamp::from_str("2023-04-27 12:09:53").unwrap(),
            Date::from_str("2023-04-28").unwrap(),
            Time::from_str("12:09:53").unwrap(),
        ),
        (
            "Bluetooth-enabled speaker",
            3,
            "Electronics",
            true,
            json!({"color": "Black", "location": "Canada"}),
            Timestamp::from_str("2023-04-26 09:34:11").unwrap(),
            Date::from_str("2023-04-28").unwrap(),
            Time::from_str("09:34:11").unwrap(),
        ),
        (
            "Winter woolen socks",
            5,
            "Footwear",
            false,
            json!({"color": "Gray", "location": "China"}),
            Timestamp::from_str("2023-04-25 14:55:08").unwrap(),
            Date::from_str("2023-04-27").unwrap(),
            Time::from_str("14:55:08").unwrap(),
        ),
        (
            "Rustic bookshelf",
            4,
            "Furniture",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-04-24 08:20:47").unwrap(),
            Date::from_str("2023-04-25").unwrap(),
            Time::from_str("08:20:47").unwrap(),
        ),
        (
            "Moisturizing lip balm",
            4,
            "Beauty",
            true,
            json!({"color": "Pink", "location": "Canada"}),
            Timestamp::from_str("2023-04-23 13:48:29").unwrap(),
            Date::from_str("2023-04-24").unwrap(),
            Time::from_str("13:48:29").unwrap(),
        ),
        (
            "Lightweight camera bag",
            5,
            "Photography",
            false,
            json!({"color": "Black", "location": "China"}),
            Timestamp::from_str("2023-04-22 17:10:55").unwrap(),
            Date::from_str("2023-04-24").unwrap(),
            Time::from_str("17:10:55").unwrap(),
        ),
        (
            "Historical fiction book",
            3,
            "Books",
            true,
            json!({"color": "Multicolor", "location": "United States"}),
            Timestamp::from_str("2023-04-21 10:35:40").unwrap(),
            Date::from_str("2023-04-22").unwrap(),
            Time::from_str("10:35:40").unwrap(),
        ),
        (
            "Pure honey jar",
            4,
            "Groceries",
            true,
            json!({"color": "Yellow", "location": "Canada"}),
            Timestamp::from_str("2023-04-20 15:22:14").unwrap(),
            Date::from_str("2023-04-22").unwrap(),
            Time::from_str("15:22:14").unwrap(),
        ),
        (
            "Handcrafted wooden frame",
            5,
            "Home Decor",
            false,
            json!({"color": "Brown", "location": "China"}),
            Timestamp::from_str("2023-04-19 08:55:06").unwrap(),
            Date::from_str("2023-04-21").unwrap(),
            Time::from_str("08:55:06").unwrap(),
        ),
        (
            "Plush teddy bear",
            4,
            "Toys",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-04-18 11:40:59").unwrap(),
            Date::from_str("2023-04-19").unwrap(),
            Time::from_str("11:40:59").unwrap(),
        ),
        (
            "Warm woolen sweater",
            3,
            "Apparel",
            false,
            json!({"color": "Red", "location": "Canada"}),
            Timestamp::from_str("2023-04-17 14:28:37").unwrap(),
            Date::from_str("2023-04-18").unwrap(),
            Time::from_str("14:28:37").unwrap(),
        ),
    ]
}

#[inline]
fn mock_orders_data() -> Vec<(i32, i32, f32, &'static str)> {
    vec![
        (1, 3, 99.99, "John Doe"),
        (2, 1, 49.99, "Jane Smith"),
        (3, 5, 249.95, "Alice Johnson"),
        (2, 6, 501.87, "John Doe"),
        (7, 10, 361.38, "Jane Smith"),
        (4, 6, 308.18, "Alice Johnson"),
        (5, 6, 439.05, "Michael Brown"),
        (8, 3, 104.88, "Emily Davis"),
        (3, 5, 132.75, "Chris Wilson"),
        (6, 8, 638.73, "Laura Martinez"),
        (1, 7, 633.94, "David White"),
        (9, 8, 195.11, "Sarah Lewis"),
        (10, 10, 234.32, "Mark Thomas"),
        (2, 4, 55.41, "Rachel Green"),
        (1, 5, 239.31, "Monica Geller"),
        (10, 2, 110.06, "Ross Geller"),
        (1, 1, 74.75, "Chandler Bing"),
        (10, 6, 484.98, "Phoebe Buffay"),
        (8, 9, 319.31, "Joey Tribbiani"),
        (9, 3, 150.90, "Will Smith"),
        (7, 8, 632.08, "Jada Smith"),
        (10, 9, 605.18, "Bruce Wayne"),
        (4, 4, 61.25, "Clark Kent"),
        (2, 7, 258.88, "Diana Prince"),
        (3, 10, 450.57, "Peter Parker"),
        (9, 7, 102.28, "Tony Stark"),
        (2, 7, 676.15, "Natasha Romanoff"),
        (9, 5, 237.22, "Steve Rogers"),
        (9, 4, 381.90, "Thor Odinson"),
        (8, 4, 278.91, "Bruce Banner"),
        (4, 5, 402.69, "Wanda Maximoff"),
        (8, 2, 91.16, "Vision"),
        (3, 9, 194.87, "Scott Lang"),
        (5, 9, 431.54, "Hope Van Dyne"),
        (9, 7, 361.38, "Jane Smith"),
        (4, 6, 308.18, "Alice Johnson"),
        (5, 6, 439.05, "Michael Brown"),
        (8, 3, 104.88, "Emily Davis"),
        (3, 5, 132.75, "Chris Wilson"),
        (6, 8, 638.73, "Laura Martinez"),
        (1, 7, 633.94, "David White"),
        (9, 8, 195.11, "Sarah Lewis"),
        (10, 10, 234.32, "Mark Thomas"),
        (2, 4, 55.41, "Rachel Green"),
        (1, 5, 239.31, "Monica Geller"),
        (10, 2, 110.06, "Ross Geller"),
        (1, 1, 74.75, "Chandler Bing"),
        (10, 6, 484.98, "Phoebe Buffay"),
        (8, 9, 319.31, "Joey Tribbiani"),
        (9, 3, 150.90, "Will Smith"),
        (7, 8, 632.08, "Jada Smith"),
        (10, 9, 605.18, "Bruce Wayne"),
        (4, 4, 61.25, "Clark Kent"),
        (2, 7, 258.88, "Diana Prince"),
        (3, 10, 450.57, "Peter Parker"),
        (9, 7, 102.28, "Tony Stark"),
        (2, 7, 676.15, "Natasha Romanoff"),
        (9, 5, 237.22, "Steve Rogers"),
        (9, 4, 381.90, "Thor Odinson"),
        (8, 4, 278.91, "Bruce Banner"),
        (4, 5, 402.69, "Wanda Maximoff"),
        (8, 2, 91.16, "Vision"),
        (3, 9, 194.87, "Scott Lang"),
        (5, 9, 431.54, "Hope Van Dyne"),
    ]
}

#[inline]
fn mock_parts_data() -> Vec<(i32, i32, &'static str)> {
    vec![
        (1, 0, "Chassis Assembly"),
        (2, 1, "Engine Block"),
        (3, 1, "Transmission System"),
        (4, 1, "Suspension System"),
        (5, 2, "Cylinder Head"),
        (6, 2, "Piston"),
        (7, 2, "Crankshaft"),
        (8, 3, "Gearbox"),
        (9, 3, "Clutch"),
        (10, 4, "Shock Absorber"),
        (11, 4, "Control Arm"),
        (12, 5, "Valve"),
        (13, 5, "Camshaft"),
        (14, 6, "Connecting Rod"),
        (15, 6, "Piston Rings"),
        (16, 8, "Gear Set"),
        (17, 8, "Synchromesh"),
        (18, 9, "Pressure Plate"),
        (19, 9, "Clutch Disc"),
        (20, 12, "Valve Spring"),
        (21, 13, "Timing Gear"),
        (22, 14, "Crank Pin"),
        (23, 15, "Oil Scraper Rings"),
        (24, 16, "Gear Teeth"),
        (25, 16, "Shaft Bearing"),
        (26, 17, "Synchronizer Hub"),
        (27, 18, "Clutch Bearing"),
        (28, 19, "Friction Disc"),
        (29, 24, "Bearing Cage"),
        (30, 24, "Thrust Washer"),
        (31, 25, "Sealing Ring"),
        (32, 25, "Roller Bearing"),
        (33, 26, "Hub Spring"),
        (34, 26, "Shift Fork"),
        (35, 27, "Release Bearing"),
        (36, 28, "Wear Plate"),
    ]
}
