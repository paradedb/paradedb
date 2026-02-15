# Plan: RPC-Server MPP Model for Distributed JoinScan

This document outlines the architecture and implementation steps for transitioning ParadeDB's distributed JoinScan from an "Eager Execution" model to an "RPC-Server" model.

## 1. Core Architecture

### The RPC Analogy
- **Procedure**: A sub-plan fragment (subtree) that ends at a `DsmWriterExec` node.
- **RPC Server (Worker)**: Hosts a `StreamRegistry` of available Procedures. It sits idle until a request arrives.
- **RPC Client (`DsmReaderExec`)**: When executed, it sends a `StartStream(stream_id)` request to the node(s) hosting the required partitions.
- **RPC Handler (`producer_task`)**: Triggered by the worker's Control Service. It invokes the local sub-plan and manages the "wire protocol" (partitioning and writing to DSM).

## 2. Implementation Steps

### Step 1: Fragment Registry Refactor (`exchange.rs`)
- **`StreamSource`**: Redefine to store the actual input subtree (`Arc<dyn ExecutionPlan>`) rather than the `DsmWriterExec` node.
- **`DsmWriterExec`**:
    - Make it a **passive marker**.
    - Its `execute()` method will return an empty stream.
    - Its only purpose in the plan tree is to provide metadata during planning and serve as a hook for serialization.
- **`DsmReaderExec`**:
    - Acts as the client.
    - In `execute()`, it sends the `StartStream` message.
    - It maintains its `properties()` to return `UnknownPartitioning`, preventing DataFusion from re-optimizing the distributed boundary.

### Step 2: Automatic Registration via Codec (`codec.rs`)
- **`try_decode`**:
    - When a `DsmWriterExec` is deserialized, the worker will immediately extract its input child.
    - It will call `register_stream_source(child_plan, stream_id)`.
    - This ensures that as soon as a worker receives a plan, it knows all the "Procedures" it is capable of running.
- **Leader Roundtrip**: The leader will perform a local serialization/deserialization roundtrip after mesh registration to ensure its local registry is populated identically to the workers.

### Step 3: Worker "Server" Mode (`parallel.rs`)
- **`JoinWorker::run`**:
    - Remove the call to `physical_plan.execute(0, ...)`.
    - The worker will instead:
        1. Deserialize the plan (populating the registry).
        2. Spawn the `control_service` (the RPC Listener).
        3. Enter a parked state (e.g., waiting on a `tokio::sync::oneshot` or a termination signal from the leader).
- **`run_execution_loop`**: This becomes unnecessary for workers and will be removed.

### Step 4: Refined Request Handler (`exchange.rs`)
- **`trigger_stream`**:
    - Look up the sub-plan by `stream_id`.
    - Spawn a detached `producer_task`.
- **`producer_task`**:
    - This is the detached execution engine.
    - It calls `sub_plan.execute(partition, ctx)`.
    - It uses `datafusion::physical_plan::repartition::BatchPartitioner` to handle the shuffle logic.
    - It writes the resulting fragments to the DSM ring buffers.

### Step 5: Termination Signaling
- **Teardown**:
    - The Leader, upon completing the root plan, will send a `SessionEnd` signal (or simply drop the DSM segments).
    - Workers will detect the disconnect and exit gracefully.

## 3. Benefits
- **Zero Drift**: Absolute consistency between leader and workers via physical plan serialization.
- **Lazy Efficiency**: Workers only spend CPU cycles on the specific fragments requested by the leader.
- **Decoupled Logic**: The sub-plan execution doesn't know it's being streamed; the transport logic is handled entirely by the "RPC" wrapper.
