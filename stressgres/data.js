window.BENCHMARK_DATA = {
  "lastUpdate": 1752168363542,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search bulk-updates.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168057334,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 2.536868810038378,
            "unit": "median tps",
            "extra": "avg tps: 3.650278702049354, max tps: 6.809720188752096, count: 21239"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 4.85396547468231,
            "unit": "median tps",
            "extra": "avg tps: 5.55575596198306, max tps: 7.042050255753952, count: 21239"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752168066344,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 24.096386,
            "unit": "median cpu",
            "extra": "avg cpu: 21.7662600606495, max cpu: 44.17178, count: 21239"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.39453125,
            "unit": "median mem",
            "extra": "avg mem: 224.5520188391638, max mem: 231.8671875, count: 21239"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 24.096386,
            "unit": "median cpu",
            "extra": "avg cpu: 22.225273188478045, max cpu: 33.939396, count: 21239"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.99609375,
            "unit": "median mem",
            "extra": "avg mem: 161.98778595684826, max mem: 163.625, count: 21239"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18803,
            "unit": "median block_count",
            "extra": "avg block_count: 19348.14492207731, max block_count: 22835.0, count: 21239"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.24826969254673, max segment_count: 59.0, count: 21239"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Moe",
            "username": "mdashti",
            "email": "mdashti@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe",
          "message": "feat: Added a GUC (`enable_custom_scan_for_non_indexed_fields`) for handling non-indexed fields in queries (#2767)\n\n## What\n\n- Disabled using bm25 index for handling queries that require\nHeapFilter, but do not use the `@@@` operator.\n- Added a new GUC `paradedb.enable_filter_pushdown` to control whether\nParadeDB's custom scan should handle queries that include non-indexed\nfield predicates.\n\n## Why\n\nThis GUC provides users with fine-grained control over when ParadeDB's\ncustom scan is used, particularly for queries that mix indexed and\nnon-indexed predicates. This is useful for:\n\n- **Performance tuning**: Users can compare custom scan performance\nagainst standard PostgreSQL execution\n- **Debugging**: Helps isolate issues related to HeapExpr filtering vs\nstandard execution\n- **Backward compatibility**: Allows disabling the feature if it causes\nissues in specific scenarios\n\n## How\n\n1. **Added GUC definition** in `src/gucs.rs`.\n2. **Integrated GUC checks** in\n`src/postgres/customscan/pdbscan/qual_inspect.rs`\n\n## Tests\n\nAdded a regression test (Test Case 19) in\n`score_non_indexed_predicates.sql`\n\nThe test shows that users can control execution strategy with:\n```sql\nSET paradedb.enable_filter_pushdown = false; -- Disable HeapExpr\nSET paradedb.enable_filter_pushdown = true;  -- Enable HeapExpr (default)\n```",
          "timestamp": "2025-07-05T18:06:46Z",
          "url": "https://github.com/paradedb/paradedb/commit/70f65d99d8f6cd7c112b8c4aa00d8c410b55efbe"
        },
        "date": 1752168362644,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 439.7556089628702,
            "unit": "median tps",
            "extra": "avg tps: 441.30051468238287, max tps: 611.3181645734521, count: 58468"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2725.6829093015035,
            "unit": "median tps",
            "extra": "avg tps: 2728.048010559734, max tps: 2919.859012613137, count: 58468"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 439.53250327714164,
            "unit": "median tps",
            "extra": "avg tps: 441.1274851517368, max tps: 616.2216612116403, count: 58468"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 393.8081740222367,
            "unit": "median tps",
            "extra": "avg tps: 394.0111507703895, max tps: 487.77463602134713, count: 58468"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 290.23996597527207,
            "unit": "median tps",
            "extra": "avg tps: 293.5999904956123, max tps: 338.6579239144033, count: 116936"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 250.93976299669234,
            "unit": "median tps",
            "extra": "avg tps: 251.80889078572957, max tps: 277.51842536940194, count: 58468"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.452775520804673,
            "unit": "median tps",
            "extra": "avg tps: 24.291048794026157, max tps: 1741.8719898274674, count: 58468"
          }
        ]
      }
    ]
  }
}