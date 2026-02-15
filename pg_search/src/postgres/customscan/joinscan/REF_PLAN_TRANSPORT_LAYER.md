# Refactoring Plan: Unified Transport Layer for JoinScan

## Objective

Decouple the "MPP Logic" (Plan execution, RPC dispatch) from the "Data Transport" (Shared Memory shuffling, Arrow IPC). This involves merging `dsm_stream.rs` and `dsm_transfer.rs` into a unified `transport` module and encapsulating low-level details.

## 1. Directory Structure Changes

Create a new directory `pg_search/src/postgres/customscan/joinscan/transport/`.

| Old File | New Location | Role |
| :--- | :--- | :--- |
| `dsm_stream.rs` | `transport/shmem.rs` | **Private.** Raw Shared Memory Ring Buffer, Pointers, UDS `SignalBridge`, Basic Framing. |
| `dsm_transfer.rs` | `transport/arrow.rs` | **Private.** Adapts `shmem` to Arrow `StreamWriter` / `StreamDecoder`. Handles "double-buffering". |
| N/A | `transport/mesh.rs` | **Public.** Encapsulates `DsmMesh` and the initialization logic (pointer math) currently in `parallel.rs`. |
| N/A | `transport/mod.rs` | **Public.** Exports high-level types (`DsmSender`, `DsmReceiver`) and the `ControlService` abstraction. |

## 2. Encapsulation & Logic Changes

### A. Decouple Control Protocol (`transport/mod.rs` & `transport/shmem.rs`)
- **Goal:** `shmem.rs` should not know about `ControlMessage::StartStream`.
- **Action:**
    - Change `MultiplexedDsmWriter::read_control_messages` to return raw `Vec<u8>` frames (or a generic/opaque type).
    - Move the `ControlMessage` enum definition to `transport/mod.rs` (or `exchange.rs` if strictly MPP logic, but `transport` is a good place if it's the "Transport Protocol").
    - Implement the decoding/dispatch loop in `transport/mod.rs` or `transport/control.rs`.

### B. Abstract Mesh Initialization (`transport/mesh.rs`)
- **Goal:** Remove unsafe pointer arithmetic and layout knowledge from `parallel.rs`.
- **Action:**
    - Create a `TransportMesh` struct (replacing/wrapping `DsmMesh`).
    - Implement a `TransportMesh::init(base_ptr: *mut u8, total_size: usize, n_participants: usize, ...)` function.
    - Move the logic that loops over `total_participants` and calculates offsets from `parallel.rs` into this init function.

### C. Centralize Control Service (`transport/mod.rs`)
- **Goal:** `exchange.rs` should not manually poll `mux_writers`.
- **Action:**
    - Expose a `spawn_control_service` function from `transport`.
    - It accepts a closure/callback: `F: Fn(ControlMessage) -> Future<...>`.
    - `exchange.rs` passes the `trigger_stream` logic as the callback.

## 3. Implementation Steps

1.  **Scaffold**: Create `transport/` directory and move files. Update `mod.rs` to expose `transport`.
2.  **Refactor `shmem.rs`**: Rename `dsm_stream.rs` content. Remove `ControlMessage` specifics if possible (or keep them for now and refactor in step 4). Make it `pub(super)`.
3.  **Refactor `arrow.rs`**: Rename `dsm_transfer.rs` content. Update imports to point to `super::shmem`. Make it `pub(super)`.
4.  **Implement `transport/mod.rs`**: Re-export necessary types. Implement the unified `spawn_control_service`.
5.  **Implement `transport/mesh.rs`**: Move the pointer math from `parallel.rs` here.
6.  **Update `exchange.rs`**: Use `transport::*`. Remove direct dependency on `dsm_stream`/`dsm_transfer`. Update `spawn_control_service` usage.
7.  **Update `parallel.rs`**: Use `transport::Mesh::init`. Remove raw pointer calculations.
8.  **Update `scan_state.rs` / `mod.rs`**: Fix imports.
9.  **Verify**: Run tests.

## 4. Expected Public API (`transport::*`)

```rust
pub mod transport {
    pub struct DsmSender { ... } // Wraps Arrow IPC Writer
    pub struct DsmReceiver { ... } // Wraps Arrow IPC Reader
    
    pub enum ControlMessage { StartStream(u32), CancelStream(u32) }
    
    pub struct TransportMesh { ... }
    impl TransportMesh {
        pub unsafe fn init(...) -> Self;
        pub fn register(&self); // Registers to global static
    }
    
    pub fn spawn_control_service<F>(callback: F);
}
```
