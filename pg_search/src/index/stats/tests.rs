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
#[pgrx::pg_schema]
mod tests {
    use std::ops::Bound;

    use pgrx::prelude::*;
    use tantivy::Index;
    use tantivy::index::SegmentId;

    use super::super::*;
    use crate::api::FieldName;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::scan::range_partitioning::RangePartitioning;

    fn open_index(index: &str) -> PgSearchRelation {
        let oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{index}'::regclass::oid"))
            .unwrap()
            .unwrap();
        PgSearchRelation::open(oid)
    }

    /// Every persisted segment's `.stats`, with the tantivy field for `field_name`.
    fn segment_stats(indexrel: &PgSearchRelation, field_name: &str) -> (Field, Vec<SegmentStats>) {
        let directory = MvccSatisfies::Snapshot.directory(indexrel);
        let index = Index::open(directory.clone()).unwrap();
        let field = index.schema().get_field(field_name).unwrap();
        let stats = index
            .searchable_segments()
            .unwrap()
            .iter()
            .map(|segment| {
                SegmentStats::of_segment(segment)
                    .unwrap()
                    .expect("every segment of a fresh index carries a .stats component")
            })
            .collect();
        drop(directory);
        (field, stats)
    }

    fn below(a: &PdbOwnedValue, b: &PdbOwnedValue) -> bool {
        a.total_cmp(b) == Ordering::Less
    }

    #[pg_test]
    fn empirical_stats_match_the_table() {
        Spi::run(
            r#"
            CREATE TABLE stats_src (
                id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT, seen TIMESTAMP,
                score FLOAT8, flag BOOLEAN
            );
            INSERT INTO stats_src (tenant_id, name, seen, score, flag)
            SELECT CASE WHEN i % 10 = 0 THEN NULL ELSE (i * 7919) % 100 END,
                   'name' || lpad(i::text, 5, '0'),
                   TIMESTAMP '2024-01-01' + (i || ' minutes')::interval,
                   (i % 1000)::float8 / 10,
                   i % 3 = 0
            FROM generate_series(1, 5000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_src_idx ON stats_src
                USING paradedb (id, tenant_id, name, seen, score, flag)
                WITH (key_field = 'id', target_segment_count = 1,
                      text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}');
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_src_idx");
        let index = Index::open(MvccSatisfies::Snapshot.directory(&indexrel)).unwrap();
        let schema = index.schema();
        let (_, stats) = segment_stats(&indexrel, "id");
        let [stats] = stats.as_slice() else {
            panic!("expected one segment, got {}", stats.len());
        };
        let empirical = |name: &str| {
            stats
                .empirical(schema.get_field(name).unwrap())
                .unwrap()
                .unwrap_or_else(|| panic!("no empirical stats for {name}"))
        };

        let id = empirical("id");
        assert_eq!(
            (id.min, id.max, id.nullable),
            (PdbOwnedValue::I64(1), PdbOwnedValue::I64(5000), false)
        );

        // The NULL tenants are exactly the multiples of ten, so the extremes survive them.
        let tenant = empirical("tenant_id");
        assert_eq!(
            (tenant.min, tenant.max, tenant.nullable),
            (PdbOwnedValue::I64(1), PdbOwnedValue::I64(99), true)
        );

        let name = empirical("name");
        assert_eq!(name.min, PdbOwnedValue::Str("name00001".into()));
        assert_eq!(name.max, PdbOwnedValue::Str("name05000".into()));

        // Datetimes live in an I64 column of raw Postgres microseconds.
        let raw = |agg: &str| {
            Spi::get_one::<i64>(&format!(
                "SELECT ((EXTRACT(EPOCH FROM {agg}(seen)) - 946684800) * 1000000)::bigint FROM stats_src"
            ))
            .unwrap()
            .unwrap()
        };
        let seen = empirical("seen");
        assert_eq!(
            (seen.min, seen.max),
            (
                PdbOwnedValue::I64(raw("min")),
                PdbOwnedValue::I64(raw("max"))
            )
        );

        let score = empirical("score");
        assert_eq!(
            (score.min, score.max),
            (PdbOwnedValue::F64(0.0), PdbOwnedValue::F64(99.9))
        );

        let flag = empirical("flag");
        assert_eq!(
            (flag.min, flag.max),
            (PdbOwnedValue::Bool(false), PdbOwnedValue::Bool(true))
        );

        // No logical bounds without a partitioned build.
        assert!(
            stats
                .logical(schema.get_field("id").unwrap())
                .unwrap()
                .is_none()
        );
    }

    /// A regular build whose rows outgrow the writer budget flushes several segments and merges
    /// them at commit, so the surviving segment's statistics come from the merge path.
    #[pg_test]
    fn merge_recomputes_empirical_stats() {
        Spi::run(
            r#"
            CREATE TABLE stats_merge (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_merge (tenant_id, name)
            SELECT (i * 7919) % 4,
                   (SELECT string_agg(md5((i * 32 + j)::text), ' ') FROM generate_series(1, 32) j)
            FROM generate_series(1, 24000) i;
            SET max_parallel_maintenance_workers = 0;
            SET maintenance_work_mem = '16MB';
            CREATE INDEX stats_merge_idx ON stats_merge USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', target_segment_count = 1);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_merge_idx");
        let (field, stats) = segment_stats(&indexrel, "id");
        assert_eq!(stats.len(), 1, "the build merges down to one segment");
        let id = stats[0].empirical(field).unwrap().unwrap();
        assert_eq!(
            (id.min, id.max),
            (PdbOwnedValue::I64(1), PdbOwnedValue::I64(24000))
        );
    }

    /// A partitioned build stamps each partition's box on its segment. The persisted split
    /// points then replace sampling, and each partition of a range scan maps to one segment.
    #[pg_test]
    fn partitioned_build_stamps_bounds_and_prunes() {
        Spi::run(
            r#"
            CREATE TABLE stats_part (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_part (tenant_id, name)
            SELECT CASE WHEN i % 10 = 0 THEN NULL ELSE (i * 7919) % 100 END,
                   'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_part_idx ON stats_part USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_part_idx");
        let (field, stats) = segment_stats(&indexrel, "tenant_id");
        assert_eq!(stats.len(), 8, "one segment per partition");

        let mut open_below = 0;
        let mut open_above = 0;
        for segment in &stats {
            let bounds = segment
                .logical(field)
                .unwrap()
                .expect("every partition has a box");
            open_below += usize::from(matches!(bounds.lower, Bound::Unbounded));
            open_above += usize::from(matches!(bounds.upper, Bound::Unbounded));
            // The rows inside the box: what the build routed there. NULLs route below every
            // split, so only the bottom partition may hold them.
            let empirical = segment.empirical(field).unwrap().unwrap();
            assert_eq!(empirical.nullable, bounds.may_hold_nulls(), "{bounds:?}");
            if let Bound::Included(lower) = &bounds.lower {
                assert!(!below(&empirical.min, lower), "{empirical:?} vs {bounds:?}");
            }
            if let Bound::Excluded(upper) = &bounds.upper {
                assert!(below(&empirical.max, upper), "{empirical:?} vs {bounds:?}");
            }
        }
        assert_eq!((open_below, open_above), (1, 1));

        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("every partition's segment carries its box");
        assert_eq!(split_points.len(), 7, "{split_points:?}");
        assert!(split_points.windows(2).all(|w| below(&w[0], &w[1])));

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points,
        };
        let mut chosen = Vec::new();
        for partition in 0..8 {
            let segments = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(
                segments.total_scanned(),
                1,
                "partition {partition}: {segments:?}"
            );
            assert_eq!(
                segments.included.len(),
                1,
                "partition {partition} should be fully included"
            );
            assert_eq!(
                segments.partially_included.len(),
                0,
                "partition {partition} should have 0 partially included"
            );
            assert_eq!(
                segments.pruned_count, 7,
                "partition {partition} should prune 7 segments"
            );
            chosen.extend(segments.included);
        }
        chosen.sort();
        chosen.dedup();
        assert_eq!(chosen.len(), 8, "each partition maps to its own segment");

        // A coarser layout keeps every segment its range reaches, and nothing else.
        let coarse = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points: vec![boundaries.split_points[3].clone()],
        };
        let low = segments_for_partition(&reader, &coarse, 0);
        let high = segments_for_partition(&reader, &coarse, 1);
        assert_eq!(
            (
                low.included.len(),
                low.partially_included.len(),
                low.pruned_count
            ),
            (4, 0, 4)
        );
        assert_eq!(
            (
                high.included.len(),
                high.partially_included.len(),
                high.pruned_count
            ),
            (4, 0, 4)
        );
    }

    /// Without a partitioned build, the empirical `min`/`max` still prunes: a serial build over
    /// a heap in key order gives every segment a disjoint key range.
    #[pg_test]
    fn empirical_stats_prune_unpartitioned_segments() {
        Spi::run(
            r#"
            CREATE TABLE stats_plain (id BIGSERIAL PRIMARY KEY, name TEXT);
            INSERT INTO stats_plain (name)
            SELECT 'row ' || i || ' ' || repeat('padding word here ', 50) FROM generate_series(1, 20000) i;
            ANALYZE stats_plain;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_plain_idx ON stats_plain USING paradedb (id, name)
                WITH (key_field = 'id', target_segment_count = 4);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_plain_idx");
        let (field, stats) = segment_stats(&indexrel, "id");
        assert!(stats.len() > 1, "need several segments to prune between");
        let ranges: Vec<EmpiricalStats> = stats
            .iter()
            .map(|s| s.empirical(field).unwrap().unwrap())
            .collect();

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::I64(5001), PdbOwnedValue::I64(15001)],
        };
        let mut pruned_somewhere = false;
        for partition in 0..3 {
            let range = boundaries.partition_range(partition).unwrap();
            let expected = ranges
                .iter()
                .filter(|r| r.intersects(&range.lower, &range.upper))
                .count();
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(chosen.total_scanned(), expected, "partition {partition}");
            pruned_somewhere |= chosen.pruned_count > 0;
        }
        assert!(pruned_somewhere);
    }

    /// A partition that outgrows the writer budget flushes several segments, and
    /// `finish_partition` merges them: the merged segment must keep the partition's box.
    #[pg_test]
    fn partition_merge_keeps_logical_bounds() {
        Spi::run(
            r#"
            CREATE TABLE stats_partition_merge (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_partition_merge (tenant_id, name)
            SELECT (i * 7919) % 4,
                   (SELECT string_agg(md5((i * 32 + j)::text), ' ') FROM generate_series(1, 32) j)
            FROM generate_series(1, 24000) i;
            SET max_parallel_maintenance_workers = 0;
            SET maintenance_work_mem = '16MB';
            CREATE INDEX stats_partition_merge_idx ON stats_partition_merge USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 2);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_partition_merge_idx");
        let (field, stats) = segment_stats(&indexrel, "tenant_id");
        assert_eq!(stats.len(), 2, "each partition merges down to one segment");
        let mut lowers = Vec::new();
        for segment in &stats {
            let bounds = segment
                .logical(field)
                .unwrap()
                .expect("the merge keeps the box");
            let empirical = segment.empirical(field).unwrap().unwrap();
            if let Bound::Included(lower) = &bounds.lower {
                assert!(!below(&empirical.min, lower));
            }
            if let Bound::Excluded(upper) = &bounds.upper {
                assert!(below(&empirical.max, upper));
            }
            lowers.push(bounds.lower);
        }
        assert!(lowers.contains(&Bound::Unbounded));
        assert_eq!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .map(|p| p.len()),
            Some(1)
        );
    }

    /// An insert after the build lands in a mutable segment, which carries no statistics at all.
    /// The split points stay, and every partition keeps the new segment.
    #[pg_test]
    fn insert_after_build_keeps_the_split_points() {
        Spi::run(
            r#"
            CREATE TABLE stats_grow (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_grow (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_grow_idx ON stats_grow USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_grow_idx");
        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("fresh build");
        assert_eq!(split_points.len(), 3);

        Spi::run("INSERT INTO stats_grow (tenant_id, name) VALUES (42, 'late row');").unwrap();
        assert_eq!(
            persisted_split_points(&indexrel, "tenant_id").unwrap(),
            Some(split_points.clone())
        );

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        assert_eq!(
            reader.segment_ids().len(),
            5,
            "four partitions plus the new segment"
        );
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points,
        };
        let mut everywhere: Option<Vec<SegmentId>> = None;
        for partition in 0..4 {
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(
                chosen.total_scanned(),
                2,
                "its own segment and the unboxed one: {chosen:?}"
            );
            assert_eq!(chosen.included.len(), 1, "boxed segment is fully included");
            assert_eq!(
                chosen.partially_included.len(),
                1,
                "unboxed segment is partially included"
            );
            assert_eq!(chosen.pruned_count, 3, "3 other boxed segments are pruned");
            let scanned: Vec<SegmentId> = chosen.all_scanned().copied().collect();
            everywhere = Some(match everywhere {
                None => scanned,
                Some(prev) => prev.into_iter().filter(|id| scanned.contains(id)).collect(),
            });
        }
        assert_eq!(
            everywhere.unwrap().len(),
            1,
            "the unboxed segment survives every partition"
        );
    }

    /// Without a mutable segment, a late insert lands in an immutable segment that carries
    /// empirical statistics but no box. The split points stay and the segment prunes on its own range.
    /// A merge that takes it in keeps the statistics and drops the box, and once no segment has a
    /// box the split points are gone.
    #[pg_test]
    fn unboxed_segment_prunes_on_empirical_stats_and_merges_without_a_box() {
        Spi::run(
            r#"
            CREATE TABLE stats_unboxed (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_unboxed (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_unboxed_idx ON stats_unboxed USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4,
                      mutable_segment_rows = 0);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_unboxed_idx");
        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("fresh build");
        assert_eq!(split_points.len(), 3);

        Spi::run(
            r#"
            INSERT INTO stats_unboxed (tenant_id, name)
            SELECT 42, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 5000) i;
            "#,
        )
        .unwrap();
        assert_eq!(
            persisted_split_points(&indexrel, "tenant_id").unwrap(),
            Some(split_points.clone())
        );
        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        assert_eq!(reader.segment_ids().len(), 5);
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points,
        };
        let late_row = EmpiricalStats {
            min: PdbOwnedValue::I64(42),
            max: PdbOwnedValue::I64(42),
            nullable: false,
        };
        for partition in 0..4 {
            let range = boundaries.partition_range(partition).unwrap();
            let expected = if late_row.intersects(&range.lower, &range.upper) {
                2
            } else {
                1
            };
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(
                chosen.total_scanned(),
                expected,
                "partition {partition}: {chosen:?}"
            );
        }
        // `ALTER INDEX` refuses a relation this transaction still holds open.
        drop(reader);
        drop(indexrel);

        // A layer takes segments no larger than itself and closes a candidate once it fills the
        // layer by a third over. Five near-equal segments fill 3.4 layers only with the fifth one
        // in, so one candidate takes them all. Background layers would hand the merge to a worker
        // that cannot see this transaction's segments.
        let largest: i64 = Spi::get_one(
            "SELECT max(byte_size)::bigint FROM paradedb.index_info('stats_unboxed_idx');",
        )
        .unwrap()
        .unwrap();
        Spi::run(&format!(
            "ALTER INDEX stats_unboxed_idx SET (layer_sizes = '{}', background_layer_sizes = '0');",
            largest * 17 / 5
        ))
        .unwrap();
        // The row that triggers the merge lands in its own segment beside the merged one.
        Spi::run("INSERT INTO stats_unboxed (tenant_id, name) VALUES (43, 'later row');").unwrap();
        let indexrel = open_index("stats_unboxed_idx");
        let (field, stats) = segment_stats(&indexrel, "tenant_id");
        assert_eq!(stats.len(), 2, "the merged segment and the trigger row");
        let ranges: Vec<EmpiricalStats> = stats
            .iter()
            .map(|s| {
                assert!(
                    s.logical(field).unwrap().is_none(),
                    "a source without a box leaves the merge without one"
                );
                s.empirical(field).unwrap().unwrap()
            })
            .collect();
        assert!(
            ranges
                .iter()
                .any(|r| r.min == PdbOwnedValue::I64(0) && r.max == PdbOwnedValue::I64(99)),
            "the merge recomputes the range over every source: {ranges:?}"
        );
        assert!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .is_none()
        );
    }

    /// A recent index keeps timestamps in an `I64` column while the partition bounds arrive as
    /// `Date`, so the empirical statistics must lift to `Date` before they can prune.
    #[pg_test]
    fn empirical_stats_prune_on_a_timestamp_key() {
        Spi::run(
            r#"
            CREATE TABLE stats_ts (id BIGSERIAL PRIMARY KEY, created_at TIMESTAMP, name TEXT);
            INSERT INTO stats_ts (created_at, name)
            SELECT TIMESTAMP '2024-01-01' + (i || ' minutes')::interval,
                   'row ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            ANALYZE stats_ts;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_ts_idx ON stats_ts USING paradedb (id, created_at, name)
                WITH (key_field = 'id', target_segment_count = 4);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_ts_idx");
        let (field, stats) = segment_stats(&indexrel, "created_at");
        assert!(stats.len() > 1, "need several segments to prune between");
        let ranges: Vec<EmpiricalStats> = stats
            .iter()
            .map(|s| {
                let raw = s.empirical(field).unwrap().unwrap();
                assert!(
                    matches!(raw.min, PdbOwnedValue::I64(_)),
                    "stored as the column's raw micros"
                );
                raw.into_dates().unwrap()
            })
            .collect();
        let mut mins: Vec<PdbOwnedValue> = ranges.iter().map(|r| r.min.clone()).collect();
        mins.sort_by(PdbOwnedValue::total_cmp);

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("created_at"),
            split_points: vec![mins[1].clone(), mins[mins.len() - 1].clone()],
        };
        let mut pruned_somewhere = false;
        for partition in 0..3 {
            let range = boundaries.partition_range(partition).unwrap();
            let expected = ranges
                .iter()
                .filter(|r| r.intersects(&range.lower, &range.upper))
                .count();
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(chosen.total_scanned(), expected, "partition {partition}");
            pruned_somewhere |= chosen.pruned_count > 0;
        }
        assert!(pruned_somewhere);
    }

    fn text_partitioned_index(table: &str, index: &str, normalizer: &str) {
        Spi::run(&format!(
            r#"
            CREATE TABLE {table} (id BIGSERIAL PRIMARY KEY, name TEXT, about TEXT);
            INSERT INTO {table} (name, about)
            SELECT CASE WHEN i % 3 = 0 THEN 'Zed' || i WHEN i % 3 = 1 THEN 'alice' || i ELSE 'Bob' || i END,
                   repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX {index} ON {table} USING paradedb (id, name)
                WITH (key_field = 'id', partition_by = 'name', target_segment_count = 4,
                      text_fields = '{{"name": {{"tokenizer": {{"type": "keyword"}}, "fast": true, "normalizer": "{normalizer}"}}}}');
            "#
        ))
        .unwrap();
    }

    /// Routing compares raw text, but the partition query reads the fast column. A normalizer
    /// reorders that column, so such a field gets no box.
    #[pg_test]
    fn normalized_text_field_gets_no_logical_bounds() {
        text_partitioned_index("stats_text_lower", "stats_text_lower_idx", "lowercase");
        let indexrel = open_index("stats_text_lower_idx");
        let (field, stats) = segment_stats(&indexrel, "name");
        assert!(stats.len() > 1);
        for segment in &stats {
            assert!(segment.logical(field).unwrap().is_none());
            let empirical = segment.empirical(field).unwrap().unwrap();
            assert!(matches!(empirical.min, PdbOwnedValue::Str(_)));
        }
        assert!(persisted_split_points(&indexrel, "name").unwrap().is_none());
    }

    /// With the raw normalizer the fast column keeps the routing order, so the box holds.
    #[pg_test]
    fn raw_text_field_gets_logical_bounds() {
        text_partitioned_index("stats_text_raw", "stats_text_raw_idx", "raw");
        let indexrel = open_index("stats_text_raw_idx");
        let (field, stats) = segment_stats(&indexrel, "name");
        assert!(stats.len() > 1);
        for segment in &stats {
            let bounds = segment
                .logical(field)
                .unwrap()
                .expect("raw text keeps its box");
            let empirical = segment.empirical(field).unwrap().unwrap();
            if let Bound::Included(lower) = &bounds.lower {
                assert!(!below(&empirical.min, lower));
            }
            if let Bound::Excluded(upper) = &bounds.upper {
                assert!(below(&empirical.max, upper));
            }
        }
        assert!(persisted_split_points(&indexrel, "name").unwrap().is_some());
    }

    #[test]
    fn bounds_union_and_intersection() {
        let i = |v: i64| PdbOwnedValue::I64(v);
        let a = LogicalBounds {
            lower: Bound::Included(i(10)),
            upper: Bound::Excluded(i(20)),
        };
        let b = LogicalBounds {
            lower: Bound::Included(i(20)),
            upper: Bound::Excluded(i(30)),
        };
        assert_eq!(
            a.union(&b),
            LogicalBounds {
                lower: Bound::Included(i(10)),
                upper: Bound::Excluded(i(30)),
            }
        );
        assert_eq!(
            a.union(&LogicalBounds {
                lower: Bound::Unbounded,
                upper: Bound::Excluded(i(10)),
            })
            .lower,
            Bound::Unbounded
        );
        // Half-open boxes that touch at 20 share no value.
        assert!(!a.intersects(&Bound::Included(i(20)), &Bound::Unbounded));
        assert!(a.intersects(&Bound::Included(i(19)), &Bound::Unbounded));
        assert!(a.intersects(&Bound::Unbounded, &Bound::Excluded(i(11))));
        assert!(!a.intersects(&Bound::Unbounded, &Bound::Excluded(i(10))));
        // A bound of another kind proves nothing.
        assert!(a.intersects(
            &Bound::Included(PdbOwnedValue::Str("x".into())),
            &Bound::Unbounded
        ));

        let e = EmpiricalStats {
            min: i(5),
            max: i(9),
            nullable: false,
        };
        assert!(e.intersects(&Bound::Included(i(9)), &Bound::Unbounded));
        assert!(!e.intersects(&Bound::Excluded(i(9)), &Bound::Unbounded));
        assert!(!e.intersects(&Bound::Unbounded, &Bound::Excluded(i(5))));
        assert!(e.intersects(&Bound::Unbounded, &Bound::Included(i(5))));
    }

    #[test]
    fn bounds_subset_containment() {
        let i = |v: i64| PdbOwnedValue::I64(v);
        let a = LogicalBounds {
            lower: Bound::Included(i(10)),
            upper: Bound::Excluded(i(20)),
        };

        // Fully containing range [10, 20)
        assert!(a.is_subset_of(&Bound::Included(i(10)), &Bound::Excluded(i(20))));
        // Wider range [0, 30)
        assert!(a.is_subset_of(&Bound::Included(i(0)), &Bound::Excluded(i(30))));
        // Unbounded range [-inf, +inf)
        assert!(a.is_subset_of(&Bound::Unbounded, &Bound::Unbounded));
        // Lower unbounded [-inf, 20)
        assert!(a.is_subset_of(&Bound::Unbounded, &Bound::Excluded(i(20))));
        // Upper unbounded [10, +inf)
        assert!(a.is_subset_of(&Bound::Included(i(10)), &Bound::Unbounded));

        // Tighter lower [15, 20) -> not a subset
        assert!(!a.is_subset_of(&Bound::Included(i(15)), &Bound::Excluded(i(20))));
        // Tighter upper [10, 15) -> not a subset
        assert!(!a.is_subset_of(&Bound::Included(i(10)), &Bound::Excluded(i(15))));
        // Shifted range [15, 25) -> not a subset
        assert!(!a.is_subset_of(&Bound::Included(i(15)), &Bound::Excluded(i(25))));
        // Disjoint range [20, 30) -> not a subset
        assert!(!a.is_subset_of(&Bound::Included(i(20)), &Bound::Excluded(i(30))));

        let e = EmpiricalStats {
            min: i(10),
            max: i(19),
            nullable: false,
        };
        // [10, 19] is fully inside [10, 20)
        assert!(e.is_subset_of(&Bound::Included(i(10)), &Bound::Excluded(i(20))));
        // [10, 19] is fully inside [10, 19]
        assert!(e.is_subset_of(&Bound::Included(i(10)), &Bound::Included(i(19))));
        // [10, 19] is not inside [10, 19) because 19 is excluded
        assert!(!e.is_subset_of(&Bound::Included(i(10)), &Bound::Excluded(i(19))));
        // [10, 19] is not inside [11, 20)
        assert!(!e.is_subset_of(&Bound::Included(i(11)), &Bound::Excluded(i(20))));
    }
}
