use pgrx::{name_data_to_str, pg_sys, PgTupleDesc};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::ptr::NonNull;
use std::rc::Rc;

type NeedClose = bool;

/// Represents an opened Postgres relation to be used by pg_search.
///
/// [`PgSearchRelation`] is reference counted and will close the underlying
/// [`pg_sys::Relation`] when the last reference is dropped, accounting for
/// the state of the current transaction.
///
/// Instances of [`PgSearchRelation`] can be closed as necessary.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
#[repr(transparent)]
pub struct PgSearchRelation(
    Option<
        Rc<(
            NonNull<pg_sys::RelationData>,
            NeedClose,
            Option<pg_sys::LOCKMODE>,
        )>,
    >,
);

impl Debug for PgSearchRelation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgSearchRelation")
            .field("relation", &self.oid())
            .field("lockmode", &self.lockmode())
            .finish()
    }
}

impl Drop for PgSearchRelation {
    fn drop(&mut self) {
        if let Some(rc) = self.0.take() {
            if let Some((relation, need_close, lockmode)) = Rc::into_inner(rc) {
                unsafe {
                    if need_close && pg_sys::IsTransactionState() {
                        match lockmode {
                            Some(lockmode) => pg_sys::relation_close(relation.as_ptr(), lockmode),
                            None => pg_sys::RelationClose(relation.as_ptr()),
                        }
                    }
                }
            }
        }
    }
}

impl Deref for PgSearchRelation {
    type Target = pg_sys::RelationData;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the backing pointer is always correct for use by Rust as we couldn't have
        // gotten here otherwise
        unsafe { self.as_ptr().as_ref().unwrap_unchecked() }
    }
}

impl PgSearchRelation {
    pub unsafe fn from_pg(relation: pg_sys::Relation) -> Self {
        unsafe {
            Self(Some(Rc::new((
                NonNull::new_unchecked(relation),
                false,
                None,
            ))))
        }
    }

    pub fn open(oid: pg_sys::Oid) -> Self {
        unsafe {
            Self(Some(Rc::new((
                NonNull::new_unchecked(pg_sys::RelationIdGetRelation(oid)),
                true,
                None,
            ))))
        }
    }

    pub fn with_lock(oid: pg_sys::Oid, lockmode: pg_sys::LOCKMODE) -> Self {
        unsafe {
            Self(Some(Rc::new((
                NonNull::new_unchecked(pg_sys::relation_open(oid, lockmode)),
                true,
                Some(lockmode),
            ))))
        }
    }

    pub fn lockmode(&self) -> Option<pg_sys::LOCKMODE> {
        unsafe { self.0.as_ref().unwrap_unchecked().2 }
    }

    pub fn oid(&self) -> pg_sys::Oid {
        unsafe { (*self.as_ptr()).rd_id }
    }

    pub fn name(&self) -> &str {
        unsafe { name_data_to_str(&(*self.rd_rel).relname) }
    }

    pub fn namespace(&self) -> &str {
        unsafe {
            core::ffi::CStr::from_ptr(pg_sys::get_namespace_name((*self.rd_rel).relnamespace))
        }
        .to_str()
        .expect("unable to convert namespace name to UTF8")
    }

    pub fn tuple_desc(&self) -> PgTupleDesc {
        unsafe { PgTupleDesc::from_pg_unchecked(self.rd_att) }
    }

    pub fn reltuples(&self) -> Option<f32> {
        let reltuples = unsafe { (*self.rd_rel).reltuples };

        if reltuples == 0f32 {
            None
        } else {
            Some(reltuples)
        }
    }

    pub fn as_ptr(&self) -> pg_sys::Relation {
        // SAFETY: self.0 is always Some
        unsafe { self.0.as_ref().unwrap_unchecked().0.as_ptr() }
    }

    pub fn heap_relation(&self) -> Option<PgSearchRelation> {
        unsafe {
            if self.rd_index.is_null() {
                None
            } else {
                Some(PgSearchRelation::open((*self.rd_index).indrelid))
            }
        }
    }

    pub fn indices(&self, lockmode: pg_sys::LOCKMODE) -> impl Iterator<Item = PgSearchRelation> {
        use crate::PgList;
        // SAFETY: we know self.as_ptr() is a valid pointer as we created it
        let list =
            unsafe { PgList::<pg_sys::Oid>::from_pg(pg_sys::RelationGetIndexList(self.as_ptr())) };

        list.iter_oid()
            .filter(|oid| *oid != pg_sys::InvalidOid)
            .map(|oid| PgSearchRelation::with_lock(oid, lockmode))
            .collect::<Vec<_>>()
            .into_iter()
    }
}
