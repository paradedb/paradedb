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

#[pgrx::pg_schema]
pub mod pdb {
    use std::sync::Arc;

    use crate::index::fast_fields_helper::FFHelper;
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::index::synonym_index::rule_analyzer;
    use crate::postgres::access::IndexOid;
    use crate::postgres::heap::VisibilityChecker;
    use crate::postgres::index::is_partitioned_index;
    use anyhow::{Result, anyhow};
    use pgrx::prelude::*;
    use tantivy::SegmentOrdinal;
    use tantivy::synonyms::{SynonymExpander, SynonymRule};

    /// The check of the `pdb.synonym_rule` domain. It raises instead of returning false, so the
    /// error says which rule failed and why.
    #[pg_extern(immutable, parallel_safe, strict)]
    fn synonym_rule_is_valid(rule: &str) -> bool {
        if let Err(err) = SynonymRule::parse(rule) {
            pgrx::error!("invalid synonym rule '{rule}': {err}");
        }
        true
    }

    extension_sql!(
        r#"
        CREATE DOMAIN pdb.synonym_rule AS text CHECK (pdb.synonym_rule_is_valid(VALUE));
        "#,
        name = "synonym_rule_domain",
        requires = [synonym_rule_is_valid]
    );

    /// How the synonym index expands `input`, one row per run of tokens. A run that no rule
    /// matches lists only itself.
    #[pg_extern(stable)]
    fn synonym_expansion(
        index: IndexOid,
        input: &str,
    ) -> Result<
        TableIterator<
            'static,
            (
                name!(position, i32),
                name!(token, String),
                name!(alternatives, Vec<String>),
            ),
        >,
    > {
        if is_partitioned_index(index.oid()) {
            return Err(anyhow!(
                "pdb.synonym_expansion reads one synonym index, not a partitioned one"
            ));
        }
        let index = index.read();
        let options = index.options();
        let rule_field = options
            .synonym_rule_field()
            .ok_or_else(|| anyhow!("'{}' is not a synonym index", index.name()))?;
        let mut analyzer = rule_analyzer(options, &rule_field);
        let mut tokens = Vec::new();
        {
            let mut stream = analyzer.token_stream(input);
            while stream.advance() {
                tokens.push(stream.token().text.clone());
            }
        }

        let reader = SearchIndexReader::empty(&index, MvccSatisfies::Snapshot)?;
        let field = reader
            .schema()
            .search_field(rule_field.as_ref())
            .ok_or_else(|| anyhow!("synonym rule field `{rule_field}` is not in the index"))?
            .field();
        // A deleted rule stays in its segment until `VACUUM`, so each matched rule is checked
        // against the heap.
        let mut visibility = VisibilityChecker::with_rel_and_snap(index.heap(), unsafe {
            pg_sys::GetActiveSnapshot()
        })
        .with_ffhelper(Arc::new(FFHelper::for_ctid(&reader)));
        let expander = SynonymExpander::new(reader.segment_readers(), field, analyzer)?;
        let expansions = expander.expand_with(&tokens, |segment, doc| {
            visibility.check_doc(segment as SegmentOrdinal, doc)
        })?;

        let mut position = 0;
        let rows = expansions
            .into_iter()
            .map(|expansion| {
                let token = expansion.tokens.join(" ");
                let alternatives = match expansion.alternatives {
                    Some(alternatives) => alternatives.iter().map(|alt| alt.join(" ")).collect(),
                    None => vec![token.clone()],
                };
                let row = (position, token, alternatives);
                position += expansion.tokens.len() as i32;
                row
            })
            .collect::<Vec<_>>();
        Ok(TableIterator::new(rows))
    }
}
