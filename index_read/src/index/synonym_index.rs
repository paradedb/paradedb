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

pub use storage::index::synonym_index::*;

use std::ffi::{CString, c_char};
use std::sync::Arc;

use crate::gucs;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::access::IndexAccess;
use crate::postgres::catalog::OidExt;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::IndexType;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::ActiveSnapshotGuard;
use anyhow::anyhow;
use pgrx::{FromDatum, IntoDatum, pg_sys};
use tantivy::synonyms::{CompiledRule, Expansion, KeyDocs, SegmentSynonyms, SynonymSource, expand};
use tantivy::tokenizer::TextAnalyzer;
use tantivy::{DocId, SegmentOrdinal, SegmentReader};

/// Opens the synonym index that a text index in `namespace` names. The name is resolved again on
/// each query, so a rename or a `REINDEX` of the synonym index never leaves a stale reference.
/// The owner of the text index attached the rules, so a query reads them with no check of the
/// querying role's rights on the rules table.
pub fn open_synonym_index(namespace: pg_sys::Oid, name: &str) -> anyhow::Result<IndexAccess> {
    let oid = resolve_relation(namespace, name)
        .ok_or_else(|| anyhow!("synonym index \"{name}\" does not exist"))?;
    if unsafe { pg_sys::get_rel_relkind(oid) } != pg_sys::RELKIND_INDEX as c_char {
        return Err(anyhow!("\"{name}\" is not an index"));
    }
    let index = IndexAccess::open(oid, pg_sys::AccessShareLock as _);
    if !unsafe { (*index.rd_rel).relam.is_paradedb_am() }
        || index.index_type() != IndexType::Synonym
    {
        return Err(anyhow!("\"{name}\" is not a synonym index"));
    }
    Ok(index)
}

/// A bare name is looked up in `namespace` first, so the lookup does not depend on the
/// `search_path` of the querying role.
fn resolve_relation(namespace: pg_sys::Oid, name: &str) -> Option<pg_sys::Oid> {
    let in_namespace = CString::new(name)
        .ok()
        .map(|name| unsafe { pg_sys::get_relname_relid(name.as_ptr(), namespace) })
        .filter(|&oid| oid != pg_sys::InvalidOid);
    in_namespace.or_else(|| {
        unsafe {
            pgrx::direct_function_call::<pg_sys::Oid>(pg_sys::to_regclass, &[name.into_datum()])
        }
        .filter(|&oid| oid != pg_sys::InvalidOid)
    })
}

/// Expands `tokens` with the rules of the synonym index, under the active snapshot and the
/// limits of the `paradedb.synonyms_max_*` settings.
pub fn expand_with_index(index: &IndexAccess, tokens: &[String]) -> anyhow::Result<Vec<Expansion>> {
    let _snapshot = ActiveSnapshotGuard::ensure();
    let reader = SearchIndexReader::empty(index, MvccSatisfies::Snapshot)?;
    let mut source =
        unsafe { HeapRuleSource::open(index, index.heap(), &reader, pg_sys::GetActiveSnapshot())? };
    let expansions = expand(tokens, &mut source);
    source.close();
    let mut expansions = expansions?;
    let max_alternatives = gucs::synonyms_max_alternatives();
    for expansion in &mut expansions {
        expansion.cap(max_alternatives);
    }
    Ok(expansions)
}

/// The rules of a synonym index, read through its table. A matched document's rule text comes
/// from its heap row, fetched under the snapshot, and the same fetch says whether the rule is
/// alive.
pub struct HeapRuleSource {
    segments: Vec<(SegmentSynonyms, SegmentReader)>,
    ctids: Arc<FFHelper>,
    visibility: VisibilityChecker,
    slot: *mut pg_sys::TupleTableSlot,
    attnum: i32,
    analyzer: TextAnalyzer,
    max_rules_per_key: usize,
}

impl HeapRuleSource {
    /// A source over the segments of `reader`, an open reader of the synonym index `index`.
    ///
    /// # Safety
    ///
    /// `snapshot` must stay registered while the source is in use, and the caller must call
    /// [`Self::close`] when it is done.
    pub unsafe fn open(
        index: &PgSearchRelation,
        heaprel: &PgSearchRelation,
        reader: &SearchIndexReader,
        snapshot: pg_sys::Snapshot,
    ) -> anyhow::Result<Self> {
        let options = index.options();
        let rule_field = options
            .synonym_rule_field()
            .ok_or_else(|| anyhow!("'{}' is not a synonym index", index.name()))?;
        let field = reader
            .schema()
            .search_field(rule_field.as_ref())
            .ok_or_else(|| anyhow!("synonym rule field `{rule_field}` is not in the index"))?
            .field();
        let attno = options
            .attributes()
            .get(&rule_field)
            .and_then(|attr| attr.source.heap_attno())
            .ok_or_else(|| anyhow!("synonym rule field `{rule_field}` reads no table column"))?;
        let segments = reader
            .segment_readers()
            .iter()
            .map(|segment_reader| {
                Ok((
                    SegmentSynonyms::open(segment_reader, field)?,
                    segment_reader.clone(),
                ))
            })
            .collect::<tantivy::Result<_>>()?;
        let ctids = Arc::new(FFHelper::for_ctid(reader));
        let visibility =
            VisibilityChecker::with_rel_and_snap(heaprel, snapshot).with_ffhelper(ctids.clone());
        let slot = unsafe {
            pg_sys::MakeSingleTupleTableSlot(heaprel.rd_att, &pg_sys::TTSOpsBufferHeapTuple)
        };
        Ok(Self {
            segments,
            ctids,
            visibility,
            slot,
            attnum: attno as i32 + 1,
            analyzer: rule_analyzer(options, &rule_field),
            max_rules_per_key: gucs::synonyms_max_rules_per_key(),
        })
    }

    /// Releases the slot and its buffer pin.
    pub fn close(self) {
        unsafe { pg_sys::ExecDropSingleTupleTableSlot(self.slot) };
    }
}

impl SynonymSource for HeapRuleSource {
    fn token_keys(&mut self, token: &str) -> tantivy::Result<Vec<Arc<[KeyDocs]>>> {
        self.segments
            .iter()
            .map(|(synonyms, _)| Ok(synonyms.token_keys(token, self.max_rules_per_key)?.into()))
            .collect()
    }

    fn alive_rules(
        &mut self,
        segment: usize,
        doc: DocId,
    ) -> tantivy::Result<Option<Arc<[CompiledRule]>>> {
        // VACUUM marks a deleted rule in the bitset before Postgres reuses its ctid, so the bitset
        // comes first and a reused ctid never reads another row's rule.
        if self.segments[segment].1.is_deleted(doc) {
            return Ok(None);
        }
        let Some(ctid) = self.ctids.ctid(segment as SegmentOrdinal).as_u64(doc) else {
            return Ok(None);
        };
        let (slot, attnum) = (self.slot, self.attnum);
        let text = self.visibility.exec_if_visible(ctid, slot, |_| unsafe {
            let mut isnull = false;
            let datum = pg_sys::slot_getattr(slot, attnum, &mut isnull);
            String::from_datum(datum, isnull)
        });
        let Some(text) = text else {
            return Ok(None);
        };
        let rules: Vec<CompiledRule> = text
            .iter()
            .filter_map(|line| CompiledRule::from_line(line, &mut self.analyzer))
            .collect();
        Ok(Some(rules.into()))
    }
}
