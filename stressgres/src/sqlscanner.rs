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

// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use serde::{Deserialize, Serialize};
use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Eq, PartialEq, PartialOrd, Ord, Hash, Debug, Clone, Default, Deserialize, Serialize)]
pub enum StatementDestination {
    #[default]
    DefaultServer,
    SpecificServers(Vec<String>),
    AllServers,
}

/// A single parsed SQL statement plus optional COPY payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScannedStatement<'a> {
    pub sql: &'a str,
    pub payload: Option<&'a str>,
}

/// A scanner for multiple SQL statements in a single string, even if separated by `;`.
pub struct SqlStatementScanner<'a> {
    sql: &'a str,
}

impl<'a> SqlStatementScanner<'a> {
    pub fn new(sql: &'a str) -> Self {
        SqlStatementScanner { sql }
    }
}

impl<'a> IntoIterator for SqlStatementScanner<'a> {
    type Item = ScannedStatement<'a>;
    type IntoIter = SqlStatementScannerIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SqlStatementScannerIterator {
            sql: self.sql,
            start: 0,
        }
    }
}

pub struct SqlStatementScannerIterator<'a> {
    pub sql: &'a str,
    pub start: usize,
}

impl<'a> Iterator for SqlStatementScannerIterator<'a> {
    type Item = ScannedStatement<'a>;

    fn next(&mut self) -> Option<ScannedStatement<'a>> {
        self.scan_statement()
    }
}

impl<'a> SqlStatementScannerIterator<'a> {
    fn scan_statement(&mut self) -> Option<ScannedStatement<'a>> {
        if self.start >= self.sql.len() {
            return None;
        }

        let input = &self.sql[self.start..];
        let mut offset = 0;
        let mut sql: Option<&'a str> = None;

        let mut in_sl_comment = false;
        let mut in_ml_comment = false;
        let mut in_squote = false;
        let mut in_dquote = false;
        let mut current_dollar_quote = None;

        let mut iter = input.char_indices().peekable();
        let mut putback = None;

        fn get_next(
            pb: &mut Option<(usize, char)>,
            it: &mut Peekable<CharIndices>,
        ) -> Option<(usize, char)> {
            if pb.is_some() {
                pb.take()
            } else {
                it.next()
            }
        }

        while let Some((mut idx, c)) = get_next(&mut putback, &mut iter) {
            let mut nextc = match iter.peek() {
                Some((_, nc)) => *nc,
                None => '\0',
            };

            match c {
                // handle $foo$ quoting
                '$' => {
                    if !(in_sl_comment || in_ml_comment) {
                        let begin = idx;
                        let mut end = idx;
                        let mut incomplete = false;
                        loop {
                            match iter.next() {
                                Some((i2, c2)) => {
                                    end = i2;
                                    if c2 == '$' {
                                        break;
                                    } else if !c2.is_alphanumeric() && c2 != '_' {
                                        // not a valid "dollar tag"
                                        putback = Some((i2, c2));
                                        break;
                                    }
                                }
                                None => {
                                    incomplete = true;
                                    break;
                                }
                            }
                        }
                        if putback.is_some() {
                            continue;
                        }
                        if !incomplete {
                            let quote = &input[begin..=end];
                            match current_dollar_quote.as_ref() {
                                Some(q) if quote == *q => {
                                    // end the quote
                                    current_dollar_quote = None;
                                }
                                None => {
                                    // start a new quote
                                    current_dollar_quote = Some(quote);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // toggles
                '"' => {
                    if !(in_sl_comment || in_ml_comment || in_squote) {
                        in_dquote = !in_dquote;
                    }
                }
                '\'' => {
                    if !(in_sl_comment || in_ml_comment || in_dquote) {
                        in_squote = !in_squote;
                    }
                }

                // slash slash or dash dash for single line comments
                '/' if nextc == '/' => {
                    if !in_ml_comment {
                        in_sl_comment = true;
                    }
                }
                '-' if nextc == '-' => {
                    if !in_ml_comment {
                        in_sl_comment = true;
                    }
                }
                '\r' | '\n' => {
                    if in_sl_comment && !(in_squote || in_dquote || current_dollar_quote.is_some())
                    {
                        offset = idx;
                    }
                    in_sl_comment = false;
                }

                // slash star to start or star slash to end multi line
                '/' if nextc == '*' => {
                    if !in_sl_comment {
                        in_ml_comment = true;
                    }
                }
                '*' if nextc == '/' => {
                    in_ml_comment = false;
                }

                // skip escapes
                '\\' => {
                    if !(in_sl_comment || in_ml_comment) {
                        // skip next char
                        iter.next();
                    }
                }

                // semicolon ends the statement if not inside quotes/comments
                ';' => {
                    if !(in_sl_comment
                        || in_ml_comment
                        || in_squote
                        || in_dquote
                        || current_dollar_quote.is_some())
                    {
                        // consume trailing whitespace
                        while nextc.is_whitespace() && iter.next().is_some() {
                            nextc = match iter.peek() {
                                Some((_, cc)) => *cc,
                                None => '\0',
                            };
                            idx += 1;
                        }
                        sql = Some(&input[offset..=idx]);
                        self.start += idx + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        // If none found but input not empty, that's the last statement
        if sql.is_none() {
            let trimmed = input.trim_end();
            if !trimmed.is_empty() {
                sql = Some(input);
                self.start += input.len();
            }
        }

        let statement_sql = sql.unwrap_or("").trim();
        if statement_sql.is_empty() {
            return None;
        }

        // If it's COPY, handle possible payload
        let payload = if statement_sql.to_ascii_lowercase().starts_with("copy") {
            self.scan_copy_data()
        } else {
            None
        };

        Some(ScannedStatement {
            sql: statement_sql,
            payload,
        })
    }

    /// If `COPY ... FROM stdin;` is present, we parse all data until "\." line
    fn scan_copy_data(&mut self) -> Option<&'a str> {
        let input = &self.sql[self.start..];
        let mut prevc = '\n';
        let mut iter = input.char_indices().peekable();
        while let Some((i, c)) = iter.next() {
            let nextc = match iter.peek() {
                Some((_, cc)) => *cc,
                None => '\0',
            };
            match c {
                '\\' if nextc == '.' && prevc == '\n' => {
                    self.start += i + 2;
                    return Some(&input[..=i + 1]);
                }
                _ => {
                    prevc = c;
                }
            }
        }
        None
    }
}

#[test]
fn test_scan_statement_with_sl_comment() {
    let input = r#"
DROP EXTENSION IF EXISTS pg_search CASCADE;
DROP TABLE IF EXISTS test CASCADE;
CREATE EXTENSION pg_search;
CREATE TABLE test (
    id SERIAL8 NOT NULL PRIMARY KEY,
    message TEXT,
    old_message TEXT
) WITH (autovacuum_enabled = false);

INSERT INTO test (message) VALUES ('beer wine cheese a');
INSERT INTO test (message) VALUES ('beer wine a');
INSERT INTO test (message) VALUES ('beer cheese a');
INSERT INTO test (message) VALUES ('beer a');
INSERT INTO test (message) VALUES ('wine cheese a');
INSERT INTO test (message) VALUES ('wine a');
INSERT INTO test (message) VALUES ('cheese a');
INSERT INTO test (message) VALUES ('beer wine cheese a');
INSERT INTO test (message) VALUES ('beer wine a');
INSERT INTO test (message) VALUES ('beer cheese a');
INSERT INTO test (message) VALUES ('beer a');
INSERT INTO test (message) VALUES ('wine cheese a');
INSERT INTO test (message) VALUES ('wine a');
INSERT INTO test (message) VALUES ('cheese a');

-- INSERT INTO test (message) SELECT 'space fillter ' || x FROM generate_series(1, 10000000) x;

CREATE INDEX idxtest ON test USING bm25(id, message) WITH (key_field = 'id');
CREATE OR REPLACE FUNCTION assert(a bigint, b bigint) RETURNS bool STABLE STRICT LANGUAGE plpgsql AS $$
DECLARE
    current_txid bigint;
BEGIN
    -- Get the current transaction ID
    current_txid := txid_current();

    -- Check if the values are not equal
    IF a <> b THEN
        RAISE EXCEPTION 'Assertion failed: % <> %. Transaction ID: %', a, b, current_txid;
    END IF;

    RETURN true;
END;
$$;
    "#;

    let scanner = SqlStatementScanner::new(input);
    for sql in scanner.into_iter() {
        eprintln!("{}", sql.sql);
    }
}
