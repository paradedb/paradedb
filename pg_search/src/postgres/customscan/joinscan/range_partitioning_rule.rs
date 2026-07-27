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

use std::collections::HashSet;
use std::sync::Arc;

use datafusion::catalog::default_table_source::DefaultTableSource;
use datafusion::common::tree_node::{Transformed, TreeNode};
use datafusion::common::{Column, DataFusionError, Result};
use datafusion::logical_expr::{Expr, LogicalPlan, TableScan};
use datafusion::optimizer::{optimizer::ApplyOrder, OptimizerConfig, OptimizerRule};
use rand::seq::SliceRandom;
use tantivy::SegmentReader;

use crate::api::FieldName;
use crate::index::fast_fields_helper::FFType;
use crate::index::fast_fields_helper::WhichFastField;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;

use crate::scan::late_materialization::trace_column;
use crate::scan::range_partitioning::RangePartitioningSample;
use crate::scan::table_provider::PgSearchTableProvider;

/// Optimizer rule that injects range partitioning boundaries into `PgSearchTableProvider`
/// when it participates in a join on partitioned columns.
#[derive(Debug, Default)]
pub struct RangePartitioningRule;

impl RangePartitioningRule {
    pub fn new() -> Self {
        Self
    }
}

fn pg_search_provider_from_scan(scan: &TableScan) -> Option<&PgSearchTableProvider> {
    let source = scan.source.as_ref();
    if let Some(default_source) = source.downcast_ref::<DefaultTableSource>() {
        default_source
            .table_provider
            .downcast_ref::<PgSearchTableProvider>()
    } else {
        (source as &dyn std::any::Any).downcast_ref::<PgSearchTableProvider>()
    }
}

impl OptimizerRule for RangePartitioningRule {
    fn name(&self) -> &str {
        "RangePartitioningRule"
    }

    fn apply_order(&self) -> Option<ApplyOrder> {
        None
    }

    fn rewrite(
        &self,
        plan: LogicalPlan,
        _config: &dyn OptimizerConfig,
    ) -> Result<Transformed<LogicalPlan>> {
        let mut base_join_keys = HashSet::new();

        plan.apply(|node| {
            if let LogicalPlan::Join(join) = node {
                for (l, r) in &join.on {
                    if let Expr::Column(c) = l {
                        if let Some(base_col) = trace_column(node, c) {
                            base_join_keys.insert(base_col);
                        }
                    }
                    if let Expr::Column(c) = r {
                        if let Some(base_col) = trace_column(node, c) {
                            base_join_keys.insert(base_col);
                        }
                    }
                }
            }
            Ok(datafusion::common::tree_node::TreeNodeRecursion::Continue)
        })?;

        if base_join_keys.is_empty() {
            return Ok(Transformed::no(plan));
        }

        plan.transform_up(|node| {
            if let LogicalPlan::TableScan(mut scan) = node {
                if let Some(provider) = pg_search_provider_from_scan(&scan) {
                    let mut matched_partition_by = None;

                    for field in &provider.scan_info.partition_by {
                        let col = Column::new_unqualified(field.as_ref());
                        if base_join_keys.contains(&col) {
                            matched_partition_by = Some(field.clone());
                            break;
                        }
                    }

                    if let Some(partition_by) = matched_partition_by {
                        let sample = sample_fast_field(provider, &partition_by)?;
                        let mut new_provider = provider.clone();
                        new_provider.with_range_partitioning(Some(sample));

                        let new_source = if scan
                            .source
                            .as_ref()
                            .downcast_ref::<DefaultTableSource>()
                            .is_some()
                        {
                            Arc::new(DefaultTableSource::new(Arc::new(new_provider)))
                                as Arc<dyn datafusion::logical_expr::TableSource>
                        } else {
                            return Err(DataFusionError::Internal(
                                "PgSearchTableProvider was not wrapped in a DefaultTableSource"
                                    .to_string(),
                            ));
                        };

                        scan.source = new_source;
                        return Ok(Transformed::yes(LogicalPlan::TableScan(scan)));
                    }
                }
                Ok(Transformed::no(LogicalPlan::TableScan(scan)))
            } else {
                Ok(Transformed::no(node))
            }
        })
    }
}

fn sample_fast_field(
    provider: &PgSearchTableProvider,
    partition_by: &FieldName,
) -> Result<RangePartitioningSample> {
    let index_rel = PgSearchRelation::open(provider.scan_info.indexrelid);
    // TODO: Reading the index during planning adds latency to DataFusion logical planning.
    // This is a temporary situation for M1. In M2, we will migrate this sampling to
    // something that happens prior to CREATE INDEX, or switch to using pg_statistic.
    let reader = SearchIndexReader::open(
        &index_rel,
        SearchQueryInput::All,
        false,
        MvccSatisfies::LargestSegment,
    )
    .map_err(|e| DataFusionError::Internal(format!("Failed to open index for sampling: {e}")))?;

    let segment_readers = reader.segment_readers();
    if segment_readers.is_empty() {
        return Ok(RangePartitioningSample {
            partition_by: partition_by.clone(),
            sample_points: vec![],
        });
    }

    let largest_segment: &SegmentReader =
        segment_readers.iter().max_by_key(|s| s.num_docs()).unwrap();

    let num_docs = largest_segment.num_docs();
    if num_docs == 0 {
        return Ok(RangePartitioningSample {
            partition_by: partition_by.clone(),
            sample_points: vec![],
        });
    }

    let ff_type = FFType::new(largest_segment.fast_fields(), partition_by.as_ref());

    let search_field_type = provider.fields.iter().find_map(|f| {
        if let WhichFastField::Named(name, sft) = f {
            if name == partition_by.as_ref() {
                return Some(*sft);
            }
        }
        None
    });

    let target_samples = 512;
    let mut rng = rand::rng();

    let mut doc_ids: Vec<u32> = (0..num_docs).collect();
    doc_ids.shuffle(&mut rng);
    let sample_doc_ids: Vec<u32> = doc_ids.into_iter().take(target_samples).collect();

    let mut sample_points = Vec::with_capacity(sample_doc_ids.len());
    for doc_id in sample_doc_ids {
        let val = ff_type.value(doc_id, search_field_type);
        sample_points.push(val.0);
    }

    // Sort points ascending so that build() produces sequential ranges
    sample_points.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    Ok(RangePartitioningSample {
        partition_by: partition_by.clone(),
        sample_points,
    })
}
