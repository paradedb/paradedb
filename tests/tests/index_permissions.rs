// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

fn direct_reads(index: &str) -> Vec<String> {
    let mut queries = vec![
        format!("SELECT * FROM paradedb.index_info({index})"),
        format!("SELECT * FROM paradedb.index_info({index}, true)"),
        format!("SELECT * FROM pdb.index_segments({index})"),
    ];
    for visibility in ["transaction", "raw", "threshold"] {
        queries.push(format!(
            r#"SELECT paradedb.aggregate(
                {index}, paradedb.all(),
                '{{"emails":{{"terms":{{"field":"email","size":100}}}}}}',
                visibility => '{visibility}')"#
        ));
    }
    queries
}

fn assert_denied(conn: &mut PgConnection, queries: &[String]) {
    for query in queries {
        let err = query.execute_result(conn).expect_err(query);
        assert_eq!(
            err.as_database_error()
                .and_then(|err| err.code())
                .as_deref(),
            Some("42501"),
            "{query}: {err}"
        );
    }
}

#[rstest]
fn standalone_index_reads_enforce_permissions(mut conn: PgConnection) {
    // Roles are cluster-wide; use a unique name so parallel tests cannot share
    // grants and accidentally turn a negative assertion into an authorized read.
    let role = format!("index_reader_{}", uuid::Uuid::new_v4().simple());
    format!(
        r#"
        CREATE ROLE {role} NOSUPERUSER NOBYPASSRLS NOINHERIT;
        CREATE SCHEMA index_permissions;
        GRANT USAGE ON SCHEMA index_permissions TO {role};
        CREATE TABLE index_permissions.secrets (id integer PRIMARY KEY, email text);
        INSERT INTO index_permissions.secrets VALUES (1, 'private@example.com');
        CREATE INDEX secrets_idx ON index_permissions.secrets
            USING paradedb (id, (email::pdb.literal));
        "#
    )
    .execute(&mut conn);
    let (oid,) = "SELECT 'index_permissions.secrets_idx'::regclass::oid::bigint"
        .fetch_one::<(i64,)>(&mut conn);
    let by_name = direct_reads("'index_permissions.secrets_idx'");
    let by_oid = direct_reads(&format!("{oid}::regclass"));

    format!("SET ROLE {role}").execute(&mut conn);
    assert_denied(
        &mut conn,
        &["SELECT email FROM index_permissions.secrets".into()],
    );
    assert_denied(&mut conn, &by_name);
    assert_denied(&mut conn, &by_oid);
    "RESET ROLE".execute(&mut conn);

    // A grant on an unrelated column must not authorize arbitrary index fields.
    format!("GRANT SELECT (id) ON index_permissions.secrets TO {role}").execute(&mut conn);
    format!("SET ROLE {role}").execute(&mut conn);
    "SELECT id FROM index_permissions.secrets".execute(&mut conn);
    assert_denied(&mut conn, &by_name);
    assert_denied(&mut conn, &by_oid);
    "RESET ROLE".execute(&mut conn);

    format!("GRANT SELECT ON index_permissions.secrets TO {role}").execute(&mut conn);
    format!("SET ROLE {role}").execute(&mut conn);
    for query in by_name.iter().chain(&by_oid) {
        query.execute(&mut conn);
    }
    let (result,) = format!(
        r#"SELECT paradedb.aggregate({oid}::regclass, paradedb.all(),
            '{{"emails":{{"terms":{{"field":"email","size":100}}}}}}')"#
    )
    .fetch_one::<(serde_json::Value,)>(&mut conn);
    assert_eq!(result["emails"]["buckets"][0]["key"], "private@example.com");
    "RESET ROLE".execute(&mut conn);

    // Numeric OIDs bypass name resolution, but must not bypass schema USAGE.
    format!("REVOKE USAGE ON SCHEMA index_permissions FROM {role}").execute(&mut conn);
    format!("SET ROLE {role}").execute(&mut conn);
    assert_denied(&mut conn, &by_oid);
    "RESET ROLE".execute(&mut conn);
    format!("GRANT USAGE ON SCHEMA index_permissions TO {role}").execute(&mut conn);

    // Table SELECT alone is insufficient when RLS applies. With no policies,
    // ordinary SELECT returns no rows, and direct index reads must fail closed.
    "ALTER TABLE index_permissions.secrets ENABLE ROW LEVEL SECURITY".execute(&mut conn);
    format!("SET ROLE {role}").execute(&mut conn);
    let (count,) = "SELECT count(*) FROM index_permissions.secrets".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 0);
    assert_denied(&mut conn, &by_name);
    assert_denied(&mut conn, &by_oid);
    "SET row_security = off".execute(&mut conn);
    assert_denied(&mut conn, &by_oid);
    "RESET row_security; RESET ROLE".execute(&mut conn);

    // PostgreSQL's BYPASSRLS and owner semantics still apply.
    format!("ALTER ROLE {role} BYPASSRLS; SET ROLE {role}").execute(&mut conn);
    for query in &by_oid {
        query.execute(&mut conn);
    }
    "RESET ROLE".execute(&mut conn);
    format!(
        "ALTER ROLE {role} NOBYPASSRLS;
         ALTER TABLE index_permissions.secrets OWNER TO {role}; SET ROLE {role}"
    )
    .execute(&mut conn);
    for query in &by_oid {
        query.execute(&mut conn);
    }
    "ALTER TABLE index_permissions.secrets FORCE ROW LEVEL SECURITY".execute(&mut conn);
    assert_denied(&mut conn, &by_oid);
    "RESET ROLE".execute(&mut conn);
    format!("DROP SCHEMA index_permissions CASCADE; DROP ROLE {role}").execute(&mut conn);
}

#[rstest]
fn partitioned_index_metadata_checks_the_requested_table(mut conn: PgConnection) {
    let role = format!("partition_reader_{}", uuid::Uuid::new_v4().simple());
    format!(
        r#"
        CREATE ROLE {role} NOSUPERUSER NOBYPASSRLS;
        CREATE SCHEMA partition_permissions;
        GRANT USAGE ON SCHEMA partition_permissions TO {role};
        CREATE TABLE partition_permissions.parent (id integer) PARTITION BY RANGE (id);
        CREATE TABLE partition_permissions.child PARTITION OF partition_permissions.parent
            FOR VALUES FROM (0) TO (10);
        INSERT INTO partition_permissions.parent VALUES (1);
        CREATE INDEX parent_idx ON partition_permissions.parent USING paradedb (id);
        "#
    )
    .execute(&mut conn);
    let (parent_oid,) = "SELECT 'partition_permissions.parent_idx'::regclass::oid::bigint"
        .fetch_one::<(i64,)>(&mut conn);
    let (child_oid,) = "SELECT indexrelid::bigint FROM pg_index
        WHERE indrelid = 'partition_permissions.child'::regclass"
        .fetch_one::<(i64,)>(&mut conn);
    let metadata = |oid| {
        vec![
            format!("SELECT * FROM paradedb.index_info({oid}::regclass)"),
            format!("SELECT * FROM pdb.index_segments({oid}::regclass)"),
        ]
    };
    format!("SET ROLE {role}").execute(&mut conn);
    assert_denied(&mut conn, &metadata(parent_oid));
    assert_denied(&mut conn, &metadata(child_oid));
    "RESET ROLE".execute(&mut conn);
    format!("GRANT SELECT ON partition_permissions.parent TO {role}; SET ROLE {role}")
        .execute(&mut conn);
    // Access via the parent follows the parent's grants, as a SQL scan does.
    for query in metadata(parent_oid) {
        query.execute(&mut conn);
    }
    // Naming the child directly requires its own privileges.
    assert_denied(&mut conn, &metadata(child_oid));
    "RESET ROLE".execute(&mut conn);
    "ALTER TABLE partition_permissions.parent ENABLE ROW LEVEL SECURITY".execute(&mut conn);
    format!("SET ROLE {role}").execute(&mut conn);
    assert_denied(&mut conn, &metadata(parent_oid));
    "RESET ROLE".execute(&mut conn);
    format!("DROP SCHEMA partition_permissions CASCADE; DROP ROLE {role}").execute(&mut conn);
}

#[rstest]
fn standalone_index_reads_honor_definer_and_invoker(mut conn: PgConnection) {
    let owner = format!("index_owner_{}", uuid::Uuid::new_v4().simple());
    let caller = format!("index_caller_{}", uuid::Uuid::new_v4().simple());
    // Transactional role creation also cleans up if an assertion fails.
    format!(
        r#"
        BEGIN;
        CREATE ROLE {owner} NOSUPERUSER NOBYPASSRLS NOINHERIT;
        CREATE ROLE {caller} NOSUPERUSER NOBYPASSRLS NOINHERIT;
        CREATE SCHEMA index_identity AUTHORIZATION {owner};
        GRANT USAGE ON SCHEMA index_identity TO {caller};
        CREATE TABLE index_identity.secrets(id integer PRIMARY KEY, email text);
        INSERT INTO index_identity.secrets VALUES (1, 'first'), (2, 'second');
        CREATE INDEX secrets_idx ON index_identity.secrets
            USING paradedb (id, (email::pdb.literal));
        ALTER TABLE index_identity.secrets OWNER TO {owner};
        ALTER TABLE index_identity.secrets ENABLE ROW LEVEL SECURITY;
        CREATE FUNCTION index_identity.definer_segments() RETURNS bigint
        LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog, pg_temp AS $$
          SELECT coalesce(sum(num_docs), 0)::bigint
          FROM pdb.index_segments('index_identity.secrets_idx'::regclass)
        $$;
        CREATE FUNCTION index_identity.invoker_segments() RETURNS bigint
        LANGUAGE sql SECURITY INVOKER SET search_path = pg_catalog, pg_temp AS $$
          SELECT coalesce(sum(num_docs), 0)::bigint
          FROM pdb.index_segments('index_identity.secrets_idx'::regclass)
        $$;
        ALTER FUNCTION index_identity.definer_segments() OWNER TO {owner};
        ALTER FUNCTION index_identity.invoker_segments() OWNER TO {owner};
        REVOKE EXECUTE ON FUNCTION index_identity.definer_segments(),
            index_identity.invoker_segments() FROM PUBLIC;
        GRANT EXECUTE ON FUNCTION index_identity.definer_segments(),
            index_identity.invoker_segments() TO {caller};
        "#
    )
    .execute(&mut conn);
    let ownership = "SELECT count(*), bool_and(p.proowner = c.relowner)
        FROM pg_proc p JOIN pg_class c ON c.oid = 'index_identity.secrets'::regclass
        WHERE p.pronamespace = 'index_identity'::regnamespace"
        .fetch_one::<(i64, bool)>(&mut conn);
    assert_eq!(ownership, (2, true));
    let (privileged_owner,) =
        format!("SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = '{owner}'")
            .fetch_one::<(bool,)>(&mut conn);
    assert!(!privileged_owner);

    format!("SET ROLE {caller}").execute(&mut conn);
    let privileges = "SELECT
        has_table_privilege(current_user, 'index_identity.secrets', 'SELECT'),
        pg_has_role(current_user, 'pg_stat_scan_tables', 'USAGE')"
        .fetch_one::<(bool, bool)>(&mut conn);
    assert_eq!(privileges, (false, false));
    let (docs,) = "SELECT index_identity.definer_segments()".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(docs, 2);

    // Recover each expected error without aborting the fixture transaction.
    for query in [
        "SELECT index_identity.invoker_segments()",
        "SELECT * FROM pdb.index_segments('index_identity.secrets_idx')",
        "SELECT email FROM index_identity.secrets",
    ] {
        "SAVEPOINT denied_call".execute(&mut conn);
        assert_denied(&mut conn, &[query.into()]);
        "ROLLBACK TO SAVEPOINT denied_call".execute(&mut conn);
    }
    "RESET ROLE; ALTER TABLE index_identity.secrets FORCE ROW LEVEL SECURITY".execute(&mut conn);
    format!("SET ROLE {caller}").execute(&mut conn);
    "SAVEPOINT forced_rls".execute(&mut conn);
    let err = "SELECT index_identity.definer_segments()"
        .execute_result(&mut conn)
        .expect_err("a non-superuser definer must honor FORCE RLS");
    let db_err = err.as_database_error().expect("PostgreSQL error");
    assert_eq!(db_err.code().as_deref(), Some("42501"));
    assert_eq!(
        db_err.message(),
        "direct index reads are not supported when row-level security applies to table index_identity.secrets"
    );
    "ROLLBACK TO SAVEPOINT forced_rls; RESET ROLE; ROLLBACK".execute(&mut conn);
}
