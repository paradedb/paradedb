#[cfg(any(feature = "pg12", feature = "pg13"))]
use core::ffi::c_int;
use pgrx::*;
use std::ptr::addr_of_mut;
use thiserror::Error;

use crate::storage::metadata::{MetadataError, PgMetadata};

#[inline]
fn relation_estimate_size(
    rel: pg_sys::Relation,
    attr_widths: *mut pg_sys::int32,
    pages: *mut pg_sys::BlockNumber,
    tuples: *mut f64,
    allvisfrac: *mut f64,
) -> Result<(), MetadataError> {
    info!("relation_estimate_size");
    unsafe {
        // If the relation has no storage manager, create one
        if (*rel).rd_smgr.is_null() {
            #[cfg(feature = "pg16")]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_locator, (*rel).rd_backend),
            );
            #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_node, (*rel).rd_backend),
            );
        }

        // Set tuple count
        *tuples = (rel.read_next_row_number()? - 1) as f64;

        // Set page count and visibility
        *pages = pg_sys::smgrnblocks((*rel).rd_smgr, pg_sys::ForkNumber_MAIN_FORKNUM);
        *allvisfrac = 1.0;

        pg_sys::get_rel_data_width(rel, attr_widths);
    }

    Ok(())
}

#[pg_guard]
pub extern "C" fn deltalake_relation_size(
    rel: pg_sys::Relation,
    fork_number: pg_sys::ForkNumber,
) -> pg_sys::uint64 {
    info!("deltalake_relation_size");
    unsafe {
        if (*rel).rd_smgr.is_null() {
            #[cfg(feature = "pg16")]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_locator, (*rel).rd_backend),
            );
            #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
            pg_sys::smgrsetowner(
                addr_of_mut!((*rel).rd_smgr),
                pg_sys::smgropen((*rel).rd_node, (*rel).rd_backend),
            );
        }

        let mut nblocks: pg_sys::uint64 = 0;

        match fork_number {
            pg_sys::ForkNumber_InvalidForkNumber => {
                for i in 0..pg_sys::ForkNumber_INIT_FORKNUM {
                    nblocks += pg_sys::smgrnblocks((*rel).rd_smgr, i) as pg_sys::uint64;
                }
            }
            fork_number => {
                nblocks = pg_sys::smgrnblocks((*rel).rd_smgr, fork_number) as pg_sys::uint64;
            }
        };

        nblocks * pg_sys::BLCKSZ as pg_sys::uint64
    }
}

#[pg_guard]
pub extern "C" fn deltalake_relation_estimate_size(
    rel: pg_sys::Relation,
    attr_widths: *mut pg_sys::int32,
    pages: *mut pg_sys::BlockNumber,
    tuples: *mut f64,
    allvisfrac: *mut f64,
) {
    info!("deltalake_relation_estimate_size");
    relation_estimate_size(rel, attr_widths, pages, tuples, allvisfrac).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13"))]
pub extern "C" fn deltalake_compute_xid_horizon_for_tuples(
    _rel: pg_sys::Relation,
    _items: *mut pg_sys::ItemPointerData,
    _nitems: c_int,
) -> pg_sys::TransactionId {
    info!("deltalake_compute_xid_horizon_for_tuples");
    panic!("{}", PlanError::XIDHorizonNotSupported.to_string())
}

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum PlanError {
    #[error("compute_xid_horizon_for_tuples not implemented")]
    XIDHorizonNotSupported,
}
