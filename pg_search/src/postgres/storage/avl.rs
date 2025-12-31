// Copyright (C) 2023-2026 ParadeDB, Inc.
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

#![allow(clippy::too_many_arguments)]
use std::num::NonZeroU16;

/// Index type for slots. You can switch to u16 if capacity <= 65_535 by
/// replacing NonZeroU32 with NonZeroU16 and casts below.
type Idx = u16;
type NzIdx = NonZeroU16;

#[inline]
fn ix_some(i: usize) -> Option<NzIdx> {
    NzIdx::new((i as Idx) + 1) // store i+1 so 0 encodes None
}
#[inline]
fn ix_to_usize(o: Option<NzIdx>) -> Option<usize> {
    o.map(|nz| (nz.get() - 1) as usize)
}

#[inline]
fn meta_height(meta: u8) -> u8 {
    meta & 0x7F
}
#[inline]
fn meta_set_used(meta: &mut u8, used: bool) {
    *meta = (*meta & 0x7F) | ((used as u8) << 7);
}

#[inline]
fn meta_is_used(meta: u8) -> bool {
    (meta & 0x80) != 0
}

#[inline]
fn meta_set_height(meta: &mut u8, h: u8) {
    *meta = (*meta & 0x80) | (h & 0x7F);
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Slot<K: Copy, V: Copy, T: Copy = ()> {
    left: Option<NzIdx>,  // 2 bytes (None = 0)
    right: Option<NzIdx>, // 2 bytes
    pub tag: T,
    pub key: K,
    pub val: V,
    meta: u8, // used bit + height (0 means empty)
              // padding depends on K/V alignment; layout stays compact & linear
}

impl<K: Copy, V: Copy, T: Copy> Slot<K, V, T> {
    #[inline]
    pub fn is_used(&self) -> bool {
        meta_is_used(self.meta)
    }
    #[inline]
    fn set_used(&mut self, u: bool) {
        meta_set_used(&mut self.meta, u)
    }
    #[inline]
    fn height(&self) -> u8 {
        meta_height(self.meta)
    }
    #[inline]
    fn set_height(&mut self, h: u8) {
        meta_set_height(&mut self.meta, h)
    }
    #[inline]
    fn l(&self) -> Option<usize> {
        ix_to_usize(self.left)
    }
    #[inline]
    fn r(&self) -> Option<usize> {
        ix_to_usize(self.right)
    }
    #[inline]
    fn set_l(&mut self, i: Option<usize>) {
        self.left = i.and_then(ix_some);
    }
    #[inline]
    fn set_r(&mut self, i: Option<usize>) {
        self.right = i.and_then(ix_some);
    }
}

impl<K: Copy, V: Copy, T: Copy> Default for Slot<K, V, T> {
    fn default() -> Self {
        // Keys/values in unused slots are ignored. We avoid reading them when used=false.
        // Zeroing is fine for common PODs; if your K/V cannot be zeroed, just ensure you never
        // read key/val unless `is_used()` is true.
        Self {
            key: unsafe { core::mem::zeroed() },
            val: unsafe { core::mem::zeroed() },
            tag: unsafe { core::mem::zeroed() },
            left: None,
            right: None,
            meta: 0, // used=false, height=0
        }
    }
}

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("AVL map is full")]
    Full,
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
pub struct AvlTreeMapHeader {
    root: Option<usize>,
    free_head: Option<usize>,
    len: usize,
}

/// Read-only view of an Array-backed AVL Tree living inside a borrowed [`&[Slot<K,V>]`]
#[repr(C)]
pub struct AvlTreeMapView<'a, K: Ord + Copy, V: Copy, T: Copy = ()> {
    header: &'a AvlTreeMapHeader,
    pub(crate) arena: &'a [Slot<K, V, T>],
}

/// Array-backed mutable AVL Tree living inside a borrowed [`&mut [Slot<K,V>]`]
#[repr(C)]
pub struct AvlTreeMap<'a, K: Ord + Copy, V: Copy, T: Copy = ()> {
    header: &'a mut AvlTreeMapHeader,
    arena: &'a mut [Slot<K, V, T>],
}

impl<'a, K: Ord + Copy, V: Copy, T: Copy> AvlTreeMapView<'a, K, V, T> {
    /// Use the provided `header` and `arena` as pre-existing structures
    ///
    /// # Safety
    ///
    /// This method is unsafe as it cannot guarantee that the provided `header` and `arena` represent
    /// a valid AVLTree and header information
    pub unsafe fn with_header_and_arena(
        header: &'a AvlTreeMapHeader,
        arena: &'a [Slot<K, V, T>],
    ) -> Self {
        Self { header, arena }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.header.len
    }
    #[inline]
    pub fn capacity(&self) -> usize {
        self.arena.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.header.len == 0
    }

    /// Returns a copy of the value for `key`, if present.
    pub fn get(&self, key: &K) -> Option<(V, T)> {
        let mut cur = self.header.root;
        while let Some(i) = cur {
            let n = &self.arena[i];
            use core::cmp::Ordering::*;
            match key.cmp(&{ n.key }) {
                Equal => return Some((n.val, n.tag)),
                Less => cur = n.l(),
                Greater => cur = n.r(),
            }
        }
        None
    }

    #[inline]
    pub fn contains(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    /// Returns the entry with the greatest key `<= key`.
    /// If no such key exists, returns `None`.
    #[inline]
    pub fn get_lte(&self, key: &K) -> Option<(K, V, T)> {
        let mut cur = self.header.root;
        let mut best: Option<usize> = None;

        while let Some(i) = cur {
            use core::cmp::Ordering::*;
            let n = &self.arena[i];
            match key.cmp(&{ n.key }) {
                Equal => return Some((n.key, n.val, n.tag)),
                Less => {
                    // Must be in the left subtree (all keys there are < n.key)
                    cur = n.l();
                }
                Greater => {
                    // n.key is a candidate (<= key). Go right to try to get closer.
                    best = Some(i);
                    cur = n.r();
                }
            }
        }

        best.map(|i| {
            let n = &self.arena[i];
            (n.key, n.val, n.tag)
        })
    }

    #[inline]
    fn height(&self, i: Option<usize>) -> i16 {
        i.map(|ix| self.arena[ix].height() as i16).unwrap_or(0)
    }

    #[inline]
    fn balance_factor(&self, i: usize) -> i16 {
        let lh = self.height(self.arena[i].l());
        let rh = self.height(self.arena[i].r());
        lh - rh
    }

    fn min_index(&self, mut i: usize) -> usize {
        while let Some(l) = self.arena[i].l() {
            i = l;
        }
        i
    }

    // Optional safety check (debug only)
    #[cfg(debug_assertions)]
    pub fn assert_ok(&self)
    where
        K: Ord + Copy + core::fmt::Debug,
    {
        fn dfs<K: Ord + Copy + core::fmt::Debug, V: Copy, T: Copy>(
            t: &AvlTreeMapView<'_, K, V, T>,
            i: Option<usize>,
        ) -> (i8, K, K) {
            let ix = i.expect("dfs called with None");
            let n = &t.arena[ix];

            // Left subtree
            let (lh, lmin, lmax) = if let Some(li) = n.l() {
                let (h, min_k, max_k) = dfs(t, Some(li));
                assert!(
                    max_k <= { n.key },
                    "BST violation: left max {:?} > node {:?}",
                    max_k,
                    { n.key }
                );
                (h, min_k, max_k)
            } else {
                (0, n.key, n.key)
            };

            // Right subtree
            let (rh, rmin, rmax) = if let Some(ri) = n.r() {
                let (h, min_k, max_k) = dfs(t, Some(ri));
                assert!(
                    { n.key } <= min_k,
                    "BST violation: node {:?} > right min {:?}",
                    { n.key },
                    min_k
                );
                (h, min_k, max_k)
            } else {
                (0, n.key, n.key)
            };

            // Check height and balance
            let h = 1 + lh.max(rh);
            assert_eq!(n.height() as i8, h, "Height mismatch at {:?}", { n.key });
            assert!(
                (lh - rh).abs() <= 1,
                "Balance factor out of range at {:?}",
                { n.key }
            );

            // Return overall min/max for this subtree
            (h, lmin.min(n.key).min(rmin), rmax.max(n.key).max(lmax))
        }

        if let Some(r) = self.header.root {
            let _ = dfs(self, Some(r));
        }
        assert!(self.header.len <= self.capacity());
    }
}

impl<'a, K: Ord + Copy, V: Copy, T: Copy> AvlTreeMap<'a, K, V, T> {
    /// Initialize over the given linear memory; marks all slots as free.
    pub fn new(header: &'a mut AvlTreeMapHeader, arena: &'a mut [Slot<K, V, T>]) -> Self {
        // Build free list by chaining `left` as the "next" pointer.
        header.free_head = None;
        for i in (0..arena.len()).rev() {
            arena[i].set_used(false);
            arena[i].set_height(0);
            arena[i].set_r(None);
            // push i on free list
            arena[i].set_l(header.free_head);
            header.free_head = Some(i);
        }
        header.root = None;
        header.len = 0;
        Self { header, arena }
    }

    /// Use the provided `header` and `arena` as pre-existing structures
    ///
    /// # Safety
    ///
    /// This method is unsafe as it cannot guarantee that the provided `header` and `arena` represent
    /// a valid AVLTree and header information
    pub unsafe fn with_header_and_arena(
        header: &'a mut AvlTreeMapHeader,
        arena: &'a mut [Slot<K, V, T>],
    ) -> Self {
        Self { header, arena }
    }

    #[inline(always)]
    pub fn view(&self) -> AvlTreeMapView<'_, K, V, T> {
        AvlTreeMapView {
            header: self.header,
            arena: self.arena,
        }
    }

    /// Insert or update.
    /// - If key exists: replaces its value and returns `Some(old_value)`.
    /// - If key does not exist: inserts and returns `None`.
    pub fn insert(&mut self, key: K, val: V) -> Result<(Option<V>, T)> {
        let mut out_old: Option<V> = None;
        let mut out_tag: Option<T> = None;
        let mut inserted_new = false;
        let mut occupied_at = 0;
        self.header.root = self.insert_rec(
            self.header.root,
            key,
            Some(val),
            &mut out_old,
            &mut out_tag,
            &mut inserted_new,
            &mut occupied_at,
        )?;
        if inserted_new {
            self.header.len += 1;
        }
        Ok((out_old, out_tag.unwrap()))
    }

    /// Remove by key; returns value if found.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut removed_val: Option<V> = None;
        let mut did_remove = false;
        self.header.root =
            self.delete_rec(self.header.root, key, &mut removed_val, &mut did_remove);
        if did_remove {
            self.header.len -= 1;
        }
        removed_val
    }

    /// Returns a mutable reference to the value if the `key` exists
    pub fn get_slot_mut(&mut self, key: &K) -> Option<&mut Slot<K, V, T>> {
        let mut cur = self.header.root;
        while let Some(i) = cur {
            let n = &self.arena[i];
            use core::cmp::Ordering::*;
            match key.cmp(&{ n.key }) {
                Equal => return Some(&mut self.arena[i]),
                Less => cur = n.l(),
                Greater => cur = n.r(),
            }
        }
        None
    }

    /// Returns a mutable reference to the slot with the maximum key in the tree.
    /// If the tree is empty, returns `None`.
    pub fn get_max_slot(&mut self) -> Option<&mut Slot<K, V, T>> {
        let mut cur = self.header.root;
        let mut max = None;

        while let Some(i) = cur {
            max = Some(i);
            cur = self.arena[i].r();
        }

        max.map(|i| &mut self.arena[i])
    }

    #[inline]
    fn update_height(&mut self, i: usize) {
        let view = self.view();
        let lh = view.height(view.arena[i].l());
        let rh = view.height(view.arena[i].r());
        self.arena[i].set_height((1 + lh.max(rh)) as u8);
    }

    fn rotate_right(&mut self, y: usize) -> usize {
        // y's left must exist
        let x = self.arena[y].l().expect("rotate_right requires left child");
        let t2 = self.arena[x].r();

        // Perform rotation
        self.arena[x].set_r(Some(y));
        self.arena[y].set_l(t2);

        // Update heights
        self.update_height(y);
        self.update_height(x);
        x
    }

    fn rotate_left(&mut self, x: usize) -> usize {
        // x's right must exist
        let y = self.arena[x].r().expect("rotate_left requires right child");
        let t2 = self.arena[y].l();

        self.arena[y].set_l(Some(x));
        self.arena[x].set_r(t2);

        self.update_height(x);
        self.update_height(y);
        y
    }

    fn rebalance(&mut self, i: usize) -> usize {
        self.update_height(i);
        let bf = self.view().balance_factor(i);

        if bf > 1 {
            // Left heavy
            let l = self.arena[i].l().unwrap();
            if self.view().balance_factor(l) < 0 {
                // LR
                let nl = self.rotate_left(l);
                self.arena[i].set_l(Some(nl));
            }
            return self.rotate_right(i);
        } else if bf < -1 {
            // Right heavy
            let r = self.arena[i].r().unwrap();
            if self.view().balance_factor(r) > 0 {
                // RL
                let nr = self.rotate_right(r);
                self.arena[i].set_r(Some(nr));
            }
            return self.rotate_left(i);
        }
        i
    }

    fn alloc_slot(&mut self, key: K, val: Option<V>) -> Result<usize> {
        let idx = self.header.free_head.ok_or(Error::Full)?;
        // pop from free list (stored in `left`)
        self.header.free_head = self.arena[idx].l();
        let n = &mut self.arena[idx];
        n.key = key;
        if let Some(val) = val {
            n.val = val;
        }
        n.set_l(None);
        n.set_r(None);
        n.set_height(1);
        n.set_used(true);
        Ok(idx)
    }

    fn free_slot(&mut self, idx: usize) {
        let n = &mut self.arena[idx];
        n.set_used(false);
        n.set_height(0);
        n.set_r(None);
        // push to free list via left
        n.set_l(self.header.free_head);
        self.header.free_head = Some(idx);
    }

    fn insert_rec(
        &mut self,
        root: Option<usize>,
        key: K,
        val: Option<V>,
        out_old: &mut Option<V>,
        out_tag: &mut Option<T>,
        inserted_new: &mut bool,
        occupied_at: &mut usize,
    ) -> Result<Option<usize>> {
        match root {
            None => {
                // insert a new value
                *inserted_new = true;
                let slot = self.alloc_slot(key, val)?;
                *out_tag = Some(self.arena[slot].tag);
                *occupied_at = slot;
                Ok(Some(slot))
            }
            Some(i) => {
                use core::cmp::Ordering::*;
                match key.cmp(&{ self.arena[i].key }) {
                    Equal => {
                        // Update value, return old
                        let old = self.arena[i].val;
                        if let Some(val) = val {
                            self.arena[i].val = val;
                        }
                        *out_old = Some(old);
                        *out_tag = Some(self.arena[i].tag);
                        *occupied_at = i;
                        return Ok(Some(i));
                    }
                    Less => {
                        let left = self.insert_rec(
                            self.arena[i].l(),
                            key,
                            val,
                            out_old,
                            out_tag,
                            inserted_new,
                            occupied_at,
                        )?;
                        self.arena[i].set_l(left);
                    }
                    Greater => {
                        let right = self.insert_rec(
                            self.arena[i].r(),
                            key,
                            val,
                            out_old,
                            out_tag,
                            inserted_new,
                            occupied_at,
                        )?;
                        self.arena[i].set_r(right);
                    }
                }
                let new_i = self.rebalance(i);
                Ok(Some(new_i))
            }
        }
    }

    fn delete_rec(
        &mut self,
        root: Option<usize>,
        key: &K,
        removed_val: &mut Option<V>,
        did_remove: &mut bool,
    ) -> Option<usize> {
        let i = root?;
        use core::cmp::Ordering::*;
        match key.cmp(&{ self.arena[i].key }) {
            Less => {
                let left = self.delete_rec(self.arena[i].l(), key, removed_val, did_remove);
                self.arena[i].set_l(left);
                Some(self.rebalance(i))
            }
            Greater => {
                let right = self.delete_rec(self.arena[i].r(), key, removed_val, did_remove);
                self.arena[i].set_r(right);
                Some(self.rebalance(i))
            }
            Equal => {
                *did_remove = true;
                let retv = self.arena[i].val;
                match (self.arena[i].l(), self.arena[i].r()) {
                    (None, None) => {
                        self.free_slot(i);
                        *removed_val = Some(retv);
                        None
                    }
                    (Some(c), None) | (None, Some(c)) => {
                        self.free_slot(i);
                        *removed_val = Some(retv);
                        Some(c)
                    }
                    (Some(_), Some(_)) => {
                        // Record the value being removed BEFORE overwriting the node.
                        *removed_val = Some(retv);

                        // In-order successor in right subtree
                        let view = self.view();
                        let succ = view.min_index(view.arena[i].r().unwrap());
                        let (succ_key, succ_val, succ_tag) = {
                            let s = &self.arena[succ];
                            (s.key, s.val, s.tag)
                        };

                        // Overwrite this node with successor's key/val and swap tags
                        self.arena[i].key = succ_key;
                        self.arena[i].val = succ_val;
                        let temp_tag = self.arena[i].tag; // Save original tag of slot i
                        self.arena[i].tag = succ_tag; // Move successor's tag to slot i
                        self.arena[succ].tag = temp_tag; // Move original tag to successor's slot

                        // Delete successor, but do NOT overwrite removed_val
                        let mut ignored_val: Option<V> = None;
                        let mut ignored_flag = false;
                        let right = self.delete_rec(
                            self.arena[i].r(),
                            &succ_key,
                            &mut ignored_val,
                            &mut ignored_flag,
                        );
                        self.arena[i].set_r(right);

                        Some(self.rebalance(i))
                    }
                }
            }
        }
    }
}

const MAX_HEIGHT: usize = 128;

pub struct AvlIter<'a, K: Copy, V: Copy, T: Copy> {
    arena: &'a [Slot<K, V, T>],
    stack: [usize; MAX_HEIGHT],
    sp: usize,
    cursor: Option<usize>,
}

impl<'a, K: Ord + Copy, V: Copy, T: Copy> AvlTreeMapView<'a, K, V, T> {
    /// In-order iterator over (&copy) (K, V) pairs.
    #[inline]
    pub fn iter(&'a self) -> AvlIter<'a, K, V, T> {
        AvlIter {
            arena: self.arena,
            stack: [0; MAX_HEIGHT],
            sp: 0,
            cursor: self.header.root,
        }
    }
}

impl<'a, K: Copy, V: Copy, T: Copy> AvlIter<'a, K, V, T> {
    #[inline]
    fn push_left_chain(&mut self) {
        // Descend left from `cursor`, pushing nodes onto the fixed stack.
        while let Some(i) = self.cursor {
            debug_assert!(self.sp < MAX_HEIGHT, "AVL height exceeded MAX_HEIGHT");
            self.stack[self.sp] = i;
            self.sp += 1;
            self.cursor = self.arena[i].l();
        }
    }
}

impl<'a, K: Ord + Copy, V: Copy, T: Copy> Iterator for AvlIter<'a, K, V, T> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        // Go as far left as possible from the current cursor.
        self.push_left_chain();

        // Pop the top of the stack = next in-order node.
        if self.sp == 0 {
            return None;
        }
        self.sp -= 1;
        let i = self.stack[self.sp];
        let n = &self.arena[i];

        // After visiting `i`, the next subtree to process is its right child.
        self.cursor = n.r();

        Some((n.key, n.val))
    }
}

impl<'a, K: Ord + Copy, V: Copy, T: Copy> IntoIterator for &'a AvlTreeMapView<'a, K, V, T> {
    type Item = (K, V);
    type IntoIter = AvlIter<'a, K, V, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::postgres::storage::avl::{AvlTreeMap, AvlTreeMapHeader, Error, Slot};
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_avl_map_basics() {
        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32>::default(); 32];
        let mut m = AvlTreeMap::new(&mut header, &mut buf);

        assert_eq!(m.insert(10, 100), Ok((None, ())));
        assert_eq!(m.insert(5, 50), Ok((None, ())));
        assert_eq!(m.insert(15, 150), Ok((None, ())));
        assert_eq!(m.view().get(&10), Some((100, ())));
        assert_eq!(m.view().get(&7), None);

        // update existing key
        assert_eq!(m.insert(10, 111), Ok((Some(100), ())));
        assert_eq!(m.view().get(&10), Some((111, ())));

        // remove
        assert_eq!(m.remove(&5), Some(50));
        assert_eq!(m.view().get(&5), None);

        // fill more and ensure balanced behavior
        for k in [3, 7, 13, 17, 6, 8, 12, 14, 16, 18] {
            let _ = m.insert(k, k * 10);
        }
        assert!(m.view().contains(&16));
        assert_eq!(m.remove(&16), Some(160));
        assert!(!m.view().contains(&16));

        #[cfg(debug_assertions)]
        m.view().assert_ok();

        for (k, v) in m.view().iter() {
            eprintln!("{k}: {v}");
        }

        eprintln!("{}", size_of::<Slot<u32, u32>>())
    }

    #[test]
    fn test_avl_map_full() {
        const SIZE: usize = 8192 / size_of::<Slot<i32, i32>>();
        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32>::default(); SIZE];
        let mut m = AvlTreeMap::new(&mut header, &mut buf);

        for i in 0..SIZE {
            assert_eq!(m.insert(i as i32, i as i32), Ok((None, ())));
        }

        assert_eq!(m.insert((SIZE + 1) as i32, 0), Err(Error::Full));

        assert_eq!(m.remove(&32), Some(32));
        assert_eq!(m.insert((SIZE + 1) as i32, 0), Ok((None, ())));
        assert_eq!(m.view().get(&((SIZE + 1) as i32)), Some((0, ())));

        #[cfg(debug_assertions)]
        m.view().assert_ok();
    }

    #[test]
    fn test_avl_map_delete_returns_original_value_minimal_case() {
        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32>::default(); 32];
        let mut m = AvlTreeMap::new(&mut header, &mut buf);

        // Minimal failing sequence from the prop-test
        assert_eq!(m.insert(-4, -8), Ok((None, ()))); // left subtree
        assert_eq!(m.insert(63, 126), Ok((None, ()))); // target node
        assert_eq!(m.insert(-4, -8), Ok((Some(-8), ()))); // update same key (no-op on structure)
        assert_eq!(m.insert(64, 128), Ok((None, ()))); // right child of 63
        assert_eq!(m.insert(0, 0), Ok((None, ()))); // right child of -4 -> gives 63 two children

        // Deleting 63 should return its ORIGINAL value (126), not the successor’s (128)
        assert_eq!(m.remove(&63), Some(126));

        #[cfg(debug_assertions)]
        m.view().assert_ok();
    }

    #[test]
    fn test_avl_map_delete_returns_original_value_two_children_general() {
        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32>::default(); 64];
        let mut m = AvlTreeMap::new(&mut header, &mut buf);

        // Build a standard BST shape where the root has two children
        for &(k, v) in &[
            (50, 500),
            (30, 300),
            (70, 700),
            (20, 200),
            (40, 400),
            (60, 600),
            (80, 800),
        ] {
            assert_eq!(m.insert(k, v), Ok((None, ())));
        }

        // Delete node with two children (50). Must return the original value stored at 50.
        assert_eq!(m.remove(&50), Some(500));

        // Structure and inorder remain valid and match a reference BTreeMap after the delete.
        let mut reference = std::collections::BTreeMap::new();
        for &(k, v) in &[
            (30, 300),
            (70, 700),
            (20, 200),
            (40, 400),
            (60, 600),
            (80, 800),
        ] {
            reference.insert(k, v);
        }
        assert!(Iterator::eq(
            m.view().iter(),
            reference.iter().map(|(&k, &v)| (k, v))
        ));

        #[cfg(debug_assertions)]
        m.view().assert_ok();
    }

    #[test]
    fn test_avl_map_get_lte_basic() {
        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32>::default(); 64];
        let mut m = AvlTreeMap::new(&mut header, &mut buf);

        for &(k, v) in &[(10, 100), (20, 200), (30, 300), (40, 400)] {
            assert_eq!(m.insert(k, v), Ok((None, ())));
        }

        // Exact hits
        assert_eq!(m.view().get_lte(&10), Some((10, 100, ())));
        assert_eq!(m.view().get_lte(&40), Some((40, 400, ())));

        // Between keys -> greatest smaller
        assert_eq!(m.view().get_lte(&35), Some((30, 300, ())));
        assert_eq!(m.view().get_lte(&21), Some((20, 200, ())));
        assert_eq!(m.view().get_lte(&11), Some((10, 100, ())));

        // Below minimum
        assert_eq!(m.view().get_lte(&-1), None);

        // Above maximum
        assert_eq!(m.view().get_lte(&99), Some((40, 400, ())));

        #[cfg(debug_assertions)]
        m.view().assert_ok();
    }

    #[test]
    fn test_avl_map_delete_tag_preservation() {
        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32, i32>::default(); 32];
        // Initialize the tree with a proper free list
        let mut m = AvlTreeMap::new(&mut header, &mut buf);
        // Set unique tags after initialization
        for (i, slot) in m.arena.iter_mut().enumerate() {
            slot.tag = i as i32;
        }
        // Insert nodes: 50 (two children), 30 (left), 60 (right, successor)
        assert_eq!(m.insert(50, 500), Ok((None, 0)));
        assert_eq!(m.insert(30, 300), Ok((None, 1)));
        assert_eq!(m.insert(60, 600), Ok((None, 2)));
        // Before delete, key=60 has tag=2
        assert_eq!(m.view().get(&60), Some((600, 2)));
        // Delete 50 (two children)
        assert_eq!(m.remove(&50), Some(500));
        // After delete, key=60 should still have tag=2, now in slot 0
        assert_eq!(m.view().get(&60), Some((600, 2)));
        // Slot 2 (successor) is free, but its tag remains (e.g., for reuse)
        assert_eq!(m.arena[2].tag, 0);
        #[cfg(debug_assertions)]
        m.view().assert_ok();
    }

    #[test]
    fn test_avl_map_deep_tree_and_iterator() {
        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32, i32>::default(); 128];
        let mut m = AvlTreeMap::new(&mut header, &mut buf);
        let mut reference = BTreeMap::new();

        // Initialize unique tags
        for (i, slot) in m.arena.iter_mut().enumerate() {
            slot.tag = i as i32;
        }

        // Insert keys in a way that creates a deep but balanced tree
        // Use a sequence that approximates a balanced tree (e.g., insert in sorted order with some variation)
        let keys = vec![
            100, 50, 150, 25, 75, 125, 175, 12, 37, 62, 87, 112, 137, 162, 187, 6, 18, 31, 43, 56,
            68, 81, 93, 106, 118, 131, 143, 156, 168, 181, 193,
        ];
        for &k in &keys {
            let v = k * 2;
            let result = m.insert(k, v);
            assert!(result.is_ok(), "Insert failed for key {k}");
            let (old_val, tag) = result.unwrap();
            assert_eq!(old_val, None, "Unexpected old value for key {k}");
            reference.insert(k, (v, tag));
        }

        // Verify iterator produces keys in order
        let mut expected_keys = keys.clone();
        expected_keys.sort();
        let iter_keys: Vec<i32> = m.view().iter().map(|(k, _)| k).collect();
        assert_eq!(
            iter_keys, expected_keys,
            "Iterator produced incorrect key order"
        );

        // Verify key/value/tag consistency
        for (&k, &(v, t)) in &reference {
            let got = m.view().get(&k);
            assert_eq!(got, Some((v, t)), "Tag mismatch for key {k}");
        }

        // Delete some keys to trigger two-child deletions and slot reuse
        let delete_keys = vec![100, 50, 150];
        for &k in &delete_keys {
            let removed = m.remove(&k);
            let expected_val = reference.remove(&k).map(|(v, _)| v);
            assert_eq!(removed, expected_val, "Remove failed for key {k}");
        }

        // Re-insert into freed slots
        let new_keys = vec![90, 110, 130];
        for &k in &new_keys {
            let v = k * 2;
            let result = m.insert(k, v);
            assert!(result.is_ok(), "Insert failed for key {k}");
            let (old_val, tag) = result.unwrap();
            assert_eq!(old_val, None, "Unexpected old value for key {k}");
            reference.insert(k, (v, tag));
        }

        // Verify iterator and tags again
        let mut expected_keys: Vec<i32> = reference.keys().copied().collect();
        expected_keys.sort();
        let iter_keys: Vec<i32> = m.view().iter().map(|(k, _)| k).collect();
        assert_eq!(
            iter_keys, expected_keys,
            "Iterator produced incorrect key order after deletions/insertions"
        );

        for (&k, &(v, t)) in &reference {
            let got = m.view().get(&k);
            assert_eq!(got, Some((v, t)), "Tag mismatch for key {k}");
        }

        // Verify tree height is reasonable (should be ~log2(32) ≈ 5-6)
        let max_height = m.view().height(m.header.root);
        assert!(
            max_height <= 10,
            "Tree height {max_height} is unexpectedly large"
        );

        #[cfg(debug_assertions)]
        m.view().assert_ok();
    }

    proptest! {
        #[test]
        fn test_avl_map_random_ops(ops in prop::collection::vec((0..3i32, -100i32..100i32), 1..100)) {
            let mut header = AvlTreeMapHeader::default();
            let mut buf = vec![Slot::<i32, i32>::default(); 1024];
            let mut m = AvlTreeMap::new(&mut header, &mut buf);
            let mut reference = std::collections::BTreeMap::new();

            for (op, k) in ops {
                match op {
                    0 => {
                        // Insert
                        if m.view().len() < m.view().capacity() {
                            let (old_m, _) = m.insert(k, k*2).unwrap();
                            let old_ref = reference.insert(k, k*2);
                            prop_assert_eq!(old_m, old_ref);
                        }
                    },
                    1 => {
                        // Remove
                        let removed_m = m.remove(&k);
                        let removed_ref = reference.remove(&k);
                        prop_assert_eq!(removed_m, removed_ref);
                    },
                    2 => {
                        // Get
                        let val_m = m.view().get(&k);
                        let val_ref = reference.get(&k).copied();
                        prop_assert_eq!(val_m, val_ref.map(|v| (v, ())));
                    },
                    _ => unreachable!(),
                }

                // Verify tree properties
                #[cfg(debug_assertions)]
                m.view().assert_ok();

                // Verify in-order iteration matches reference
                prop_assert!(Iterator::eq(
                    m.view().iter(),
                    reference.iter().map(|(&k,&v)| (k,v))
                ));
            }
        }

        #[test]
        fn test_avl_mapget_lte_matches_btreemap(query in -1000i32..=1000i32, kvs in prop::collection::btree_set(-1000i32..=1000i32, 0..200)) {
            let mut header = AvlTreeMapHeader::default();
            let mut buf = vec![Slot::<i32, i32>::default(); 2048];
            let mut m = AvlTreeMap::new(&mut header, &mut buf);
            let mut refmap = std::collections::BTreeMap::new();

            for &k in &kvs {
                let v = k * 2;
                m.insert(k, v).unwrap();
                refmap.insert(k, v);
            }

            let got = m.view().get_lte(&query);
            let exp = refmap.range(..=query).next_back().map(|(&k, &v)| (k, v, ()));
            prop_assert_eq!(got, exp);

            #[cfg(debug_assertions)]
            m.view().assert_ok();
        }
    }

    #[test]
    fn test_avl_map_stress_delete_reinsert_tags() {
        use rand::Rng;

        let mut header = AvlTreeMapHeader::default();
        let mut buf = vec![Slot::<i32, i32, i32>::default(); 1024];
        let mut m = AvlTreeMap::new(&mut header, &mut buf);
        for (i, slot) in m.arena.iter_mut().enumerate() {
            slot.tag = i as i32; // Initial tags: 0..1023
        }
        let mut key_tags = std::collections::HashMap::new(); // Track expected tag per key

        let mut rng = rand::rng();

        for i in 0..10000 {
            let k: i32 = (rng.random::<i32>() % 100) as i32; // Small key range for duplicates
            let v = k * 2;

            match rng.random_range(0..4) {
                0 => {
                    // Insert/update
                    let result = m.insert(k, v);
                    if let Ok((old_val, tag)) = result {
                        if old_val.is_none() {
                            // New insert: record tag
                            key_tags.insert(k, tag);
                        } else {
                            // Update: tag should match previous (no direct tag change here)
                            let expected_tag = *key_tags.get(&k).unwrap();
                            assert_eq!(tag, expected_tag, "Tag changed on update for key {k}");
                        }
                    }
                }
                1 => {
                    // Delete
                    let removed = m.remove(&k);
                    if removed.is_some() {
                        key_tags.remove(&k); // Tag no longer associated
                    }
                }
                2 => {
                    // Direct tag change (outside initial range)
                    if let Some(slot) = m.get_slot_mut(&k) {
                        let new_tag = 2000 + rng.random_range(0..1000) as i32; // e.g., 2000..2999
                        let old_tag = slot.tag;
                        slot.tag = new_tag;
                        // Update tracking
                        *key_tags.get_mut(&k).unwrap() = new_tag;
                        // Verify the change took effect
                        let got = m.view().get(&k);
                        assert_eq!(
                            got,
                            Some((v, new_tag)),
                            "Direct tag change didn't persist for key {k}"
                        );
                        eprintln!(
                            "Direct tag change: key {k} old_tag {old_tag} -> new_tag {new_tag}"
                        );
                    }
                }
                3 => {
                    // Get/validate existing tag
                    if let Some((_, got_tag)) = m.view().get(&k) {
                        let expected_tag = *key_tags.get(&k).unwrap();
                        assert_eq!(
                            got_tag, expected_tag,
                            "Unexpected tag for key {k} during validation"
                        );
                    }
                }
                _ => unreachable!(),
            }

            // Periodic full validation (every ~100 ops or random)
            if rng.random_bool(0.01) || (i % 100 == 0) {
                for (&k, &expected_tag) in &key_tags {
                    if let Some((_, got_tag)) = m.view().get(&k) {
                        assert_eq!(
                            got_tag, expected_tag,
                            "Tag mismatch during periodic validation for key {k}"
                        );
                    }
                }
                #[cfg(debug_assertions)]
                m.view().assert_ok();
            }
        }

        // Final validation
        for (&k, &expected_tag) in &key_tags {
            let got = m.view().get(&k);
            assert_eq!(
                got.map(|(_, t)| t),
                Some(expected_tag),
                "Final tag mismatch for key {k}"
            );
        }
        #[cfg(debug_assertions)]
        m.view().assert_ok();
    }
    proptest! {
        #[test]
        fn test_avl_map_random_ops_with_tags(
            ops in prop::collection::vec((0..5u8, -100i32..100i32, -100i32..100i32), 1..200)
        ) {
            let mut header = AvlTreeMapHeader::default();
            let mut buf = vec![Slot::<i32, i32, i32>::default(); 1024];
            let mut m = AvlTreeMap::new(&mut header, &mut buf);
            // Initialize unique tags
            for (i, slot) in m.arena.iter_mut().enumerate() {
                slot.tag = i as i32;
            }
            let mut reference = BTreeMap::new();

            for (op, k, v) in ops {
                match op {
                    0 | 1 => {
                        // Insert or update
                        if m.view().len() < m.view().capacity() {
                            let result = m.insert(k, v);
                            match result {
                                Ok((old_val, tag)) => {
                                    // Update reference
                                    let old_ref = reference.insert(k, (v, tag));
                                    // Verify old value matches
                                    prop_assert_eq!(old_val, old_ref.map(|(v, _)| v));
                                    // Verify tag: for new insert, tag comes from slot; for update, it’s the existing tag
                                    let expected_tag = match old_ref {
                                        None => tag, // New insert: tag from inserted slot
                                        Some((_, t)) => t, // Update: tag from previous entry
                                    };
                                    prop_assert_eq!(tag, expected_tag, "Tag mismatch for key {}", k);
                                }
                                Err(Error::Full) => {
                                    // Arena full, skip tag validation and reference update
                                    prop_assert_eq!(m.view().len(), m.view().capacity(), "Expected full arena");
                                }
                            }
                        }
                    }
                    2 => {
                        // Remove
                        let removed_m = m.remove(&k);
                        let removed_ref = reference.remove(&k);
                        prop_assert_eq!(removed_m, removed_ref.map(|(v, _)| v));
                    }
                    3 => {
                        // Get
                        let val_m = m.view().get(&k);
                        let val_ref = reference.get(&k).map(|&(v, t)| (v, t));
                        prop_assert_eq!(val_m, val_ref);
                    }
                    4 => {
                        // Contains
                        let contains_m = m.view().contains(&k);
                        let contains_ref = reference.contains_key(&k);
                        prop_assert_eq!(contains_m, contains_ref);
                    }
                    _ => unreachable!(),
                }

                // Verify tree properties
                #[cfg(debug_assertions)]
                m.view().assert_ok();

                // Verify in-order iteration matches reference (key/value only)
                prop_assert!(Iterator::eq(
                    m.view().iter(),
                    reference.iter().map(|(&k, &(v, _))| (k, v))
                ));

                // Verify tags for all keys in the tree
                for (&k, &(v, t)) in &reference {
                    let got = m.view().get(&k);
                    prop_assert_eq!(got, Some((v, t)), "Tag mismatch for key {}", k);
                }
            }
        }
    }
}
