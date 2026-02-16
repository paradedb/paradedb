# Feedback: Safety & Tests

## Summary

The testing strategy covers the core functionality and the integration test `test_dsm_gather_execution` is particularly valuable as it exercises the full RPC loop.

## Strengths

1.  **Integration Test:** `test_dsm_gather_execution` in `dsm_test.rs` validates the most complex path: Leader requesting data -> Worker starting task -> Data flowing back.
2.  **SQL Regression Tests:** The SQL tests cover various join scenarios (driving side, build side, both) and force parallelism, ensuring the code paths are exercised.

## Issues & Recommendations

### 1. Error Propagation

**Observation:**
If a background `producer_task` panics or fails, how does the `DsmExchangeExec` (Consumer) know?

- The `SocketBridge` might detect a disconnect (if process dies).
- But if the task just errors (e.g. DataFusion error), it logs a warning and exits.
- The `DsmExchangeExec` (Consumer) will likely wait forever (or until timeout/hang) if the writer stops without sending EOS (len=0).
  **Analysis:**
  In `producer_task`:

```rust
Err(e) => {
    pgrx::warning!(...);
    return Err(e);
}
```

It returns, dropping the `writers`.
`DsmSharedMemoryWriter::finish` is NOT called on error.
The `writer` (StreamWriter) is dropped.
Does `StreamWriter` drop send EOS? No.
**Risk:**
If a producer task fails, the consumer will hang waiting for data.
**Recommendation:**
Wrap the `producer_task` body in a `catch_unwind` or ensure that a "Stream Error" control message or EOS is sent in the `Err` path.
_Alternative:_ The `MultiplexedDsmWriter` could have a `poison()` method that sets a flag in the header, causing readers to error.

### 3. Test Coverage

**Observation:**
The tests cover happy paths well.
**Recommendation:**
Add a test case where the producer **fails** (e.g. injects an error) to verify that the query fails gracefully instead of hanging. This addresses the "Error Propagation" concern above.
