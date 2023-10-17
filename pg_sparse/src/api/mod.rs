use pgrx::*;
use std::collections::HashMap;

use crate::sparse_index::Sparse;

#[pg_extern(immutable, strict, parallel_safe)]
pub fn sparse_cosine_distance(left: Sparse, right: Sparse) -> f32 {
    let mut left_map = HashMap::new();
    let mut right_map = HashMap::new();

    for entry in &left.entries {
        left_map.insert(entry.0, entry.1 as f32);
    }
    for entry in &right.entries {
        right_map.insert(entry.0, entry.1 as f32);
    }

    let max_length = left.n.max(right.n);

    let mut dot_product: f32 = 0.0;
    let mut left_norm: f32 = 0.0;
    let mut right_norm: f32 = 0.0;

    for position in 0..max_length {
        let left_value = *left_map.get(&(position + 1)).unwrap_or(&0.0) as f32;
        let right_value = *right_map.get(&(position + 1)).unwrap_or(&0.0) as f32;

        dot_product += left_value * right_value;
        left_norm += left_value.powi(2);
        right_norm += right_value.powi(2);
    }

    if left_norm == 0.0 || right_norm == 0.0 {
        return -1.0;
    }

    (dot_product / (left_norm.sqrt() * right_norm.sqrt())) as f32
}

extension_sql!(
    r#"
CREATE OPERATOR <==> (
    LEFTARG = sparse, RIGHTARG = sparse, PROCEDURE = sparse_cosine_distance,
    COMMUTATOR = '<==>'
);

CREATE OPERATOR CLASS sparse_cosine_ops 
    DEFAULT FOR TYPE sparse USING sparse_hnsw AS
    OPERATOR 1 <==> (sparse, sparse) FOR ORDER BY float_ops,
    FUNCTION 1 sparse_cosine_distance(sparse, sparse);
"#,
    name = "sparse_operator"
);

// #[pg_extern(immutable, strict, parallel_safe)]
// pub fn cosine_distance(left: Vec<f32>, right: Vec<f32>) -> f32 {
//     1.0 as f32
// }

// extension_sql!(
//     r#"
// CREATE OPERATOR <=> (
//     LEFTARG = real[], RIGHTARG = real[], PROCEDURE = cosine_distance,
//     COMMUTATOR = '<=>'
// );

// CREATE OPERATOR CLASS ann_cos_ops 
//     DEFAULT FOR TYPE real[] USING sparse_hnsw AS
// 	OPERATOR 1 <=> (real[], real[]) FOR ORDER BY float_ops,
// 	FUNCTION 1 cosine_distance(real[], real[]);
// "#,
//     name = "sparse_operator"
// );
