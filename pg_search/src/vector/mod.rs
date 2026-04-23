// Copyright (c) 2023-2026 ParadeDB, Inc.
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

pub mod metric;
pub(crate) mod sampler;

use pgrx::{pg_sys, FromDatum};

/// Owned `Vec<f32>` extracted from a pgvector datum. pgvector's on-disk
/// layout (after the varlena header) is `[int16 dim][int16 unused]
/// [float4; dim]`. `from_datum` detoasts the datum, copies the floats
/// out, and frees the detoasted copy if it differs from the original
/// pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct PgVector(pub Vec<f32>);

impl FromDatum for PgVector {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        _typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            return None;
        }
        let ptr = datum.cast_mut_ptr::<pg_sys::varlena>();
        let detoasted = pg_sys::pg_detoast_datum(ptr);
        let data = pgrx::varlena::vardata_any(detoasted);
        let dim = *(data as *const i16) as usize;
        let floats_ptr = (data as *const u8).add(4) as *const f32;
        let floats = std::slice::from_raw_parts(floats_ptr, dim).to_vec();
        if detoasted != ptr {
            pg_sys::pfree(detoasted as *mut std::ffi::c_void);
        }
        Some(Self(floats))
    }
}
