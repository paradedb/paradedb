use pgrx::*;
use std::collections::HashMap;

use crate::sparse_index::sparse::Sparse;

#[pg_extern(immutable, strict, parallel_safe)]
pub fn sparse_cosine_distance(left: Sparse, right: Sparse) -> f32 {
    let mut left_map = HashMap::new();
    let mut right_map = HashMap::new();

    for entry in &left.entries {
        left_map.insert(entry.0, entry.1);
    }
    for entry in &right.entries {
        right_map.insert(entry.0, entry.1);
    }

    let max_length = left.n.max(right.n);

    let mut dot_product: f32 = 0.0;
    let mut left_norm: f32 = 0.0;
    let mut right_norm: f32 = 0.0;

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

    f32::max(
        1.0 - dot_product / (left_norm.sqrt() * right_norm.sqrt()),
        0.0,
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_sparse_cosine_distance_identical() {
        let sparse1 = Sparse {
            entries: vec![(1, 1.0), (2, 2.0), (3, 3.0)],
            n: 3,
        };
        let sparse2 = Sparse {
            entries: vec![(1, 1.0), (2, 2.0), (3, 3.0)],
            n: 3,
        };
        let distance = sparse_cosine_distance(sparse1, sparse2);
        assert_relative_eq!(distance, 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_sparse_cosine_distance_orthogonal() {
        let sparse1 = Sparse {
            entries: vec![(1, 1.0)],
            n: 2,
        };
        let sparse2 = Sparse {
            entries: vec![(2, 1.0)],
            n: 2,
        };
        let distance = sparse_cosine_distance(sparse1, sparse2);
        assert_relative_eq!(distance, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_sparse_cosine_distance_partial_overlap() {
        let sparse1 = Sparse {
            entries: vec![(1, 1.0), (2, 1.0)],
            n: 2,
        };
        let sparse2 = Sparse {
            entries: vec![(2, 1.0), (3, 1.0)],
            n: 3,
        };
        let distance = sparse_cosine_distance(sparse1, sparse2);
        assert!(distance > 0.0 && distance < 1.0);
    }

    #[test]
    fn test_sparse_cosine_distance_no_overlap() {
        let sparse1 = Sparse {
            entries: vec![(1, 1.0)],
            n: 1,
        };
        let sparse2 = Sparse {
            entries: vec![(2, 1.0)],
            n: 2,
        };
        let distance = sparse_cosine_distance(sparse1, sparse2);
        assert_relative_eq!(distance, 1.0, epsilon = 1e-6);
    }
}
