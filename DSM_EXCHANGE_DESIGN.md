# Unified DsmExchangeExec Design

## Objective

Merge the separate `DsmReaderExec` and `DsmWriterExec` nodes into a single `DsmExchangeExec` node to simplify the query plan structure while maintaining the "Lazy Request" / "RPC-style" distributed execution model.

## Core Design

### 1. Unified Node Structure

The `DsmReaderExec` (Consumer) and `DsmWriterExec` (Producer) will be replaced by:

```rust
pub struct DsmExchangeExec {
    /// The plan fragment that produces data (formerly DsmWriterExec.input).
    pub input: Arc<dyn ExecutionPlan>,
    /// How the producer partitions its output (formerly DsmWriterExec.partitioning).
    pub producer_partitioning: Partitioning,
    /// How this node appears to its parent (formerly DsmReaderExec.properties.output_partitioning).
    pub properties: PlanProperties,
    pub config: DsmExchangeConfig,
}
```

### 2. Execution Logic

The node acts as both the "Service Definition" (Producer) and the "Client" (Consumer), depending on how it is invoked.

#### As a Consumer (Client)

When `execute(partition)` is called by DataFusion on the parent node:

1.  It **does not** execute `self.input`.
2.  It sends a `StartStream` control message to the remote (or local) producer via `DsmReaderExec`'s original logic (reused in `DsmExchangeExec`).
3.  It returns a stream that reads from the shared memory ring buffer.

#### As a Producer (Service)

When the plan is deserialized or initialized:

1.  The node registers itself in the process-local `StreamRegistry`.
2.  It stores `self.input` and `self.producer_partitioning` as the "Procedure" definition.

When a `StartStream` request is received:

1.  The Control Service spawns `DsmExchangeExec::producer_task`.
2.  This task executes `self.input`, partitions the output according to `producer_partitioning`, and writes to the ring buffer.

### 3. Changes by File

#### `pg_search/src/postgres/customscan/joinscan/exchange.rs`

- **Rename**: `collect_dsm_writers` -> `collect_dsm_exchanges`.
- **Struct**: Define `DsmExchangeExec` replacing Reader/Writer structs.
- **Logic**:
  - Move `DsmWriterExec::producer_task` to `DsmExchangeExec`.
  - Move `DsmReaderExec::create_consumer_stream` to `DsmExchangeExec`.
  - Update `trigger_stream` to call `DsmExchangeExec::producer_task`.
- **Optimizer**: Update `EnforceDsmShuffle` to output `DsmExchangeExec`.

#### `pg_search/src/scan/codec.rs`

- **Enum**: Update `PhysicalNode` to have a `DsmExchange` variant instead of `DsmReader` and `DsmWriter`.
- **Decoding**:
  - When decoding `DsmExchange`, immediately call `register_stream_source` to populate the registry.
  - This ensures that simply deserializing the plan on a worker prepares it to handle requests.

#### `pg_search/src/postgres/customscan/joinscan/parallel.rs`

- **Tests**: Update manual plan construction in tests to use `DsmExchangeExec`.

## Benefits

- **Simplified Plans**: Reduced node count in `EXPLAIN` and internal representation.
- **Consistency**: A single node represents the network boundary.
- **Preserved Semantics**: The strict "Request -> Execute" causality is maintained.
