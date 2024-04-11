use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use crate::tableam::{deltalake_tableam_oid, TableAMError};

pub trait CreateClassifier {
    #[allow(clippy::wrong_self_convention)]
    unsafe fn is_parquet(self) -> Result<bool, CreateHookError>;
}

impl CreateClassifier for *mut pg_sys::CreateStmt {
    unsafe fn is_parquet(self) -> Result<bool, CreateHookError> {
        let stmt_am = (*self).accessMethod;
        Ok(!stmt_am.is_null() && pg_sys::get_am_oid(stmt_am, false) == deltalake_tableam_oid()?)
    }
}

pub trait PartitionDescriptor {
    unsafe fn has_partition_strategy(self) -> bool;
    unsafe fn partition_strategy_is_list(self) -> bool;
    unsafe fn partition_list_columns(self) -> Result<Vec<String>, CreateHookError>;
    unsafe fn partition_columns_ordered_at_end(self) -> Result<bool, CreateHookError>;
}

impl PartitionDescriptor for *mut pg_sys::CreateStmt {
    unsafe fn has_partition_strategy(self) -> bool {
        let partspec = (*self).partspec;
        !partspec.is_null()
    }

    unsafe fn partition_strategy_is_list(self) -> bool {
        if !self.has_partition_strategy() {
            return false;
        }

        let partspec = (*self).partspec;

        #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
        {
            let list_strategy = pg_sys::PARTITION_STRATEGY_LIST;
            *(*partspec).strategy == list_strategy as i8
        }
        #[cfg(feature = "pg16")]
        {
            let list_strategy = pg_sys::PartitionStrategy_PARTITION_STRATEGY_LIST;
            (*partspec).strategy == list_strategy
        }
    }

    unsafe fn partition_list_columns(self) -> Result<Vec<String>, CreateHookError> {
        if !self.partition_strategy_is_list() {
            return Err(CreateHookError::Partition);
        }

        let mut partitions = vec![];

        let partspec = (*self).partspec;
        let part_params = PgList::<pg_sys::PartitionElem>::from_pg((*partspec).partParams);
        for pelem in part_params.iter_ptr() {
            partitions.push(CStr::from_ptr((*pelem).name).to_str()?.to_string());
        }

        Ok(partitions)
    }

    unsafe fn partition_columns_ordered_at_end(self) -> Result<bool, CreateHookError> {
        if !self.partition_strategy_is_list() {
            return Err(CreateHookError::Partition);
        }

        let partition_columns = self.partition_list_columns()?;

        // Get column definitions
        let col_list = PgList::<pg_sys::ColumnDef>::from_pg((*self).tableElts);

        let num_normal_fields = col_list.len() - partition_columns.len();

        for (i, col_def) in col_list
            .iter_ptr()
            .enumerate()
            .filter(|&(i, _)| i >= num_normal_fields)
        {
            let col_name = CStr::from_ptr((*col_def).colname).to_str()?.to_string();
            if col_name != partition_columns[i - num_normal_fields] {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[derive(Error, Debug)]
pub enum CreateHookError {
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("PARTITION BY on parquet tables is only supported with LIST")]
    Partition,

    #[error(transparent)]
    TableAM(#[from] TableAMError),
}