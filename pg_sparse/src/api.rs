use pgrx::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(PostgresType, Serialize, Deserialize)]
pub struct Sparse {
    // Each entry is a tuple of (position, value), representing the position and value of a non-zero element
    entries: Vec<(i32, f64)>,
    // n is the length of the sparse vector
    n: i32,
}

#[pg_extern]
pub fn to_sparse(input_vector: Array<f64>) -> Sparse {
    let compressed: Vec<(i32, f64)> = input_vector
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            if let Some(v) = value {
                if v != 0.0 {
                    Some(((index + 1) as i32, v))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    Sparse {
        entries: compressed,
        n: input_vector.len() as i32,
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn sparse_cosine_distance(left: Sparse, right: Sparse) -> f64 {
    let mut left_map = HashMap::new();
    let mut right_map = HashMap::new();

    for entry in &left.entries {
        left_map.insert(entry.0, entry.1);
    }
    for entry in &right.entries {
        right_map.insert(entry.0, entry.1);
    }

    let max_length = left.n.max(right.n);

    let mut dot_product: f64 = 0.0;
    let mut left_norm: f64 = 0.0;
    let mut right_norm: f64 = 0.0;

    for position in 0..max_length {
        let left_value = *left_map.get(&(position + 1)).unwrap_or(&0.0);
        let right_value = *right_map.get(&(position + 1)).unwrap_or(&0.0);

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
