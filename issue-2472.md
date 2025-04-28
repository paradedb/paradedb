# Plan for Issue #2472: Query Planner Bug with BM25 Search Results

## Issue Summary

PostgreSQL's query planner fails to properly join records with ParadeDB BM25 search results under specific conditions:

- Multiple tables with relationships
- A filter on company_id that includes company_id 15
- ParadeDB fulltext search on companies
- A GROUP BY clause following JOIN operations

This results in missing matches in the query results, despite the data existing and matching the search criteria.

## Reproduction and Verification

We've verified the issue with a simple test case:

```sql
-- With status filter (nested loop join) - Missing company_id 15 in results
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

```
-- Results:
id | company_id | mc_company_id | score
----+------------+---------------+-------
  3 |         13 |               |     0
  4 |         15 |               |     0
```

```sql
-- Without status filter (hash join) - Correct results
WITH target_users AS (
    SELECT u.id, u.company_id
    FROM "user" u
    WHERE u.company_id in (13, 15)
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

```
-- Results:
id | company_id | mc_company_id |   score
----+------------+---------------+------------
  3 |         13 |               |          0
  4 |         15 |            15 | 0.32542244
```

## Root Cause Analysis

We've identified that the issue is directly related to the join strategy chosen by PostgreSQL:

1. **Query Plan with Status Filter:**

   - Uses a **Nested Loop Left Join** (cost=1000.00..1026.50)
   - Join Filter: (u.company_id = c.id)
   - Parallel Custom Scan (ParadeDB Scan) on company
   - Missing company_id 15 in the results

2. **Query Plan without Status Filter:**
   - Uses a **Hash Left Join** (cost=1000.45..1023.85)
   - Hash Cond: (u.company_id = c.id)
   - Results are correctly materialized into a hash table before joining
   - Company_id 15 appears correctly in the results

The problem appears to be related to how our ParadeDB custom scan operator interacts with PostgreSQL's nested loop join execution. When PostgreSQL uses a nested loop, it expects to be able to rescan the inner relation multiple times, which may not be correctly handled by our custom scan operator.

## Detailed Investigation Approach

To pinpoint the exact issue in the code, we'll add detailed logging to the custom scan operator's iteration methods:

### 1. Iterator and Rescan Logic

- Add logging to trace the execution flow of the custom scan operator's iterator functions
- Focus on the `ExecMethod::next` function and related rescan operations
- Track how many times the custom scan is executed for each outer tuple in the nested loop join

```rust
// Areas to add logging:
// - Custom scan iterator (next() method)
// - Rescan functionality
// - State management between iterations
```

### 2. Track Inner/Outer Relation Iteration

We'll monitor how many times the inner relation (ParadeDB scan) is accessed per outer relation row:

- Add counters for number of times the inner scan is executed
- Track which outer rows are being joined
- Verify if the custom scan state is properly reset between iterations

### 3. Visibility Checking

We noticed the visibility check passes for company_id 15 when tested directly but fails during the join with nested loop:

- Add detailed logging to visibility check functions
- Track which rows are being checked for visibility
- Monitor which rows are being filtered out due to visibility issues

## Solution Approach

Based on our improved understanding, we're focusing on these potential solutions:

### Option 1: Fix Iterator Reset Logic

- Ensure the custom scan operator properly resets its state between iterations in a nested loop join
- Add appropriate state tracking to ensure all results are returned for each iteration

### Option 2: Improve Custom Scan Integration

- Enhance how the ParadeDB scan operator interacts with PostgreSQL's join strategies
- Implement proper handling of rescan requests from the PostgreSQL executor

### Option 3: Optimizer Hint to Prefer Hash Joins

- Add logic to the query planner to suggest hash joins when a ParadeDB scan is used in specific join contexts
- This would be less invasive than forcing materialization but still guide the planner to use more compatible join strategies

## Implementation Timeline

- Investigation with added logging: By April 28, 2025
- Fix implementation: By May 2, 2025
- Testing and refinement: By May 5, 2025
- PR submission: By May 6, 2025

## Testing Strategy

1. **Unit Tests**

   - Create tests specifically for the fixed component
   - Test with various data distributions and filter values

2. **Integration Tests**

   - Test the original failure case and variations
   - Create comprehensive test suite with edge cases:
     - Different filter values and combinations
     - Various table sizes
     - Different GROUP BY clauses
     - Multiple JOINs with different conditions

3. **Performance Testing**
   - Ensure fix doesn't degrade performance in other scenarios
   - Benchmark query runtime with and without the fix

## Potential Challenges

1. **PostgreSQL Version Compatibility**

   - The fix may need to be adapted for different PostgreSQL versions
   - Planner behavior changes between major versions

2. **General vs. Specific Fix**

   - Need to balance fixing this specific case vs. general planner improvements
   - Avoid over-optimization for this test case that might harm other queries

3. **Upstream Collaboration**
   - May require discussion with PostgreSQL core if the issue is in the core planner
   - Could require upstreaming changes if the issue is fundamental to custom scan operators

## Implementation Timeline

- Investigation and reproduction: By April 28, 2025
- Initial implementation: By May 2, 2025
- Testing and refinement: By May 5, 2025
- PR submission: By May 6, 2025
