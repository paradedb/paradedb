# Documentation snippet verification

`extract_code_snippets.py` requires SQL and all five ORM variants in application
CodeGroups. Duplicate and unrecognized language fences fail rather than being
silently overwritten or dropped. Deployment-only CodeGroups are inventoried
separately. Historical release notes are excluded from current-client validation.

Every executable fence outside application CodeGroups is listed in
`docs_snippet_coverage.json` with a reviewed mode, reason and SHA-256 digest:

- `sql`: execute with fresh demo fixtures, including any listed setup fences.
- `setup`: execute as part of `test_docs_getting_started.py` in its own database.
- `skip`: explicitly excluded operational example, fragment, alternative host
  configuration, or example needing an unavailable fixture.

New fences and edits to existing inventoried fences fail until their entries are
reviewed. Use the extractor's `inventory()` function to get fence identities and
digests; do not mark a runnable example skipped just to make CI pass. Execute new
SQL examples locally or in CI before committing their coverage decision.

Getting-started scenarios use the actual documented commands, model files,
configuration and migration inserts, then create the first index and run the first
query. Interactive REPL commands run headlessly. Connection placeholders and
settings ellipses are replaced with the isolated test environment. The scenarios
assert populated data, a known demo row, a valid created index, and the expected
first query rows. Setup entries fail if their scenario does not consume them.

Drizzle builder snippets are attached to the documented table, passed through
Drizzle Kit's migration generator, and executed against Postgres. EF Core model
snippets produce migration operations and execute generated index SQL. Both verify
that an index was generated and exists in the database. Query examples continue
to execute against shared demo fixtures; they do not all have result assertions.

Run coverage and its regressions before the full smoke test:

```bash
python3 .github/scripts/extract_code_snippets.py
python3 -m unittest discover -s .github/scripts/tests -p test_doc_snippets.py
.github/scripts/smoke_test_code_snippets.sh
```

The smoke test accepts exact target names, such as `django sqlalchemy`. Empty
target extraction fails. Run only the setup sequence with:

```bash
python3 .github/scripts/test_docs_getting_started.py django sqlalchemy
```
