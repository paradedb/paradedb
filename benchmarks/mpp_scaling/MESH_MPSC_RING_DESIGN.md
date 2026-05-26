# DSM-backed MPSC Ring for MPP Inboxes

## Why

The K=1-shm_mq design (`MESH_MULTIPLEXING_DESIGN.md`) is blocked by PostgreSQL's
`shm_mq` SPSC contract. To get the N²→N mesh savings we need a multi-producer,
single-consumer primitive — PG core doesn't ship one, so we build it in
`dsm_segment`-backed shared memory.

Bench data (PR #5155, 1M rows, queue=4MB, target_partitions=2) shows wall time
tracks N² as N grows:

| N   | wall (ms) | mesh edges (3 meshes × N(N-1)) |
| --- | --------: | -----------------------------: |
| 4   |       397 |                             36 |
| 8   |       449 |                            168 |
| 12  |       557 |                            396 |
| 16  |       695 |                            720 |
| 24  |      1540 |                           1800 |

The custom MPSC ring lets us collapse to N inboxes (one per receiver), exact
shape the original design intended — just on a primitive that allows multiple
senders.

## What the primitive looks like

`DsmMpscRing` is a fixed-size ring of byte messages, allocated as a contiguous
chunk of a `dsm_segment`. All state lives in shared memory; no
process-private allocator. Multi-process safety comes from atomic CAS on
slot ownership.

### Layout

```text
+-- DsmMpscRingHeader (cache-line aligned) -+
|  receiver_proc:  AtomicU32                |
|  detached:       AtomicBool               |
|  ring_size:      u32           (immutable)|
|  slot_capacity:  u32           (immutable)|
|  head:           AtomicU64     (consumer) |
|  tail:           AtomicU64     (producers)|
|  receiver_latch: *mut pg_sys::Latch       |
+-------------------------------------------+
|  Slot[0]                                  |
|  Slot[1]                                  |
|  ...                                      |
|  Slot[ring_size - 1]                      |
+-------------------------------------------+

Slot {
  seq:  AtomicU64,   // 0 = empty, even = ready, odd = being written
  len:  u32,
  data: [u8; slot_capacity - SLOT_HEADER_SIZE]
}
```

- `head` and `tail` are monotonically increasing u64 counters. `head %
ring_size` and `tail % ring_size` map to slot index.
- `slot_capacity` is fixed at create time. Frames larger than
  `slot_capacity - SLOT_HEADER_SIZE` get rejected at send time (caller
  should pick a slot_capacity that fits the largest frame).
- `seq` is a Dybvig-style sequence counter that lets the consumer detect
  "this slot is for me to read" without coordinating directly with
  producers:
  - At slot index `i`, the consumer expects `seq == (head_value * 2) + 2`
    where head_value is the head value when this slot is consumed.
  - Producers CAS seq from `expected_empty` (= `tail_value * 2`) to
    `(tail_value * 2) + 1` (odd = being written), then write
    data + len, then store `(tail_value * 2) + 2` (even = ready).

### Producer send path

```rust
fn try_send(&self, bytes: &[u8]) -> Result<(), SendError> {
    if self.detached.load(Acquire) { return Err(Detached); }
    if bytes.len() > self.slot_capacity - SLOT_HEADER_SIZE {
        return Err(MessageTooLarge);
    }
    loop {
        let tail = self.tail.load(Acquire);
        let slot_idx = (tail % self.ring_size) as usize;
        let slot = &self.slots[slot_idx];
        let expected_seq = tail.wrapping_mul(2);
        // Try to claim: seq must equal expected_empty.
        match slot.seq.compare_exchange(
            expected_seq,
            expected_seq.wrapping_add(1),
            AcqRel, Acquire,
        ) {
            Ok(_) => {}
            Err(_) => {
                // Either ring is full (slot still holds old data) or another
                // producer beat us. Re-check tail; if it didn't advance, we're full.
                if self.tail.load(Acquire) == tail { return Err(Full); }
                continue;
            }
        }
        // We own the slot. Advance tail (any other producer racing us on the
        // same tail value already lost the CAS above and is retrying).
        self.tail.compare_exchange(tail, tail + 1, AcqRel, Relaxed).ok();
        // Write payload.
        slot.len.store(bytes.len() as u32, Relaxed);
        unsafe { std::ptr::copy_nonoverlapping(
            bytes.as_ptr(), slot.data.as_mut_ptr(), bytes.len(),
        ); }
        // Mark ready.
        slot.seq.store(expected_seq.wrapping_add(2), Release);
        // Wake the consumer if it's waiting on its latch.
        unsafe { pg_sys::SetLatch(self.receiver_latch); }
        return Ok(());
    }
}
```

### Consumer recv path

```rust
fn try_recv(&self, out: &mut Vec<u8>) -> Option<RecvOutcome> {
    let head = self.head.load(Relaxed);
    let slot_idx = (head % self.ring_size) as usize;
    let slot = &self.slots[slot_idx];
    let expected_seq = head.wrapping_mul(2).wrapping_add(2);
    if slot.seq.load(Acquire) != expected_seq {
        if self.detached.load(Acquire) && self.tail.load(Acquire) == head {
            return Some(Detached);
        }
        return None; // empty for now
    }
    let len = slot.len.load(Relaxed) as usize;
    out.clear();
    out.extend_from_slice(unsafe { std::slice::from_raw_parts(slot.data.as_ptr(), len) });
    // Release the slot for reuse by setting seq to the next round's "empty" value.
    slot.seq.store(head.wrapping_add(self.ring_size).wrapping_mul(2), Release);
    self.head.store(head + 1, Release);
    Some(Bytes)
}
```

### Detach

- Consumer calls `set_detached(&self)` on drop / query teardown. Stores
  `true` in `detached` and `SetLatch`es any producer waiting in a future
  blocking-send variant (we only have `try_send` so far).
- Producers check `detached` at start of `try_send` and fail-fast.

### Wakeup

- Producers always `SetLatch` after a successful send. Cheap, idempotent.
- Consumer's drain loop calls `WaitLatch` between `try_recv` passes when
  the ring is empty. This is exactly the pattern shm_mq uses internally
  (see `shm_mq_receive`).

## Integration with MppMesh

`DsmMpscRing` replaces `ShmMqSender`/`ShmMqReceiver` in
`pg_search/src/postgres/customscan/mpp/mesh.rs` only for the mesh inboxes.
The non-mesh shm_mq uses elsewhere in pg_search
(`parallel_worker/mqueue.rs`) are unchanged.

### DSM layout change

```text
+--- MppDsmHeader ---+
| ... existing fields, header_version bumped to 4 ...
+--------------------+
| plan bytes         |
+--------------------+
| padding            |
+--------------------+
| inbox[0]           |    <- one DsmMpscRing per receiver_proc
| inbox[1]           |
| ...                |
| inbox[n_procs-1]   |
+--------------------+
```

- `compute_dsm_layout`: queue_bytes is now the per-inbox ring capacity
  (header + slots); total queues area is `n_procs · queue_bytes`. Same
  shrink the K=1 design targeted, this time on a primitive that works.
- `leader_init`: calls `DsmMpscRing::create_at(addr, ring_size, slot_capacity)`
  per inbox.
- `attach_proc`: each proc gets ONE `DsmMpscReceiver` (its own inbox) and
  N-1 `DsmMpscSender` clones (one per peer inbox).

### MppMesh shape

Same end state the dead K=1 design proposed:

```rust
pub struct MppMesh {
    pub this_proc: u32,
    pub n_procs: u32,
    pub(super) inbound_drain: Arc<DrainHandle>,
}
```

`DrainHandle::cooperative(vec![own_inbox_receiver, self_loop_in_proc])`
just like before, but `own_inbox_receiver` wraps `DsmMpscReceiver` instead
of the broken multi-sender shm_mq.

### Frame demux

Same as Phase 1: every frame carries `sender_proc` in its
`MppFrameHeader.flags` upper 16 bits. Channel-buffer registry keyed by
`(sender_proc, stage_id, partition)`.

## Phase plan

| Phase | Scope                                                                                                                                                                                                                                                                             |  LOC (est) | Risk                         |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------: | ---------------------------- |
| 2     | `DsmMpscRing` primitive: layout, header, slot, `try_send`, `try_recv`, `set_detached`. Unit tests using `Box::into_raw` and threads. No PG integration yet.                                                                                                                       |       ~350 | High (lock-free correctness) |
| 3     | Wire `DsmMpscRing` into `mesh.rs` as `DsmMpscSender`/`DsmMpscReceiver`. Both implement `BatchChannelSender`/`BatchChannelReceiver` trait, drop-in for `ShmMqSender`/`ShmMqReceiver`.                                                                                              |       ~150 | Low (trait shape known)      |
| 4     | DSM layout change in `dsm.rs`: N inboxes, `inbox_offset(receiver)`, `attach_proc` split, ProcAttach singular receiver. Plus `MppMesh.inbound_drain` collapse + ChannelBufferRegistry sender_proc key (everything the dead Phase 2 already did, but now on the working primitive). |       ~250 | Medium                       |
| 5     | Local smoke at N={4,8} confirms correctness + `mesh_init create_ms` drops O(N²)→O(N). Re-trigger cloud bench, look for the N=24 cliff flattening.                                                                                                                                 | bench-only | Verification                 |

Total: ~750 LOC (~350 for the primitive + ~400 for integration). About double the dead K=1 budget because the primitive replaces an existing one.

## Test strategy

Phase 2 unit tests must include:

1. **Single-producer/single-consumer round trip**: send N messages, receive
   them in order.
2. **Multi-producer concurrent send**: spawn K threads each sending M
   messages; consumer receives K\*M unique messages.
3. **Ring fullness**: when producers run faster than consumer, `try_send`
   returns `Full` eventually; messages already in flight still get
   delivered.
4. **Detach mid-stream**: consumer drops; producers see `Detached` on
   subsequent sends.
5. **CAS contention regression**: with N=24 producers hammering one ring,
   send latency stays bounded and total throughput beats per-pair
   shm_mq's per-thread cost.

Phase 3+ integration tests reuse the existing transport test suite
(`drain_handle_*`, `frame_*`) by swapping the in-proc channel for
`DsmMpscRing`. Critical: add at least ONE test that runs N>=2 real PG
parallel workers writing to the same inbox, end-to-end, before claiming
the primitive is production-ready.

## Invariants

1. **`seq` sequence**: at a freshly-created ring, every slot's `seq` is
   `0` (empty). After consumer reads slot i for the k-th time,
   `slot[i].seq = (k * ring_size + i) * 2`. Producers identify the
   "next-empty for tail T" by `(T * 2) == slot[T mod ring_size].seq`.
2. **Tail monotonicity**: `tail` only ever increases. Producers CAS to
   advance it after claiming a slot.
3. **Head monotonicity**: `head` only ever increases. Single consumer
   advances it after reading.
4. **Detach is sticky**: once `detached == true`, it never reverts.
5. **Wakeup is best-effort**: `SetLatch` may fire when the consumer
   isn't waiting; that's fine. Consumer's drain loop is correct under
   spurious wakeup.

## Out of scope (follow-ups)

1. **G7-MT compatibility**: the FFI relay still routes via per-process
   single-thread. Custom MPSC works on top of the same constraint.
2. **Backpressure quality**: today's `try_send` returns `Full` after one
   CAS pass. A blocking-send variant with `ConditionVariable` is possible
   but the current consumer cooperative-drain pattern handles
   backpressure already.
3. **Variable-size slots**: today's design pins `slot_capacity` at create
   time. A future ring could segment large messages across multiple slots
   if frame sizes vary widely.

## Related

- [project_mpp_mesh_decision](../../../memory/project_mpp_mesh_decision.md)
- `MESH_MULTIPLEXING_DESIGN.md` (in the same dir) — dead K=1-over-shm_mq
  design, kept for record.
- `pg_search/src/postgres/customscan/mpp/mesh.rs` — current ShmMq
  primitives this design will partially replace.
- PR #5155 — investigation PR + bench data this design responds to.
- Citus's `pg_dist_*` extension for a real-world precedent of DSM-backed
  custom MPSC primitives in a PG extension.
