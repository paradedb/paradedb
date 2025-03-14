mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

// this test needs to be alone in its own module as it requires sole access to the database to ensure
// segments merge properly, due to vacuum horizon rules.
#[rstest]
fn segment_merge_scale_factor(mut conn: PgConnection) {
    "CREATE TABLE test_table (id SERIAL PRIMARY KEY, value TEXT NOT NULL) WITH (autovacuum_enabled = off);"
        .execute(&mut conn);
    "CREATE INDEX idxtest_table ON test_table USING bm25(id, value) WITH (key_field = 'id');"
        .execute(&mut conn);

    let parallelism = std::thread::available_parallelism().unwrap().get();

    "SET paradedb.segment_merge_scale_factor = 2;".execute(&mut conn);
    for i in 0..(parallelism * 2) {
        format!("INSERT INTO test_table (value) VALUES ('{i}')").execute(&mut conn);
    }
    let (nsegments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments as usize, parallelism * 2);

    format!(
        "INSERT INTO test_table (value) VALUES ('this should create {parallelism}*2+1 segments')"
    )
    .execute(&mut conn);
    let (nsegments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments as usize, parallelism * 2 + 1);

    // wait out possible concurrent test job connections
    // we need to be the only one that can see the transaction's we're about to make
    // to ensure the index got merged
    {
        const MAX_RETRIES: usize = 30;
        let mut retries = 0;
        while retries != MAX_RETRIES {
            let (none_active,) = "SELECT count(*) = 1 FROM pg_stat_activity WHERE state = 'active'"
                .fetch_one::<(bool,)>(&mut conn);
            if none_active {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
            eprintln!("Waiting for active backends to finish");
            retries += 1;
        }
        if retries == MAX_RETRIES {
            panic!("Active backends did not finish after ~{MAX_RETRIES} seconds");
        }
    }

    format!("INSERT INTO test_table (value) VALUES ('this should cause a merge to {parallelism}')")
        .execute(&mut conn);
    let (nsegments,) =
        "SELECT count(*) FROM paradedb.index_info('idxtest_table')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(nsegments as usize, parallelism + 1);
}
