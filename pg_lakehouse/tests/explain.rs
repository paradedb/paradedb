mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

const S3_BUCKET: &str = "test-trip-setup";
const S3_KEY: &str = "test_trip_setup.parquet";

#[rstest]
async fn test_explain_fdw(#[future(awt)] s3: S3, mut conn: PgConnection) -> Result<()> {
    NycTripsTable::setup().execute(&mut conn);
    let rows: Vec<NycTripsTable> = "SELECT * FROM nyc_trips".fetch(&mut conn);
    s3.client.create_bucket().bucket(S3_BUCKET).send().await?;
    s3.create_bucket(S3_BUCKET).await?;
    s3.put_rows(S3_BUCKET, S3_KEY, &rows).await?;

    NycTripsTable::setup_s3_listing_fdw(&s3.url.clone(), &format!("s3://{S3_BUCKET}/{S3_KEY}"))
        .execute(&mut conn);

    let explain: Vec<(String,)> =
        "EXPLAIN SELECT COUNT(*) FROM trips WHERE tip_amount <> 0".fetch(&mut conn);

    assert!(explain[0].0.contains("DataFusionScan: LogicalPlan"));
    assert!(explain[1].0.contains("Projection: COUNT(*)"));
    assert!(explain[2]
        .0
        .contains("Aggregate: groupBy=[[]], aggr=[[COUNT(*)]]"));
    assert!(explain[3]
        .0
        .contains("Filter: trips.tip_amount != Int64(0)"));
    assert!(explain[4].0.contains("TableScan: trips"));

    Ok(())
}

#[rstest]
async fn test_explain_heap(mut conn: PgConnection) -> Result<()> {
    NycTripsTable::setup().execute(&mut conn);

    let explain: Vec<(String,)> =
        "EXPLAIN SELECT COUNT(*) FROM nyc_trips WHERE tip_amount <> 0".fetch(&mut conn);

    assert!(explain[0].0.contains("Aggregate"));
    assert!(explain[1].0.contains("Seq Scan on nyc_trips"));
    assert!(explain[2].0.contains("Filter"));

    Ok(())
}

#[rstest]
async fn test_explain_federated(#[future(awt)] s3: S3, mut conn: PgConnection) -> Result<()> {
    NycTripsTable::setup().execute(&mut conn);
    let rows: Vec<NycTripsTable> = "SELECT * FROM nyc_trips".fetch(&mut conn);
    s3.client.create_bucket().bucket(S3_BUCKET).send().await?;
    s3.create_bucket(S3_BUCKET).await?;
    s3.put_rows(S3_BUCKET, S3_KEY, &rows).await?;

    NycTripsTable::setup_s3_listing_fdw(&s3.url.clone(), &format!("s3://{S3_BUCKET}/{S3_KEY}"))
        .execute(&mut conn);

    let explain: Vec<(String,)> =
        "EXPLAIN SELECT COUNT(*) FROM trips LEFT JOIN nyc_trips ON trips.\"VendorID\" = nyc_trips.\"VendorID\"".fetch(&mut conn);

    assert!(explain[0].0.contains("Aggregate"));
    assert!(explain[1].0.contains("Hash Right Join"));
    assert!(explain[2]
        .0
        .contains("Hash Cond: (nyc_trips.\"VendorID\" = trips.\"VendorID\")"));
    assert!(explain[3].0.contains("Seq Scan on nyc_trips"));
    assert!(explain[4].0.contains("Hash"));
    assert!(explain[5].0.contains("Foreign Scan on trips"));

    Ok(())
}
