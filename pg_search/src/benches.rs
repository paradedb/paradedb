use std::sync::Arc;

use pgrx::prelude::*;
use pgrx_bench::{Bencher, black_box};
use tantivy::DocId;

use crate::index::directory::mvcc::MvccSatisfies;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;

/// Benchmark batch size matching DataFusion's default batch size (~8k rows).
const BATCH_SIZE: usize = 8192;

/// Number of dense batches streamed in sequence (30 * 8,192 = 245,760 docs).
const DENSE_BATCH_COUNT: usize = 30;

/// Number of sparse batches streamed in sequence (3 * 8,192 = 24,576 docs).
const SPARSE_BATCH_COUNT: usize = 3;

/// Stride between sparse document matches, spanning across heap blocks and boundary chunks.
const SPARSE_STRIDE: DocId = 10;

/// Sets the visibility map `all-visible` bit for all heap blocks of the specified relation.
///
/// This avoids running `VACUUM` (which cannot run inside a transaction block in `pgrx-bench`),
/// ensuring that `is_block_all_visible` fast-path succeeds for benchmarked pages.
fn mark_all_blocks_visible(table_name: &str) {
    let heap_oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{table_name}'::regclass::oid"))
        .expect("spi get heap oid")
        .expect("heap relation not found");
    let heap_rel = PgSearchRelation::open(heap_oid);
    unsafe {
        let mut vmbuffer = pg_sys::InvalidBuffer as pg_sys::Buffer;
        let nblocks =
            pg_sys::RelationGetNumberOfBlocksInFork(heap_rel.as_ptr(), heap_rel.fork_number());
        for blkno in 0..nblocks {
            pg_sys::visibilitymap_pin(heap_rel.as_ptr(), blkno, &mut vmbuffer);
            let heapbuf = pg_sys::ReadBuffer(heap_rel.as_ptr(), blkno);
            pg_sys::LockBuffer(heapbuf, pg_sys::BUFFER_LOCK_EXCLUSIVE as i32);
            let page = pg_sys::BufferGetPage(heapbuf);
            pg_sys::PageSetAllVisible(page);
            pg_sys::MarkBufferDirty(heapbuf);
            pg_sys::visibilitymap_set(
                heap_rel.as_ptr(),
                blkno,
                heapbuf,
                pg_sys::InvalidXLogRecPtr as u64,
                vmbuffer,
                pg_sys::InvalidTransactionId,
                pg_sys::VISIBILITYMAP_ALL_VISIBLE as u8,
            );
            pg_sys::LockBuffer(heapbuf, pg_sys::BUFFER_LOCK_UNLOCK as i32);
            pg_sys::ReleaseBuffer(heapbuf);
        }
        if vmbuffer != pg_sys::InvalidBuffer as pg_sys::Buffer {
            pg_sys::ReleaseBuffer(vmbuffer);
        }
    }
}

/// Sets up a table with 250,000 rows where all heap pages are marked all-visible.
/// Spans ~2,500 blocks and ~10 boundary chunks (> 256 blocks per chunk) to benchmark
/// multi-chunk boundary decoding, chunk cache behavior, and batch streaming.
fn setup_fully_visible() {
    Spi::run("DROP TABLE IF EXISTS bench_vis_full CASCADE;").unwrap();
    Spi::run("CREATE TABLE bench_vis_full (id serial, data text);").unwrap();
    Spi::run(
        "INSERT INTO bench_vis_full (data) \
         SELECT 'bench ' || i || ' ' || repeat('x', 50) FROM generate_series(1, 250000) i;",
    )
    .unwrap();
    Spi::run(
        "CREATE INDEX bench_vis_full_idx ON bench_vis_full USING paradedb (id, data) \
         WITH (target_segment_count = 1);",
    )
    .unwrap();
    mark_all_blocks_visible("bench_vis_full");
}

/// Sets up a table with 250,000 rows where ~10% of heap pages have their all-visible bit cleared by row deletions,
/// spanning ~2,500 blocks and ~10 boundary chunks.
fn setup_partially_visible() {
    Spi::run("DROP TABLE IF EXISTS bench_vis_partial CASCADE;").unwrap();
    Spi::run("CREATE TABLE bench_vis_partial (id serial, data text);").unwrap();
    Spi::run(
        "INSERT INTO bench_vis_partial (data) \
         SELECT 'bench ' || i || ' ' || repeat('x', 50) FROM generate_series(1, 250000) i;",
    )
    .unwrap();
    Spi::run(
        "CREATE INDEX bench_vis_partial_idx ON bench_vis_partial USING paradedb (id, data) \
         WITH (target_segment_count = 1);",
    )
    .unwrap();
    mark_all_blocks_visible("bench_vis_partial");
    // Clear visibility map all-visible bit for ~10% of pages by deleting 1 row per 10 blocks:
    Spi::run(
        "DELETE FROM bench_vis_partial \
         WHERE substring(ctid::text from 2 for position(',' in ctid::text) - 2)::int % 10 = 0 \
           AND ctid::text LIKE '%,1)';",
    )
    .unwrap();
}

fn prepare_checker(table_name: &str, index_name: &str, max_doc: DocId) -> VisibilityChecker {
    unsafe {
        pg_sys::CommandCounterIncrement();
        if pg_sys::GetActiveSnapshot().is_null() {
            pg_sys::PushActiveSnapshot(pg_sys::GetTransactionSnapshot());
        }
    }
    let heap_oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{table_name}'::regclass::oid"))
        .expect("spi get heap oid")
        .expect("heap relation not found");
    let index_oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{index_name}'::regclass::oid"))
        .expect("spi get index oid")
        .expect("index relation not found");

    let heap_rel = PgSearchRelation::open(heap_oid);
    let index_rel = PgSearchRelation::open(index_oid);

    let reader = SearchIndexReader::open(
        &index_rel,
        SearchQueryInput::All,
        false,
        MvccSatisfies::Snapshot,
    )
    .expect("Failed to open search index reader");

    let segment_reader = &reader.searcher().segment_readers()[0];
    assert!(
        segment_reader.max_doc() > max_doc,
        "Segment 0 must contain at least {} docs, found {}",
        max_doc + 1,
        segment_reader.max_doc()
    );

    let ffhelper = Arc::new(FFHelper::for_ctid(&reader));
    let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
    VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot).with_ffhelper(ffhelper)
}

/// Prepares a `VisibilityChecker` and consecutive 8,192-sized dense batches spanning the table.
fn prepare_dense_stream(
    table_name: &str,
    index_name: &str,
) -> (VisibilityChecker, Vec<Vec<DocId>>) {
    let mut batches = Vec::with_capacity(DENSE_BATCH_COUNT);
    for b in 0..DENSE_BATCH_COUNT {
        let start = (b * BATCH_SIZE) as DocId;
        let batch: Vec<DocId> = (start..start + BATCH_SIZE as DocId).collect();
        batches.push(batch);
    }
    let max_doc = (DENSE_BATCH_COUNT * BATCH_SIZE) as DocId - 1;
    let checker = prepare_checker(table_name, index_name, max_doc);
    (checker, batches)
}

/// Prepares a `VisibilityChecker` and consecutive 8,192-sized sparse batches striding across the table.
fn prepare_sparse_stream(
    table_name: &str,
    index_name: &str,
) -> (VisibilityChecker, Vec<Vec<DocId>>) {
    let mut batches = Vec::with_capacity(SPARSE_BATCH_COUNT);
    for b in 0..SPARSE_BATCH_COUNT {
        let start_i = (b * BATCH_SIZE) as DocId;
        let batch: Vec<DocId> = (start_i..start_i + BATCH_SIZE as DocId)
            .map(|i| i * SPARSE_STRIDE)
            .collect();
        batches.push(batch);
    }
    let max_doc = ((SPARSE_BATCH_COUNT * BATCH_SIZE - 1) as DocId) * SPARSE_STRIDE;
    let checker = prepare_checker(table_name, index_name, max_doc);
    (checker, batches)
}

/// Streams 30 consecutive 8,192-doc dense batches (245,760 docs total) through boolean mask
/// visibility checking on a fully visible relation.
#[pg_bench(
    setup = setup_fully_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_full_dense_mask(b: &mut Bencher) {
    let (mut checker, batches) = prepare_dense_stream("bench_vis_full", "bench_vis_full_idx");
    let mut mask = vec![false; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs_mask(0, batch, &mut mask);
            black_box(&mask);
        }
    });
}

/// Streams 30 consecutive 8,192-doc dense batches (245,760 docs total) through CTID resolution
/// on a fully visible relation.
#[pg_bench(
    setup = setup_fully_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_full_dense_ctid(b: &mut Bencher) {
    let (mut checker, batches) = prepare_dense_stream("bench_vis_full", "bench_vis_full_idx");
    let mut ctids = vec![None; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs(0, batch, &mut ctids);
            black_box(&ctids);
        }
    });
}

/// Streams 3 consecutive 8,192-doc sparse batches (24,576 docs with stride 10 spanning 245,750 docs)
/// through boolean mask visibility checking on a fully visible relation.
#[pg_bench(
    setup = setup_fully_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_full_sparse_mask(b: &mut Bencher) {
    let (mut checker, batches) = prepare_sparse_stream("bench_vis_full", "bench_vis_full_idx");
    let mut mask = vec![false; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs_mask(0, batch, &mut mask);
            black_box(&mask);
        }
    });
}

/// Streams 3 consecutive 8,192-doc sparse batches (24,576 docs with stride 10 spanning 245,750 docs)
/// through CTID resolution on a fully visible relation.
#[pg_bench(
    setup = setup_fully_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_full_sparse_ctid(b: &mut Bencher) {
    let (mut checker, batches) = prepare_sparse_stream("bench_vis_full", "bench_vis_full_idx");
    let mut ctids = vec![None; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs(0, batch, &mut ctids);
            black_box(&ctids);
        }
    });
}

/// Streams 30 consecutive 8,192-doc dense batches (245,760 docs total) through boolean mask
/// visibility checking on a partially visible relation (~10% missing VM bit).
#[pg_bench(
    setup = setup_partially_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_partial_dense_mask(b: &mut Bencher) {
    let (mut checker, batches) = prepare_dense_stream("bench_vis_partial", "bench_vis_partial_idx");
    let mut mask = vec![false; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs_mask(0, batch, &mut mask);
            black_box(&mask);
        }
    });
}

/// Streams 30 consecutive 8,192-doc dense batches (245,760 docs total) through CTID resolution
/// on a partially visible relation (~10% missing VM bit).
#[pg_bench(
    setup = setup_partially_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_partial_dense_ctid(b: &mut Bencher) {
    let (mut checker, batches) = prepare_dense_stream("bench_vis_partial", "bench_vis_partial_idx");
    let mut ctids = vec![None; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs(0, batch, &mut ctids);
            black_box(&ctids);
        }
    });
}

/// Streams 3 consecutive 8,192-doc sparse batches (24,576 docs with stride 10 spanning 245,750 docs)
/// through boolean mask visibility checking on a partially visible relation (~10% missing VM bit).
#[pg_bench(
    setup = setup_partially_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_partial_sparse_mask(b: &mut Bencher) {
    let (mut checker, batches) = prepare_sparse_stream("bench_vis_partial", "bench_vis_partial_idx");
    let mut mask = vec![false; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs_mask(0, batch, &mut mask);
            black_box(&mask);
        }
    });
}

/// Streams 3 consecutive 8,192-doc sparse batches (24,576 docs with stride 10 spanning 245,750 docs)
/// through CTID resolution on a partially visible relation (~10% missing VM bit).
#[pg_bench(
    setup = setup_partially_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_partial_sparse_ctid(b: &mut Bencher) {
    let (mut checker, batches) = prepare_sparse_stream("bench_vis_partial", "bench_vis_partial_idx");
    let mut ctids = vec![None; BATCH_SIZE];

    b.iter(move || {
        for batch in &batches {
            checker.check_segment_docs(0, batch, &mut ctids);
            black_box(&ctids);
        }
    });
}
