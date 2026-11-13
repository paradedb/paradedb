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

//! Keys and compiled rules a backend has read, by segment id.
//!
//! A segment of a synonym index never changes, and a HOT update cannot change the rule column, so
//! an entry never goes stale. Liveness depends on the snapshot, so it is not cached.

use std::cell::RefCell;
use std::hash::Hash;
use std::sync::Arc;

use crate::api::HashMap;
use tantivy::DocId;
use tantivy::index::SegmentId;
use tantivy::synonyms::{CompiledRule, KeyDocs};

/// The estimated bytes that one generation of a map holds. Every backend has its own two maps, so
/// the bound is small.
const BYTES_PER_GENERATION: usize = 2 << 20;

/// An estimate of what the map slot, the `Arc` and the allocator add to an entry. A token with no
/// key is an entry too, so the estimate also bounds those.
const ENTRY_OVERHEAD: usize = 192;

thread_local! {
    static CACHE: RefCell<SynonymCache> = RefCell::new(SynonymCache::default());
}

#[derive(Default)]
struct SynonymCache {
    /// The keys of a segment that start with a token, read with a limit of rules per key.
    keys: BoundedMap<(SegmentId, String, usize), Arc<[KeyDocs]>>,
    /// The compiled rules of a document of a segment.
    rules: BoundedMap<(SegmentId, DocId), Arc<[CompiledRule]>>,
}

pub fn cached_keys(
    segment: SegmentId,
    token: &str,
    max_rules_per_key: usize,
    read: impl FnOnce() -> tantivy::Result<Arc<[KeyDocs]>>,
) -> tantivy::Result<Arc<[KeyDocs]>> {
    let key = (segment, token.to_string(), max_rules_per_key);
    if let Some(keys) = CACHE.with_borrow_mut(|cache| cache.keys.get(&key)) {
        return Ok(keys);
    }
    let keys = read()?;
    let bytes = ENTRY_OVERHEAD
        + token.len()
        + keys
            .iter()
            .map(|key| {
                size_of::<KeyDocs>() + tokens_bytes(&key.key) + size_of_val(key.docs.as_slice())
            })
            .sum::<usize>();
    CACHE.with_borrow_mut(|cache| cache.keys.insert(key, keys.clone(), bytes));
    Ok(keys)
}

pub fn cached_rules(segment: SegmentId, doc: DocId) -> Option<Arc<[CompiledRule]>> {
    CACHE.with_borrow_mut(|cache| cache.rules.get(&(segment, doc)))
}

pub fn cache_rules(segment: SegmentId, doc: DocId, rules: Arc<[CompiledRule]>) {
    let terms_bytes = |terms: &[Vec<String>]| {
        terms
            .iter()
            .map(|term| size_of::<Vec<String>>() + tokens_bytes(term))
            .sum::<usize>()
    };
    let bytes = ENTRY_OVERHEAD
        + rules
            .iter()
            .map(|rule| {
                size_of::<CompiledRule>()
                    + terms_bytes(&rule.keys)
                    + terms_bytes(&rule.alternatives)
            })
            .sum::<usize>();
    CACHE.with_borrow_mut(|cache| cache.rules.insert((segment, doc), rules, bytes));
}

fn tokens_bytes(tokens: &[String]) -> usize {
    tokens
        .iter()
        .map(|token| size_of::<String>() + token.len())
        .sum()
}

/// A map that keeps the entries used in its last two generations. When the young generation is
/// full it becomes the old one, so an entry not used for a whole generation is dropped.
struct BoundedMap<K, V> {
    young: HashMap<K, (V, usize)>,
    old: HashMap<K, (V, usize)>,
    young_weight: usize,
    capacity: usize,
}

impl<K, V> Default for BoundedMap<K, V> {
    fn default() -> Self {
        Self::with_capacity(BYTES_PER_GENERATION)
    }
}

impl<K, V> BoundedMap<K, V> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            young: HashMap::default(),
            old: HashMap::default(),
            young_weight: 0,
            capacity,
        }
    }
}

impl<K: Eq + Hash, V: Clone> BoundedMap<K, V> {
    fn get(&mut self, key: &K) -> Option<V> {
        if let Some((value, _)) = self.young.get(key) {
            return Some(value.clone());
        }
        let (key, (value, weight)) = self.old.remove_entry(key)?;
        self.insert(key, value.clone(), weight);
        Some(value)
    }

    fn insert(&mut self, key: K, value: V, weight: usize) {
        if self.young_weight + weight > self.capacity && !self.young.is_empty() {
            self.old = std::mem::take(&mut self.young);
            self.young_weight = 0;
        }
        self.young.insert(key, (value, weight));
        self.young_weight += weight;
    }
}

#[cfg(test)]
mod tests {
    use super::BoundedMap;

    #[test]
    fn an_entry_used_in_a_generation_survives_the_next() {
        let mut map = BoundedMap::with_capacity(2);
        map.insert(1, "a", 1);
        map.insert(2, "b", 1);
        map.insert(3, "c", 1);
        assert_eq!(map.get(&1), Some("a"));
        map.insert(4, "d", 1);
        map.insert(5, "e", 1);
        assert_eq!(map.get(&1), Some("a"));
        assert_eq!(map.get(&2), None);
    }
}
