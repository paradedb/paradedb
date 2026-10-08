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

use std::sync::Arc;

use parking_lot::Mutex;
use tantivy::aggregation::DistributedAggregationCollector;
use tantivy::aggregation::intermediate_agg_result::{
    IntermediateAggregationResult, IntermediateAggregationResults, IntermediateBucketResult,
    IntermediateMetricResult,
};
use tantivy::aggregation::metric::{IntermediateCount, IntermediateStats};
use tantivy::collector::{Collector, SegmentCollector};
use tantivy::query::Weight;
use tantivy::{SegmentOrdinal, SegmentReader};

use crate::postgres::customscan::aggregatescan::build::{AggregationKey, DocCountKey};
use crate::postgres::heap::VisibilityChecker;

use super::interrupt_collector::InterruptableCollector;
use super::mvcc_collector::MVCCFilterCollector;

/// Uses query counts for fully visible segments of a bare COUNT(*).
pub struct CountAllCollector {
    inner: InterruptableCollector<MVCCFilterCollector<DistributedAggregationCollector>>,
    checker: Arc<Mutex<VisibilityChecker>>,
    matches_all: bool,
    include_doc_count: bool,
}

impl CountAllCollector {
    pub fn new(
        inner: DistributedAggregationCollector,
        checker: VisibilityChecker,
        matches_all: bool,
        include_doc_count: bool,
    ) -> Self {
        let inner = MVCCFilterCollector::new(inner, checker);
        Self {
            checker: inner.lock.clone(),
            inner: InterruptableCollector::new(inner),
            matches_all,
            include_doc_count,
        }
    }
}

unsafe impl Send for CountAllCollector {}
unsafe impl Sync for CountAllCollector {}

impl Collector for CountAllCollector {
    type Fruit = IntermediateAggregationResults;
    type Child = <InterruptableCollector<MVCCFilterCollector<DistributedAggregationCollector>> as Collector>::Child;

    fn for_segment(
        &self,
        ord: SegmentOrdinal,
        segment: &SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        self.inner.for_segment(ord, segment)
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(
        &self,
        fruits: Vec<<Self::Child as SegmentCollector>::Fruit>,
    ) -> tantivy::Result<Self::Fruit> {
        self.inner.merge_fruits(fruits)
    }

    fn collect_segment(
        &self,
        weight: &dyn Weight,
        ord: SegmentOrdinal,
        segment: &SegmentReader,
    ) -> tantivy::Result<<Self::Child as SegmentCollector>::Fruit> {
        pgrx::check_for_interrupts!();
        if VisibilityChecker::for_segment_arc(&self.checker, ord)?.is_none() {
            let doc_count = if self.matches_all {
                segment.num_docs()
            } else {
                weight.count(segment)?
            };
            let mut result = IntermediateAggregationResults::default();
            result.push(
                "0".to_string(),
                IntermediateAggregationResult::Bucket(IntermediateBucketResult::Filter {
                    doc_count: u64::from(doc_count),
                    sub_aggregations: IntermediateAggregationResults::default(),
                }),
            )?;
            if self.include_doc_count {
                result.push(
                    DocCountKey::NAME.to_string(),
                    IntermediateAggregationResult::Metric(IntermediateMetricResult::Count(
                        IntermediateCount::from_stats(IntermediateStats::from_parts(
                            u64::from(doc_count),
                            0.0,
                            0.0,
                            0.0,
                        )),
                    )),
                )?;
            }
            return Ok(Ok(result));
        }
        self.inner.collect_segment(weight, ord, segment)
    }
}
