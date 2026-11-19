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

//! An immutable B+ tree implementation.
//!
//! Suppose records with keys 0 through 199 are stored in four leaf pages. We want
//! to find the page containing key 130 without reading all four pages. Index each
//! page by its first key. If each directory page holds only two entries, we get:
//!
//! ```text
//! root (level 1)
//! ├──   0 -> A (level 0)
//! │          ├──  0 -> leaf a: records with keys   0-39
//! │          └── 40 -> leaf b: records with keys  40-99
//! └── 100 -> B (level 0)
//!            ├── 100 -> leaf c: records with keys 100-159
//!            └── 160 -> leaf d: records with keys 160-199
//! ```
//!
//! To look up key 130:
//! 1. At the root, choose 100 -> B: 100 is the largest start not greater than 130.
//! 2. At B, choose 100 -> leaf c: the next page starts at 160, past our key.
//! 3. Return leaf c; the caller finds record 130 inside it.
//!
//! Only root, B, and leaf c are visited. A and the other leaf pages stay unloaded.
//! Level 0 points to leaf data; higher levels point to lower directory nodes.
//!
//! Add a layer whenever the root would need more pointers than it can hold.
//! With room for two pointers in every node:
//!
//! - 1–2 leaf pages: root -> leaves.
//! - 3–4 leaf pages: root -> nodes -> leaves (as above).
//! - 5–8 leaf pages: root -> nodes -> nodes -> leaves.
//!
//! The caller sets the actual capacities, which can differ for the root and other nodes.

use std::marker::PhantomData;
use std::sync::Arc;

/// An entry mapping a starting key to a child node or leaf page address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<K, A> {
    /// Starting key covered by this child entry.
    pub start: K,
    /// Storage address of the child node or leaf page.
    pub address: A,
}

/// Half-open key interval `[start, end)` covered by a node or leaf.
#[derive(Debug, Clone, Copy)]
pub struct Bounds<K> {
    /// Inclusive lower bound of keys covered.
    pub start: K,
    /// Optional exclusive upper bound of keys covered.
    pub end: Option<K>,
}

impl<K: Ord + Copy> Bounds<K> {
    fn contains(&self, key: K) -> bool {
        key >= self.start && self.end.is_none_or(|end| key < end)
    }
}

/// Level zero points to caller-defined leaves; higher levels point to directory nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<K, A> {
    pub level: u32,
    pub entries: Vec<Entry<K, A>>,
}

impl<K: Ord + Copy, A: Copy> Node<K, A> {
    /// Builds a tree bottom-up from sorted entries, spilling full nodes via `write`.
    pub fn build(
        entries: Vec<Entry<K, A>>,
        root_capacity: usize,
        page_capacity: usize,
        mut write: impl FnMut(&Self) -> A,
    ) -> Self {
        assert!(root_capacity > 0 && page_capacity > 1);
        assert!(!entries.is_empty());
        assert!(entries.windows(2).all(|pair| pair[0].start < pair[1].start));
        let mut node = Self { level: 0, entries };
        while node.entries.len() > root_capacity {
            let entries = node
                .entries
                .chunks(page_capacity)
                .map(|entries| {
                    let child = Self {
                        level: node.level,
                        entries: entries.to_vec(),
                    };
                    Entry {
                        start: entries[0].start,
                        address: write(&child),
                    }
                })
                .collect();
            node = Self {
                level: node.level + 1,
                entries,
            };
        }
        node
    }
}

/// Interface for reading node entries and levels, whether from memory or pinned pages.
pub trait ReadNode<K: Ord + Copy, A: Copy> {
    /// Returns the tree level of this node (0 points to leaves).
    fn level(&self) -> u32;
    /// Returns the number of entries in this node.
    fn len(&self) -> usize;
    /// Returns the entry at the given index.
    fn entry(&self, index: usize) -> Entry<K, A>;
    /// Returns the partition point for `key`, identifying the child covering `key`.
    fn partition_point(&self, key: K) -> usize;

    /// Verifies that this node's entries are monotonic and satisfy the expected `bounds`.
    fn valid_for(&self, bounds: Bounds<K>) -> bool {
        if self.len() == 0 || self.entry(0).start != bounds.start {
            return false;
        }
        let mut previous = None;
        (0..self.len()).all(|i| {
            let start = self.entry(i).start;
            let valid = bounds.contains(start) && previous.is_none_or(|prev| prev < start);
            previous = Some(start);
            valid
        })
    }
}

impl<K: Ord + Copy, A: Copy> ReadNode<K, A> for Arc<Node<K, A>> {
    fn level(&self) -> u32 {
        self.level
    }
    fn len(&self) -> usize {
        self.entries.len()
    }
    fn entry(&self, index: usize) -> Entry<K, A> {
        self.entries[index]
    }
    fn partition_point(&self, key: K) -> usize {
        self.entries.partition_point(|entry| entry.start <= key)
    }
}

/// Error returned when a tree node fails structural or boundary validation.
#[derive(Debug)]
pub struct InvalidNode;

/// Supplies immutable pages. `read_leaf` validates any known bounds before caching the leaf.
pub trait PageReader<K: Ord + Copy, A: Copy> {
    type Node: ReadNode<K, A>;
    type Leaf;

    /// Reads and validates a directory node at `address`.
    fn read_node(&self, address: A, bounds: Bounds<K>) -> Result<Self::Node, InvalidNode>;
    /// Reads and validates a leaf page at `address`.
    fn read_leaf(&self, address: A, bounds: Bounds<K>) -> Result<Self::Leaf, InvalidNode>;
}

type CachedChild<K, A, L, N> = Option<Box<Child<K, A, L, N>>>;

/// Lazily traverses an immutable B+ tree, caching loaded child nodes and leaves.
#[derive(Debug)]
pub struct TreeReader<K, A, L, N = Arc<Node<K, A>>> {
    node: N,
    address: PhantomData<A>,
    bounds: Bounds<K>,
    children: Vec<CachedChild<K, A, L, N>>,
    current: Option<(usize, Bounds<K>)>,
}

#[derive(Debug)]
enum Child<K, A, L, N> {
    Node(TreeReader<K, A, L, N>),
    Leaf(L),
}

impl<K: Ord + Copy, A: Copy, L, N: ReadNode<K, A>> TreeReader<K, A, L, N> {
    /// Creates a reader rooted at `node` covering keys up to `end`.
    pub fn new(node: N, end: Option<K>) -> Result<Self, InvalidNode> {
        let bounds = Bounds {
            start: if node.len() == 0 {
                return Err(InvalidNode);
            } else {
                node.entry(0).start
            },
            end,
        };
        if !node.valid_for(bounds) {
            return Err(InvalidNode);
        }
        let children = (0..node.len()).map(|_| None).collect();
        Ok(Self {
            node,
            address: PhantomData,
            bounds,
            children,
            current: None,
        })
    }

    /// Traverses the tree to find and return the leaf containing `key`, loading nodes on demand.
    pub fn get<R: PageReader<K, A, Node = N, Leaf = L>>(
        &mut self,
        reader: &R,
        key: K,
    ) -> Result<Option<&mut L>, InvalidNode> {
        if !self.bounds.contains(key) {
            return Ok(None);
        }
        let (pos, bounds) = match self.current {
            Some(current) if current.1.contains(key) => current,
            _ => {
                let pos = self.node.partition_point(key) - 1;
                let bounds = Bounds {
                    start: self.node.entry(pos).start,
                    end: if pos + 1 < self.node.len() {
                        Some(self.node.entry(pos + 1).start)
                    } else {
                        self.bounds.end
                    },
                };
                (pos, bounds)
            }
        };
        if self.children[pos].is_none() {
            let address = self.node.entry(pos).address;
            let child = if self.node.level() == 0 {
                Child::Leaf(reader.read_leaf(address, bounds)?)
            } else {
                let node = reader.read_node(address, bounds)?;
                if node.level() != self.node.level() - 1
                    || node.len() == 0
                    || node.entry(0).start != bounds.start
                {
                    return Err(InvalidNode);
                }
                Child::Node(Self::new(node, bounds.end)?)
            };
            self.children[pos] = Some(Box::new(child));
        }
        self.current = Some((pos, bounds));
        match self.children[pos].as_deref_mut().unwrap() {
            Child::Leaf(leaf) => Ok(Some(leaf)),
            Child::Node(node) => node.get(reader, key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::ops::Range;

    struct MemoryPages {
        nodes: Vec<Node<u32, usize>>,
        leaves: Vec<Range<u32>>,
        reads: Cell<usize>,
    }

    impl PageReader<u32, usize> for MemoryPages {
        type Node = Arc<Node<u32, usize>>;
        type Leaf = Range<u32>;

        fn read_node(
            &self,
            address: usize,
            _: Bounds<u32>,
        ) -> Result<Arc<Node<u32, usize>>, InvalidNode> {
            self.reads.set(self.reads.get() + 1);
            self.nodes
                .get(address)
                .cloned()
                .map(Arc::new)
                .ok_or(InvalidNode)
        }

        fn read_leaf(
            &self,
            address: usize,
            bounds: Bounds<u32>,
        ) -> Result<Self::Leaf, InvalidNode> {
            self.reads.set(self.reads.get() + 1);
            let leaf = self.leaves.get(address).ok_or(InvalidNode)?;
            assert_eq!(bounds.start, leaf.start);
            assert!(bounds.end.is_none_or(|end| end == leaf.end));
            Ok(leaf.clone())
        }
    }

    #[test]
    fn immutable_tree_paths_and_bounds() {
        for count in [1, 2, 3, 6, 7, 19, 55, 200] {
            for known_end in [false, true] {
                let mut pages = MemoryPages {
                    nodes: Vec::new(),
                    leaves: (0..count).map(|i| 10 + i * 7..17 + i * 7).collect(),
                    reads: Cell::new(0),
                };
                let entries = pages
                    .leaves
                    .iter()
                    .enumerate()
                    .map(|(address, range)| Entry {
                        start: range.start,
                        address,
                    })
                    .collect();
                let root = Node::build(entries, 2, 3, |node| {
                    let address = pages.nodes.len();
                    pages.nodes.push(node.clone());
                    address
                });
                let level = root.level;
                let end = 10 + count * 7;
                let mut tree = TreeReader::new(Arc::new(root), known_end.then_some(end)).unwrap();
                assert!(tree.get(&pages, 9).unwrap().is_none());
                assert_eq!(pages.reads.get(), 0);
                assert_eq!(
                    tree.get(&pages, end - 1).unwrap().unwrap(),
                    pages.leaves.last().unwrap()
                );
                assert_eq!(pages.reads.get(), level as usize + 1);
                for _ in 0..5 {
                    assert_eq!(
                        tree.get(&pages, end - 1).unwrap().unwrap(),
                        pages.leaves.last().unwrap()
                    );
                }
                assert_eq!(pages.reads.get(), level as usize + 1);
                for key in (10..end).rev() {
                    assert_eq!(
                        tree.get(&pages, key).unwrap().unwrap(),
                        &pages.leaves[((key - 10) / 7) as usize]
                    );
                }
                assert_eq!(pages.reads.get(), pages.nodes.len() + pages.leaves.len());
                if known_end {
                    assert!(tree.get(&pages, end).unwrap().is_none());
                } else {
                    assert_eq!(
                        tree.get(&pages, end).unwrap().unwrap(),
                        pages.leaves.last().unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn immutable_tree_rejects_invalid_children() {
        let entries = vec![
            Entry {
                start: 10,
                address: 0,
            },
            Entry {
                start: 20,
                address: 1,
            },
        ];
        let child = Node { level: 0, entries };
        for invalid in [
            Node {
                level: 1,
                ..child.clone()
            },
            Node {
                level: 0,
                entries: vec![Entry {
                    start: 11,
                    address: 0,
                }],
            },
            Node {
                level: 0,
                entries: vec![
                    Entry {
                        start: 10,
                        address: 0,
                    },
                    Entry {
                        start: 30,
                        address: 1,
                    },
                ],
            },
            Node {
                level: 0,
                entries: vec![
                    Entry {
                        start: 10,
                        address: 0,
                    },
                    Entry {
                        start: 10,
                        address: 1,
                    },
                ],
            },
            Node {
                level: 0,
                entries: Vec::new(),
            },
        ] {
            let pages = MemoryPages {
                nodes: vec![invalid],
                leaves: vec![10..20, 20..30],
                reads: Cell::new(0),
            };
            let root = Node {
                level: 1,
                entries: vec![Entry {
                    start: 10,
                    address: 0,
                }],
            };
            let mut tree = TreeReader::new(Arc::new(root), Some(30)).unwrap();
            assert!(tree.get(&pages, 10).is_err());
            assert_eq!(pages.reads.get(), 1);
        }
    }
}
