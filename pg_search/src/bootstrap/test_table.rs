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

use std::str::FromStr;

use anyhow::Result;
use pgrx::spi::Spi;
use pgrx::{datum::RangeBound, prelude::*, Inet, Json, JsonB, Uuid};
use serde::Serialize;
use serde_json::json;
use std::fmt::{Display, Formatter};

#[derive(PostgresEnum, Serialize)]
pub enum TestTable {
    Items,
    Orders,
    Parts,
    Deliveries,
    Customers,
}

impl Display for TestTable {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            TestTable::Items => write!(f, "Items"),
            TestTable::Orders => write!(f, "Orders"),
            TestTable::Parts => write!(f, "Parts"),
            TestTable::Deliveries => write!(f, "Deliveries"),
            TestTable::Customers => write!(f, "Customers"),
        }
    }
}

type MockDeliveryRow = (
    i32,
    Range<i32>,
    Range<i64>,
    Range<AnyNumeric>,
    Range<Date>,
    Range<Timestamp>,
    Range<TimestampWithTimeZone>,
);

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
    let full_table_name = format!("{schema_name}.{table_name}");

    Spi::connect_mut(|client| {
        let original_client_min_messages: String = client
            .select("SELECT current_setting('client_min_messages')", None, &[])?
            .first()
            .get(1)? // 1-based indexing
            .unwrap_or("NOTICE".into()); // Postgres default

        client.update("SET client_min_messages TO WARNING", None, &[])?;

        let table_not_found = client
            .select(
                &format!(
                    "SELECT FROM pg_catalog.pg_tables WHERE schemaname = '{schema_name}' AND tablename = '{table_name}'"
                ),
                None,
                &[],
            )?
            .is_empty();

        if table_not_found {
            match table_type {
                TestTable::Items => {
                    client.update(
                        &format!(
                            "CREATE TABLE {full_table_name} (
                                id SERIAL PRIMARY KEY,
                                description TEXT,
                                rating INTEGER CHECK (rating BETWEEN 1 AND 5),
                                category VARCHAR(255),
                                in_stock BOOLEAN,
                                metadata JSONB,
                                created_at TIMESTAMP,
                                last_updated_date DATE,
                                latest_available_time TIME,
                                weight_range INT4RANGE,
                                shelf_number SMALLINT,
                                barcode BIGINT,
                                unit_weight_kg REAL,
                                popularity_score DOUBLE PRECISION,
                                price NUMERIC,
                                sku UUID,
                                updated_from_ip INET,
                                specifications JSON,
                                last_restocked_at TIMESTAMPTZ,
                                store_opens_at TIMETZ
                            )"
                        ),
                        None,
                        &[],
                    )?;

                    for record in mock_items_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {full_table_name} (description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range, shelf_number, barcode, unit_weight_kg, popularity_score, price, sku, updated_from_ip, specifications, last_restocked_at, store_opens_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)"
                            ),
                            Some(1),
                            &[
                                record.description.into(),
                                record.rating.into(),
                                record.category.into(),
                                record.in_stock.into(),
                                JsonB(record.metadata).into(),
                                record.created_at.into(),
                                record.last_updated_date.into(),
                                record.latest_available_time.into(),
                                record.weight_range.into(),
                                record.shelf_number.into(),
                                record.barcode.into(),
                                record.unit_weight_kg.into(),
                                record.popularity_score.into(),
                                record.price.into(),
                                record.sku.into(),
                                record.updated_from_ip.into(),
                                Json(record.specifications).into(),
                                record.last_restocked_at.into(),
                                record.store_opens_at.into(),
                            ],
                        )?;
                    }
                }
                TestTable::Orders => {
                    client.update(
                        &format!(
                            "CREATE TABLE {full_table_name} (
                                order_id SERIAL PRIMARY KEY,
                                product_id INTEGER NOT NULL,
                                order_quantity INTEGER NOT NULL,
                                order_total DECIMAL(10, 2) NOT NULL,
                                customer_name VARCHAR(255) NOT NULL
                            )"
                        ),
                        None,
                        &[],
                    )?;

                    for record in mock_orders_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {full_table_name} (product_id, order_quantity, order_total, customer_name) VALUES ($1, $2, $3, $4)"
                            ),
                            Some(1),
                            &[
                                record.0.into(),
                                record.1.into(),
                                record.2.into(),
                                record.3.into(),
                            ],
                        )?;
                    }
                }
                TestTable::Parts => {
                    client.update(
                        &format!(
                            "CREATE TABLE {full_table_name} (
                                part_id SERIAL PRIMARY KEY,
                                parent_part_id INTEGER NOT NULL,
                                description VARCHAR(255) NOT NULL
                            )"
                        ),
                        None,
                        &[],
                    )?;

                    for record in mock_parts_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {full_table_name} (part_id, parent_part_id, description) VALUES ($1, $2, $3)"
                            ),
                            Some(1),
                            &[
                                record.0.into(),
                                record.1.into(),
                                record.2.into(),
                            ],
                        )?;
                    }
                }
                TestTable::Deliveries => {
                    client.update(
                        &format!(
                            "CREATE TABLE {full_table_name} (
                                delivery_id SERIAL PRIMARY KEY,
                                weights INT4RANGE,
                                quantities INT8RANGE,
                                prices NUMRANGE,
                                ship_dates DATERANGE,
                                facility_arrival_times TSRANGE,
                                delivery_times TSTZRANGE
                            )"
                        ),
                        None,
                        &[],
                    )?;

                    for record in mock_deliveries_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {full_table_name} (weights, quantities, prices, ship_dates, facility_arrival_times, delivery_times) 
                                VALUES ($1, $2, $3, $4, $5, $6)"
                            ),
                            Some(1),
                            &[
                                record.1.into(),
                                record.2.into(),
                                record.3.into(),
                                record.4.into(),
                                record.5.into(),
                                record.6.into(),
                            ],
                        )?;
                    }
                }
                // Then in the match statement, add the Customers variant:
                TestTable::Customers => {
                    client.update(
                        &format!(
                            "CREATE TABLE {full_table_name} (
                id SERIAL PRIMARY KEY,
                name TEXT,
                crm_data JSONB
            )"
                        ),
                        None,
                        &[],
                    )?;

                    for record in mock_customers_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {full_table_name} (name, crm_data) VALUES ($1, $2)"
                            ),
                            Some(1),
                            &[record.0.into(), JsonB(record.1).into()],
                        )?;
                    }
                }
            }
        } else {
            pgrx::warning!("The table {} already exists, skipping.", full_table_name);
        }

        client.update(
            &format!("SET client_min_messages TO '{original_client_min_messages}'"),
            None,
            &[],
        )?;

        Ok(())
    })
}

struct MockItemsRow {
    description: &'static str,
    rating: i32,
    category: &'static str,
    in_stock: bool,
    metadata: serde_json::Value,
    created_at: Timestamp,
    last_updated_date: Date,
    latest_available_time: Time,
    weight_range: Range<i32>,
    shelf_number: i16,
    barcode: i64,
    unit_weight_kg: f32,
    popularity_score: f64,
    price: AnyNumeric,
    sku: Uuid,
    updated_from_ip: Inet,
    specifications: serde_json::Value,
    last_restocked_at: TimestampWithTimeZone,
    store_opens_at: TimeWithTimeZone,
}

#[inline]
fn mock_items_data() -> Vec<MockItemsRow> {
    vec![
        MockItemsRow {
            description: "Ergonomic metal keyboard",
            rating: 4,
            category: "Electronics",
            in_stock: true,
            metadata: json!({"color": "Silver", "location": "United States"}),
            created_at: Timestamp::from_str("2023-05-01 09:12:34").unwrap(),
            last_updated_date: Date::from_str("2023-05-03").unwrap(),
            latest_available_time: Time::from_str("09:12:34").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
            shelf_number: 1,
            barcode: 4006381333931,
            unit_weight_kg: 0.5,
            popularity_score: 53.7,
            price: AnyNumeric::from_str("23.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01]),
            updated_from_ip: Inet("192.168.1.1".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-02 09:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Plastic Keyboard",
            rating: 4,
            category: "Electronics",
            in_stock: false,
            metadata: json!({"color": "Black", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-15 13:27:09").unwrap(),
            last_updated_date: Date::from_str("2023-04-16").unwrap(),
            latest_available_time: Time::from_str("13:27:09").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Inclusive(9)),
            shelf_number: 2,
            barcode: 4006381333932,
            unit_weight_kg: 0.75,
            popularity_score: 57.4,
            price: AnyNumeric::from_str("36.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02]),
            updated_from_ip: Inet("192.168.1.2".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-03 10:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Sleek running shoes",
            rating: 5,
            category: "Footwear",
            in_stock: true,
            metadata: json!({"color": "Blue", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-28 10:55:43").unwrap(),
            last_updated_date: Date::from_str("2023-04-29").unwrap(),
            latest_available_time: Time::from_str("10:55:43").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(2), RangeBound::Exclusive(10)),
            shelf_number: 3,
            barcode: 4006381333933,
            unit_weight_kg: 1.0,
            popularity_score: 61.1,
            price: AnyNumeric::from_str("49.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x03]),
            updated_from_ip: Inet("192.168.1.3".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-04 11:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "White jogging shoes",
            rating: 3,
            category: "Footwear",
            in_stock: false,
            metadata: json!({"color": "White", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-20 16:38:02").unwrap(),
            last_updated_date: Date::from_str("2023-04-22").unwrap(),
            latest_available_time: Time::from_str("16:38:02").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
            shelf_number: 4,
            barcode: 4006381333934,
            unit_weight_kg: 1.25,
            popularity_score: 64.8,
            price: AnyNumeric::from_str("62.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x04]),
            updated_from_ip: Inet("192.168.1.4".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-05 12:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Generic shoes",
            rating: 4,
            category: "Footwear",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-05-02 08:45:11").unwrap(),
            last_updated_date: Date::from_str("2023-05-03").unwrap(),
            latest_available_time: Time::from_str("08:45:11").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(3), RangeBound::Infinite),
            shelf_number: 5,
            barcode: 4006381333935,
            unit_weight_kg: 1.5,
            popularity_score: 68.5,
            price: AnyNumeric::from_str("75.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x05]),
            updated_from_ip: Inet("10.0.0.0/24".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-06 13:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Compact digital camera",
            rating: 5,
            category: "Photography",
            in_stock: false,
            metadata: json!({"color": "Black", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-25 11:20:35").unwrap(),
            last_updated_date: Date::from_str("2023-04-26").unwrap(),
            latest_available_time: Time::from_str("11:20:35").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(1), RangeBound::Inclusive(9)),
            shelf_number: 6,
            barcode: 4006381333936,
            unit_weight_kg: 1.75,
            popularity_score: 72.2,
            price: AnyNumeric::from_str("88.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x06]),
            updated_from_ip: Inet("192.168.1.6".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-07 14:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Hardcover book on history",
            rating: 2,
            category: "Books",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-18 14:59:27").unwrap(),
            last_updated_date: Date::from_str("2023-04-19").unwrap(),
            latest_available_time: Time::from_str("14:59:27").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Inclusive(11)),
            shelf_number: 7,
            barcode: 4006381333937,
            unit_weight_kg: 2.0,
            popularity_score: 75.9,
            price: AnyNumeric::from_str("11.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x07]),
            updated_from_ip: Inet("192.168.1.7".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-08 15:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Organic green tea",
            rating: 3,
            category: "Groceries",
            in_stock: true,
            metadata: json!({"color": "Green", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-30 09:18:45").unwrap(),
            last_updated_date: Date::from_str("2023-05-01").unwrap(),
            latest_available_time: Time::from_str("09:18:45").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(9)),
            shelf_number: 8,
            barcode: 4006381333938,
            unit_weight_kg: 0.25,
            popularity_score: 79.6,
            price: AnyNumeric::from_str("24.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x08]),
            updated_from_ip: Inet("192.168.1.8".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-09 16:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Modern wall clock",
            rating: 4,
            category: "Home Decor",
            in_stock: false,
            metadata: json!({"color": "Silver", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-24 12:37:52").unwrap(),
            last_updated_date: Date::from_str("2023-04-25").unwrap(),
            latest_available_time: Time::from_str("12:37:52").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Infinite),
            shelf_number: 9,
            barcode: 4006381333939,
            unit_weight_kg: 0.5,
            popularity_score: 83.3,
            price: AnyNumeric::from_str("37.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x09]),
            updated_from_ip: Inet("192.168.1.9".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-10 17:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Colorful kids toy",
            rating: 1,
            category: "Toys",
            in_stock: true,
            metadata: json!({"color": "Multicolor", "location": "United States"}),
            created_at: Timestamp::from_str("2023-05-04 15:29:12").unwrap(),
            last_updated_date: Date::from_str("2023-05-06").unwrap(),
            latest_available_time: Time::from_str("15:29:12").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(11)),
            shelf_number: 10,
            barcode: 4006381333940,
            unit_weight_kg: 0.75,
            popularity_score: 87.0,
            price: AnyNumeric::from_str("50.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10]),
            updated_from_ip: Inet("2001:db8::10".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-11 18:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Soft cotton shirt",
            rating: 5,
            category: "Apparel",
            in_stock: true,
            metadata: json!({"color": "Blue", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-29 08:10:17").unwrap(),
            last_updated_date: Date::from_str("2023-04-30").unwrap(),
            latest_available_time: Time::from_str("08:10:17").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(10)),
            shelf_number: 11,
            barcode: 4006381333941,
            unit_weight_kg: 1.0,
            popularity_score: 90.7,
            price: AnyNumeric::from_str("63.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x11]),
            updated_from_ip: Inet("192.168.1.11".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-12 19:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Innovative wireless earbuds",
            rating: 5,
            category: "Electronics",
            in_stock: true,
            metadata: json!({"color": "Black", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-22 10:05:39").unwrap(),
            last_updated_date: Date::from_str("2023-04-23").unwrap(),
            latest_available_time: Time::from_str("10:05:39").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(8)),
            shelf_number: 12,
            barcode: 4006381333942,
            unit_weight_kg: 1.25,
            popularity_score: 94.4,
            price: AnyNumeric::from_str("76.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x12]),
            updated_from_ip: Inet("192.168.1.12".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-13 08:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Sturdy hiking boots",
            rating: 4,
            category: "Footwear",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "United States"}),
            created_at: Timestamp::from_str("2023-05-05 13:45:22").unwrap(),
            last_updated_date: Date::from_str("2023-05-07").unwrap(),
            latest_available_time: Time::from_str("13:45:22").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(9)),
            shelf_number: 1,
            barcode: 4006381333943,
            unit_weight_kg: 1.5,
            popularity_score: 98.1,
            price: AnyNumeric::from_str("89.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x13]),
            updated_from_ip: Inet("192.168.1.13".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-14 09:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Elegant glass table",
            rating: 3,
            category: "Furniture",
            in_stock: true,
            metadata: json!({"color": "Clear", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-26 17:22:58").unwrap(),
            last_updated_date: Date::from_str("2023-04-28").unwrap(),
            latest_available_time: Time::from_str("17:22:58").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Exclusive(10)),
            shelf_number: 2,
            barcode: 4006381333944,
            unit_weight_kg: 1.75,
            popularity_score: 51.8,
            price: AnyNumeric::from_str("12.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x14]),
            updated_from_ip: Inet("192.168.1.14".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-15 10:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Refreshing face wash",
            rating: 2,
            category: "Beauty",
            in_stock: false,
            metadata: json!({"color": "White", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-27 09:52:04").unwrap(),
            last_updated_date: Date::from_str("2023-04-29").unwrap(),
            latest_available_time: Time::from_str("09:52:04").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Infinite),
            shelf_number: 3,
            barcode: 4006381333945,
            unit_weight_kg: 2.0,
            popularity_score: 55.5,
            price: AnyNumeric::from_str("25.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x15]),
            updated_from_ip: Inet("192.168.1.15".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-16 11:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "High-resolution DSLR",
            rating: 4,
            category: "Photography",
            in_stock: true,
            metadata: json!({"color": "Black", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-21 14:30:19").unwrap(),
            last_updated_date: Date::from_str("2023-04-23").unwrap(),
            latest_available_time: Time::from_str("14:30:19").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(11)),
            shelf_number: 4,
            barcode: 4006381333946,
            unit_weight_kg: 0.25,
            popularity_score: 59.2,
            price: AnyNumeric::from_str("38.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x16]),
            updated_from_ip: Inet("192.168.1.16".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-17 12:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Paperback romantic novel",
            rating: 3,
            category: "Books",
            in_stock: true,
            metadata: json!({"color": "Multicolor", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-05-03 10:08:57").unwrap(),
            last_updated_date: Date::from_str("2023-05-04").unwrap(),
            latest_available_time: Time::from_str("10:08:57").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(3), RangeBound::Inclusive(9)),
            shelf_number: 5,
            barcode: 4006381333947,
            unit_weight_kg: 0.5,
            popularity_score: 62.9,
            price: AnyNumeric::from_str("51.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x17]),
            updated_from_ip: Inet("192.168.1.17".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-18 13:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Freshly ground coffee beans",
            rating: 5,
            category: "Groceries",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-23 08:40:15").unwrap(),
            last_updated_date: Date::from_str("2023-04-25").unwrap(),
            latest_available_time: Time::from_str("08:40:15").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
            shelf_number: 6,
            barcode: 4006381333948,
            unit_weight_kg: 0.75,
            popularity_score: 66.6,
            price: AnyNumeric::from_str("64.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x18]),
            updated_from_ip: Inet("192.168.1.18".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-19 14:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Artistic ceramic vase",
            rating: 4,
            category: "Home Decor",
            in_stock: false,
            metadata: json!({"color": "Multicolor", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-19 15:17:29").unwrap(),
            last_updated_date: Date::from_str("2023-04-21").unwrap(),
            latest_available_time: Time::from_str("15:17:29").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
            shelf_number: 7,
            barcode: 4006381333949,
            unit_weight_kg: 1.0,
            popularity_score: 70.3,
            price: AnyNumeric::from_str("77.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x19]),
            updated_from_ip: Inet("192.168.1.19".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-20 15:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Interactive board game",
            rating: 3,
            category: "Toys",
            in_stock: true,
            metadata: json!({"color": "Multicolor", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-05-01 12:25:06").unwrap(),
            last_updated_date: Date::from_str("2023-05-02").unwrap(),
            latest_available_time: Time::from_str("12:25:06").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(3), RangeBound::Exclusive(10)),
            shelf_number: 8,
            barcode: 4006381333950,
            unit_weight_kg: 1.25,
            popularity_score: 74.0,
            price: AnyNumeric::from_str("90.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x20]),
            updated_from_ip: Inet("2001:db8::20".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-21 16:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Slim-fit denim jeans",
            rating: 5,
            category: "Apparel",
            in_stock: false,
            metadata: json!({"color": "Blue", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-28 16:54:33").unwrap(),
            last_updated_date: Date::from_str("2023-04-30").unwrap(),
            latest_available_time: Time::from_str("16:54:33").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(8)),
            shelf_number: 9,
            barcode: 4006381333951,
            unit_weight_kg: 1.5,
            popularity_score: 77.7,
            price: AnyNumeric::from_str("13.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x21]),
            updated_from_ip: Inet("192.168.1.21".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-22 17:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Fast charging power bank",
            rating: 4,
            category: "Electronics",
            in_stock: true,
            metadata: json!({"color": "Black", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-17 11:35:52").unwrap(),
            last_updated_date: Date::from_str("2023-04-19").unwrap(),
            latest_available_time: Time::from_str("11:35:52").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(4), RangeBound::Inclusive(9)),
            shelf_number: 10,
            barcode: 4006381333952,
            unit_weight_kg: 1.75,
            popularity_score: 81.4,
            price: AnyNumeric::from_str("26.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x22]),
            updated_from_ip: Inet("192.168.1.22".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-23 18:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Comfortable slippers",
            rating: 3,
            category: "Footwear",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-16 09:20:37").unwrap(),
            last_updated_date: Date::from_str("2023-04-17").unwrap(),
            latest_available_time: Time::from_str("09:20:37").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Inclusive(9)),
            shelf_number: 11,
            barcode: 4006381333953,
            unit_weight_kg: 2.0,
            popularity_score: 85.1,
            price: AnyNumeric::from_str("39.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x23]),
            updated_from_ip: Inet("192.168.1.23".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-24 19:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Classic leather sofa",
            rating: 5,
            category: "Furniture",
            in_stock: false,
            metadata: json!({"color": "Brown", "location": "China"}),
            created_at: Timestamp::from_str("2023-05-06 14:45:27").unwrap(),
            last_updated_date: Date::from_str("2023-05-08").unwrap(),
            latest_available_time: Time::from_str("14:45:27").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(10)),
            shelf_number: 12,
            barcode: 4006381333954,
            unit_weight_kg: 0.25,
            popularity_score: 88.8,
            price: AnyNumeric::from_str("52.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x24]),
            updated_from_ip: Inet("192.168.1.24".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-25 08:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Anti-aging serum",
            rating: 4,
            category: "Beauty",
            in_stock: true,
            metadata: json!({"color": "White", "location": "United States"}),
            created_at: Timestamp::from_str("2023-05-09 10:30:15").unwrap(),
            last_updated_date: Date::from_str("2023-05-10").unwrap(),
            latest_available_time: Time::from_str("10:30:15").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Infinite),
            shelf_number: 1,
            barcode: 4006381333955,
            unit_weight_kg: 0.5,
            popularity_score: 92.5,
            price: AnyNumeric::from_str("65.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x25]),
            updated_from_ip: Inet("192.168.1.25".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-26 09:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Portable tripod stand",
            rating: 4,
            category: "Photography",
            in_stock: true,
            metadata: json!({"color": "Black", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-05-07 15:20:48").unwrap(),
            last_updated_date: Date::from_str("2023-05-09").unwrap(),
            latest_available_time: Time::from_str("15:20:48").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(3), RangeBound::Inclusive(11)),
            shelf_number: 2,
            barcode: 4006381333956,
            unit_weight_kg: 0.75,
            popularity_score: 96.2,
            price: AnyNumeric::from_str("78.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x26]),
            updated_from_ip: Inet("192.168.1.26".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-27 10:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Mystery detective novel",
            rating: 2,
            category: "Books",
            in_stock: false,
            metadata: json!({"color": "Multicolor", "location": "China"}),
            created_at: Timestamp::from_str("2023-05-04 11:55:23").unwrap(),
            last_updated_date: Date::from_str("2023-05-05").unwrap(),
            latest_available_time: Time::from_str("11:55:23").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(4), RangeBound::Exclusive(9)),
            shelf_number: 3,
            barcode: 4006381333957,
            unit_weight_kg: 1.0,
            popularity_score: 99.9,
            price: AnyNumeric::from_str("91.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x27]),
            updated_from_ip: Inet("192.168.1.27".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-28 11:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Organic breakfast cereal",
            rating: 5,
            category: "Groceries",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "United States"}),
            created_at: Timestamp::from_str("2023-05-02 07:40:59").unwrap(),
            last_updated_date: Date::from_str("2023-05-03").unwrap(),
            latest_available_time: Time::from_str("07:40:59").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(10)),
            shelf_number: 4,
            barcode: 4006381333958,
            unit_weight_kg: 1.25,
            popularity_score: 53.6,
            price: AnyNumeric::from_str("14.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x28]),
            updated_from_ip: Inet("192.168.1.28".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-01 12:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Designer wall paintings",
            rating: 5,
            category: "Home Decor",
            in_stock: true,
            metadata: json!({"color": "Multicolor", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-30 14:18:37").unwrap(),
            last_updated_date: Date::from_str("2023-05-01").unwrap(),
            latest_available_time: Time::from_str("14:18:37").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(9)),
            shelf_number: 5,
            barcode: 4006381333959,
            unit_weight_kg: 1.5,
            popularity_score: 57.3,
            price: AnyNumeric::from_str("27.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x29]),
            updated_from_ip: Inet("192.168.1.29".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-02 13:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Robot building kit",
            rating: 4,
            category: "Toys",
            in_stock: true,
            metadata: json!({"color": "Multicolor", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-29 16:25:42").unwrap(),
            last_updated_date: Date::from_str("2023-05-01").unwrap(),
            latest_available_time: Time::from_str("16:25:42").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
            shelf_number: 6,
            barcode: 4006381333960,
            unit_weight_kg: 1.75,
            popularity_score: 61.0,
            price: AnyNumeric::from_str("40.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x30]),
            updated_from_ip: Inet("2001:db8::30".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-03 14:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Sporty tank top",
            rating: 4,
            category: "Apparel",
            in_stock: true,
            metadata: json!({"color": "Blue", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-27 12:09:53").unwrap(),
            last_updated_date: Date::from_str("2023-04-28").unwrap(),
            latest_available_time: Time::from_str("12:09:53").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(9)),
            shelf_number: 7,
            barcode: 4006381333961,
            unit_weight_kg: 2.0,
            popularity_score: 64.7,
            price: AnyNumeric::from_str("53.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x31]),
            updated_from_ip: Inet("192.168.1.31".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-04 15:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Bluetooth-enabled speaker",
            rating: 3,
            category: "Electronics",
            in_stock: true,
            metadata: json!({"color": "Black", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-26 09:34:11").unwrap(),
            last_updated_date: Date::from_str("2023-04-28").unwrap(),
            latest_available_time: Time::from_str("09:34:11").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(8)),
            shelf_number: 8,
            barcode: 4006381333962,
            unit_weight_kg: 0.25,
            popularity_score: 68.4,
            price: AnyNumeric::from_str("66.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x32]),
            updated_from_ip: Inet("192.168.1.32".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-05 16:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Winter woolen socks",
            rating: 5,
            category: "Footwear",
            in_stock: false,
            metadata: json!({"color": "Gray", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-25 14:55:08").unwrap(),
            last_updated_date: Date::from_str("2023-04-27").unwrap(),
            latest_available_time: Time::from_str("14:55:08").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(3), RangeBound::Inclusive(9)),
            shelf_number: 9,
            barcode: 4006381333963,
            unit_weight_kg: 0.5,
            popularity_score: 72.1,
            price: AnyNumeric::from_str("79.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x33]),
            updated_from_ip: Inet("192.168.1.33".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-06 17:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Rustic bookshelf",
            rating: 4,
            category: "Furniture",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-24 08:20:47").unwrap(),
            last_updated_date: Date::from_str("2023-04-25").unwrap(),
            latest_available_time: Time::from_str("08:20:47").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
            shelf_number: 10,
            barcode: 4006381333964,
            unit_weight_kg: 0.75,
            popularity_score: 75.8,
            price: AnyNumeric::from_str("92.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x34]),
            updated_from_ip: Inet("192.168.1.34".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-07 18:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Moisturizing lip balm",
            rating: 4,
            category: "Beauty",
            in_stock: true,
            metadata: json!({"color": "Pink", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-23 13:48:29").unwrap(),
            last_updated_date: Date::from_str("2023-04-24").unwrap(),
            latest_available_time: Time::from_str("13:48:29").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
            shelf_number: 11,
            barcode: 4006381333965,
            unit_weight_kg: 1.0,
            popularity_score: 79.5,
            price: AnyNumeric::from_str("15.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x35]),
            updated_from_ip: Inet("192.168.1.35".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-08 19:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Lightweight camera bag",
            rating: 5,
            category: "Photography",
            in_stock: false,
            metadata: json!({"color": "Black", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-22 17:10:55").unwrap(),
            last_updated_date: Date::from_str("2023-04-24").unwrap(),
            latest_available_time: Time::from_str("17:10:55").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(9)),
            shelf_number: 12,
            barcode: 4006381333966,
            unit_weight_kg: 1.25,
            popularity_score: 83.2,
            price: AnyNumeric::from_str("28.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x36]),
            updated_from_ip: Inet("192.168.1.36".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-09 08:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Historical fiction book",
            rating: 3,
            category: "Books",
            in_stock: true,
            metadata: json!({"color": "Multicolor", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-21 10:35:40").unwrap(),
            last_updated_date: Date::from_str("2023-04-22").unwrap(),
            latest_available_time: Time::from_str("10:35:40").unwrap(),
            weight_range: Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(8)),
            shelf_number: 1,
            barcode: 4006381333967,
            unit_weight_kg: 1.5,
            popularity_score: 86.9,
            price: AnyNumeric::from_str("41.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x37]),
            updated_from_ip: Inet("192.168.1.37".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-10 09:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Pure honey jar",
            rating: 4,
            category: "Groceries",
            in_stock: true,
            metadata: json!({"color": "Yellow", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-20 15:22:14").unwrap(),
            last_updated_date: Date::from_str("2023-04-22").unwrap(),
            latest_available_time: Time::from_str("15:22:14").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(9)),
            shelf_number: 2,
            barcode: 4006381333968,
            unit_weight_kg: 1.75,
            popularity_score: 90.6,
            price: AnyNumeric::from_str("54.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x38]),
            updated_from_ip: Inet("192.168.1.38".into()),
            specifications: json!({"warranty_years": 2, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-11 10:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Handcrafted wooden frame",
            rating: 5,
            category: "Home Decor",
            in_stock: false,
            metadata: json!({"color": "Brown", "location": "China"}),
            created_at: Timestamp::from_str("2023-04-19 08:55:06").unwrap(),
            last_updated_date: Date::from_str("2023-04-21").unwrap(),
            latest_available_time: Time::from_str("08:55:06").unwrap(),
            weight_range: Range::new(RangeBound::Infinite, RangeBound::Exclusive(10)),
            shelf_number: 3,
            barcode: 4006381333969,
            unit_weight_kg: 2.0,
            popularity_score: 94.3,
            price: AnyNumeric::from_str("67.99").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x39]),
            updated_from_ip: Inet("192.168.1.39".into()),
            specifications: json!({"warranty_years": 3, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-12 11:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("07:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Plush teddy bear",
            rating: 4,
            category: "Toys",
            in_stock: true,
            metadata: json!({"color": "Brown", "location": "United States"}),
            created_at: Timestamp::from_str("2023-04-18 11:40:59").unwrap(),
            last_updated_date: Date::from_str("2023-04-19").unwrap(),
            latest_available_time: Time::from_str("11:40:59").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(9)),
            shelf_number: 4,
            barcode: 4006381333970,
            unit_weight_kg: 0.25,
            popularity_score: 98.0,
            price: AnyNumeric::from_str("0.123456789012345678").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x40]),
            updated_from_ip: Inet("2001:db8::40".into()),
            specifications: json!({"warranty_years": 0, "returnable": true}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-13 12:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("08:00:00-05").unwrap(),
        },
        MockItemsRow {
            description: "Warm woolen sweater",
            rating: 3,
            category: "Apparel",
            in_stock: false,
            metadata: json!({"color": "Red", "location": "Canada"}),
            created_at: Timestamp::from_str("2023-04-17 14:28:37").unwrap(),
            last_updated_date: Date::from_str("2023-04-18").unwrap(),
            latest_available_time: Time::from_str("14:28:37").unwrap(),
            weight_range: Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(10)),
            shelf_number: 5,
            barcode: 4006381333971,
            unit_weight_kg: 0.5,
            popularity_score: 51.7,
            price: AnyNumeric::from_str("12345678901234567890.123456789").unwrap(),
            sku: Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x41]),
            updated_from_ip: Inet("192.168.1.41".into()),
            specifications: json!({"warranty_years": 1, "returnable": false}),
            last_restocked_at: TimestampWithTimeZone::from_str("2023-06-14 13:00:00+00").unwrap(),
            store_opens_at: TimeWithTimeZone::from_str("09:00:00-05").unwrap(),
        },
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

#[inline]
fn mock_deliveries_data() -> Vec<MockDeliveryRow> {
    vec![
        (
            1,
            Range::new(Some(1), Some(10)),
            Range::new(None, Some(200000)),
            Range::new(
                AnyNumeric::try_from(1.5).unwrap(),
                RangeBound::Exclusive(AnyNumeric::try_from(10.5).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-01-01").unwrap()),
                Some(Date::from_str("2024-01-10").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-01-01T12:00:00Z").unwrap()),
                Some(Timestamp::from_str("2024-01-01T18:00:00Z").unwrap()),
            ),
            Range::new(
                Some(TimestampWithTimeZone::from_str("2024-01-01T10:00:00+00:00").unwrap()),
                Some(TimestampWithTimeZone::from_str("2024-01-01T17:00:00+00:00").unwrap()),
            ),
        ),
        (
            2,
            Range::new(None, Some(13)),
            Range::new(Some(150000), Some(250000)),
            Range::new(
                Some(AnyNumeric::try_from(2.5).unwrap()),
                Some(AnyNumeric::try_from(12.5).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-02-01").unwrap()),
                RangeBound::Exclusive(Date::from_str("2024-02-05").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-02-01T08:00:00Z").unwrap()),
                Some(Timestamp::from_str("2024-02-01T14:00:00Z").unwrap()),
            ),
            Range::new(
                Some(TimestampWithTimeZone::from_str("2024-02-01T06:00:00+00:00").unwrap()),
                RangeBound::Exclusive(
                    TimestampWithTimeZone::from_str("2024-02-01T13:00:00+00:00").unwrap(),
                ),
            ),
        ),
        (
            3,
            Range::new(Some(8), RangeBound::Exclusive(18)),
            Range::new(Some(120000), Some(220000)),
            Range::new(
                RangeBound::Exclusive(AnyNumeric::try_from(4.0).unwrap()),
                Some(AnyNumeric::try_from(9.2).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-03-01").unwrap()),
                Some(Date::from_str("2024-03-06").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-03-01T09:30:00Z").unwrap()),
                Some(Timestamp::from_str("2024-03-01T15:00:00Z").unwrap()),
            ),
            Range::new(
                Some(TimestampWithTimeZone::from_str("2024-03-01T07:00:00+00:00").unwrap()),
                Some(TimestampWithTimeZone::from_str("2024-03-01T13:00:00+00:00").unwrap()),
            ),
        ),
        (
            4,
            Range::new(None, RangeBound::Exclusive(20)),
            Range::new(Some(180000), Some(280000)),
            Range::new(
                Some(AnyNumeric::try_from(3.0).unwrap()),
                RangeBound::Exclusive(AnyNumeric::try_from(4.1).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-04-01").unwrap()),
                RangeBound::Exclusive(Date::from_str("2024-04-10").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-04-01T11:45:00Z").unwrap()),
                Some(Timestamp::from_str("2024-04-01T16:30:00Z").unwrap()),
            ),
            Range::new(
                None,
                Some(TimestampWithTimeZone::from_str("2024-04-01T12:00:00+00:00").unwrap()),
            ),
        ),
        (
            5,
            Range::new(Some(2), Some(12)),
            Range::new(RangeBound::Exclusive(170000), RangeBound::Exclusive(270000)),
            Range::new(
                RangeBound::Exclusive(AnyNumeric::try_from(3.5).unwrap()),
                Some(AnyNumeric::try_from(11.2).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-05-01").unwrap()),
                Some(Date::from_str("2024-05-07").unwrap()),
            ),
            Range::new(
                RangeBound::Exclusive(Timestamp::from_str("2024-05-01T14:00:00Z").unwrap()),
                Some(Timestamp::from_str("2024-05-01T19:00:00Z").unwrap()),
            ),
            Range::new(
                RangeBound::Exclusive(
                    TimestampWithTimeZone::from_str("2024-05-01T10:30:00+00:00").unwrap(),
                ),
                Some(TimestampWithTimeZone::from_str("2024-05-01T16:00:00+00:00").unwrap()),
            ),
        ),
    ]
}

#[inline]
fn mock_customers_data() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "Customer A",
            json!([{
                "interaction": "call",
                "details": {
                    "subject": "Welcome Call",
                    "date": "2023-09-01"
                }
            }, {
                "interaction": "email",
                "details": {
                    "subject": "Goodbye Email",
                    "date": "2023-09-02"
                }
            }]),
        ),
        (
            "Customer Deep",
            json!({
                "level1": {
                    "level2": [{
                        "level3": "deep_value",
                        "extra": "metadata"
                    }]
                },
                "other_data": "some value"
            }),
        ),
        (
            "Customer B",
            json!([{
                "interaction": "sms",
                "details": {
                    "subject": "Reminder",
                    "date": "2023-09-01"
                }
            }, {
                "interaction": "email",
                "details": {
                    "subject": "Update",
                    "date": "2023-09-03"
                }
            }]),
        ),
        (
            "Customer C",
            json!([{
                "interaction": "call",
                "details": {
                    "subject": "Service Call",
                    "date": "2023-09-04"
                }
            }, {
                "interaction": "sms",
                "details": {
                    "subject": "Follow-up",
                    "date": "2023-09-05"
                }
            }]),
        ),
        (
            "Customer D",
            json!([{
                "interaction": "email",
                "details": {
                    "subject": "Promotion",
                    "date": "2023-09-06"
                }
            }, {
                "interaction": "sms",
                "details": {
                    "subject": "Discount",
                    "date": "2023-09-07"
                }
            }]),
        ),
        (
            "Customer E",
            json!([{
                "interaction": "call",
                "details": {
                    "subject": "Inquiry",
                    "date": "2023-09-08"
                }
            }, {
                "interaction": "email",
                "details": {
                    "subject": "Notification",
                    "date": "2023-09-09"
                }
            }]),
        ),
    ]
}
