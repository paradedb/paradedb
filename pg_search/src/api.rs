use pgrx::*;
use serde::{Deserialize, Serialize};
use std::f64;
use std::ffi::CStr;
use std::str::FromStr;

#[derive(Copy, Clone, PostgresType, Serialize, Deserialize)]
#[pgvarlena_inoutfuncs]
pub struct L2NormState {
    sum_of_squares: f64,
    count: i32,
}

impl PgVarlenaInOutFuncs for L2NormState {
    fn input(input: &CStr) -> PgVarlena<Self> {
        let mut result = PgVarlena::<Self>::new();
        let input_str = input.to_str().expect("failed to convert CStr to str");

        let mut split = input_str.split(',');
        let sum_of_squares = split
            .next()
            .and_then(|value| f64::from_str(value).ok())
            .expect("expected sum_of_squares to be a valid f64");

        let count = split
            .next()
            .and_then(|value| i32::from_str(value).ok())
            .expect("expected count to be a valid i32");

        result.sum_of_squares = sum_of_squares;
        result.count = count;

        result
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(&format!("{},{}", self.sum_of_squares, self.count));
    }
}

#[pg_aggregate]
impl Aggregate for L2NormState {
    type State = PgVarlena<Self>;
    type Args = f64;
    type Finalize = f64;
    const NAME: &'static str = "l2_norm";
    const INITIAL_CONDITION: Option<&'static str> = Some("0.0,0");

    fn state(
        mut current: Self::State,
        arg: Self::Args,
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> Self::State {
        current.sum_of_squares += arg.powi(2);
        current.count += 1;
        current
    }

    fn finalize(
        current: Self::State,
        _args: (),
        _fcinfo: pg_sys::FunctionCallInfo,
    ) -> Self::Finalize {
        (current.sum_of_squares / (current.count as f64)).sqrt()
    }
}

#[pg_extern]
pub fn weighted_mean(a: Option<f64>, b: Option<f64>, weights: Vec<Option<f64>>) -> f64 {
    assert!(weights.len() == 2, "There must be exactly 2 weights");

    let weight_a = weights[0].unwrap_or(0.0);
    let weight_b = weights[1].unwrap_or(0.0);

    assert!(
        (0.0..=1.0).contains(&weight_a) && (0.0..=1.0).contains(&weight_b),
        "Weights must be between 0 and 1"
    );

    assert!(
        (weight_a + weight_b - 1.0).abs() < std::f64::EPSILON,
        "Weights must add up to 1"
    );

    let a_val = a.unwrap_or(0.0);
    let b_val = b.unwrap_or(0.0);

    a_val * weight_a + b_val * weight_b
}
