// Copyright (c) 2023-2025 ParadeDB, Inc.
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
//! Provides a reference-counted wrapper around an open Postgres [`pg_sys::Relation`].
use crate::postgres::build::is_bm25_index;
use crate::postgres::options::BM25IndexOptions;
use crate::schema::SearchIndexSchema;
use pgrx::{name_data_to_str, pg_sys, PgList, PgTupleDesc};
use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::ptr::NonNull;
use std::rc::Rc;
use tantivy::TantivyError;

type NeedClose = bool;

#[derive(Debug, Clone)]
pub enum SchemaError {
    RelationNotBM25Index,
    Other(TantivyError),
}

impl Display for SchemaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RelationNotBM25Index => write!(f, "relation is not a BM25 index"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl Error for SchemaError {}

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
            RefCell<Option<Result<SearchIndexSchema, SchemaError>>>,
            BM25IndexOptions,
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
        let Some(rc) = self.0.take() else {
            return;
        };
        let Some((relation, need_close, lockmode, ..)) = Rc::into_inner(rc) else {
            return;
        };
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

impl Deref for PgSearchRelation {
    type Target = pg_sys::RelationData;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the backing pointer is always correct for use by Rust as we couldn't have
        // gotten here otherwise
        unsafe { self.as_ptr().as_ref().unwrap_unchecked() }
    }
}

impl PgSearchRelation {
    /// Take ownership of a [`pg_sys::Relation`] pointer previously created by Postgres
    ///
    /// This relation will not be closed when we're dropped.
    pub unsafe fn from_pg(relation: pg_sys::Relation) -> Self {
        Self(Some(Rc::new((
            NonNull::new(relation)
                .expect("PgSearchRelation::from_pg: provided relation cannot be NULL"),
            false,
            None,
            Default::default(),
            BM25IndexOptions::from_relation(relation),
        ))))
    }

    /// Open a relation with the specified [`pg_sys::Oid`].
    ///
    /// This relation will be closed when we're the last of our reference-counted clones to be dropped.
    pub fn open(oid: pg_sys::Oid) -> Self {
        unsafe {
            // SAFETY: RelationIdGetRelation() should return a valid RelationData pointer
            // unless no pg_class row could be found (suggesting the relation was just dropped)
            let relation = pg_sys::RelationIdGetRelation(oid);
            if relation.is_null() {
                panic!("relation not found, suggesting it was just dropped");
            }

            Self(Some(Rc::new((
                NonNull::new_unchecked(relation),
                true,
                None,
                Default::default(),
                BM25IndexOptions::from_relation(relation),
            ))))
        }
    }

    /// Open a relation with the specified [`pg_sys::Oid`]
    ///
    /// Like `open`, but fallible
    pub fn try_open(oid: pg_sys::Oid) -> Option<Self> {
        // SAFETY: See `open`
        unsafe {
            let relation = pg_sys::RelationIdGetRelation(oid);
            if relation.is_null() {
                None
            } else {
                Some(Self(Some(Rc::new((
                    NonNull::new_unchecked(relation),
                    true,
                    None,
                    Default::default(),
                    BM25IndexOptions::from_relation(relation),
                )))))
            }
        }
    }

    /// Open a relation with the specified [`pg_sys::Oid`] under the specified [`pg_sys::LOCKMODE`].
    ///
    /// This relation will be closed when we're the last of our reference-counted clones to be dropped.
    pub fn with_lock(oid: pg_sys::Oid, lockmode: pg_sys::LOCKMODE) -> Self {
        unsafe {
            // SAFETY: relation_open() always returns a valid RelationData pointer
            let relation = pg_sys::relation_open(oid, lockmode);
            Self(Some(Rc::new((
                NonNull::new_unchecked(relation),
                true,
                Some(lockmode),
                Default::default(),
                BM25IndexOptions::from_relation(relation),
            ))))
        }
    }

    pub fn lockmode(&self) -> Option<pg_sys::LOCKMODE> {
        // SAFETY: self.0 is always Some
        unsafe { self.0.as_ref().unwrap_unchecked().2 }
    }

    pub fn oid(&self) -> pg_sys::Oid {
        // SAFETY: self.as_ptr() is always a valid pointer
        unsafe { (*self.as_ptr()).rd_id }
    }

    pub fn rel_oid(&self) -> Option<pg_sys::Oid> {
        if self.rd_index.is_null() {
            None
        } else {
            unsafe { Some((*self.rd_index).indrelid) }
        }
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
        if self.rd_index.is_null() {
            None
        } else {
            unsafe { Some(PgSearchRelation::open((*self.rd_index).indrelid)) }
        }
    }

    pub fn indices(&self, lockmode: pg_sys::LOCKMODE) -> impl Iterator<Item = PgSearchRelation> {
        // SAFETY: we know self.as_ptr() is a valid pointer as we created it
        let list =
            unsafe { PgList::<pg_sys::Oid>::from_pg(pg_sys::RelationGetIndexList(self.as_ptr())) };

        list.iter_oid()
            .filter(|oid| *oid != pg_sys::InvalidOid)
            .map(|oid| PgSearchRelation::with_lock(oid, lockmode))
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn options(&self) -> &BM25IndexOptions {
        unsafe {
            // SAFETY: self.0 is always Some
            &self.0.as_ref().unwrap_unchecked().4
        }
    }

    pub fn schema(&self) -> Result<SearchIndexSchema, SchemaError> {
        let rc = self.0.as_ref().unwrap();
        let mut borrow = rc.3.borrow_mut();
        let schema = borrow.get_or_insert_with(|| {
            if !is_bm25_index(self) {
                return Err(SchemaError::RelationNotBM25Index);
            }

            SearchIndexSchema::open(self).map_err(SchemaError::Other)
        });

        match schema {
            Ok(schema) => Ok(schema.clone()),
            Err(e) => Err(e.clone()),
        }
    }
}
