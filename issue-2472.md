# Plan for Issue #2472: Query Planner Bug with BM25 Search Results

## Issue Summary
PostgreSQL's query planner fails to properly join records with ParadeDB BM25 search results under specific conditions:
- Multiple tables with relationships
- A filter on company_id that includes company_id 15
- ParadeDB fulltext search on companies
- A GROUP BY clause following JOIN operations

This results in missing matches in the query results, despite the data existing and matching the search criteria.

## Investigation Approach

### 1. Query Plan Analysis
- Compare EXPLAIN ANALYZE output from both failing and working queries
- Focus specifically on:
  - Join order differences
  - Materialization behavior
  - How the custom BM25 scan operator is integrated into the plan
  - Filter pushdown behavior

```sql
-- Capture detailed query plans for analysis
EXPLAIN (ANALYZE, VERBOSE, BUFFERS) WITH target_users AS (...)
-- [rest of problematic query]

-- Repeat for workarounds
```

### 2. Root Cause Analysis
- Determine why the planner makes different decisions for company_id 15
- Check if the issue is related to:
  - Cost estimation for the ParadeDB custom scan
  - Inability to push filters to the BM25 scan
  - Join ordering heuristics when faced with custom scan operators
  - How selectivity is calculated for custom scans

### 3. Isolation Test
- Create minimal test cases that gradually add complexity until the issue appears
- Identify exact trigger conditions that cause the planner to make incorrect decisions

## Solution Approach

Based on investigation results, we'll likely need to modify one of these components:

### Option 1: Improve Custom Scan Integration
- Enhance the ParadeDB scan operator's interaction with PostgreSQL's planner
- Ensure proper passing of parameter information between scan stages
- Add hints to the planner about materialization needs

```c
// In particular, examine these functions in our custom scan implementation:
// - GetParadedbRelSize()
// - GetParadedbPaths()
// - BeginParadedbScan()
// - ParadedbNext()
```

### Option 2: Enhance Cost Model
- Update cost estimation for ParadeDB scan operations
- Make path costs more accurate for common join patterns
- Ensure selectivity estimates account for filter conditions correctly

### Option 3: Add Auto-Materialization Logic
- Add logic to automatically materialize certain CTEs when ParadeDB scans are involved
- Detect cases where materialization would benefit plan quality

## Implementation Plan

1. Complete investigation to determine exact issue (1-2 days)
2. Create minimal test/reproduction case (0.5 day)
3. Implement fix in the identified component (1-3 days, depending on complexity):
   - If custom scan issue: Modify how our scan operator integrates with planner
   - If cost model issue: Update costing functions
   - If materialization issue: Add auto-materialization detection
4. Add regression tests to ensure issue doesn't return (0.5 day)
5. Test against realistic workloads (0.5 day)
6. Submit PR with documentation of the fix (0.5 day)

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
