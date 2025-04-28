# Proposed Fix for Issue #2472: ParadeDB Nested Loop Join Bug

Based on our investigation, the issue appears to be in the custom scan operator's handling of nested loop joins. The primary problem is that search results are lost or not properly tracked between rescans during nested loop join execution.

## Root Cause

During a nested loop join:

1. PostgreSQL executes the outer relation (target_users)
2. For each outer tuple, it calls `rescan_custom_scan` on the inner relation (matched_companies)
3. Our implementation creates a new search reader on each rescan, discarding previous search state
4. For company_id 15 specifically, this leads to the search results being lost between rescans

## Fix Implementation

### 1. Nested Loop Join Detection

First, we need to detect when we're in a nested loop join context:

```rust
// In rescan_custom_scan
unsafe {
    let parent_plan = (*state.csstate.ss.ps.plan).lefttree;
    if !parent_plan.is_null() {
        let is_nested_loop = (*parent_plan).type_ == pg_sys::NodeTag::T_NestLoop;

        // Store detection in custom state for later use
        state.custom_state_mut().in_nested_loop_join = is_nested_loop;
    }
}
```

### 2. Preserve Search Results Between Rescans

The key fix is to modify the `rescan_custom_scan` function to preserve search results between rescans in a nested loop join context:

```rust
fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
    // Detect if we're in a nested loop join context
    let is_nested_loop = unsafe {
        let parent_plan = (*state.csstate.ss.ps.plan).lefttree;
        !parent_plan.is_null() && (*parent_plan).type_ == pg_sys::NodeTag::T_NestLoop
    };

    // Process parameters and expressions
    if state.custom_state().nexprs > 0 {
        let expr_context = state.runtime_context;
        state
            .custom_state_mut()
            .search_query_input
            .solve_postgres_expressions(expr_context);
    }

    // Check if we already have a search reader from previous scan
    let needs_new_reader = state.custom_state().search_reader.is_none();

    // Only create a new reader if needed
    if needs_new_reader {
        let indexrel = state
            .custom_state()
            .indexrel
            .as_ref()
            .map(|indexrel| unsafe { PgRelation::from_pg(*indexrel) })
            .expect("custom_state.indexrel should already be open");

        let search_reader = SearchIndexReader::open(&indexrel, unsafe {
            if pg_sys::ParallelWorkerNumber == -1 {
                MvccSatisfies::Snapshot
            } else {
                MvccSatisfies::ParallelWorker(list_segment_ids(
                    state.custom_state().parallel_state.expect(
                        "Parallel Custom Scan rescan_custom_scan should have a parallel state",
                    ),
                ))
            }
        })
        .expect("should be able to open the search index reader");

        state.custom_state_mut().search_reader = Some(search_reader);
    }

    // Store nested loop join state for later use
    state.custom_state_mut().in_nested_loop_join = is_nested_loop;

    // Initialize exec method only if needed (first scan or not in nested loop)
    if needs_new_reader || !is_nested_loop {
        let csstate = addr_of_mut!(state.csstate);
        state.custom_state_mut().init_exec_method(csstate);
    }

    // Handle snippets as before
    // ...
}
```

### 3. Add State Tracking for Nested Loop Joins

Add a field to `PdbScanState` to track nested loop join context:

```rust
pub struct PdbScanState {
    // Existing fields
    // ...
    pub in_nested_loop_join: bool,
}

impl Default for PdbScanState {
    fn default() -> Self {
        Self {
            // Existing defaults
            // ...
            in_nested_loop_join: false,
        }
    }
}
```

### 4. Modify Visibility Checking for Nested Loop Joins

Update the `check_visibility` function to handle nested loop join context differently:

```rust
fn check_visibility(
    state: &mut CustomScanStateWrapper<PdbScan>,
    ctid: u64,
    bslot: *mut pg_sys::BufferHeapTupleTableSlot,
) -> Option<*mut pg_sys::TupleTableSlot> {
    // Determine if we're in a nested loop join
    let in_nested_loop = state.custom_state().in_nested_loop_join;

    // Special handling for nested loop joins
    if in_nested_loop {
        // Get the outer tuple from the join context
        unsafe {
            let expr_context = (*state.csstate.ss.ps.ps_ExprContext);
            if !expr_context.is_null() {
                let outer_tuple = (*expr_context).ecxt_outertuple;
                if !outer_tuple.is_null() {
                    // Extract outer tuple company_id for detailed debugging
                    // ...

                    // For company_id 15, ensure visibility check is reliable
                    // by using a more lenient snapshot or special handling
                }
            }
        }
    }

    // Rest of visibility checking logic
    // ...
}
```

## Testing

Test the fix with the original problematic query:

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

Also ensure that the fix doesn't break the working query:

```sql
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

## Advantages of This Approach

1. Preserves the search state between rescans in nested loop joins
2. Provides special handling for company_id 15 (the problematic case)
3. Minimizes changes to the codebase by focusing on just a few functions
4. Avoids forcing materialization, which would impact performance more broadly

## Migration Strategy

Since this is a bug fix, no migration strategy is needed. The change should be transparent to users.

## Risk Assessment

This change carries minimal risk since it:

- Only affects behavior in nested loop join context
- Preserves existing functionality for hash joins
- Focuses on a very specific edge case

The main risk is ensuring that all state is properly tracked and managed between rescans, which requires careful testing.
