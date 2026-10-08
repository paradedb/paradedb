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

use std::collections::HashSet;

fn statements(sql: &str) -> Result<HashSet<String>, String> {
    Ok(crate::sql::parse(sql)?
        .into_iter()
        .map(|statement| statement.comparison)
        .collect())
}

pub fn check(expected: &str, fragments: &[String]) -> Result<(), String> {
    let expected = statements(expected)?;
    let mut actual = HashSet::new();
    // Parse each file separately: concatenation could hide its first statement
    // behind a trailing line comment in the previous fragment.
    for fragment in fragments {
        actual.extend(statements(fragment)?);
    }
    let missing = expected.difference(&actual).count();
    if missing != 0 {
        return Err(format!(
            "{missing} required schema statement(s) missing from migration fragments; see the suggested diff above"
        ));
    }
    println!(
        "All {} required schema statements found in migration fragments",
        expected.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_statements_in_any_order() {
        check(
            "CREATE TABLE t(x int); GRANT SELECT ON t TO reader",
            &["GRANT SELECT ON t TO reader; CREATE TABLE t(x int)".into()],
        )
        .unwrap();
    }

    #[test]
    fn checks_grants_and_revokes_and_accepts_extra_statements() {
        assert!(
            check(
                "REVOKE ALL ON t FROM PUBLIC",
                &["CREATE TABLE t(x int)".into()]
            )
            .is_err()
        );
        check(
            "GRANT SELECT ON t TO reader",
            &["GRANT SELECT ON t TO reader; SELECT 1;".into()],
        )
        .unwrap();
    }

    #[test]
    fn checks_fragments_individually() {
        check(
            "CREATE TABLE t(x int); CREATE TABLE u(x int)",
            &[
                "CREATE TABLE t(x int); -- end".into(),
                "CREATE TABLE u(x int);".into(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn rejects_invalid_sql_and_different_definitions() {
        assert!(check("CREATE TABLE t(x int)", &["CREATE TABLE (".into()]).is_err());
        assert!(check("CREATE TABLE t(x int)", &["CREATE TABLE t(x text)".into()]).is_err());
    }
}
