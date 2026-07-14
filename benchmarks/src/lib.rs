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

use std::collections::VecDeque;

pub struct Window {
    capacity: usize,
    contents: VecDeque<f64>,
}
impl Window {
    pub fn new(capacity: usize) -> Self {
        Window {
            capacity,
            contents: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, el: f64) {
        if self.contents.len() == self.capacity {
            self.contents.pop_front();
        }
        self.contents.push_back(el);
    }

    /// Returns the variance of the window contents as a percent of the mean
    /// Requires self to be mutable so that self.contents can be made contiguous
    pub fn variance_over_mean(&mut self) -> Option<f64> {
        if self.contents.is_empty() {
            None
        } else {
            self.contents.make_contiguous();
            let contents = self.contents.as_slices().0;
            let var_over_mean = variance(contents) / mean(contents);
            Some(var_over_mean)
        }
    }

    pub fn is_full(&self) -> bool {
        self.contents.len() == self.capacity
    }
}

pub fn mean(data: &[f64]) -> f64 {
    assert!(!data.is_empty());
    data.iter().sum::<f64>() / data.len() as f64
}

/// Returns the half-width of the confidence interval for the provided confidence level
///
/// The math for this comes from single-level version of what is shown in this paper:
/// <https://dl.acm.org/doi/10.1145/2555670.2464160>.
pub fn confidence_interval_half_width(data: &[f64], confidence_level: f64) -> f64 {
    assert!(!data.is_empty());
    assert!(confidence_level > 0.0);
    assert!(confidence_level < 1.0);

    let sample_count = data.len() as f64;
    let variance = variance(data);

    let half_alpha = (1.0 - confidence_level) / 2.0;
    let t = distrs::StudentsT::ppf(1.0 - half_alpha, sample_count - 1.0);

    t * ((variance / sample_count).sqrt())
}

/// Returns the variance of the slice contents
fn variance(data: &[f64]) -> f64 {
    assert!(!data.is_empty());

    let mean: f64 = mean(data);
    let sample_count = data.len() as f64;

    (1.0 / (sample_count - 1.0)) * (data.iter().map(|v| (v - mean).powi(2)).sum::<f64>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    use approx::assert_relative_eq;

    // Mavro and lottery datasets, along with their expected variance and half-widths
    // are taken from NIST reference datasets

    const DATA_MAVRO: [f64; 50] = [
        2.00180, 2.00170, 2.00180, 2.00190, 2.00180, 2.00170, 2.00150, 2.00140, 2.00150, 2.00150,
        2.00170, 2.00180, 2.00180, 2.00190, 2.00190, 2.00210, 2.00200, 2.00160, 2.00140, 2.00130,
        2.00130, 2.00150, 2.00150, 2.00160, 2.00150, 2.00140, 2.00130, 2.00140, 2.00150, 2.00140,
        2.00150, 2.00160, 2.00150, 2.00160, 2.00190, 2.00200, 2.00200, 2.00210, 2.00220, 2.00230,
        2.00240, 2.00250, 2.00270, 2.00260, 2.00260, 2.00260, 2.00270, 2.00260, 2.00250, 2.00240,
    ];
    const VARIANCE_MAVRO: f64 = 1.8414693877551e-07;
    const HALF_WIDTH_95_MAVRO: f64 = 1.21955536247e-04;

    const DATA_LOTTERY: [f64; 218] = [
        162.0, 671.0, 933.0, 414.0, 788.0, 730.0, 817.0, 33.0, 536.0, 875.0, 670.0, 236.0, 473.0,
        167.0, 877.0, 980.0, 316.0, 950.0, 456.0, 92.0, 517.0, 557.0, 956.0, 954.0, 104.0, 178.0,
        794.0, 278.0, 147.0, 773.0, 437.0, 435.0, 502.0, 610.0, 582.0, 780.0, 689.0, 562.0, 964.0,
        791.0, 28.0, 97.0, 848.0, 281.0, 858.0, 538.0, 660.0, 972.0, 671.0, 613.0, 867.0, 448.0,
        738.0, 966.0, 139.0, 636.0, 847.0, 659.0, 754.0, 243.0, 122.0, 455.0, 195.0, 968.0, 793.0,
        59.0, 730.0, 361.0, 574.0, 522.0, 97.0, 762.0, 431.0, 158.0, 429.0, 414.0, 22.0, 629.0,
        788.0, 999.0, 187.0, 215.0, 810.0, 782.0, 47.0, 34.0, 108.0, 986.0, 25.0, 644.0, 829.0,
        630.0, 315.0, 567.0, 919.0, 331.0, 207.0, 412.0, 242.0, 607.0, 668.0, 944.0, 749.0, 168.0,
        864.0, 442.0, 533.0, 805.0, 372.0, 63.0, 458.0, 777.0, 416.0, 340.0, 436.0, 140.0, 919.0,
        350.0, 510.0, 572.0, 905.0, 900.0, 85.0, 389.0, 473.0, 758.0, 444.0, 169.0, 625.0, 692.0,
        140.0, 897.0, 672.0, 288.0, 312.0, 860.0, 724.0, 226.0, 884.0, 508.0, 976.0, 741.0, 476.0,
        417.0, 831.0, 15.0, 318.0, 432.0, 241.0, 114.0, 799.0, 955.0, 833.0, 358.0, 935.0, 146.0,
        630.0, 830.0, 440.0, 642.0, 356.0, 373.0, 271.0, 715.0, 367.0, 393.0, 190.0, 669.0, 8.0,
        861.0, 108.0, 795.0, 269.0, 590.0, 326.0, 866.0, 64.0, 523.0, 862.0, 840.0, 219.0, 382.0,
        998.0, 4.0, 628.0, 305.0, 747.0, 247.0, 34.0, 747.0, 729.0, 645.0, 856.0, 974.0, 24.0,
        568.0, 24.0, 694.0, 608.0, 480.0, 410.0, 729.0, 947.0, 293.0, 53.0, 930.0, 223.0, 203.0,
        677.0, 227.0, 62.0, 455.0, 387.0, 318.0, 562.0, 242.0, 428.0, 968.0,
    ];
    const VARIANCE_LOTTERY: f64 = 85088.7310066376;
    const HALF_WIDTH_95_LOTTERY: f64 = 38.93899800733134;

    #[rstest]
    #[case(&DATA_MAVRO, VARIANCE_MAVRO)]
    #[case(&DATA_LOTTERY, VARIANCE_LOTTERY)]
    fn test_variance(#[case] input: &[f64], #[case] expected: f64) {
        assert_relative_eq!(variance(input), expected);
    }

    #[rstest]
    #[case(&DATA_MAVRO, HALF_WIDTH_95_MAVRO)]
    #[case(&DATA_LOTTERY, HALF_WIDTH_95_LOTTERY)]
    fn test_confidence_interval_half_width(#[case] input: &[f64], #[case] expected: f64) {
        assert_relative_eq!(confidence_interval_half_width(input, 0.95), expected);
    }

    // Inputs for case 2 & 3 are taken from actual benchmark samples. 2 is "warm"; 3 is "cold".
    // Case 1 is just hard-coded values to get a sense of expected values
    #[rstest]
    #[case(&[1.9, 2.0, 2.1], 0.005000000000000009)]
    #[case(&[7.642748, 7.543157000000001, 7.5808230000000005], 0.0003332011622838335)]
    #[case(&[13.000181000000001, 7.735271, 7.588471], 1.0067008138703109)]
    fn test_window_variance_over_mean(#[case] inputs: &[f64], #[case] expected: f64) {
        let mut window = Window::new(3);

        assert_eq!(window.variance_over_mean(), None);
        for i in inputs {
            window.push(*i);
        }
        assert!(window.is_full());
        assert_relative_eq!(window.variance_over_mean().unwrap(), expected);
    }
}
