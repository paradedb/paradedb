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

use crate::query::synonyms::{compiled_set, tokenize};
use pgrx::datum::TimestampWithTimeZone;
use pgrx::heap_tuple::{PgHeapTuple, PgHeapTupleError};
use pgrx::pgbox::WhoAllocated;
use pgrx::trigger_support::PgTrigger;
use pgrx::{JsonB, extension_sql, iter::TableIterator, name, pg_extern, pg_sys, pg_trigger};
use tokenizers::SearchTokenizer;

extension_sql!(
    r#"
CREATE TABLE paradedb.synonym_sets (
    name       text NOT NULL PRIMARY KEY,
    rules      text[] NOT NULL,
    version    bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.now()
);
SELECT pg_catalog.pg_extension_config_dump('paradedb.synonym_sets', '');

-- Every role that runs a query reads the set its index names, so PUBLIC gets SELECT. Who may
-- write a set is the DBA's decision, through ordinary table grants.
GRANT SELECT ON TABLE paradedb.synonym_sets TO PUBLIC;

CREATE TRIGGER synonym_sets_bump_version
    BEFORE UPDATE ON paradedb.synonym_sets
    FOR EACH ROW EXECUTE FUNCTION paradedb.synonym_sets_bump_version();
"#,
    name = "synonym_sets",
    requires = [synonym_sets_bump_version],
);

/// Stamps every update, so a backend can tell from one small column whether its compiled copy of
/// a set is current.
#[pg_trigger]
fn synonym_sets_bump_version<'a>(
    trigger: &'a PgTrigger<'a>,
) -> Result<Option<PgHeapTuple<'a, impl WhoAllocated>>, PgHeapTupleError> {
    let old = trigger
        .old()
        .expect("a row-level UPDATE trigger has an old tuple");
    let mut new = trigger
        .new()
        .expect("a row-level UPDATE trigger has a new tuple")
        .into_owned();
    let version = old
        .get_by_name::<i64>("version")
        .expect("synonym_sets has a version column")
        .expect("synonym_sets.version is NOT NULL");
    new.set_by_name("version", version + 1)
        .expect("synonym_sets has a version column");
    let now =
        TimestampWithTimeZone::try_from(unsafe { pg_sys::GetCurrentTransactionStartTimestamp() })
            .expect("the transaction start time is a valid timestamp");
    new.set_by_name("updated_at", now)
        .expect("synonym_sets has an updated_at column");
    Ok(Some(new))
}

/// Shows how `input_text` expands under a set before a query runs. `tokenizer_setting` is a
/// `paradedb.tokenizer()` value and should be the analyzer the query would use.
#[pg_extern(stable, parallel_safe)]
pub fn synonym_expansion(
    set_name: &str,
    tokenizer_setting: JsonB,
    input_text: &str,
) -> TableIterator<
    'static,
    (
        name!(position, i32),
        name!(token, String),
        name!(alternatives, Vec<String>),
    ),
> {
    let tokenizer_setting = serde_json::to_value(tokenizer_setting)
        .expect("invalid tokenizer setting, expected paradedb.tokenizer()");
    let tokenizer = SearchTokenizer::from_json_value(&tokenizer_setting)
        .expect("invalid tokenizer setting, expected paradedb.tokenizer()");
    let mut analyzer = tokenizer
        .to_tantivy_tokenizer()
        .expect("failed to convert tokenizer to tantivy tokenizer");
    let compiled =
        compiled_set(set_name, &tokenizer.name(), &mut analyzer).unwrap_or_else(|e| panic!("{e}"));

    let tokens = tokenize(&mut analyzer, input_text);
    let mut position = 0;
    let rows = compiled
        .expand(&tokens)
        .into_iter()
        .map(|expansion| {
            let row = (
                position as i32,
                expansion.tokens.join(" "),
                expansion
                    .alternatives
                    .unwrap_or_else(|| vec![expansion.tokens.clone()])
                    .iter()
                    .map(|alternative| alternative.join(" "))
                    .collect(),
            );
            position += expansion.tokens.len();
            row
        })
        .collect::<Vec<_>>();
    TableIterator::new(rows)
}
