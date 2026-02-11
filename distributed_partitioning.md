# HOWTO: Distributed Plan Partitioning in DataFusion (MPP Model)

This document describes how modern distributed SQL engines (such as Databend, InfluxDB, GreptimeDB, HoraeDB, and CeresDB) leverage Apache DataFusion to partition and execute query plans across a cluster. Unlike stage-based "Bulk Synchronous Parallel" (BSP) systems like Apache Spark or Ballista, these systems follow a Massively Parallel Processing (MPP) model, where data is streamed between operators across nodes without mandatory materialization at stage boundaries.

## 1. Core Concepts: Partitioning and Distribution

In DataFusion, parallel execution is built around the concept of **Partitions**. An `ExecutionPlan` is executed by calling its `execute(partition, context)` method, which returns a `SendableRecordBatchStream` for that specific partition.

### DataFusion APIs:
*   **[`Partitioning`](https://docs.rs/datafusion/latest/datafusion/physical_expr/partitioning/enum.Partitioning.html)**: Describes how the output of an `ExecutionPlan` is split.
    *   `RoundRobinBatch(n)`: Rows are distributed round-robin across $n$ partitions.
    *   `Hash(exprs, n)`: Rows are distributed based on the hash of expressions across $n$ partitions.
    *   `UnknownPartitioning(n)`: $n$ partitions exist, but the distribution logic is opaque.
*   **[`Distribution`](https://docs.rs/datafusion/latest/datafusion/physical_expr/partitioning/enum.Distribution.html)**: Describes the *requirement* an operator has for its input data.
    *   `SinglePartition`: Requires all data in one stream (e.g., for a global `LIMIT`).
    *   `HashPartitioned(exprs)`: Requires data to be partitioned by specific keys (e.g., for a `HashJoin` or `HashAggregate`).
    *   `UnspecifiedDistribution`: No specific requirement.

## 2. The MPP "Streaming Exchange" Model

In an MPP system, the query plan is "cut" into fragments (also called "sub-plans" or "stages") that are distributed to different nodes. The connection between these fragments is handled by an **Exchange** operator.

In DataFusion, the local version of this is **[`RepartitionExec`](https://docs.rs/datafusion/latest/datafusion/physical_plan/repartition/struct.RepartitionExec.html)**. Distributed engines extend this concept by implementing custom `ExecutionPlan` nodes that move data over the network.

### How it works:
1.  **Logical Planning**: The SQL query is parsed into a `LogicalPlan`.
2.  **Physical Planning**: The `LogicalPlan` is converted into a physical `ExecutionPlan`.
3.  **Distribution Optimization**: The optimizer inspects `required_input_distribution()` for each node. If a node's requirement (e.g., `HashPartitioned`) is not met by its child's `output_partitioning()`, a "Distributed Exchange" is injected.
4.  **Fragmenting**: The plan is split at Exchange boundaries into fragments. A fragment is a subtree of the execution plan that can be executed on a single worker node.

## 3. The $M \times N$ Connectivity Problem

When a shuffle (repartitioning) occurs between two fragments in a cluster of $K$ nodes, every producer node potentially needs to send data to every consumer node. This is known as an **All-to-All Exchange**.

### Example: 4-Node Cluster
Consider a cluster with 4 machines (Nodes 1, 2, 3, and 4) performing a `HashJoin`.
- **Fragment A (Producers)**: Scans a table and performs a hash-based repartition on the join key.
- **Fragment B (Consumers)**: Receives the repartitioned data and performs the actual join.

If Fragment A and Fragment B are both distributed across all 4 machines:
1.  **Logical Streams**: Each of the 4 producer nodes must be capable of sending data to all 4 consumer nodes. This results in $4 \times 4 = 16$ logical data streams.
2.  **Per-Node Load**: Each `ShuffleWriterExec` on a producer node maintains 4 outgoing logical streams (one for each partition).
3.  **Connectivity Diagram**:
    ```text
    Producers (Fragment A)          Consumers (Fragment B)
    [ Node 1 ] ──────────────────┬──▶ [ Node 1 ]
    [ Node 2 ] ───────────────┐  │  ▶ [ Node 2 ]
    [ Node 3 ] ────────────┐  │  │  ▶ [ Node 3 ]
    [ Node 4 ] ─────────┐  │  │  │  ▶ [ Node 4 ]
                        ▼  ▼  ▼  ▼
                     (16 Logical Paths)
    ```

### Logical Streams vs. Physical Connections
Modern systems (Databend, InfluxDB IOx, GreptimeDB) typically use **Arrow Flight** (built on **gRPC** and **HTTP/2**) to manage this complexity:
- **Multiplexing**: HTTP/2 allows multiple logical streams (the 16 paths) to be multiplexed over a single physical TCP connection between any two nodes.
- **Connection Pooling**: In our 4-node example, Node 1 only needs **3 physical TCP connections** (to Node 2, Node 3, and Node 4). The internal "loopback" to itself is handled via memory.
- **Efficiency**: Total physical connections for a cluster of size $K$ is $K \times (K-1)$. For 4 nodes, this is $4 \times 3 = 12$ physical connections, regardless of the number of concurrent queries or partitions.

## 4. Implementing Distributed Exchanges

To build an MPP engine, you typically implement two custom `ExecutionPlan` nodes:

### A. ShuffleWriterExec (The Producer)
This node sits at the top of a plan fragment on a worker node. It partitions the output of the local plan and "pushes" it to downstream nodes.

```rust
impl ExecutionPlan for ShuffleWriterExec {
    fn execute(&self, partition: usize, context: Arc<TaskContext>) -> Result<SendableRecordBatchStream> {
        let input_stream = self.input.execute(partition, context)?;
        // 1. Evaluate partitioning expressions (e.g., hash the join key).
        // 2. Buffer batches for each destination partition.
        // 3. Send batches over the network (e.g., Flight Put) as they reach size limits.
        Ok(Box::pin(FlightSenderStream::new(input_stream, self.destinations)))
    }
}
```

### B. ShuffleReaderExec (The Consumer)
This node sits at the bottom of a plan fragment. It "pulls" (or receives pushed) data from all upstream workers.

```rust
impl ExecutionPlan for ShuffleReaderExec {
    fn properties(&self) -> &PlanProperties {
        // The reader defines the output partitioning for the next fragment.
        // E.g., if we are node 1 of 4, our execute(0, ...) will return 
        // partition index 0.
        &self.cache 
    }

    fn execute(&self, partition: usize, context: Arc<TaskContext>) -> Result<SendableRecordBatchStream> {
        // 1. Identify all N upstream nodes producing data for this partition.
        // 2. Open N logical streams (via Flight Get or similar).
        // 3. Merge these N streams into a single SendableRecordBatchStream.
        Ok(self.fetch_remote_partition(partition))
    }
}
```

## 5. Pipelined vs. Staged Execution

The key differentiator in MPP systems (Databend, InfluxDB, etc.) is the lack of "Shuffle Barriers."

*   **Pipelined Execution**: As soon as a `RecordBatch` is produced by a `Scan` on Node 1, it is hashed, sent via `ShuffleWriterExec` over the network, and received by `ShuffleReaderExec` on Node 3, which immediately feeds it into a `Join`.
*   **Backpressure**: These systems leverage `SendableRecordBatchStream`, which is a `Stream` of `Future`s. If Node 3's `Join` is slow (e.g., waiting for memory), the `ShuffleReaderExec` stops polling its network buffers. The gRPC layer then applies HTTP/2 flow control, which eventually pauses the `Scan` on Node 1.
*   **Fault Tolerance**: Unlike Ballista/Spark, if a node fails during this process, the entire pipeline typically fails and the query is restarted, as there are no intermediate checkpoints on disk.

## 6. Summary of DataFusion API Usage for MPP

| Task | DataFusion API |
| :--- | :--- |
| **Defining Data Flow** | `ExecutionPlan::execute` returning `SendableRecordBatchStream` |
| **Expressing Parallelism** | `ExecutionPlan::properties().output_partitioning()` |
| **Enforcing Join/Agg Alignment** | `ExecutionPlan::required_input_distribution()` returning `Distribution::HashPartitioned` |
| **Inserting Exchanges** | `EnforceDistribution` physical optimizer rule |
| **Memory Management** | `MemoryPool` and `RuntimeConfig` to manage stream buffers |
| **Inter-node Transport** | Custom `ExecutionPlan` + `datafusion_proto` for plan serialization + `Arrow Flight` for data |

By using these APIs, MPP engines achieve sub-second latency for interactive queries by keeping data in flight and utilizing all cluster cores simultaneously without the overhead of intermediate disk I/O.