### What feature are you requesting?

Add support for pushing Sort-Merge Join (SMJ) dynamic filters deep into Tantivy's query execution by introducing a custom `SeekableQuery` wrapper. This will allow the search index to actively `seek()` past documents (and entire compressed blocks) that fall outside the dynamically progressing min/max bounds of a concurrent join, rather than evaluating them after materialization.

### Why are you requesting this feature?

When DataFusion executes a `SortMergeJoinExec`, it publishes progressive min/max dynamic filters that tighten as the join advances. Currently, our `Scanner` in `batch_scanner.rs` evaluates `PreFilters`, but this only filters documents *after* they have been yielded by the Tantivy `Scorer`. By pushing these dynamic bounds into the Tantivy `DocSet` iteration via a seekable query, we can skip decoding posting lists, get "sudden death" early termination when the join finishes, and improve the performance of equijoins on sorted indexed columns.

### What is your proposed implementation for this feature?

To implement this, we need to create a wrapper query, with implementations of `Query`, `Weight`, and `Scorer`, wrapping the underlying query's implementations (similar to how [`BoostQuery`](https://github.com/paradedb/tantivy/blob/b12ba0aae3a60dff03fa0daa684bf62f49ba8c59/src/query/boost_query.rs#L14-L64), [`BoostWeight`](https://github.com/paradedb/tantivy/blob/b12ba0aae3a60dff03fa0daa684bf62f49ba8c59/src/query/boost_query.rs#L55-L83), and [`BoostScorer`](https://github.com/paradedb/tantivy/blob/b12ba0aae3a60dff03fa0daa684bf62f49ba8c59/src/query/boost_query.rs#L85-L114) are implemented).

1. **`SeekableQuery`, `SeekableWeight`, and `SeekableScorer`**:
   The core logic will live in `SeekableScorer`, which wraps the underlying `Scorer` and periodically checks a shared threshold to seek forward:

   ```rust
   use std::sync::atomic::{AtomicU32, Ordering};
   use std::sync::Arc;
   use tantivy::docset::{DocSet, SeekDangerResult, TERMINATED};
   use tantivy::query::Scorer;
   use tantivy::DocId;

   pub struct SeekableScorer<S: Scorer> {
       underlying: S,
       min_doc_threshold: Arc<AtomicU32>,
   }

   impl<S: Scorer> DocSet for SeekableScorer<S> {
       fn advance(&mut self) -> DocId {
           // 1. Check the threshold.
           let min_doc = self.min_doc_threshold.load(Ordering::Relaxed);
           
           // 2. Advance the underlying docset.
           let next_doc = self.underlying.advance();
           
           // 3. If the next doc is valid but below our threshold, seek forward.
           if next_doc != TERMINATED && next_doc < min_doc {
               return self.underlying.seek(min_doc);
           }
           
           next_doc
       }

       fn seek(&mut self, target: DocId) -> DocId {
           let min_doc = self.min_doc_threshold.load(Ordering::Relaxed);
           // Seek to the larger of the caller's target or our background threshold.
           let actual_target = target.max(min_doc);
           self.underlying.seek(actual_target)
       }

       fn seek_danger(&mut self, target: DocId) -> SeekDangerResult {
           let min_doc = self.min_doc_threshold.load(Ordering::Relaxed);
           let actual_target = target.max(min_doc);
           self.underlying.seek_danger(actual_target)
       }

       fn doc(&self) -> DocId {
           self.underlying.doc()
       }

       fn size_hint(&self) -> u32 {
           self.underlying.size_hint()
       }
   }
   ```
   *Performance consideration:* Since `advance()` is in the hot loop, even `AtomicU32::load(Ordering::Relaxed)` has a small cost. We may need to batch the atomic checks (e.g., only checking every 128 iterations via a local countdown counter).

2. **Query Instantiation and State Sharing**:
   - Wrap the root query in `SeekableQuery` during index search.
   - Tantivy operates on segment-local `DocId`s (`0..max_doc_in_segment`), so the `min_doc_threshold` must track segment-local IDs. We will maintain a mapping (e.g., `SeekHandle`) from `SegmentOrdinal` to these `Arc<AtomicU32>` thresholds and pass this into the `Scanner` in `batch_scanner.rs`.

3. **Consuming Dynamic Filters in `batch_scanner.rs`**:
   - In `Scanner::next`, before invoking `try_get_batch_ids()`, we will inspect the `pre_filters` for `DynamicFilterPhysicalExpr` updates.
   - For the current segment (`self.search_results.current_segment()`), we evaluate the dynamic filter's `current()` state to obtain the latest bounds.

4. **Translating Value Bounds to `DocId` Thresholds**:
   - Because the segment is sorted by the join key, a minimum value bound maps monotonically to a minimum `DocId`.
   - We will use the fast field reader (`FFHelper`) for the constrained column to perform a binary search, locating the first segment-local `DocId` that satisfies the dynamic value bound.
   - We then update the `Arc<AtomicU32>` threshold for the segment. When `try_get_batch_ids()` iterates the scorer, `SeekableScorer` will instantly jump to the target `DocId`, efficiently skipping irrelevant blocks.

----

It's also possible that some of this would be easier to do on the Tantivy side, and if there is a reasonable stable API that we can expose.

### Full Name:

Stu Hood

### Affiliation:

ParadeDB