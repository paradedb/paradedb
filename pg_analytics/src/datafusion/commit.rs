use shared::postgres::transaction::Transaction;

use crate::datafusion::session::Session;
use crate::datafusion::writer::Writer;
use crate::errors::ParadeError;

pub static TRANSACTION_CALLBACK_CACHE_ID: &str = "parade_parquet_table";

pub fn needs_commit() -> Result<bool, ParadeError> {
    Ok(Transaction::needs_commit(TRANSACTION_CALLBACK_CACHE_ID)?)
}

pub async fn commit_writer() -> Result<(), ParadeError> {
    if let Some((schema_name, table_path, mut delta_table)) = Writer::commit().await? {
        delta_table.update().await?;

        Session::with_tables(&schema_name, |mut tables| {
            Box::pin(async move { tables.register(&table_path, delta_table) })
        })?;
    }

    Ok(())
}
