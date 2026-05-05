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
    pub fn percent_variance(&mut self) -> Option<f64> {
        if self.contents.is_empty() {
            None
        } else {
            self.contents.make_contiguous();
            let contents = self.contents.as_slices().0;
            let pct_variance = variance(contents) / mean(contents);
            Some(pct_variance)
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
/// https://dl.acm.org/doi/10.1145/2555670.2464160.
///
///
pub fn confidence_interval(data: &[f64], confidence_level: f64) -> f64 {
    assert!(!data.is_empty());
    assert!(confidence_level > 0.0);
    assert!(confidence_level < 1.0);

    let reps = data.len() as f64;
    let variance = variance(data);

    let alpha = (1.0 - confidence_level) / 2.0;
    let t = distrs::StudentsT::ppf(alpha, reps - 1.0);

    t * ((variance / reps).sqrt())
}

/// Returns the variance of the slice contents
fn variance(data: &[f64]) -> f64 {
    assert!(!data.is_empty());

    let mean: f64 = mean(data);
    let reps = data.len() as f64;

    (1.0 / (reps - 1.0)) * (data.iter().map(|v| (v - mean).powi(2)).sum::<f64>())
}
