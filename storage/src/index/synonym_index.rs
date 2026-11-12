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

use crate::api::FieldName;
use crate::nodecast;
use crate::postgres::catalog::synonym_rule_typoid;
use crate::postgres::options::BM25IndexOptions;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::strip_tokenizer_cast;
use crate::schema::{SearchFieldConfig, SearchIndexSchema};
use pgrx::{PgList, pg_sys};
use tantivy::TantivyDocument;
use tantivy::schema::{Field, TextOptions, Value};
use tantivy::synonyms::{SynonymRule, SynonymRuleTokenizer};
use tantivy::tokenizer::{TextAnalyzer, TokenizerManager};
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
                    .and_then(|expression| nodecast!(Var, strip_tokenizer_cast(expression)))
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

/// Refuses a `pdb.synonym_rule` column in a search index. A document of a synonym index is one
/// rule, so a rule column next to other columns would mix rules with other fields.
pub(crate) fn refuse_synonym_rule_columns(options: &BM25IndexOptions) {
    if let Some(column) = options.synonym_rule_columns().first() {
        panic!("the `pdb.synonym_rule` column `{column}` must be the only column of its index");
    }
}

pub(crate) fn validate_synonym_index(options: &BM25IndexOptions) {
    let rule_field = options
        .synonym_rule_field()
        .expect("a synonym index has a rule field");
    if options.attributes().len() != 1 {
        panic!("a synonym index has one column, the `pdb.synonym_rule` column `{rule_field}`");
    }
    // A partitioned build indexes its rows after the scan, where the rule check does not run.
    if !options.partition_by().is_empty() {
        panic!("a synonym index cannot use `partition_by`");
    }
}

pub(crate) fn register_rule_tokenizer(options: &BM25IndexOptions, manager: &TokenizerManager) {
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

/// A key matches only the tokens of the tokenizer that analyzed it, so a query compares its
/// tokenizer with this one.
pub fn rule_tokenizer(index: &PgSearchRelation) -> Option<SearchTokenizer> {
    let options = index.options();
    let field = options.synonym_rule_field()?;
    options.field_config_or_default(&field).tokenizer().cloned()
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
    /// `None` for a search index.
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
