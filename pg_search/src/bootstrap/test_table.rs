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
use pgrx::{datum::RangeBound, prelude::*, JsonB};
use serde::Serialize;
use serde_json::json;
use std::fmt::{Display, Formatter};

#[derive(PostgresEnum, Serialize)]
pub enum TestTable {
    Items,
    ItemsNoEmbedding,
    Orders,
    Parts,
    Deliveries,
    Customers,
}

impl Display for TestTable {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            TestTable::Items => write!(f, "Items"),
            TestTable::ItemsNoEmbedding => write!(f, "ItemsNoEmbedding"),
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
            // `Items` ships a hardcoded 8-dim embedding of `description`; `ItemsNoEmbedding`
            // is the same schema without it, for tests sensitive to physical table layout.
            let with_embedding = matches!(table_type, TestTable::Items);
            match table_type {
                TestTable::Items | TestTable::ItemsNoEmbedding => {
                    if with_embedding {
                        client.update("CREATE EXTENSION IF NOT EXISTS vector", None, &[])?;
                    }

                    let embedding_column = if with_embedding {
                        ",\n                                embedding VECTOR(8)"
                    } else {
                        ""
                    };
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
                                weight_range INT4RANGE{embedding_column}
                            )"
                        ),
                        None,
                        &[],
                    )?;

                    for record in mock_items_data() {
                        let (embedding_column, embedding_value) = if with_embedding {
                            (", embedding", format!(", '{}'", mock_embedding(record.0)))
                        } else {
                            ("", String::new())
                        };
                        client.update(
                            &format!(
                                "INSERT INTO {full_table_name} (description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range{embedding_column}) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9{embedding_value})"
                            ),
                            Some(1),
                            &[
                                record.0.into(),
                                record.1.into(),
                                record.2.into(),
                                record.3.into(),
                                JsonB(record.4).into(),
                                record.5.into(),
                                record.6.into(),
                                record.7.into(),
                                record.8.into(),
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

// Real `nomic-ai/nomic-embed-text-v1.5` embeddings of each description, encoded
// with the "search_document:" prefix, truncated to 8 dims via Matryoshka
// Representation Learning, and L2-normalized. Precomputed offline (the model
// cannot run inside the extension) so the quickstart can demonstrate realistic
// vector(8) search over mock_items. A description without a precomputed vector
// falls back to a deterministic byte-folded unit vector.
fn mock_embedding(description: &str) -> String {
    let precomputed = match description {
        "Ergonomic metal keyboard" => {
            "[0.013747,0.275260,-0.820081,-0.088759,0.334551,0.292097,-0.084229,-0.198224]"
        }
        "Plastic Keyboard" => {
            "[0.012114,0.510939,-0.785573,0.003402,0.201446,0.195153,-0.125991,0.164678]"
        }
        "Sleek running shoes" => {
            "[-0.017080,0.467566,-0.757486,0.134082,0.339330,0.044030,0.185394,-0.194613]"
        }
        "White jogging shoes" => {
            "[-0.042016,0.399574,-0.663812,-0.071006,0.430261,0.210370,0.392919,-0.095506]"
        }
        "Generic shoes" => {
            "[-0.286090,0.368785,-0.810201,0.011777,0.132327,0.199342,0.056281,-0.255287]"
        }
        "Compact digital camera" => {
            "[0.192255,0.148959,-0.826215,-0.036278,-0.031085,0.000193,0.070958,-0.500900]"
        }
        "Hardcover book on history" => {
            "[0.180652,0.237757,-0.845183,-0.175107,0.174689,0.257565,-0.188089,-0.183318]"
        }
        "Organic green tea" => {
            "[-0.180339,0.311079,-0.746309,0.199163,0.063495,0.073963,-0.494147,-0.142765]"
        }
        "Modern wall clock" => {
            "[0.210198,0.154962,-0.807460,-0.133120,0.101520,0.140358,0.386951,-0.286971]"
        }
        "Colorful kids toy" => {
            "[-0.113454,0.519705,-0.661959,-0.101069,0.025229,0.125561,-0.001951,-0.502219]"
        }
        "Soft cotton shirt" => {
            "[0.240812,0.343087,-0.805771,0.089726,-0.032232,0.174167,-0.256043,-0.264673]"
        }
        "Innovative wireless earbuds" => {
            "[-0.080216,0.194348,-0.881002,0.162080,0.303501,0.028384,-0.081238,-0.232037]"
        }
        "Sturdy hiking boots" => {
            "[-0.278831,0.208544,-0.803739,0.279712,0.369904,0.096324,-0.059708,-0.069681]"
        }
        "Elegant glass table" => {
            "[-0.082145,0.416424,-0.800947,-0.327356,0.261584,-0.024049,0.042189,-0.019515]"
        }
        "Refreshing face wash" => {
            "[0.034705,0.239483,-0.882926,0.092522,0.234872,-0.040837,0.119964,-0.286532]"
        }
        "High-resolution DSLR" => {
            "[0.285554,0.310700,-0.819018,0.070856,0.225115,-0.229637,-0.089132,0.186440]"
        }
        "Paperback romantic novel" => {
            "[0.230803,0.227814,-0.764656,-0.090599,0.025157,0.226187,-0.340781,-0.366059]"
        }
        "Freshly ground coffee beans" => {
            "[0.085049,0.101883,-0.918864,-0.111129,0.273488,0.072625,-0.091506,-0.193089]"
        }
        "Artistic ceramic vase" => {
            "[0.134580,0.297808,-0.852239,0.014472,0.223534,0.017951,-0.009123,-0.341034]"
        }
        "Interactive board game" => {
            "[0.127982,0.492142,-0.764924,-0.023066,0.229990,0.045856,0.075451,-0.308360]"
        }
        "Slim-fit denim jeans" => {
            "[0.197377,-0.205968,-0.892619,-0.224507,-0.133416,-0.001279,-0.188535,-0.134541]"
        }
        "Fast charging power bank" => {
            "[0.207739,0.367225,-0.774436,0.131898,0.409067,-0.121126,-0.039155,-0.145953]"
        }
        "Comfortable slippers" => {
            "[-0.206288,0.572354,-0.714623,0.120633,0.168308,0.274744,0.003270,0.028192]"
        }
        "Classic leather sofa" => {
            "[-0.234628,0.065473,-0.867242,-0.055602,0.006776,-0.109428,-0.382065,-0.165736]"
        }
        "Anti-aging serum" => {
            "[-0.019110,-0.057627,-0.929571,-0.036740,0.227351,0.089859,-0.056188,-0.260658]"
        }
        "Portable tripod stand" => {
            "[-0.186837,0.115800,-0.793012,-0.331453,0.407497,0.102769,0.055952,-0.182230]"
        }
        "Mystery detective novel" => {
            "[0.065804,-0.072932,-0.899088,0.134864,0.237981,0.055288,-0.090989,-0.309568]"
        }
        "Organic breakfast cereal" => {
            "[0.005331,0.270409,-0.717601,-0.041141,-0.154310,-0.156883,-0.426518,-0.424105]"
        }
        "Designer wall paintings" => {
            "[0.223116,0.278920,-0.839319,-0.072784,0.007243,-0.180849,0.262090,-0.247424]"
        }
        "Robot building kit" => {
            "[-0.304641,0.358616,-0.730933,0.231342,0.209916,0.259518,-0.188091,-0.209795]"
        }
        "Sporty tank top" => {
            "[-0.166826,0.219880,-0.857260,0.249994,0.156614,-0.136412,-0.050026,-0.284237]"
        }
        "Bluetooth-enabled speaker" => {
            "[-0.115133,-0.029575,-0.806071,0.025679,0.460650,-0.078361,-0.269488,0.210944]"
        }
        "Winter woolen socks" => {
            "[0.015772,0.242298,-0.913427,0.054438,0.147715,0.017361,-0.221913,-0.179898]"
        }
        "Rustic bookshelf" => {
            "[-0.165415,0.132113,-0.898147,-0.303050,0.147599,-0.115115,-0.147060,0.003475]"
        }
        "Moisturizing lip balm" => {
            "[0.262279,-0.068510,-0.882136,0.164562,0.005592,-0.275725,-0.147090,-0.153558]"
        }
        "Lightweight camera bag" => {
            "[0.001849,-0.009700,-0.883651,0.112157,0.331538,0.202440,0.126173,-0.199159]"
        }
        "Historical fiction book" => {
            "[0.063854,0.086140,-0.861102,-0.193388,0.321135,0.319594,-0.023897,0.061387]"
        }
        "Pure honey jar" => {
            "[0.082462,0.102008,-0.917573,0.182863,0.030252,-0.179037,-0.257788,-0.089396]"
        }
        "Handcrafted wooden frame" => {
            "[0.182882,0.320314,-0.897928,0.002444,0.159097,-0.151881,-0.032191,0.090862]"
        }
        "Plush teddy bear" => {
            "[-0.236903,0.374606,-0.769857,-0.230782,-0.128262,-0.084558,-0.290299,-0.223007]"
        }
        "Warm woolen sweater" => {
            "[-0.145696,0.227357,-0.886308,-0.088362,0.094946,-0.050334,-0.237952,-0.256052]"
        }
        _ => "",
    };
    if !precomputed.is_empty() {
        return precomputed.to_string();
    }

    let mut dims = [0f32; 8];
    for (i, byte) in description.bytes().enumerate() {
        dims[i % 8] += byte as f32;
    }
    let norm = dims.iter().map(|d| d * d).sum::<f32>().sqrt();
    if norm > 0.0 {
        for d in &mut dims {
            *d /= norm;
        }
    }
    let values: Vec<String> = dims.iter().map(|d| format!("{d:.4}")).collect();
    format!("[{}]", values.join(","))
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
    Range<i32>,
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Infinite, RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Inclusive(2), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
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
            Range::new(RangeBound::Inclusive(3), RangeBound::Infinite),
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
            Range::new(RangeBound::Exclusive(1), RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Inclusive(11)),
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
            Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(9)),
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
            Range::new(RangeBound::Infinite, RangeBound::Infinite),
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
            Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(11)),
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
            Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(8)),
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
            Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(9)),
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
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Infinite),
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
            Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(11)),
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
            Range::new(RangeBound::Inclusive(3), RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
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
            Range::new(RangeBound::Exclusive(3), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(8)),
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
            Range::new(RangeBound::Exclusive(4), RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Infinite, RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Infinite),
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
            Range::new(RangeBound::Inclusive(3), RangeBound::Inclusive(11)),
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
            Range::new(RangeBound::Exclusive(4), RangeBound::Exclusive(9)),
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
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(10)),
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
            Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(8)),
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
            Range::new(RangeBound::Exclusive(3), RangeBound::Inclusive(9)),
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
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
            Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(9)),
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
            Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(8)),
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
            Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(9)),
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
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(10)),
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
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(9)),
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
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(10)),
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
