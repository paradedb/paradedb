mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;

#[rstest]
async fn main(#[future(awt)] s3: S3) -> Result<()> {
    s3.client
        .create_bucket()
        .bucket("example-bucket")
        .send()
        .await?;

    let list_buckets_output = s3.client.list_buckets().send().await?;
    assert!(list_buckets_output.buckets.is_some());
    let buckets_list = list_buckets_output.buckets.unwrap();
    assert_eq!(1, buckets_list.len());
    assert_eq!("example-bucket", buckets_list[0].name.as_ref().unwrap());

    Ok(())
}
