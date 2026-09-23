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

/// Benchmark batch size matching DataFusion's default (~8k rows).
const BATCH_SIZE: usize = 8192;

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

/// Sets up a 10,000-row table where all heap pages are marked all-visible.
fn setup_fully_visible() {
    Spi::run("DROP TABLE IF EXISTS bench_vis_full CASCADE;").unwrap();
    Spi::run("CREATE TABLE bench_vis_full (id serial, data text);").unwrap();
    Spi::run(
        "INSERT INTO bench_vis_full (data) \
         SELECT 'bench ' || i FROM generate_series(1, 10000) i;",
    )
    .unwrap();
    Spi::run("CREATE INDEX bench_vis_full_idx ON bench_vis_full USING bm25 (id, data);").unwrap();
    mark_all_blocks_visible("bench_vis_full");
}

/// Sets up a 10,000-row table where ~10% of heap pages have their all-visible bit cleared by row deletions.
fn setup_partially_visible() {
    Spi::run("DROP TABLE IF EXISTS bench_vis_partial CASCADE;").unwrap();
    Spi::run("CREATE TABLE bench_vis_partial (id serial, data text);").unwrap();
    Spi::run(
        "INSERT INTO bench_vis_partial (data) \
         SELECT 'bench ' || i FROM generate_series(1, 10000) i;",
    )
    .unwrap();
    Spi::run("CREATE INDEX bench_vis_partial_idx ON bench_vis_partial USING bm25 (id, data);")
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

/// Opens the relation and index, preparing a `VisibilityChecker` and document IDs for benchmarking.
fn prepare_checker(table_name: &str, index_name: &str) -> (VisibilityChecker, Vec<DocId>) {
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
        segment_reader.max_doc() >= BATCH_SIZE as u32,
        "Segment 0 must contain at least {BATCH_SIZE} docs, found {}",
        segment_reader.max_doc()
    );

    let ffhelper = Arc::new(FFHelper::for_ctid(&reader));
    let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
    let checker = VisibilityChecker::with_rel_and_snap(&heap_rel, snapshot).with_ffhelper(ffhelper);

    let doc_ids: Vec<DocId> = (0..BATCH_SIZE as DocId).collect();
    (checker, doc_ids)
}

/// Benchmarks boolean mask visibility checking over 8,192 docs on fully visible blocks.
#[pg_bench(
    setup = setup_fully_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_fully_visible_mask(b: &mut Bencher) {
    let (mut checker, doc_ids) = prepare_checker("bench_vis_full", "bench_vis_full_idx");
    let mut mask = vec![false; BATCH_SIZE];

    b.iter(move || {
        checker.check_segment_docs_mask(0, &doc_ids, &mut mask);
        black_box(&mask);
    });
}

/// Benchmarks CTID array visibility checking over 8,192 docs on fully visible blocks.
#[pg_bench(
    setup = setup_fully_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_fully_visible_ctid(b: &mut Bencher) {
    let (mut checker, doc_ids) = prepare_checker("bench_vis_full", "bench_vis_full_idx");
    let mut ctids = vec![None; BATCH_SIZE];

    b.iter(move || {
        checker.check_segment_docs(0, &doc_ids, &mut ctids);
        black_box(&ctids);
    });
}

/// Benchmarks boolean mask visibility checking over 8,192 docs on partially visible blocks (~10% missing VM bit).
#[pg_bench(
    setup = setup_partially_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_partially_visible_mask(b: &mut Bencher) {
    let (mut checker, doc_ids) = prepare_checker("bench_vis_partial", "bench_vis_partial_idx");
    let mut mask = vec![false; BATCH_SIZE];

    b.iter(move || {
        checker.check_segment_docs_mask(0, &doc_ids, &mut mask);
        black_box(&mask);
    });
}

/// Benchmarks CTID array visibility checking over 8,192 docs on partially visible blocks (~10% missing VM bit).
#[pg_bench(
    setup = setup_partially_visible,
    transaction = "shared",
    warm_up_time_ms = 5_000,
    measurement_time_ms = 25_000
)]
fn bench_visibility_partially_visible_ctid(b: &mut Bencher) {
    let (mut checker, doc_ids) = prepare_checker("bench_vis_partial", "bench_vis_partial_idx");
    let mut ctids = vec![None; BATCH_SIZE];

    b.iter(move || {
        checker.check_segment_docs(0, &doc_ids, &mut ctids);
        black_box(&ctids);
    });
}
