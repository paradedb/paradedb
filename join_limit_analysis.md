# Analysis: JOIN + ORDER BY + LIMIT Performance Issue

## Problem Description

When using complex JOINs with ParadeDB custom scans that include `ORDER BY` and `LIMIT` clauses, PostgreSQL materializes the entire join result before applying the sorting and limiting. This leads to significant performance degradation compared to what could be achieved with early termination.

## Root Cause Analysis

### 1. Limit Detection Logic

In `pg_search/src/postgres/customscan/pdbscan/mod.rs` lines 246-256, the limit detection logic only works for single relations:

```rust
let limit = if (*builder.args().root).limit_tuples > -1.0 {
    // Check if this is a single relation or a partitioned table setup
    let rel_is_single_or_partitioned = pg_sys::bms_equal((*rel).relids, baserels)
        || is_partitioned_table_setup(builder.args().root, (*rel).relids, baserels);

    if rel_is_single_or_partitioned {
        // We can use the limit for estimates if:
        // a) we have a limit, and
        // b) we're querying a single relation OR partitions of a partitioned table
        Some((*builder.args().root).limit_tuples)
    } else {
        None
    }
} else {
    None
};
```

This means that when we have JOINs, `limit` becomes `None`, and therefore `is_topn` (line 270) becomes `false`, preventing the use of TopN execution methods.

### 2. TopN Execution Method Requirements

The TopN execution method (`TopNScanExecState`) can only be used when:
- We have both a limit AND a sort key (`is_topn = limit.is_some() && pathkey.is_some()`)
- The custom scan is for a single relation

### 3. Score Expression Materialization

When using complex score expressions like `paradedb.score(documents.id) + paradedb.score(files.id) + paradedb.score(pages.id)`, PostgreSQL cannot push this computation down to individual scans because it requires data from multiple relations.

## Current Behavior vs Desired Behavior

### Current Behavior:
1. Each table is scanned completely using fast field methods
2. All results are materialized in memory
3. Hash joins are performed on the full result sets
4. The complete joined result is sorted
5. Only then is the LIMIT applied

### Desired Behavior:
1. Use a more intelligent join strategy that can terminate early
2. Implement incremental join processing with top-k maintenance
3. Push partial limits down to individual scans where possible

## Evidence from Query Plan

From the provided query plan:

```
->  Gather Merge  (cost=4304.85..5083.43 rows=6296 width=115) (actual time=2981.483..3029.051 rows=1000 loops=1)
    ->  Sort  (cost=3304.71..3306.67 rows=787 width=115) (actual time=2932.565..2932.620 rows=542 loops=8)
        ->  Parallel Hash Join  (cost=2956.89..3266.85 rows=787 width=115) (actual time=2916.294..2931.656 rows=795 loops=8)
```

The execution time breakdown shows:
- Total time: ~3000ms
- Most time spent in hash joins (~2900ms)
- Actual rows processed: 795 per worker × 8 workers = 6360 total rows
- But only 1000 rows needed for the final result

## Reproduction Script

The `limit_pushdown_repro.sql` script demonstrates:

1. **Test 1**: Single table with ORDER BY + LIMIT → Uses TopN (efficient)
2. **Test 2**: JOIN without ORDER BY → No sorting overhead
3. **Test 3**: JOIN with ORDER BY + LIMIT → Full materialization (inefficient)
4. **Test 4**: Three-way JOIN → Even worse materialization
5. **Test 5**: With parallelism → Shows the exact issue from customer query

## Potential Solutions

### 1. Implement Custom Join Operator (Recommended)

Create a specialized join operator that:
- Maintains a top-k heap during join processing
- Can terminate early when enough results are found
- Integrates with ParadeDB's scoring system

### 2. Improve Limit Pushdown Logic

Modify the limit detection to work with joins:
- Detect when all relations in a join use ParadeDB scans
- Apply heuristic limits to individual scans
- Use incremental join processing

### 3. Query Rewriting

Implement query transformations that:
- Rewrite complex score expressions into subqueries
- Use window functions for efficient top-k processing
- Apply query hints to guide the planner

### 4. PostgreSQL Planner Hooks

Implement custom planner hooks that:
- Recognize ParadeDB join patterns
- Override PostgreSQL's default join order decisions
- Force the use of more efficient join algorithms

## Implementation Priority

1. **Short-term**: Implement workarounds in application logic
2. **Medium-term**: Develop custom join operator for ParadeDB
3. **Long-term**: Improve integration with PostgreSQL's planner

## Workarounds for Users

Until a fix is implemented, users can:

1. **Increase work_mem**: Allow larger in-memory operations
2. **Use pagination differently**: Instead of large LIMIT values, use cursor-based pagination
3. **Restructure queries**: Break complex joins into multiple steps
4. **Use materialized views**: Pre-compute complex joins for frequently accessed data

## Testing Requirements

Any solution should be tested against:
- Single table queries (ensure no regression)
- Two-table joins with various join conditions
- Three+ table joins
- Mixed ParadeDB + regular PostgreSQL tables
- Various ORDER BY expressions (score, field, mixed)
- Different LIMIT sizes (small, medium, large)
- Parallel vs non-parallel execution 
