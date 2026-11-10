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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

pub use index_read::postgres::var::*;

use crate::api::FieldName;
use crate::postgres::customscan::pullup::{Access, ResTarget, ResolvedFastField, SourceContext};
use pgrx::is_a;
use pgrx::pg_sys::{self, CoerceViaIO, NodeTag::*, RelabelType};

/// Given a [`pg_sys::Node`] and a [`pg_sys::PlannerInfo`], attempt to find the [`pg_sys::Var`] and
/// the [`FieldName`] that it references.
///
/// It is the caller's responsibility to ensure that the node contains a Var with a valid FieldName.
pub unsafe fn find_one_resolved_var_and_fieldname(
    context: VarContext,
    source: &SourceContext,
    node: *mut pg_sys::Node,
    access: Access,
) -> (Option<ResolvedFastField>, Option<FieldName>) {
    if is_a(node, T_CoerceViaIO) {
        let expr = node.cast::<CoerceViaIO>();
        let arg = (*expr).arg;
        find_one_resolved_var_and_fieldname(context, source, arg as *mut pg_sys::Node, access)
    } else if is_a(node, T_RelabelType) {
        let relabel_type = node.cast::<RelabelType>();
        let arg = (*relabel_type).arg;
        find_one_resolved_var_and_fieldname(context, source, arg as *mut pg_sys::Node, access)
    } else if is_a(node, T_Var) {
        let (_, varattno) = context.var_relation(node.cast::<pg_sys::Var>());
        let name = source
            .tupdesc()
            .get(varattno as usize - 1)
            .map(|n| n.name().into());
        (
            source.resolve_fast_field(ResTarget::Attno(varattno), access),
            name,
        )
    } else if let Some(name) = unsafe { json_path_fieldname(context, node) } {
        let resolved = source.resolve_fast_field(ResTarget::Field(name.as_ref()), access);
        (resolved, Some(name))
    } else {
        (
            source.resolve_fast_field(ResTarget::Expr(node), access),
            None,
        )
    }
}
