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

//! A synonym index is a `paradedb` index whose one column is a `pdb.synonym_rule`. Its rule
//! field is indexed with a [`SynonymRuleTokenizer`] over the column's own tokenizer, so each key
//! of a rule is one term of the field.

use std::sync::Arc;

use crate::api::FieldName;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::reader::index::SearchIndexReader;
use crate::nodecast;
use crate::postgres::catalog::synonym_rule_typoid;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::options::BM25IndexOptions;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::strip_tokenizer_cast;
use crate::schema::{SearchFieldConfig, SearchIndexSchema};
use anyhow::anyhow;
use pgrx::{FromDatum, PgList, pg_sys};
use tantivy::fastfield::AliveBitSet;
use tantivy::schema::{Field, TextOptions, Value};
use tantivy::synonyms::{
    CompiledRule, DEFAULT_MAX_RULES_PER_KEY, KeyDocs, SegmentSynonyms, SynonymRule,
    SynonymRuleTokenizer, SynonymSource,
};
use tantivy::tokenizer::{TextAnalyzer, TokenizerManager};
use tantivy::{DocId, SegmentOrdinal, TantivyDocument};
use tokenizers::SearchTokenizer;

/// True when the one key column of the index reads a `pdb.synonym_rule` column, plain or through
/// a tokenizer cast. It reads only the catalog, since a partitioned index has no storage.
///
/// # Safety
///
/// `indexrel` must be an open index relation.
pub unsafe fn is_synonym_index_relation(indexrel: pg_sys::Relation) -> bool {
    let Some(typoid) = synonym_rule_typoid() else {
        return false;
    };
    unsafe {
        let index = &*(*indexrel).rd_index;
        if index.indnkeyatts != 1 {
            return false;
        }
        let attnums = index.indkey.values.as_slice(index.indnkeyatts as usize);
        let expressions =
            PgList::<pg_sys::Node>::from_pg(pg_sys::RelationGetIndexExpressions(indexrel));
        let mut expressions = expressions.iter_ptr();
        attnums.iter().any(|&attnum| {
            let column = if attnum != 0 {
                Some(attnum)
            } else {
                expressions
                    .next()
                    .and_then(|expression| nodecast!(Var, T_Var, strip_tokenizer_cast(expression)))
                    .map(|var| (*var).varattno)
                    .filter(|&attno| attno > 0)
            };
            column.is_some_and(|attnum| pg_sys::get_atttype(index.indrelid, attnum) == typoid)
        })
    }
}

/// The Tantivy schema stores this name, so it depends only on the column's tokenizer.
fn rule_tokenizer_name(tokenizer: &SearchTokenizer) -> String {
    format!("synonym_rule[{}]", tokenizer.name())
}

/// The analyzer of the rule column. The keys of a rule and the tokens of a query both go
/// through it.
pub fn rule_analyzer(options: &BM25IndexOptions, field: &FieldName) -> TextAnalyzer {
    options
        .field_config_or_default(field)
        .tokenizer()
        .and_then(SearchTokenizer::to_tantivy_tokenizer)
        .unwrap_or_else(|| panic!("synonym rule field `{field}` has no tokenizer"))
}

/// The Tantivy options of the rule field, with the rule tokenizer in place of the column's.
pub fn rule_field_options(config: SearchFieldConfig) -> TextOptions {
    let name = rule_tokenizer_name(
        config
            .tokenizer()
            .expect("a synonym rule field has a tokenizer"),
    );
    let options = TextOptions::from(config);
    let indexing = options
        .get_indexing_options()
        .expect("a synonym rule field is indexed")
        .clone()
        .set_tokenizer(&name);
    options.set_indexing_options(indexing)
}

/// Registers the rule tokenizer of a synonym index. Other indexes register nothing.
pub fn register_rule_tokenizer(options: &BM25IndexOptions, manager: &TokenizerManager) {
    let Some(field) = options.synonym_rule_field() else {
        return;
    };
    let config = options.field_config_or_default(&field);
    let Some(tokenizer) = config.tokenizer() else {
        return;
    };
    if let Some(analyzer) = tokenizer.to_tantivy_tokenizer() {
        manager.register(
            &rule_tokenizer_name(tokenizer),
            SynonymRuleTokenizer::new(analyzer),
        );
    }
}

/// Raises an ERROR when `rule` is not a Solr rule, or when the analyzer drops a word of one of
/// its terms. A blank line or a `#` comment passes and indexes no key.
pub fn check_rule(rule: &str, analyzer: &mut TextAnalyzer) {
    let checked = SynonymRule::parse(rule).and_then(|parsed| match parsed {
        Some(parsed) => parsed.check(analyzer),
        None => Ok(()),
    });
    if let Err(err) = checked {
        pgrx::error!("invalid synonym rule '{rule}': {err}");
    }
}

/// Checks the rule of a row before the row reaches a synonym index, since the rule tokenizer
/// indexes no key for an invalid rule and the row would never expand.
pub struct RuleCheck {
    field: Field,
    analyzer: TextAnalyzer,
}

impl RuleCheck {
    /// `None` when the index is not a synonym index.
    pub fn for_index(schema: &SearchIndexSchema, options: &BM25IndexOptions) -> Option<Self> {
        let name = options.synonym_rule_field()?;
        let field = schema.search_field(name.as_ref())?.field();
        Some(Self {
            field,
            analyzer: rule_analyzer(options, &name),
        })
    }

    pub fn check(&mut self, document: &TantivyDocument) {
        for value in document.get_all(self.field) {
            if let Some(rule) = value.as_str() {
                check_rule(rule, &mut self.analyzer);
            }
        }
    }
}

/// The rules of a synonym index, read through its table. A matched document's rule text comes
/// from its heap row, fetched under the snapshot, and the same fetch says whether the rule is
/// alive.
pub struct HeapRuleSource {
    segments: Vec<(SegmentSynonyms, Option<AliveBitSet>)>,
    ctids: Arc<FFHelper>,
    visibility: VisibilityChecker,
    slot: *mut pg_sys::TupleTableSlot,
    attnum: i32,
    analyzer: TextAnalyzer,
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
                    segment_reader.alive_bitset().cloned(),
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
        })
    }

    /// Releases the slot and its buffer pin.
    pub fn close(self) {
        unsafe { pg_sys::ExecDropSingleTupleTableSlot(self.slot) };
    }
}

impl SynonymSource for HeapRuleSource {
    fn num_segments(&self) -> usize {
        self.segments.len()
    }

    fn token_keys(&mut self, segment: usize, token: &str) -> tantivy::Result<Arc<[KeyDocs]>> {
        Ok(self.segments[segment]
            .0
            .token_keys(token, DEFAULT_MAX_RULES_PER_KEY)?
            .into())
    }

    fn alive_rules(
        &mut self,
        segment: usize,
        doc: DocId,
    ) -> tantivy::Result<Option<Arc<[CompiledRule]>>> {
        // VACUUM marks a deleted rule in the bitset before Postgres reuses its ctid, so the bitset
        // comes first and a reused ctid never reads another row's rule.
        if self.segments[segment]
            .1
            .as_ref()
            .is_some_and(|alive| !alive.is_alive(doc))
        {
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
