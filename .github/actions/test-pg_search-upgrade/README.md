# Version Upgrade Testing

New versions of ParadeDB must maintain compatibility with old versions so that users can upgrade the extension smoothly. This action tests the upgrade flow by creating an instance with the old version of the extension, arranging some data, upgrading to the latest version, and verifying that queries can still be run against the upgraded version.

To add a test, add a folder containing files called `setup.sql` and `queries.sql`. `setup.sql` should create the tables, indexes, and data necessary to arrange your test case and will be run on the old version of the DB. `queries.sql` should contain the queries to run against the upgraded DB. The folder name will be used as the name of the DB so that there is isolation between test cases.

After upgrading, we also assert that the upgraded schema is byte-for-byte identical to a fresh `CREATE EXTENSION` of the new version (the "Verify Upgraded Schema Matches Fresh Install" step, using `schema_snapshot.sql`). This catches migrations that are incomplete on the `ALTER EXTENSION ... UPDATE` path -- e.g. when an object's DDL is emitted into an already-shipped migration file, so it reaches fresh installs but never reaches users upgrading from a later version. This is independent of the per-case `queries.sql`, which only catches drift in objects a query happens to touch.

We also run the integration tests after upgrading. This verifies that the extension symbols are upgraded properly but does not give any validation that the on-disk changes are correct because the tests don't use DBs created on the old version.
