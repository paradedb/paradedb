// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Implicit binary tree slab pool for PostgreSQL shared-memory cache.
//!
//! Provides power-of-two slab allocation from a base slab size (typically 256KB) up to the entire
//! cache arena (e.g. 64MB or 1GB).
//!
//! # Design
//!
//! Rather than using intrusive doubly-linked free lists and block headers, the allocator is
//! represented as an implicit binary tree in a flat array (identical to a binary heap).
//!
//! Each node stores `order + 1` if that order is available in its subtree, or `0` if that node
//! is allocated or completely exhausted.
//! - Order 0 corresponds to `BASE_SLAB_SIZE` (1 << `base_shift`).
//! - Left child is `node * 2`, right child is `node * 2 + 1`, parent is `node / 2`.
//! - Slabs are automatically coalesced with their buddies upon free.

/// Maximum number of tree nodes in shared memory.
///
/// With a 256KB base slab size, 8,192 nodes supports up to 4,096 leaves = 1GB arena capacity.
pub const MAX_TREE_NODES: usize = 8192;

/// Default base slab size: 256KB (1 << 18).
pub const DEFAULT_BASE_SHIFT: u8 = 18;

/// Span of allocated memory returned by the slab pool.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
#[repr(C)]
pub struct ArenaSpan {
    /// Byte offset from the start of the data arena.
    pub offset: u32,
    /// Exact byte length of the segment data.
    pub len: u32,
    /// Power-of-two order of the allocated slab.
    pub order: u8,
}

/// Shared-memory binary tree slab pool.
#[repr(C)]
pub struct SlabPool {
    /// Bit-shift for the base slab size (e.g. 18 for 256KB).
    pub base_shift: u8,
    /// Maximum order corresponding to the full arena capacity.
    pub max_order: u8,
    /// Number of base slab leaves.
    pub num_leaves: u16,
    /// Tree array storing available order + 1 (or 0 if exhausted/allocated).
    pub tree: [u8; MAX_TREE_NODES],
}

impl SlabPool {
    /// Construct an uninitialized slab pool.
    #[allow(dead_code)]
    pub const fn empty() -> Self {
        Self {
            base_shift: DEFAULT_BASE_SHIFT,
            max_order: 0,
            num_leaves: 0,
            tree: [0; MAX_TREE_NODES],
        }
    }

    /// Initialize the slab pool for `arena_capacity` in bytes.
    pub fn init(&mut self, arena_capacity: usize) {
        let mut base_shift = DEFAULT_BASE_SHIFT;
        while (1 << base_shift) > arena_capacity && base_shift > 12 {
            base_shift -= 1;
        }

        let base_size = 1 << base_shift;
        let leaves = (arena_capacity / base_size).next_power_of_two().max(1);
        let max_order = leaves.trailing_zeros() as u8;
        let total_nodes = 2 * leaves;

        assert!(
            total_nodes <= MAX_TREE_NODES,
            "arena capacity requires {total_nodes} tree nodes, exceeding maximum {MAX_TREE_NODES}"
        );

        self.base_shift = base_shift;
        self.max_order = max_order;
        self.num_leaves = leaves as u16;
        self.tree.fill(0);

        for depth in 0..=max_order {
            let order_val = max_order + 1 - depth;
            let start = 1 << depth;
            let end = 1 << (depth + 1);
            for node in start..end {
                self.tree[node] = order_val;
            }
        }
    }

    /// Calculate the power-of-two order needed to fit `needed_bytes`.
    pub fn order_for_bytes(&self, needed_bytes: usize) -> Option<u8> {
        let base_size = 1 << self.base_shift;
        let needed = needed_bytes.max(1);
        let needed_blocks = needed.div_ceil(base_size);
        let order = needed_blocks.next_power_of_two().trailing_zeros() as u8;
        if order > self.max_order {
            None
        } else {
            Some(order)
        }
    }

    /// Try to allocate a contiguous slab for `needed_bytes`.
    ///
    /// # Algorithm
    ///
    /// 1. Determines the minimum power-of-two `needed_order` to satisfy `needed_bytes`.
    /// 2. Checks the root node `tree[1]`: if `tree[1] < needed_order + 1`, no slab of sufficient
    ///    size is currently free.
    /// 3. Traverses down the implicit binary tree to find a node at `needed_order`:
    ///    - Checks left child first (`node * 2`). If available, traverses left; otherwise traverses right (`node * 2 + 1`).
    /// 4. Marks the target node as 0 (fully allocated).
    /// 5. Derives byte `offset` within the arena from the node's position in its level.
    /// 6. Traverses back up to root, updating each parent `tree[p] = max(tree[l], tree[r])`.
    ///
    /// Returns `Some(ArenaSpan)` on success, or `None` if memory is exhausted.
    pub fn allocate(&mut self, needed_bytes: usize) -> Option<ArenaSpan> {
        let needed_order = self.order_for_bytes(needed_bytes)?;
        let required_val = needed_order + 1;

        if self.tree[1] < required_val {
            return None;
        }

        let mut node = 1;
        let mut curr_order = self.max_order;

        while curr_order > needed_order {
            let left = node * 2;
            let right = left + 1;
            if self.tree[left] >= required_val {
                node = left;
            } else {
                node = right;
            }
            curr_order -= 1;
        }

        self.tree[node] = 0;

        let depth = self.max_order - needed_order;
        let index_in_level = node - (1 << depth);
        let block_size = (1 << self.base_shift) << needed_order;
        let offset = (index_in_level as u32) * (block_size as u32);

        let mut p = node / 2;
        while p > 0 {
            let l = p * 2;
            let r = l + 1;
            self.tree[p] = std::cmp::max(self.tree[l], self.tree[r]);
            p /= 2;
        }

        Some(ArenaSpan {
            offset,
            len: needed_bytes as u32,
            order: needed_order,
        })
    }

    /// Free an allocated slab back to the pool, coalescing with adjacent buddies when possible.
    ///
    /// # Algorithm
    ///
    /// 1. Maps `(offset, order)` back to its leaf/intermediate `node` index in the tree array.
    /// 2. Sets `tree[node] = order + 1`.
    /// 3. Traverses up to root:
    ///    - If both children are completely free (`tree[l] == curr_val && tree[r] == curr_val`),
    ///      the buddies coalesce into a single block of the parent's order (`curr_val + 1`).
    ///    - Otherwise, parent is updated to `max(tree[l], tree[r])`.
    pub fn free(&mut self, offset: u32, order: u8) {
        if order > self.max_order {
            return;
        }

        let depth = self.max_order - order;
        let block_size = (1 << self.base_shift) << order;
        let index_in_level = offset as usize / block_size;
        let node = (1 << depth) + index_in_level;

        if node >= MAX_TREE_NODES {
            return;
        }

        let mut curr_val = order + 1;
        self.tree[node] = curr_val;

        let mut p = node / 2;
        while p > 0 {
            let l = p * 2;
            let r = l + 1;
            if curr_val > 0 && self.tree[l] == curr_val && self.tree[r] == curr_val {
                curr_val += 1;
                self.tree[p] = curr_val;
            } else {
                self.tree[p] = std::cmp::max(self.tree[l], self.tree[r]);
                curr_val = 0;
            }
            p /= 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slab_pool_init_and_single_full_allocation() {
        let mut pool = SlabPool::empty();
        let capacity = 64 * 1024 * 1024; // 64MB
        pool.init(capacity);

        assert_eq!(pool.max_order, 8); // 256KB << 8 = 64MB
        assert_eq!(pool.tree[1], 9);

        // Allocate entire 64MB cache
        let span = pool.allocate(capacity).expect("must allocate entire cache");
        assert_eq!(span.offset, 0);
        assert_eq!(span.order, 8);
        assert_eq!(pool.tree[1], 0);

        // Cannot allocate anything while full
        assert!(pool.allocate(1024).is_none());

        // Free entire cache
        pool.free(span.offset, span.order);
        assert_eq!(pool.tree[1], 9);
    }

    #[test]
    fn test_slab_pool_split_and_coalesce() {
        let mut pool = SlabPool::empty();
        let capacity = 1024 * 1024; // 1MB
        pool.init(capacity);
        // base_shift is 18 (256KB), leaves = 4, max_order = 2
        assert_eq!(pool.max_order, 2);

        // Allocate two 256KB blocks (order 0)
        let span1 = pool.allocate(250 * 1024).expect("must allocate span1");
        assert_eq!(span1.offset, 0);
        assert_eq!(span1.order, 0);

        let span2 = pool.allocate(250 * 1024).expect("must allocate span2");
        assert_eq!(span2.offset, 256 * 1024);
        assert_eq!(span2.order, 0);

        // Allocate one 512KB block (order 1)
        let span3 = pool.allocate(500 * 1024).expect("must allocate span3");
        assert_eq!(span3.offset, 512 * 1024);
        assert_eq!(span3.order, 1);

        // Pool is now full (256K + 256K + 512K = 1MB)
        assert!(pool.allocate(100).is_none());

        // Free span1: offset 0 is free, but buddy (offset 256K) is still used -> cannot coalesce to 512K yet
        pool.free(span1.offset, span1.order);
        // We can allocate 256K, but not 512K
        assert!(pool.allocate(500 * 1024).is_none());
        let span1_realloc = pool.allocate(200 * 1024).expect("must realloc span1");
        assert_eq!(span1_realloc.offset, 0);

        // Now free both span1 and span2 -> should coalesce to 512KB
        pool.free(span1_realloc.offset, span1_realloc.order);
        pool.free(span2.offset, span2.order);

        // Now we can allocate a 512KB block at offset 0
        let span_coalesced = pool
            .allocate(512 * 1024)
            .expect("must allocate coalesced 512K");
        assert_eq!(span_coalesced.offset, 0);
        assert_eq!(span_coalesced.order, 1);

        // Free both remaining 512K blocks -> should coalesce back to 1MB
        pool.free(span_coalesced.offset, span_coalesced.order);
        pool.free(span3.offset, span3.order);

        assert_eq!(pool.tree[1], 3); // Full 1MB available (order 2 + 1)
    }

    #[test]
    fn test_slab_pool_out_of_order_frees() {
        let mut pool = SlabPool::empty();
        let capacity = 1024 * 1024; // 1MB
        pool.init(capacity);

        // Allocate 4 x 256KB blocks
        let s0 = pool.allocate(100).unwrap();
        let s1 = pool.allocate(100).unwrap();
        let s2 = pool.allocate(100).unwrap();
        let s3 = pool.allocate(100).unwrap();

        // Free in out-of-order sequence: s2, s0, s3, s1
        pool.free(s2.offset, s2.order);
        pool.free(s0.offset, s0.order);
        pool.free(s3.offset, s3.order);
        pool.free(s1.offset, s1.order);

        // All should be coalesced back to root
        assert_eq!(pool.tree[1], pool.max_order + 1);
    }
}
