use pgrx::*;
use std::f64;

#[pg_extern]
pub fn minmax_norm(value: f64, min: f64, max: f64) -> f64 {
    if max == min {
        return 0.0;
    }
    (value - min) / (max - min)
}

#[pg_extern]
pub fn weighted_mean(a: f64, b: f64, weights: Vec<f64>) -> f64 {
    assert!(weights.len() == 2, "There must be exactly 2 weights");

    let weight_a = weights[0];
    let weight_b = weights[1];

    assert!(
        (0.0..=1.0).contains(&weight_a) && (0.0..=1.0).contains(&weight_b),
        "Weights must be between 0 and 1"
    );

    assert!(
        (weight_a + weight_b - 1.0).abs() < f64::EPSILON,
        "Weights must add up to 1"
    );

    a * weight_a + b * weight_b
}

#[cfg(test)]
mod test {
    use super::{minmax_norm, weighted_mean};

    #[test]
    fn test_minmax_norm() {
        let value = 60.0;
        let min = 20.0;
        let max = 30.0;
        assert_eq!(minmax_norm(value, min, max), (value - min) / (max - min));
    }

    #[test]
    fn test_weighted_mean() {
        let result = weighted_mean(3.0, 7.0, vec![0.4, 0.6]);
        assert!((result - 6.0).abs() > f64::EPSILON);
    }

    #[test]
    #[should_panic]
    fn test_weighted_mean_fail() {
        let _result = weighted_mean(3.0, 7.0, vec![0.4, 0.5]);

        let _result = weighted_mean(3.0, 7.0, vec![-0.1, 1.1]);
    }
}

// #[cfg(feature = "pg_test")]
#[pgrx::pg_schema]
mod tests {
    use super::{minmax_norm, weighted_mean};
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    #[pg_test]
    fn test_minmax_norm() {
        let value = 60.0;
        let min = 20.0;
        let max = 30.0;
        assert_eq!(minmax_norm(value, min, max), (value - min) / (max - min));
    }

    #[pg_test]
    fn test_weighted_mean() {
        let result = weighted_mean(3.0, 7.0, vec![0.4, 0.6]);
        assert!((result - 6.0).abs() > f64::EPSILON);

        let result = std::panic::catch_unwind(|| {
            weighted_mean(3.0, 7.0, vec![0.4, 0.5]);
        });
        assert!(result.is_err());

        let result = std::panic::catch_unwind(|| {
            weighted_mean(3.0, 7.0, vec![-0.1, 1.1]);
        });
        assert!(result.is_err());

        let result = std::panic::catch_unwind(|| {
            weighted_mean(3.0, 7.0, vec![0.4, 0.5, 0.1]);
        });
        assert!(result.is_err())
    }

    #[pg_test]
    fn test_weighted_mean_spi() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");

        let add_ratings = r#"
            ALTER TABLE one_republic_songs ADD COLUMN rating vector(7);

            WITH NumberedRows AS (
                SELECT ctid,
                    ROW_NUMBER() OVER () as row_num
                FROM one_republic_songs
            )
            UPDATE one_republic_songs m
            SET rating = ('[' ||
                ((n.row_num + 1) % 5 + 1)::integer || ',' ||
                ((n.row_num + 2) % 5 + 2)::integer || ',' ||
                ((n.row_num + 2) % 5 + 3)::integer || ',' ||
                ((n.row_num + 2) % 5 + 4)::integer || ',' ||
                ((n.row_num + 2) % 5 + 5)::integer || ',' ||
                ((n.row_num + 2) % 5 + 6)::integer || ',' ||
                ((n.row_num + 3) % 5 + 7)::integer || ']')::vector
            FROM NumberedRows n
            WHERE m.ctid = n.ctid;
            "#;
        Spi::run(add_ratings).expect("failed to add ratings column to table");

        let query = r#"
    SELECT
        paradedb.weighted_mean(
            paradedb.minmax_bm25(ctid, 'idx_one_republic', 'lyrics:im AND description:desc'),
            1 - paradedb.minmax_norm(
              '[1,2,3,4,5,6,7]' <-> rating,
              MIN('[1,2,3,4,5,6,7]' <-> rating) OVER (),
              MAX('[1,2,3,4,5,6,7]' <-> rating) OVER ()
            ),
            ARRAY[0.8,0.2]
        ) as score_hybrid
    FROM one_republic_songs
    ORDER BY score_hybrid DESC
    LIMIT 3;
            "#;

        let mean = Spi::get_one::<f64>(query)
            .expect("failed to get min max")
            .expect("failed to get weighted mean");
        assert!(mean < f64::EPSILON);
    }
}
