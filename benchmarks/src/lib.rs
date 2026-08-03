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

/// Returns the `p`th percentile (0.0..=1.0) of `data`, by linear interpolation between the two
/// closest ranks.
///
/// Interpolating rather than picking a nearest rank matters at the sample counts here: with one
/// timing per held-out query vector, a nearest-rank p95 of 100 samples is just the 95th value,
/// which moves in visible jumps between runs.
pub fn percentile(data: &[f64], p: f64) -> f64 {
    assert!(!data.is_empty());
    assert!((0.0..=1.0).contains(&p));

    let mut sorted = data.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return sorted[0];
    }

    let rank = p * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    sorted[lo] + (sorted[hi] - sorted[lo]) * (rank - lo as f64)
}

/// Returns the `[lo, hi]` order-statistic confidence interval for the `p`th percentile of `data`.
///
/// A mean's t-interval describes the mean, not a quantile, so [`confidence_interval_half_width`]
/// cannot be reused here. This is the distribution-free alternative: the bounds are the samples at
/// ranks `np ± z*sqrt(np(1-p))`, clamped to the data.
///
/// The interval is asymmetric about the percentile, and wide in the tail: with one timing per
/// held-out query vector (~100 samples), p99 is bounded by only the few slowest vectors.
pub fn percentile_confidence_interval(data: &[f64], p: f64, confidence_level: f64) -> (f64, f64) {
    assert!(!data.is_empty());
    assert!((0.0..=1.0).contains(&p));
    assert!(confidence_level > 0.0);
    assert!(confidence_level < 1.0);

    let mut sorted = data.to_vec();
    sorted.sort_by(f64::total_cmp);

    let n = sorted.len() as f64;
    let half_alpha = (1.0 - confidence_level) / 2.0;
    let z = distrs::Normal::ppf(1.0 - half_alpha, 0.0, 1.0);
    let spread = z * (n * p * (1.0 - p)).sqrt();

    // The ranks the formula produces are 1-based, hence the shift onto a 0-based index.
    let index = |rank: f64| (rank.max(1.0).min(n) as usize) - 1;
    let lo = index((n * p - spread).floor());
    let hi = index((n * p + spread).ceil());

    (sorted[lo], sorted[hi])
}

/// FNV-1a over the strings in order. Stable across platforms and Rust versions, unlike
/// `DefaultHasher`, so hashes published in one benchmark run stay comparable in later ones.
pub fn fnv1a(strings: &[String]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for s in strings {
        for byte in s.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
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

#[cfg(test)]
mod percentile_tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn bounds_are_min_and_max() {
        let d = [5.0, 1.0, 3.0, 2.0, 4.0];
        assert_relative_eq!(percentile(&d, 0.0), 1.0);
        assert_relative_eq!(percentile(&d, 1.0), 5.0);
    }

    #[test]
    fn median_interpolates_for_even_counts() {
        assert_relative_eq!(percentile(&[1.0, 2.0, 3.0, 4.0], 0.5), 2.5);
        assert_relative_eq!(percentile(&[1.0, 2.0, 3.0], 0.5), 2.0);
    }

    #[test]
    fn input_order_does_not_matter() {
        let asc: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let desc: Vec<f64> = asc.iter().rev().copied().collect();
        for p in [0.5, 0.9, 0.95, 0.99] {
            assert_relative_eq!(percentile(&asc, p), percentile(&desc, p));
        }
    }

    #[test]
    fn matches_known_ranks_for_1_to_100() {
        // With 100 samples the interpolated rank is p*(n-1), so p50 sits between 50 and 51.
        let d: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        assert_relative_eq!(percentile(&d, 0.5), 50.5);
        assert_relative_eq!(percentile(&d, 0.9), 90.1);
        assert_relative_eq!(percentile(&d, 0.99), 99.01);
    }

    #[test]
    fn single_sample_is_its_own_percentile() {
        for p in [0.0, 0.5, 0.99, 1.0] {
            assert_relative_eq!(percentile(&[42.0], p), 42.0);
        }
    }

    /// Values are their own 1-based rank, so a bound reads back as the rank it selected.
    fn ranks(n: usize, p: f64) -> (usize, usize) {
        let d: Vec<f64> = (1..=n).map(|i| i as f64).collect();
        let (lo, hi) = percentile_confidence_interval(&d, p, 0.95);
        (lo as usize, hi as usize)
    }

    #[test]
    fn median_interval_matches_published_ranks() {
        // The textbook 95% interval for the median of 100 samples is the 40th to 60th value.
        assert_eq!(ranks(100, 0.5), (40, 60));
    }

    #[test]
    fn interval_brackets_the_percentile() {
        let d: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        for p in [0.5, 0.95, 0.99] {
            let (lo, hi) = percentile_confidence_interval(&d, p, 0.95);
            let point = percentile(&d, p);
            assert!(
                lo <= point && point <= hi,
                "p{p} {point} outside [{lo}, {hi}]"
            );
        }
    }

    #[test]
    fn tail_intervals_are_narrower_in_rank_but_clamp_at_the_top() {
        // p99 of 100 samples is bounded by only the slowest few, and cannot reach past the max.
        let (lo, hi) = ranks(100, 0.99);
        assert_eq!(hi, 100);
        assert!(hi - lo <= 4, "p99 spanned {} ranks", hi - lo);
        // A wider confidence level can only widen the interval.
        let d: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let (lo99, _) = percentile_confidence_interval(&d, 0.5, 0.99);
        let (lo95, _) = percentile_confidence_interval(&d, 0.5, 0.95);
        assert!(lo99 <= lo95);
    }

    #[test]
    fn small_and_degenerate_inputs_stay_in_bounds() {
        for n in 1..=5 {
            let d: Vec<f64> = (1..=n).map(|i| i as f64).collect();
            for p in [0.0, 0.5, 0.95, 0.99, 1.0] {
                let (lo, hi) = percentile_confidence_interval(&d, p, 0.95);
                assert!(lo >= 1.0 && hi <= n as f64 && lo <= hi, "n={n} p={p}");
            }
        }
    }
}
