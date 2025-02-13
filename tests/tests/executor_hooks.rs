// Copyright (c) 2023-2025 Retake, Inc.
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

mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn multiple_index_changes_in_same_xact(mut conn: PgConnection) {
    r#"
        CREATE TABLE a (id int, value text);
        CREATE TABLE b (id int, value text);
        CREATE TABLE c (id int, value text);
        CREATE INDEX idxa ON a USING bm25(id, value) WITH (key_field='id');
        CREATE INDEX idxb ON b USING bm25(id, value) WITH (key_field='id');
        CREATE INDEX idxc ON c USING bm25(id, value) WITH (key_field='id');
        INSERT INTO a (id, value) VALUES (1, 'a');
        INSERT INTO b (id, value) VALUES (1, 'b');
        INSERT INTO c (id, value) VALUES (1, 'c');
    "#
    .execute(&mut conn);

    let results = r#"
        SELECT * FROM a WHERE value @@@ 'a'
           UNION
        SELECT * FROM b WHERE value @@@ 'b'
           UNION
        SELECT * FROM c WHERE value @@@ 'c'
        ORDER BY 1, 2;
    "#
    .fetch::<(i32, String)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (1, "a".to_string()),
            (1, "b".to_string()),
            (1, "c".to_string()),
        ]
    )
}

#[rstest]
fn issue2187_executor_hooks(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test_table;
        CREATE TABLE test_table
        (
            id           UUID NOT NULL DEFAULT gen_random_uuid(),
            email        TEXT NOT NULL DEFAULT (
                'user' || floor(random() * 150)::TEXT || '@example.com'
                ),
            is_processed BOOLEAN       DEFAULT FALSE,
            PRIMARY KEY (id, email)
        ) PARTITION BY HASH (email);


        DO
        $$
            DECLARE
                i INT;
            BEGIN
                FOR i IN 0..15
                    LOOP
                        EXECUTE format(
                                'CREATE TABLE test_table_p%s PARTITION OF test_table
                                 FOR VALUES WITH (MODULUS 16, REMAINDER %s);', i, i
                                );
                    END LOOP;
            END
        $$;


        INSERT INTO test_table (is_processed)
        SELECT FALSE
        FROM generate_series(1, 100000);


        DO
        $$
            DECLARE
                i          INT;
                table_name TEXT;
                index_name TEXT;
            BEGIN
                FOR i IN 0..15
                    LOOP
                        table_name := format('test_table_p%s', i);
                        index_name := format('test_table_search_p%s', i);

                        EXECUTE format(
                                'CREATE INDEX %I ON %I
                                 USING bm25 (id, is_processed)
                                 WITH (
                                     key_field = ''id'',
                                     boolean_fields = ''{
                                         "is_processed": {
                                             "fast": true,
                                             "indexed": true
                                         }
                                     }''
                                 )', index_name, table_name);
                    END LOOP;
            END
        $$;


        DO
        $$
            DECLARE
                batch_size INT := 1000;
                uuid_batch UUID[];
            BEGIN
                LOOP
                    SELECT ARRAY_AGG(id)
                    INTO uuid_batch
                    FROM (SELECT id
                          FROM test_table
                          WHERE is_processed = FALSE
                          LIMIT batch_size) sub;

                    IF uuid_batch IS NULL OR array_length(uuid_batch, 1) = 0 THEN
                        EXIT;
                    END IF;

                    UPDATE test_table
                    SET is_processed = TRUE
                    WHERE id = ANY (uuid_batch);
                END LOOP;
            END
        $$;
    "#
    .execute(&mut conn);
}
