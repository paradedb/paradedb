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

pub fn median<'a>(data: impl Iterator<Item = &'a f64>) -> f64 {
    let mut sorted = data.copied().collect::<Vec<_>>();
    sorted.sort_unstable_by(|a, b| a.total_cmp(b));
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

pub struct QueryRunResults {
    pub cold: f64,
    pub samples: [f64; 10],
    pub num_results: usize,
}

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

    pub fn push(mut self, el: f64) {
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
            (Some(min), Some(max)) => Some(max / min),
            (None, None) => None,
            _ => unreachable!("min and max should either both be available or neither should"),
        }
    }
}
