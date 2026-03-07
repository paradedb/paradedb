# Design: Seekable Query and Dynamic Filter Pushdown

## 1. `SeekHandle` and Shared State

We need a way to pass dynamically updated target `DocId`s from the DataFusion `Scanner` (which runs `next()` and evaluates pre-filters) to the Tantivy `Scorer` (which runs `advance()` deeply inside `try_get_batch_ids`).

Tantivy operates on segment-local `DocId`s. We will introduce a `SeekHandle` to manage this state. `SeekHandle` will map a `SegmentId` to an `Arc<AtomicU32>` representing the minimum `DocId` that the scorer should seek to.

```rust
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tantivy::SegmentId;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Default, Clone)]
pub struct SeekHandle {
    thresholds: Arc<RwLock<HashMap<SegmentId, Arc<AtomicU32>>>>,
}

impl SeekHandle {
    pub fn get_threshold(&self, segment_id: SegmentId) -> Arc<AtomicU32> {
        let mut lock = self.thresholds.write().unwrap();
        lock.entry(segment_id).or_insert_with(|| Arc::new(AtomicU32::new(0))).clone()
    }
}
```

## 2. `SeekableQuery` (`pg_search/src/query/seekable.rs`)

We will introduce a query wrapper that intercepts `Scorer` iteration.

```rust
pub struct SeekableQuery {
    underlying: Box<dyn Query>,
    seek_handle: SeekHandle,
}

// Implements Query, Weight, and Scorer...
```

The `SeekableScorer` will periodically check the `AtomicU32` threshold during `advance()`, avoiding the cost of an atomic load on every single iteration by using a countdown counter (e.g. checking every 128 iterations).

## 3. Integrating with `SearchIndexReader` and `Scanner`

1.  Modify `SearchIndexReader` to hold a `SeekHandle`.
2.  When queries are instantiated in `make_query` (or `into_tantivy_query`), wrap them in `SeekableQuery` using the `SeekHandle`.
3.  Pass the `SeekHandle` and the `SortByField` (if the index is sorted) down to `MultiSegmentSearchResults` and eventually to the `Scanner`.

## 4. Translating Dynamic Filters to `DocId` Thresholds (`batch_scanner.rs`)

In `Scanner::next`, before calling `try_get_batch_ids()`, we will:
1.  Inspect the `pre_filters` for `DynamicFilterPhysicalExpr`.
2.  Extract the current bounds from the dynamic filter (e.g., lower/upper scalar values).
3.  Use the `tantivy-columnar` binary search API (`binary_search_range` on `FFHelper` columns) to map the dynamic value bounds to a `DocId` range.
4.  Update the `Arc<AtomicU32>` threshold for the current segment based on the binary search result. This signals the `SeekableScorer` to skip irrelevant blocks immediately.

If the segment is sorted in `Asc` order and we have a lower bound, we binary search and take the start of the `DocId` range.
If the segment is sorted in `Desc` order and we have an upper bound, we binary search and take the start of the `DocId` range.
