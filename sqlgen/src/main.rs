mod joingen;
mod sqlgen;

use crate::joingen::JoinGenerator;
use crate::sqlgen::QueryGenerator;
use std::collections::HashMap;
use std::str::FromStr;

pub fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let take = usize::from_str(&args[1]).expect("argument must be a number");

    let users_gen = QueryGenerator::new(
        "=",
        vec![
            ("users.name", "bob"),
            ("users.color", "blue"),
            ("users.age", "20"),
        ],
    );

    let orders_gen = QueryGenerator::new(
        "=",
        vec![
            ("orders.name", "bob"),
            ("orders.color", "blue"),
            ("orders.age", "20"),
        ],
    );

    let products_gen = QueryGenerator::new(
        "=",
        vec![
            ("products.name", "bob"),
            ("products.color", "blue"),
            ("products.age", "20"),
        ],
    );

    let mut generators = HashMap::<&str, QueryGenerator<&str>>::default();
    generators.insert("users", users_gen);
    generators.insert("orders", orders_gen);
    generators.insert("products", products_gen);

    let jgen = JoinGenerator::new(vec![
        ("users", vec!["name", "color", "age"]),
        ("orders", vec!["name", "color", "age"]),
        ("products", vec!["name", "color", "age"]),
    ])
    .take(take);

    for (join_clause, used_tables) in jgen {
        let sql = format!("SELECT COUNT(*) {join_clause} WHERE ");

        let mut where_clauses = Vec::with_capacity(used_tables.len() * 1);
        for table_name in used_tables {
            where_clauses.extend(generators.get_mut(table_name.as_str()).unwrap().take(1));
        }

        println!("{sql} {}", where_clauses.join(" AND "));
    }
}
