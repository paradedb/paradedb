use std::str::FromStr;

use anyhow::Result;
use pgrx::pg_sys::BuiltinOid;
use pgrx::spi::Spi;
use pgrx::{prelude::*, JsonB};
use serde_json::json;

#[pg_extern(sql = "
CREATE OR REPLACE PROCEDURE paradedb.create_bm25_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb')
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
fn create_bm25_test_table(table_name: Option<&str>, schema_name: Option<&str>) -> Result<()> {
    let table_name = table_name.unwrap_or("bm25_test_table");
    let schema_name = schema_name.unwrap_or("paradedb");
    let full_table_name = format!("{}.{}", schema_name, table_name);

    Spi::connect(|mut client| {
        let original_client_min_messages: String = client
            .select("SELECT current_setting('client_min_messages')", None, None)?
            .first()
            .get(1)? // 1-based indexing
            .unwrap_or("NOTICE".into()); // Postgres default

        client.update("SET client_min_messages TO WARNING", None, None)?;

        let table_exists = client
            .select(
                &format!(
                    "SELECT FROM pg_catalog.pg_tables WHERE schemaname = '{}' AND tablename = '{}'",
                    schema_name, table_name
                ),
                None,
                None,
            )?
            .is_empty();

        if table_exists {
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

            let data_to_insert = vec![
                (
                    "Ergonomic metal keyboard",
                    4,
                    "Electronics",
                    true,
                    json!({"color": "Silver", "location": "United States"}),
                    Timestamp::from_str("2023-05-01 09:12:34"),
                    Date::from_str("2023-05-03"),
                    Time::from_str("09:12:34"),
                ),
                (
                    "Plastic Keyboard",
                    4,
                    "Electronics",
                    false,
                    json!({"color": "Black", "location": "Canada"}),
                    Timestamp::from_str("2023-04-15 13:27:09"),
                    Date::from_str("2023-04-16"),
                    Time::from_str("13:27:09"),
                ),
                (
                    "Sleek running shoes",
                    5,
                    "Footwear",
                    true,
                    json!({"color": "Blue", "location": "China"}),
                    Timestamp::from_str("2023-04-28 10:55:43"),
                    Date::from_str("2023-04-29"),
                    Time::from_str("10:55:43"),
                ),
                (
                    "White jogging shoes",
                    3,
                    "Footwear",
                    false,
                    json!({"color": "White", "location": "United States"}),
                    Timestamp::from_str("2023-04-20 16:38:02"),
                    Date::from_str("2023-04-22"),
                    Time::from_str("16:38:02"),
                ),
                (
                    "Generic shoes",
                    4,
                    "Footwear",
                    true,
                    json!({"color": "Brown", "location": "Canada"}),
                    Timestamp::from_str("2023-05-02 08:45:11"),
                    Date::from_str("2023-05-03"),
                    Time::from_str("08:45:11"),
                ),
                (
                    "Compact digital camera",
                    5,
                    "Photography",
                    false,
                    json!({"color": "Black", "location": "China"}),
                    Timestamp::from_str("2023-04-25 11:20:35"),
                    Date::from_str("2023-04-26"),
                    Time::from_str("11:20:35"),
                ),
                (
                    "Hardcover book on history",
                    2,
                    "Books",
                    true,
                    json!({"color": "Brown", "location": "United States"}),
                    Timestamp::from_str("2023-04-18 14:59:27"),
                    Date::from_str("2023-04-19"),
                    Time::from_str("14:59:27"),
                ),
                (
                    "Organic green tea",
                    3,
                    "Groceries",
                    true,
                    json!({"color": "Green", "location": "Canada"}),
                    Timestamp::from_str("2023-04-30 09:18:45"),
                    Date::from_str("2023-05-01"),
                    Time::from_str("09:18:45"),
                ),
                (
                    "Modern wall clock",
                    4,
                    "Home Decor",
                    false,
                    json!({"color": "Silver", "location": "China"}),
                    Timestamp::from_str("2023-04-24 12:37:52"),
                    Date::from_str("2023-04-25"),
                    Time::from_str("12:37:52"),
                ),
                (
                    "Colorful kids toy",
                    1,
                    "Toys",
                    true,
                    json!({"color": "Multicolor", "location": "United States"}),
                    Timestamp::from_str("2023-05-04 15:29:12"),
                    Date::from_str("2023-05-06"),
                    Time::from_str("15:29:12"),
                ),
                (
                    "Soft cotton shirt",
                    5,
                    "Apparel",
                    true,
                    json!({"color": "Blue", "location": "Canada"}),
                    Timestamp::from_str("2023-04-29 08:10:17"),
                    Date::from_str("2023-04-30"),
                    Time::from_str("08:10:17"),
                ),
                (
                    "Innovative wireless earbuds",
                    5,
                    "Electronics",
                    true,
                    json!({"color": "Black", "location": "China"}),
                    Timestamp::from_str("2023-04-22 10:05:39"),
                    Date::from_str("2023-04-23"),
                    Time::from_str("10:05:39"),
                ),
                (
                    "Sturdy hiking boots",
                    4,
                    "Footwear",
                    true,
                    json!({"color": "Brown", "location": "United States"}),
                    Timestamp::from_str("2023-05-05 13:45:22"),
                    Date::from_str("2023-05-07"),
                    Time::from_str("13:45:22"),
                ),
                (
                    "Elegant glass table",
                    3,
                    "Furniture",
                    true,
                    json!({"color": "Clear", "location": "Canada"}),
                    Timestamp::from_str("2023-04-26 17:22:58"),
                    Date::from_str("2023-04-28"),
                    Time::from_str("17:22:58"),
                ),
                (
                    "Refreshing face wash",
                    2,
                    "Beauty",
                    false,
                    json!({"color": "White", "location": "China"}),
                    Timestamp::from_str("2023-04-27 09:52:04"),
                    Date::from_str("2023-04-29"),
                    Time::from_str("09:52:04"),
                ),
                (
                    "High-resolution DSLR",
                    4,
                    "Photography",
                    true,
                    json!({"color": "Black", "location": "United States"}),
                    Timestamp::from_str("2023-04-21 14:30:19"),
                    Date::from_str("2023-04-23"),
                    Time::from_str("14:30:19"),
                ),
                (
                    "Paperback romantic novel",
                    3,
                    "Books",
                    true,
                    json!({"color": "Multicolor", "location": "Canada"}),
                    Timestamp::from_str("2023-05-03 10:08:57"),
                    Date::from_str("2023-05-04"),
                    Time::from_str("10:08:57"),
                ),
                (
                    "Freshly ground coffee beans",
                    5,
                    "Groceries",
                    true,
                    json!({"color": "Brown", "location": "China"}),
                    Timestamp::from_str("2023-04-23 08:40:15"),
                    Date::from_str("2023-04-25"),
                    Time::from_str("08:40:15"),
                ),
                (
                    "Artistic ceramic vase",
                    4,
                    "Home Decor",
                    false,
                    json!({"color": "Multicolor", "location": "United States"}),
                    Timestamp::from_str("2023-04-19 15:17:29"),
                    Date::from_str("2023-04-21"),
                    Time::from_str("15:17:29"),
                ),
                (
                    "Interactive board game",
                    3,
                    "Toys",
                    true,
                    json!({"color": "Multicolor", "location": "Canada"}),
                    Timestamp::from_str("2023-05-01 12:25:06"),
                    Date::from_str("2023-05-02"),
                    Time::from_str("12:25:06"),
                ),
                (
                    "Slim-fit denim jeans",
                    5,
                    "Apparel",
                    false,
                    json!({"color": "Blue", "location": "China"}),
                    Timestamp::from_str("2023-04-28 16:54:33"),
                    Date::from_str("2023-04-30"),
                    Time::from_str("16:54:33"),
                ),
                (
                    "Fast charging power bank",
                    4,
                    "Electronics",
                    true,
                    json!({"color": "Black", "location": "United States"}),
                    Timestamp::from_str("2023-04-17 11:35:52"),
                    Date::from_str("2023-04-19"),
                    Time::from_str("11:35:52"),
                ),
                (
                    "Comfortable slippers",
                    3,
                    "Footwear",
                    true,
                    json!({"color": "Brown", "location": "Canada"}),
                    Timestamp::from_str("2023-04-16 09:20:37"),
                    Date::from_str("2023-04-17"),
                    Time::from_str("09:20:37"),
                ),
                (
                    "Classic leather sofa",
                    5,
                    "Furniture",
                    false,
                    json!({"color": "Brown", "location": "China"}),
                    Timestamp::from_str("2023-05-06 14:45:27"),
                    Date::from_str("2023-05-08"),
                    Time::from_str("14:45:27"),
                ),
                (
                    "Anti-aging serum",
                    4,
                    "Beauty",
                    true,
                    json!({"color": "White", "location": "United States"}),
                    Timestamp::from_str("2023-05-09 10:30:15"),
                    Date::from_str("2023-05-10"),
                    Time::from_str("10:30:15"),
                ),
                (
                    "Portable tripod stand",
                    4,
                    "Photography",
                    true,
                    json!({"color": "Black", "location": "Canada"}),
                    Timestamp::from_str("2023-05-07 15:20:48"),
                    Date::from_str("2023-05-09"),
                    Time::from_str("15:20:48"),
                ),
                (
                    "Mystery detective novel",
                    2,
                    "Books",
                    false,
                    json!({"color": "Multicolor", "location": "China"}),
                    Timestamp::from_str("2023-05-04 11:55:23"),
                    Date::from_str("2023-05-05"),
                    Time::from_str("11:55:23"),
                ),
                (
                    "Organic breakfast cereal",
                    5,
                    "Groceries",
                    true,
                    json!({"color": "Brown", "location": "United States"}),
                    Timestamp::from_str("2023-05-02 07:40:59"),
                    Date::from_str("2023-05-03"),
                    Time::from_str("07:40:59"),
                ),
                (
                    "Designer wall paintings",
                    5,
                    "Home Decor",
                    true,
                    json!({"color": "Multicolor", "location": "Canada"}),
                    Timestamp::from_str("2023-04-30 14:18:37"),
                    Date::from_str("2023-05-01"),
                    Time::from_str("14:18:37"),
                ),
                (
                    "Robot building kit",
                    4,
                    "Toys",
                    true,
                    json!({"color": "Multicolor", "location": "China"}),
                    Timestamp::from_str("2023-04-29 16:25:42"),
                    Date::from_str("2023-05-01"),
                    Time::from_str("16:25:42"),
                ),
                (
                    "Sporty tank top",
                    4,
                    "Apparel",
                    true,
                    json!({"color": "Blue", "location": "United States"}),
                    Timestamp::from_str("2023-04-27 12:09:53"),
                    Date::from_str("2023-04-28"),
                    Time::from_str("12:09:53"),
                ),
                (
                    "Bluetooth-enabled speaker",
                    3,
                    "Electronics",
                    true,
                    json!({"color": "Black", "location": "Canada"}),
                    Timestamp::from_str("2023-04-26 09:34:11"),
                    Date::from_str("2023-04-28"),
                    Time::from_str("09:34:11"),
                ),
                (
                    "Winter woolen socks",
                    5,
                    "Footwear",
                    false,
                    json!({"color": "Gray", "location": "China"}),
                    Timestamp::from_str("2023-04-25 14:55:08"),
                    Date::from_str("2023-04-27"),
                    Time::from_str("14:55:08"),
                ),
                (
                    "Rustic bookshelf",
                    4,
                    "Furniture",
                    true,
                    json!({"color": "Brown", "location": "United States"}),
                    Timestamp::from_str("2023-04-24 08:20:47"),
                    Date::from_str("2023-04-25"),
                    Time::from_str("08:20:47"),
                ),
                (
                    "Moisturizing lip balm",
                    4,
                    "Beauty",
                    true,
                    json!({"color": "Pink", "location": "Canada"}),
                    Timestamp::from_str("2023-04-23 13:48:29"),
                    Date::from_str("2023-04-24"),
                    Time::from_str("13:48:29"),
                ),
                (
                    "Lightweight camera bag",
                    5,
                    "Photography",
                    false,
                    json!({"color": "Black", "location": "China"}),
                    Timestamp::from_str("2023-04-22 17:10:55"),
                    Date::from_str("2023-04-24"),
                    Time::from_str("17:10:55"),
                ),
                (
                    "Historical fiction book",
                    3,
                    "Books",
                    true,
                    json!({"color": "Multicolor", "location": "United States"}),
                    Timestamp::from_str("2023-04-21 10:35:40"),
                    Date::from_str("2023-04-22"),
                    Time::from_str("10:35:40"),
                ),
                (
                    "Pure honey jar",
                    4,
                    "Groceries",
                    true,
                    json!({"color": "Yellow", "location": "Canada"}),
                    Timestamp::from_str("2023-04-20 15:22:14"),
                    Date::from_str("2023-04-22"),
                    Time::from_str("15:22:14"),
                ),
                (
                    "Handcrafted wooden frame",
                    5,
                    "Home Decor",
                    false,
                    json!({"color": "Brown", "location": "China"}),
                    Timestamp::from_str("2023-04-19 08:55:06"),
                    Date::from_str("2023-04-21"),
                    Time::from_str("08:55:06"),
                ),
                (
                    "Plush teddy bear",
                    4,
                    "Toys",
                    true,
                    json!({"color": "Brown", "location": "United States"}),
                    Timestamp::from_str("2023-04-18 11:40:59"),
                    Date::from_str("2023-04-19"),
                    Time::from_str("11:40:59"),
                ),
                (
                    "Warm woolen sweater",
                    3,
                    "Apparel",
                    false,
                    json!({"color": "Red", "location": "Canada"}),
                    Timestamp::from_str("2023-04-17 14:28:37"),
                    Date::from_str("2023-04-18"),
                    Time::from_str("14:28:37"),
                ),
            ];

            for record in data_to_insert {
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
