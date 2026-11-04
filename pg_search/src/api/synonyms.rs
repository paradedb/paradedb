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
mod pdb {
    use crate::query::synonyms::{self, compiled_set_without_store, tokenize};
    use pgrx::heap_tuple::{PgHeapTuple, PgHeapTupleError};
    use pgrx::pgbox::AllocatedByPostgres;
    use pgrx::trigger_support::PgTrigger;
    use pgrx::{
        JsonB, extension_sql, iter::TableIterator, name, pg_extern, pg_trigger, search_path,
    };
    use tokenizers::SearchTokenizer;

    extension_sql!(
        r#"
CREATE DOMAIN pdb.synonym_rule AS text
    CHECK (pdb.synonym_rule_is_valid(VALUE));

CREATE TABLE pdb.synonym_sets (
    name       text NOT NULL PRIMARY KEY CHECK (name <> 'none'),
    version    bigint NOT NULL DEFAULT 1,
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.now()
);
CREATE TABLE pdb.synonym_rules (
    set_name   text NOT NULL REFERENCES pdb.synonym_sets (name) ON DELETE CASCADE,
    rule       pdb.synonym_rule NOT NULL
);
CREATE INDEX synonym_rules_set_name_idx ON pdb.synonym_rules (set_name);
-- Derived rows, rebuilt on demand, so they are not dumped. The key is a digest of the analyzer
-- config, since a config with a long stopword list is too big for a btree entry.
CREATE TABLE pdb.synonym_fsts (
    set_name      text NOT NULL REFERENCES pdb.synonym_sets (name) ON DELETE CASCADE,
    analyzer_hash text NOT NULL,
    analyzer      jsonb NOT NULL,
    version       bigint NOT NULL,
    built_with    text NOT NULL,
    fst           bytea NOT NULL,
    alternatives  bytea NOT NULL,
    PRIMARY KEY (set_name, analyzer_hash)
);
SELECT pg_catalog.pg_extension_config_dump('pdb.synonym_sets', '');
SELECT pg_catalog.pg_extension_config_dump('pdb.synonym_rules', '');

-- Every role that runs a query reads the set its index names, so PUBLIC gets SELECT. Writing
-- rules takes a grant on pdb.synonym_rules, removing a set a grant on pdb.synonym_sets, and
-- nothing else: both trigger functions and the builder run as the extension owner.
GRANT SELECT ON pdb.synonym_sets, pdb.synonym_rules, pdb.synonym_fsts TO PUBLIC;

-- A set appears with its first rule, so a writer never touches pdb.synonym_sets. The IF NOT
-- EXISTS check keeps a COPY of many rules from paying a speculative insert per row.
CREATE FUNCTION pdb.ensure_synonym_set() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pdb.synonym_sets WHERE name = NEW.set_name) THEN
        INSERT INTO pdb.synonym_sets (name) VALUES (NEW.set_name) ON CONFLICT (name) DO NOTHING;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER synonym_rules_ensure_set
    BEFORE INSERT OR UPDATE OF set_name ON pdb.synonym_rules
    FOR EACH ROW EXECUTE FUNCTION pdb.ensure_synonym_set();

ALTER FUNCTION pdb.synonym_rules_bump_version() SECURITY DEFINER
    SET search_path = pg_catalog, pg_temp;
-- One trigger per event, because a transition table binds to a single event.
CREATE TRIGGER synonym_rules_bump_version_insert
    AFTER INSERT ON pdb.synonym_rules
    REFERENCING NEW TABLE AS new_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();
CREATE TRIGGER synonym_rules_bump_version_update
    AFTER UPDATE ON pdb.synonym_rules
    REFERENCING OLD TABLE AS old_rules NEW TABLE AS new_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();
CREATE TRIGGER synonym_rules_bump_version_delete
    AFTER DELETE ON pdb.synonym_rules
    REFERENCING OLD TABLE AS old_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();
CREATE TRIGGER synonym_rules_bump_version_truncate
    AFTER TRUNCATE ON pdb.synonym_rules
    FOR EACH STATEMENT EXECUTE FUNCTION pdb.synonym_rules_bump_version();

-- Postgres checks EXECUTE on a trigger function at CREATE TRIGGER only, so the triggers above
-- keep firing, and no other role can hang these owner-level functions on a table of its own.
REVOKE EXECUTE ON FUNCTION pdb.ensure_synonym_set(), pdb.synonym_rules_bump_version()
    FROM PUBLIC;
"#,
        name = "synonym_tables",
        requires = [synonym_rule_is_valid, synonym_rules_bump_version]
    );

    /// The check behind `pdb.synonym_rule`. It raises instead of returning false, so the error
    /// names the rule and what is wrong with it, which a domain violation would not.
    #[pg_extern(immutable, parallel_safe, strict)]
    fn synonym_rule_is_valid(rule: &str) -> bool {
        if let Some(reason) = synonyms::rule_error(rule) {
            pgrx::error!("invalid synonym rule '{rule}': {reason}");
        }
        true
    }

    /// Once per statement on `pdb.synonym_rules`: bumps the version of every set the statement
    /// touched, rewrites their stored FSTs and drops every backend's copy.
    #[pg_trigger]
    fn synonym_rules_bump_version<'a>(
        trigger: &'a PgTrigger<'a>,
    ) -> Result<Option<PgHeapTuple<'a, AllocatedByPostgres>>, PgHeapTupleError> {
        let sets = synonyms::changed_sets(trigger).unwrap_or_else(|e| panic!("{e}"));
        for set in &sets {
            synonyms::rebuild_set(set).unwrap_or_else(|e| panic!("{e}"));
        }
        Ok(None)
    }

    /// Compiles `set_name` for `analyzer`, the stored form of a tokenizer config, and stores the
    /// row. Runs as the extension owner, so a reader needs no write grant on the cache table and
    /// cannot store bytes it did not compile. Returns false when the set has its share of rows.
    #[pg_extern(security_definer, volatile, parallel_unsafe, strict)]
    #[search_path(pg_catalog, pg_temp)]
    fn synonym_fst_build(set_name: &str, analyzer: JsonB) -> bool {
        synonyms::build_and_store(set_name, analyzer.0).unwrap_or_else(|e| panic!("{e}"))
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
        let compiled = compiled_set_without_store(set_name, Some(&tokenizer), &mut analyzer)
            .unwrap_or_else(|e| panic!("{e}"));

        let tokens = tokenize(&mut analyzer, input_text);
        let max_alternatives = crate::gucs::synonyms_max_alternatives();
        let mut position = 0;
        let rows = compiled
            .expand(&tokens)
            .into_iter()
            .map(|mut expansion| {
                expansion.cap(max_alternatives);
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
}
