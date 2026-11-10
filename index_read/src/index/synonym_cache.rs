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

//! What a backend has read from the segments of synonym indexes.
//!
//! pg_search never changes a segment after it writes it, and a rule change or a merge writes a
//! new segment with a new id, so an entry keyed by segment id never goes stale and nothing has to
//! invalidate it. A rule change is never a HOT update, since the synonym index indexes the rule
//! column, so every row a document's ctid leads to has the same rule text. The entries of a
//! segment that a merge removed age out.
//!
//! The liveness of a rule is not here. It depends on the snapshot, so it is checked on every
//! expansion.

use std::cell::RefCell;
use std::hash::Hash;
use std::sync::Arc;

use crate::api::HashMap;
use tantivy::DocId;
use tantivy::index::SegmentId;
use tantivy::synonyms::{CompiledRule, KeyDocs};

/// What one generation of a map holds, in document ids of the keys or in rules. The bound is on
/// what the entries hold and not on their count, since one common key can list
/// `paradedb.synonyms_max_rules_per_key` documents in every segment.
const WEIGHT_PER_GENERATION: usize = 1 << 18;

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
    let weight = keys.iter().map(|key| key.docs.len()).sum::<usize>().max(1);
    CACHE.with_borrow_mut(|cache| cache.keys.insert(key, keys.clone(), weight));
    Ok(keys)
}

pub fn cached_rules(segment: SegmentId, doc: DocId) -> Option<Arc<[CompiledRule]>> {
    CACHE.with_borrow_mut(|cache| cache.rules.get(&(segment, doc)))
}

pub fn cache_rules(segment: SegmentId, doc: DocId, rules: Arc<[CompiledRule]>) {
    let weight = rules.len().max(1);
    CACHE.with_borrow_mut(|cache| cache.rules.insert((segment, doc), rules, weight));
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
        Self::with_capacity(WEIGHT_PER_GENERATION)
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
