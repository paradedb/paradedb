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
    size: usize,
    contents: VecDeque<f64>,
}
impl Window {
    pub fn new(size: usize) -> Self {
        Window {
            size,
            contents: VecDeque::with_capacity(size),
        }
    }

    pub fn push(&mut self, el: f64) {
        if self.contents.len() == self.size {
            self.contents.pop_front();
        }
        self.contents.push_back(el);
    }

    pub fn min(&self) -> Option<f64> {
        if self.contents.is_empty() {
            None
        } else {
            let res = self.contents.iter().fold(f64::MAX, |acc, el| acc.min(*el));
            Some(res)
        }
    }

    pub fn max(&self) -> Option<f64> {
        if self.contents.is_empty() {
            None
        } else {
            let res = self.contents.iter().fold(f64::MIN, |acc, el| acc.max(*el));
            Some(res)
        }
    }

    pub fn variance(&self) -> Option<f64> {
        match (self.min(), self.max()) {
            (Some(min), Some(max)) => Some((max / min) - 1.0),
            (None, None) => None,
            _ => unreachable!("min and max should either both be available or neither should"),
        }
    }

    pub fn is_full(&self) -> bool {
        self.contents.len() == self.size
    }
}

pub fn mean(data: &[f64]) -> f64 {
    assert!(!data.is_empty());
    data.iter().sum::<f64>() / data.len() as f64
}

pub fn single_level_confidence_interval(data: &[f64], confidence_level: f64) -> f64 {
    assert!(!data.is_empty());
    assert!(confidence_level > 0.0);
    assert!(confidence_level < 1.0);

    let reps = data.len() as f64;
    let variance = single_level_variance(data);

    let alpha = (1.0 - confidence_level) / 2.0;
    let t = distrs::StudentsT::ppf(alpha, reps - 1.0);

    t * ((variance / reps).sqrt())
}

fn single_level_variance(data: &[f64]) -> f64 {
    assert!(!data.is_empty());

    let mean: f64 = mean(data);
    let reps = data.len() as f64;

    let variance: f64 =
        (1.0 / (reps - 1.0)) * (data.iter().map(|v| (v - mean).powi(2)).sum::<f64>());
    variance
}
