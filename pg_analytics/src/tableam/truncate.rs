use pgrx::*;
use std::ptr::addr_of_mut;

use crate::storage::metadata::{MetadataError, PgMetadata};

#[inline]
fn relation_nontransactional_truncate(rel: pg_sys::Relation) -> Result<(), MetadataError> {
    unsafe {
        // Removes all blocks from the relation
        pg_sys::RelationTruncate(rel, 0);

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

        // Reset the relation's metadata
        rel.init_metadata((*rel).rd_smgr).unwrap_or_else(|err| {
            panic!("{}", err);
        });
    }

    Ok(())
}

#[pg_guard]
pub extern "C" fn deltalake_relation_nontransactional_truncate(rel: pg_sys::Relation) {
    relation_nontransactional_truncate(rel).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}
