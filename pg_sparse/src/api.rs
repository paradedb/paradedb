use pgrx::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PostgresType, Deserialize, Serialize)]
pub struct SparseEntry {
    position: i32,
    value: f64,
}

#[derive(PostgresType, Serialize, Deserialize)]
pub struct Sparse {
    entries: Vec<SparseEntry>,
    length: i32,
}

#[pg_extern(immutable, parallel_safe)]
pub fn sparse_cosine_distance(left: Sparse, right: Sparse) -> f64 {
    let mut left_map = HashMap::new();
    let mut right_map = HashMap::new();

    for entry in &left.entries {
        left_map.insert(entry.position, entry.value);
    }
    for entry in &right.entries {
        right_map.insert(entry.position, entry.value);
    }

    let max_length = left.length.max(right.length);

    let mut dot_product: f64 = 0.0;
    let mut left_norm: f64 = 0.0;
    let mut right_norm: f64 = 0.0;

    for position in 0..max_length {
        let left_value = *left_map.get(&{ position }).unwrap_or(&0.0);
        let right_value = *right_map.get(&{ position }).unwrap_or(&0.0);

        dot_product += left_value * right_value;
        left_norm += left_value.powi(2);
        right_norm += right_value.powi(2);
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        return -1.0;
    }

    dot_product / (left_norm.sqrt() * right_norm.sqrt())
}

extension_sql!(
    r#"
CREATE OPERATOR <==> (
    LEFTARG = sparse, RIGHTARG = sparse, PROCEDURE = sparse_cosine_distance,
    COMMUTATOR = '<==>'
);
"#,
    name = "sparse_operator"
);
