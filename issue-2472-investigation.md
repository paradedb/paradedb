# Investigation Plan for Issue #2472: ParadeDB Nested Loop Join Bug

## Problem Confirmation
We have confirmed the issue with a simple test case:

1. When using a query with a `status = 'NORMAL'` condition that forces a **Nested Loop Join**:
   ```sql
   WITH target_users AS (
       SELECT u.id, u.company_id
       FROM "user" u
       WHERE 
         u.status = 'NORMAL' AND
         u.company_id in (13, 15)
   ),
   matched_companies AS (
       SELECT c.id, paradedb.score(c.id) AS company_score
       FROM company c
       WHERE c.id @@@ 'name:Testing'
   )
   SELECT
       u.id,
       u.company_id,
       mc.id as mc_company_id,
       COALESCE(mc.company_score, 0) AS score
   FROM target_users u
   LEFT JOIN matched_companies mc ON u.company_id = mc.id;
   ```
   The join fails to match company_id 15 even though it exists and matches the search criteria.

2. When the query is modified to remove the `status = 'NORMAL'` condition, forcing a **Hash Join**:
   The join correctly includes company_id 15 in the results.

## Hypothesized Root Cause

The issue is most likely related to our custom scan operator's handling of rescans during nested loop join execution. When PostgreSQL performs a nested loop join, it:

1. For each row in the outer relation (target_users)
2. Rescans the inner relation (matched_companies with the custom scan operator)
3. For each matching row, produces a result

Our custom scan operator appears to be losing state or not correctly preserving search results between rescans, specifically when handling company_id 15.

## Implementation Problems to Investigate

### 1. State Reset During Rescans

- The `rescan_custom_scan` function creates a new `SearchIndexReader` each time
- This may reset any ongoing searches or lose the current position in the index
- For company_id 15, the search results might be lost between inner loop iterations

### 2. Visibility Checking Issues

- When in a nested loop context, the visibility checking for company_id 15 may be failing
- There could be an interaction between the MVCC snapshot used and the custom scan
- The same row might pass visibility checks in a hash join but fail in a nested loop

### 3. Search Result Iteration

- The `ExecMethod::next` and `internal_next` functions might not handle nested loop iterations correctly
- When the operator is asked to rescan, it might not reset internal state properly

## Exact Code Locations to Modify

1. **Rescan Logic**
   - Modify `rescan_custom_scan` in `pg_search/src/postgres/customscan/pdbscan/mod.rs`
   - Ensure search state is properly preserved between rescans
   - Add special handling for detecting and handling nested loop joins

2. **Visibility Checking**
   - Enhance `check_visibility` function to better handle nested loop join context
   - Possibly add special handling for different join types

3. **Nested Loop Detection**
   - Improve detection of nested loop joins to apply specialized handling
   - Consider using more precise context tracking during query execution

## Testing Plan

1. **Reproduce with Detailed Logging**
   - Add extensive logging to track the execution flow in both join types
   - Specifically focus on what happens with company_id 15
   - Log all nested loop iterations and visibility checks

2. **Verify Fix with Complex Cases**
   - Test with various filter combinations and JOIN types
   - Ensure the fix doesn't break other scenarios
   - Add regression tests for this specific case

## Alternative Approaches

If direct fixing of the scanning behavior is too complex, consider:

1. **Query Hint Approach**
   - Add a query hint mechanism to suggest Hash Join over Nested Loop
   - Less intrusive than restructuring the custom scan operator

2. **Force Materialization**
   - As a last resort, implementing auto-materialization for CTEs with ParadeDB operators
   - Would require identifying cases where materialization would help

## Implementation Timeline

1. **Further Investigation with Detailed Logging**: 1-2 days
2. **Fix Implementation**: 2-3 days
3. **Comprehensive Testing**: 1-2 days
4. **Code Review and Documentation**: 1 day

## Notes on PostgreSQL Join Execution

For reference, PostgreSQL join execution works as follows:

- **Nested Loop Join**:
  - For each outer tuple, scan the inner relation for matches
  - Rescan calls are made for each new outer tuple
  - State between inner scans should be completely reset unless explicitly saved

- **Hash Join**:
  - Build a hash table from the inner relation once
  - Probe the hash table for each outer tuple
  - No rescans are needed for the inner relation

Our custom scan operator likely has issues with the rescan pattern in the nested loop execution strategy. 
