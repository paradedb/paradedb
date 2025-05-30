# TopN Join Implementation Plan

## Overview

The TopN join optimization is designed to efficiently handle queries with both `LIMIT` and `ORDER BY` clauses in join operations. This optimization is crucial for queries that join tables with search predicates and only need a small subset of results.

## Motivation

Current join execution retrieves all matching documents from both sides before applying LIMIT:
- Example: 39K outer results × 39K inner results = 1.5B combinations to evaluate
- With TopN: 200 outer × 200 inner = 40K combinations (37,500× reduction)

## Implementation Steps

### Step 1: Create TopN Join Execution State Structure

**File**: `pg_search/src/postgres/customscan/pdbscan/join_exec_methods/top_n_join.rs`

```rust
pub struct TopNJoinExecState {
    // Configuration
    limit: usize,
    sort_direction: SortDirection,
    need_scores: bool,
    
    // Relations
    outer_relid: pg_sys::Oid,
    inner_relid: pg_sys::Oid,
    search_predicates: Option<JoinSearchPredicates>,
    
    // TopN specific
    match_buffer: Vec<ScoredJoinMatch>,
    buffer_position: usize,
    
    // Limited search results
    outer_results: Vec<(u64, f32)>,
    inner_results: Vec<(u64, f32)>,
    
    // State tracking
    found_visible: usize,
    chunk_size: usize,
    retry_count: usize,
    is_exhausted: bool,
}
```

Key features:
- Maintains a sorted buffer of only the top N matches
- Uses limited search results instead of full result sets
- Supports adaptive expansion when more results are needed

### Step 2: Add TopN Join to ExecMethodType

**File**: `pg_search/src/postgres/customscan/builders/custom_path.rs`

Add new variant:
```rust
TopNJoin {
    limit: usize,
    sort_direction: SortDirection,
    need_scores: bool,
    outer_relid: pg_sys::Oid,
    inner_relid: pg_sys::Oid,
}
```

### Step 3: Add GUC for TopN Join Control

**File**: `pg_search/src/gucs.rs`

Add:
```rust
static ENABLE_TOPN_JOIN_OPTIMIZATION: GucSetting<bool> = GucSetting::new(true);

pub fn is_topn_join_optimization_enabled() -> bool {
    ENABLE_TOPN_JOIN_OPTIMIZATION.get()
}
```

### Step 4: Implement TopN Join Detection Logic

**Location**: Join planning phase

Detection criteria:
1. Query has a LIMIT clause
2. Join has search predicates (@@@ operators)
3. GUC `paradedb.enable_topn_join_optimization` is enabled
4. Initially limited to bilateral search joins

### Step 5: Implement Core TopN Join Algorithm

**Key Methods**:

1. **`initial_search()`**: 
   - Execute TopN search on outer relation (limited to chunk_size)
   - Execute TopN search on inner relation (limited to chunk_size)
   - Evaluate join conditions for all combinations
   - Sort by combined score, keep only top N

2. **`next_match()`**:
   - Return next tuple from sorted buffer
   - Check visibility
   - Expand search if buffer exhausted and limit not reached

3. **`expand_search()`**:
   - Increase chunk_size by scaling factor
   - Re-execute searches with larger limit
   - Re-evaluate and re-sort matches

### Step 6: Integrate with Join Execution Framework

**Files to modify**:
- `join_exec_methods/mod.rs`: Export TopN join
- `mod.rs`: Wire up TopN join in execution path

### Step 7: Add EXPLAIN Support

Add statistics to EXPLAIN output:
- Number of visible tuples found
- Number of retry expansions
- Total combinations evaluated
- Chunk size used

### Step 8: Testing

Create test cases:
1. Basic TopN join with small LIMIT
2. TopN join with ORDER BY score
3. TopN join with many invisible tuples (tests expansion)
4. Performance comparison vs regular join

## Algorithm Details

### Initial Chunk Size Calculation
```
initial_chunk_size = min(2 * limit, 1000)
```

### Expansion Strategy
```
new_chunk_size = min(old_chunk_size * RETRY_SCALE_FACTOR, MAX_JOIN_CHUNK_SIZE)
```

Where:
- `RETRY_SCALE_FACTOR = 2`
- `MAX_JOIN_CHUNK_SIZE = 10,000` (conservative for joins)

### Score Combination
```
combined_score = outer_score + inner_score
```

### Sorting
- Desc: Higher scores first (typical for relevance)
- Asc: Lower scores first
- None: No specific order (LIMIT without ORDER BY)

## Performance Characteristics

### Benefits
1. **Reduced Search I/O**: Fetch only top candidates from each relation
2. **Reduced Join Processing**: Evaluate far fewer combinations
3. **Early Termination**: Stop once limit is satisfied
4. **Memory Efficiency**: Keep only top N matches in memory

### Trade-offs
1. Multiple search passes if many invisible tuples
2. May not find globally optimal results if early candidates are invisible
3. Conservative chunk sizes to prevent memory explosion

## Future Enhancements

1. **Sort Field Support**: Allow ordering by specific fields, not just score
2. **Multi-way Joins**: Extend to 3+ table joins
3. **Join Key Optimization**: Use fast fields for join conditions
4. **Parallel TopN Join**: Distribute work across workers
5. **Cost-based Decision**: Use statistics to decide when TopN is beneficial

## Success Metrics

1. Query latency reduction: 20-60x for typical LIMIT queries
2. Memory usage: Bounded by limit rather than result set size
3. CPU usage: Proportional to limit, not total matches 
