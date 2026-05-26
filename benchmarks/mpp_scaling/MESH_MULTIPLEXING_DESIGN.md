# MPP Mesh Multiplexing — K=1 Design (BLOCKED — see "Why this is blocked")

## Why this is blocked

The K=1 design as written assumes N-1 peers can each attach as sender to the
same shm_mq inbox. PostgreSQL's `shm_mq` is explicitly single-sender,
single-reader:

- `src/include/storage/shm_mq.h`: "single-reader, single-writer shared
  memory message queue. Each must be set exactly once."
- `shm_mq_set_sender` (`shm_mq.c:224`): `Assert(mq->mq_sender == NULL)` —
  second peer attempting to set itself as sender aborts the backend in debug
  builds, corrupts the queue silently in release.
- `mq_bytes_written` is single-writer by construction; multiple producers
  racing to bump it cause torn frames.

The pg_search wrapper at `pg_search/src/parallel_worker/mqueue.rs:38`
calls `shm_mq_set_sender` on every sender attach, so the implementation
attempted in commit `7b6bd0f5b` would abort the leader's `worker_setup`
the first time real parallel workers ran.

The 26 transport unit tests passed because they all use
`in_proc_channel` (a std::sync::mpsc), not `ShmMqSender`. No CI test
exercised the multi-sender shm_mq attach path.

**Pivot options** (none cheap):

1. **Wait for FFI-relay / G7-MT, then use a per-process serializer.** One
   dedicated thread per process owns the receiver-side shm_mq set_sender,
   and N-1 in-proc channels feed it. Doesn't change DSM size (still N²
   physical edges if we want to bypass the serializer for direct paths) —
   not actually mesh multiplexing.
2. **Custom DSM-backed MPSC ring** to replace shm_mq for the inbox. Wraps
   `dsm_segment`-allocated memory with a lock-free MPSC ring + Latch for
   wakeup. ~400 LOC of new transport primitive. Gets the N² → N savings.
3. **Pair-keyed inboxes** keeping shm_mq SPSC but consolidating receiver-
   side drain. N(N-1) edges (no DSM win); receiver-side has one
   `DrainHandle` over N-1 receivers. Cosmetic only.

Commit `7b6bd0f5b` should be reverted or left on this branch as a
non-functional checkpoint. The original layout (N×(N-1) shm_mq queues) is
what builds clean and runs correctly.

---

## Original design (kept for record)

## Problem

The MPP DSM mesh today allocates `N×(N-1)` shm_mq queues per mesh, one per
directed `(sender_proc, receiver_proc)` pair. With 3 meshes typical per
query, total queue count is `~3·N²`. The N² growth is the binding ceiling
on MPP scaling past N≈8.

Bench data from PR #5155 (1M rows, queue=4MB, target_partitions=2):

| N   | wall (ms) | ratio | mesh edges |
| --- | --------: | ----: | ---------: |
| 4   |       397 |     — |         36 |
| 8   |       449 | 1.13× |        168 |
| 12  |       557 | 1.24× |        396 |
| 16  |       695 | 1.25× |        720 |
| 24  |      1540 | 2.21× |       1800 |

The N=16→24 wall-time ratio (2.21×) matches the mesh-edge ratio (2.50×).
Wall time is tracking N², not N — the mesh is what bends the curve.

## Proposal

Collapse the directed mesh to **one shm_mq inbox per receiver process**.
Each process owns a single inbox; all N-1 peers write into it. Total queue
count drops from `N(N-1)` to `N` per mesh.

The frame demux already keys by `(stage_id, partition)`. We add `sender_proc`
to the demux key so consumers can still attribute each frame to its
originator (used by drain accounting and EOF tracking).

### Trade-offs

| Property                                | Before (N²)               | After (K=1)                            |
| --------------------------------------- | ------------------------- | -------------------------------------- |
| Queues per mesh                         | `N(N-1)`                  | `N`                                    |
| Total DSM (3 meshes, 4MB queue) at N=24 | ~21 MB × 3 = 63 MB        | 0.1 MB × 3 = 0.3 MB                    |
| Per-edge spinlock contention            | None (1 sender per queue) | N-1 senders race per inbox             |
| Per-edge ordering guarantees            | FIFO from one sender      | Interleaved (frames carry sender_proc) |
| `MppFrameHeader` size                   | 16 B                      | 16 B (unchanged)                       |
| Wire format break                       | —                         | Yes (no external compat needed)        |

Contention is the main concern. Mitigated by:

1. shm_mq spinlock holds are short (single producer-consumer ring write).
2. Cooperative drain (already in place) keeps the consumer side draining
   while senders spin.
3. Per-channel buffers downstream of the demux smooth out interleaving for
   the receiver's stage handlers.

## Frame format change

Current `MppFrameHeader` (16 bytes, `repr(C)`):

```text
| magic u32 | flags u32 | stage_id u32 | partition u32 |
```

`flags` uses only the low 8 bits today (FRAME_KIND_MASK = 0xFF). The upper
24 bits are reserved and validated to zero in `kind()`.

New layout — pack `sender_proc` into the upper 16 bits of `flags`:

```text
| magic u32 | flags u32                   | stage_id u32 | partition u32 |
              ^^^^^^^^
              bits  0..8:  frame kind (Batch | Eof)   — unchanged
              bits  8..16: reserved (must be 0)        — unchanged
              bits 16..32: sender_proc                 — NEW
```

- Supports up to 65535 procs (way past anything we'll ever run).
- `MppFrameHeader::batch(stage, partition)` / `eof(stage, partition)` constructors
  gain a `sender_proc: u32` parameter.
- `kind()` parser stays as-is; the reserved-bits check tightens to `flags & 0xFF00`
  (low 16 except kind) instead of `flags & !FRAME_KIND_MASK`.
- Pure local change; no on-disk format, no cross-version compat.

## DSM layout change

`MppDsmHeader::slot_offset(sender, receiver)` becomes
`inbox_offset(receiver)`. The DSM region layout:

```text
| header | plan_bytes | padding | inboxes[0..n_procs] |
```

Each inbox is one `aligned_queue_bytes(queue_bytes)`-sized region. Total
queues-area bytes drops from `n_procs² · aligned_queue_bytes` to
`n_procs · aligned_queue_bytes`.

`compute_dsm_layout` math:

```rust
let queues_total = (n_procs as usize)
    .checked_mul(queue_bytes)
    .ok_or("queues area overflow")?;
```

Was `n_procs² · queue_bytes`. Same overflow + `MPP_DSM_MAX_BYTES` checks.

## Leader init change

`leader_init` calls `shm_mq_create` per slot. Was N² calls; now N calls:

```rust
for r in 0..n_procs {
    let off = header.inbox_offset(r) as usize;
    let mq_addr = unsafe { base.add(off) };
    unsafe { pg_sys::shm_mq_create(mq_addr.cast(), layout.queue_bytes) };
}
```

The `mesh_init create_ms` instrument should drop from O(N²) to O(N) here —
that's the direct verification the change worked.

## Worker / leader attach change

Today: each process attaches as **sender** to its row (N-1 outbound queues)
and **receiver** to its column (N-1 inbound queues), via
`attach_proc_row_and_column`. Total: 2(N-1) `shm_mq_attach` calls per
process.

New:

- Each process attaches as **receiver** to ONE inbox (its own,
  `inbox_offset(this_proc)`).
- Each process attaches as **sender** to N-1 PEER inboxes (peer p's
  `inbox_offset(p)` for all p != this_proc).
- Total: 1 + (N-1) = N attaches per process. Down from 2(N-1).

Split helpers:

```rust
unsafe fn attach_own_inbox(
    base: *mut u8, header: &MppDsmHeader, this_proc: u32, seg: *mut pg_sys::dsm_segment,
) -> ShmMqReceiver { ... }

unsafe fn attach_peer_inboxes(
    base: *mut u8, header: &MppDsmHeader, this_proc: u32, seg: *mut pg_sys::dsm_segment,
) -> Vec<(u32, ShmMqSender)>  // (peer_proc, sender) pairs
```

Returning `(peer_proc, ShmMqSender)` lets the caller build the per-peer
sender table without re-deriving indices.

## `MppMesh` shape change

Today (`runtime.rs`):

```rust
pub struct MppMesh {
    pub this_proc: u32,
    pub n_procs: u32,
    pub inbound_receivers: Vec<Option<Arc<DrainHandle>>>,
    // outbound senders live in MppWorkerState / MppLeaderState
}
```

`inbound_receivers[sender_proc]` was the per-sender drain handle. With one
inbox, the field collapses to:

```rust
pub struct MppMesh {
    pub this_proc: u32,
    pub n_procs: u32,
    pub inbound_drain: Arc<DrainHandle>,  // single drain over the own-inbox shm_mq
    // outbound senders unchanged: Vec<Option<MppSender>> keyed by peer proc_idx
}
```

`DrainHandle` already supports the cooperative-drain spin pattern. It just
pulls bytes from its underlying channel and dispatches frames to per-channel
buffer registries by `(stage_id, partition)`. The change is in the **demux
key**: now `(sender_proc, stage_id, partition)`.

## Drain-side demux change

The channel buffer registry today is keyed by `(stage_id, partition)`. With
multiplexing it becomes `(sender_proc, stage_id, partition)` so the receiver
can route each frame to the correct logical channel without losing the
sender attribution.

EOF accounting: today, `sources_done` for a `(stage, partition)` buffer
increments once per per-sender EOF. The expected `n_sources` is the producer
fragment count for that `(stage, partition)`. That logic is unchanged — we
still see N-1 EOFs from peers, just routed through one shm_mq instead of
N-1.

## `MppSender` change

Today, each `MppSender` wraps an `Arc<dyn BatchChannelSender>` (one per
outbound shm_mq queue) plus a per-instance `MppFrameHeader` template
carrying `(stage_id, partition)`.

With multiplexing, the sender additionally needs `local_proc: u32` (the
proc that owns it) to stamp into outgoing frames. Stamp happens in
`send_batch_traced` / `send_eof_traced` when the header is materialized
into bytes.

```rust
pub struct MppSender {
    channel: Arc<dyn BatchChannelSender>,
    pub header: MppFrameHeader,
    local_proc: u32,  // NEW: stamped into header.flags upper 16 bits on every send
}
```

`MppFrameHeader::write_to` reads `local_proc` from a separate param (so the
header stays `Copy` and the sender owns the proc identity).

## Self-loop handling

Today: `slot(this, this)` is reserved in the grid but uses an
`in_proc_channel` (not the shm_mq slot) for the self-loop because
in-process bypass is much faster than shm_mq round-trip.

With multiplexing: the own-inbox is the process's incoming shm_mq queue.
The self-loop still uses `in_proc_channel` for direct delivery; no DSM
slot needed for `(this, this)`. The senders table:

- `outbound_senders[peer_proc]` for `peer_proc != this_proc` → real
  `MppSender` over the peer's inbox.
- `outbound_senders[this_proc]` → wraps the in-proc channel (matches
  today's pattern at `glue.rs:336`).

No change to self-loop logic; the DSM layout just no longer reserves a
slot for it (saves one queue's worth of DSM per process).

## Invariants

1. **Per-channel ordering** (within one `(sender, stage, partition)` tuple)
   is preserved: a single sender writes serially to the inbox, and shm_mq
   FIFO holds within one writer.
2. **Cross-sender ordering** for the SAME `(stage, partition)` is NOT
   preserved (different senders may interleave). This already holds today
   for the gather-side merge across senders — consumers don't depend on
   cross-sender ordering.
3. **Frame atomicity**: each frame is one `try_send_bytes` call, which
   shm_mq guarantees atomic for messages up to ring size.
4. **EOF semantics**: the per-channel buffer's `sources_done` counter still
   increments per per-sender EOF received; total expected EOFs unchanged.

## Phase plan

| Phase | Scope                                                                                                                                                                                                        |  LOC (est) | Risk                           |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------: | ------------------------------ |
| 1     | Frame format: pack `sender_proc` into `flags` upper 16 bits. Update constructors, parser, write_to. Add unit tests.                                                                                          |        ~80 | Low                            |
| 2     | DSM layout: rename `slot_offset` → `inbox_offset(receiver)`. Update `compute_dsm_layout`. Update `leader_init` create loop. Update tests in `dsm.rs`.                                                        |       ~120 | High (math, alignment)         |
| 3     | `attach_proc_row_and_column` → split into `attach_own_inbox` + `attach_peer_inboxes`. Update `leader_setup` and `worker_setup` to use the new helpers. `MppMesh.inbound_drain` collapses from Vec to single. |       ~200 | High (touches every init site) |
| 4     | `MppSender` carries `local_proc`. Drain-side channel-buffer registry keyed by `(sender_proc, stage_id, partition)`. EOF accounting unchanged.                                                                |       ~150 | Medium (frame demux)           |
| 5     | Local smoke (N=4, N=8) confirms correctness + `mesh_init create_ms` drops to ~constant. Re-trigger cloud bench, look for flat N-curve (no N² blow-up).                                                       | bench-only | Verification                   |

Total: ~550 LOC + bench. Each phase gets a senior-review pass per the
ongoing convention.

## Test plan

- **Phase 1**: `dsm.rs` unit tests covering `MppFrameHeader::batch(stage, part, sender)`
  / `eof(...)` / `parse` / `write_to` round-trip with sender_proc=0, 1, 65535.
  `kind()` rejects bits 8..15 set, accepts bits 16..31 set.
- **Phase 2**: `compute_dsm_layout` tests already cover overflow and alignment;
  update expected `region_total` values. Add tests confirming the DSM region
  is N×queue_bytes (not N²).
- **Phase 3**: existing `mpp_exec.sql` / `mpp_fallback.sql` regression tests
  catch any worker-attach / leader-attach regression — they exercise N=3 (the
  default `mpp_worker_count=4`) end-to-end.
- **Phase 4**: same regression tests + a new test that exercises drain demux
  with 2+ producers writing to one consumer (the inverse of today's pattern;
  catches the demux key change).
- **Phase 5** (bench): the `mesh_init` instrument from `1667f21e3` is the
  direct verification — `create_ms` should drop from O(N²) to O(N).

## Performance hypothesis

What we expect to see after this lands (re-run cloud bench at 1M, same
config):

| N   | Before (ms) |                     After (predicted) | Mechanism                                                                  |
| --- | ----------: | ------------------------------------: | -------------------------------------------------------------------------- |
| 4   |         397 | ~390 (unchanged, low N already cheap) | Mesh cost was small at N=4                                                 |
| 8   |         449 |                                  ~430 | Mesh cost was ~12% of wall; mostly producer-bound                          |
| 12  |         557 |                                  ~480 | Mesh was starting to dominate                                              |
| 16  |         695 |                                  ~540 | Mesh-edge cost cut from N(N-1)=240/proc to 1                               |
| 24  |        1540 |                                  ~700 | **The cliff goes away** — N=24 should look like a smooth extension of N=16 |

If `mesh_init create_ms` drops as predicted but wall time DOESN'T improve at
N=24, the new bottleneck is contention on the per-receiver inbox spinlock,
and we re-open the K=2/4 fanout option from the original sketch.

## Out of scope (follow-ups)

1. **G7-MT (multi-thread per producer)**: parked behind
   `paradedb.mpp_use_ffi_relay=off`. Becomes interesting once mesh stops
   capping throughput; expected to push the curve from "flat at the
   producer-CPU ceiling" to "linear in cores per worker."
2. **Per-receiver K>1 fanout**: keep the design extensible to K inboxes
   per receiver, but ship K=1 first and measure.
3. **DSM cap raise**: with N-edge mesh, `MPP_DSM_MAX_BYTES=16GiB` allows
   much higher N + larger queue_size at the same time. Tune separately.

## Related

- [project_mpp_mesh_decision](../../../memory/project_mpp_mesh_decision.md)
  — decision rationale + supporting data.
- [project_mpp_linear_scaling](../../../memory/project_mpp_linear_scaling.md)
  — full investigation context.
- `pg_search/src/postgres/customscan/mpp/dsm.rs` — current layout.
- `pg_search/src/postgres/customscan/mpp/transport.rs` — `MppFrameHeader`,
  `MppSender`, `DrainHandle`.
- `pg_search/src/postgres/customscan/mpp/glue.rs` —
  `leader_setup` / `worker_setup`.
- PR #5155 — investigation PR with the bench data this design is based on.
