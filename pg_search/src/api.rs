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
        (weight_a + weight_b - 1.0).abs() < std::f64::EPSILON,
        "Weights must add up to 1"
    );

    a * weight_a + b * weight_b
}
