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

//! Expose opaque SQL field dependencies to PostgreSQL's native permission checks.
//! RLS quals are native policy expressions, not caller SELECT dependencies.
//! PostgreSQL checks the surviving dependencies at ExecutorStart, including EXPLAIN.

use crate::api::HashSet;
use crate::api::operator::{
    field_name_from_node, is_paradedb_search_operator, query_for_permissions,
};
use crate::nodecast;
use crate::postgres::node::{NodeExt, WalkControl};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::var::{VarContext, query_output_list, query_rte, rte_source_query};
use crate::query::SearchQueryInput;
use crate::schema::FieldSource;
use pgrx::{FromDatum, JsonB, PgList, PgSqlErrorCode, pg_sys};

mod functions;

#[cfg(feature = "pg15")]
type PermissionInfo = pg_sys::RangeTblEntry;
#[cfg(not(feature = "pg15"))]
type PermissionInfo = pg_sys::RTEPermissionInfo;

// PG15 stores permissions on the RTE; PG16+ uses a separate permission list.
unsafe fn permission_info(
    query: *mut pg_sys::Query,
    rte: *mut pg_sys::RangeTblEntry,
) -> Option<*mut PermissionInfo> {
    #[cfg(feature = "pg15")]
    {
        let _ = query;
        Some(rte)
    }
    #[cfg(not(feature = "pg15"))]
    {
        ((*rte).perminfoindex != 0)
            .then(|| pg_sys::getRTEPermissionInfo((*query).rteperminfos, rte))
    }
}

unsafe fn rte_reads(query: *mut pg_sys::Query, rte: *mut pg_sys::RangeTblEntry) -> bool {
    permission_info(query, rte)
        .is_some_and(|info| (*info).requiredPerms & pg_sys::ACL_SELECT as pg_sys::AclMode != 0)
}

#[derive(Default)]
struct ReadSet {
    attributes: *mut pg_sys::Bitmapset,
}
impl ReadSet {
    unsafe fn attribute(&mut self, attno: pg_sys::AttrNumber) {
        self.attributes = pg_sys::bms_add_member(
            self.attributes,
            i32::from(attno) - pg_sys::FirstLowInvalidHeapAttributeNumber,
        );
    }
    unsafe fn whole(&mut self) {
        self.attribute(0);
    }
    unsafe fn field(&mut self, index: &PgSearchRelation, name: &str) {
        let schema = index.schema().expect("indexed schema");
        // The query engine validates unknown fields, including lenient parsing.
        let Some(field) = schema.search_field(name) else {
            return;
        };
        if field.is_ctid() {
            self.whole();
            return;
        }
        let fields = schema.categorized_fields();
        let source = fields
            .iter()
            .find(|(f, _)| f.field() == field.field())
            .expect("indexed field has a source")
            .1
            .source;
        match source {
            FieldSource::Heap { attno } => self.attribute((attno + 1) as _),
            FieldSource::Expression {
                att_idx: expression_idx,
            }
            | FieldSource::CompositeField { expression_idx, .. } => {
                pg_sys::pull_varattnos(
                    index
                        .index_expressions()
                        .get_ptr(expression_idx)
                        .expect("source expression")
                        .cast(),
                    1,
                    &mut self.attributes,
                );
            }
        }
    }
    unsafe fn query(&mut self, index: &PgSearchRelation, query: Option<&SearchQueryInput>) {
        let Some(query) = query else {
            return;
        };
        let mut fields = HashSet::default();
        query.extract_field_names(&index.schema().expect("indexed schema"), &mut fields);
        for field in fields {
            self.field(index, &field);
        }
    }

    unsafe fn search(
        &mut self,
        index: &PgSearchRelation,
        opno: pg_sys::Oid,
        lhs: *mut pg_sys::Node,
        field: Option<crate::api::FieldName>,
        rhs: *mut pg_sys::Node,
        array: bool,
    ) {
        if !array {
            self.query(index, query_for_permissions(opno, lhs, field, rhs).as_ref());
        } else if let Some(expr) = nodecast!(ArrayExpr, T_ArrayExpr, rhs) {
            for element in PgList::<pg_sys::Node>::from_pg((*expr).elements).iter_ptr() {
                self.search(index, opno, lhs, field.clone(), element, (*expr).multidims);
            }
        } else if let Some(constant) = nodecast!(Const, T_Const, rhs)
            && !(*constant).constisnull
        {
            let element_type = pg_sys::get_base_element_type((*constant).consttype);
            assert_ne!(element_type, pg_sys::InvalidOid, "scalar-array input type");
            let (mut len, mut by_value, mut align) = (0, false, 0);
            pg_sys::get_typlenbyvalalign(element_type, &mut len, &mut by_value, &mut align);
            let elements = pgrx::Array::<pg_sys::Datum>::from_datum((*constant).constvalue, false)
                .expect("non-null scalar-array input");
            for value in elements.iter().flatten() {
                let element = pg_sys::makeConst(
                    element_type,
                    -1,
                    (*constant).constcollid,
                    len as _,
                    value,
                    false,
                    by_value,
                );
                self.search(index, opno, lhs, field.clone(), element.cast(), false);
            }
        }
    }
}

fn untraceable_source() -> ! {
    pg_sys::ereport!(
        ERROR,
        PgSqlErrorCode::ERRCODE_INSUFFICIENT_PRIVILEGE,
        "cannot establish table permissions for search expression"
    );
}

unsafe fn add_read_set(
    query: *mut pg_sys::Query,
    rte: *mut pg_sys::RangeTblEntry,
    reads: &ReadSet,
) {
    let info = permission_info(query, rte).unwrap_or_else(|| untraceable_source());
    (*info).requiredPerms |= pg_sys::ACL_SELECT as pg_sys::AclMode;
    (*info).selectedCols = pg_sys::bms_union((*info).selectedCols, reads.attributes);
}

unsafe fn rte_actor(query: *mut pg_sys::Query, rte: *mut pg_sys::RangeTblEntry) -> pg_sys::Oid {
    permission_info(query, rte).map_or(pg_sys::Oid::INVALID, |info| (*info).checkAsUser)
}

unsafe fn nested_source(
    query: *mut pg_sys::Query,
    rte: *mut pg_sys::RangeTblEntry,
    parents: &[*mut pg_sys::Query],
) -> Option<(*mut pg_sys::Query, Vec<*mut pg_sys::Query>)> {
    let level = if (*rte).rtekind == pg_sys::RTEKind::RTE_CTE {
        (*rte).ctelevelsup as usize
    } else {
        0
    };
    let owner = if level == 0 {
        query
    } else {
        *parents.get(level - 1)?
    };
    let source = rte_source_query(rte, owner)?;
    let mut chain = vec![owner];
    chain.extend_from_slice(&parents[level..]);
    Some((source, chain))
}

// Borrowed from the original query tree for this annotation pass. Two aliases
// of the same heap are different sources, even when their columns match.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ColumnOrigin {
    query: *mut pg_sys::Query,
    rte: *mut pg_sys::RangeTblEntry,
    attno: pg_sys::AttrNumber,
}

unsafe fn var_source(
    query: *mut pg_sys::Query,
    var: *mut pg_sys::Var,
    parents: &[*mut pg_sys::Query],
) -> Option<(
    *mut pg_sys::Query,
    *mut pg_sys::RangeTblEntry,
    pg_sys::AttrNumber,
    &[*mut pg_sys::Query],
)> {
    pg_sys::check_stack_depth();
    let level = (*var).varlevelsup as usize;
    let (query, parents) = if level == 0 {
        (query, parents)
    } else {
        (*parents.get(level - 1)?, &parents[level..])
    };
    let rte = query_rte(query, (*var).varno as _)?;
    #[cfg(feature = "pg18")]
    if (*rte).rtekind == pg_sys::RTEKind::RTE_GROUP {
        let grouped = crate::postgres::var::resolve_rte_group_var(rte, (*var).varattno)?;
        return var_source(query, grouped, parents);
    }
    Some((query, rte, (*var).varattno, parents))
}

unsafe fn origin(
    query: *mut pg_sys::Query,
    var: *mut pg_sys::Var,
    parents: &[*mut pg_sys::Query],
    depth: usize,
) -> Option<ColumnOrigin> {
    if depth >= 64 || (*var).varno <= 0 {
        return None;
    }
    let (query, rte, attno, parents) = var_source(query, var, parents)?;
    if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
        return Some(ColumnOrigin { query, rte, attno });
    }
    let (sub, chain) = nested_source(query, rte, parents)?;
    // A set operation's output is not owned by its first branch alone.
    if !(*sub).setOperations.is_null() {
        return None;
    }
    let targets = PgList::<pg_sys::TargetEntry>::from_pg(query_output_list(sub));
    if attno > 0 {
        let tle = targets.iter_ptr().find(|tle| (**tle).resno == attno)?;
        if let Some(source) = (*tle).expr.find_single_node::<pg_sys::Var>() {
            return origin(sub, source, &chain, depth + 1);
        }
        return None;
    }
    let mut source = None;
    for tle in targets.iter_ptr().filter(|tle| !(**tle).resjunk) {
        if let Some(var) = (*tle).expr.find_single_node::<pg_sys::Var>() {
            let mut row = origin(sub, var, &chain, depth + 1)?;
            row.attno = 0;
            if source.is_some_and(|existing| existing != row) {
                return None;
            }
            source = Some(row);
        }
    }
    source
}

unsafe fn source_indexes(
    query: *mut pg_sys::Query,
    rte: *mut pg_sys::RangeTblEntry,
    parents: &[*mut pg_sys::Query],
    depth: usize,
) -> Vec<(ColumnOrigin, PgSearchRelation)> {
    if depth >= 64 {
        return vec![];
    }
    if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
        return rel_get_bm25_index((*rte).relid)
            .map(|(_, index)| {
                vec![(
                    ColumnOrigin {
                        query,
                        rte,
                        attno: 0,
                    },
                    index,
                )]
            })
            .unwrap_or_default();
    }
    let Some((sub, chain)) = nested_source(query, rte, parents) else {
        return vec![];
    };
    let mut found = vec![];
    let mut seen = HashSet::default();
    for child in PgList::<pg_sys::RangeTblEntry>::from_pg((*sub).rtable).iter_ptr() {
        for (source, index) in source_indexes(sub, child, &chain, depth + 1) {
            if seen.insert((source.query, source.rte)) {
                found.push((source, index));
            }
        }
    }
    found
}

/// Check a field absent from a derived row using its own native column
/// permission entry. An RLS source must expose the field through its policy scan.
unsafe fn extra_source_permission(
    query: *mut pg_sys::Query,
    index: &PgSearchRelation,
    actor: pg_sys::Oid,
    reads: &ReadSet,
) {
    let heap = index.heap_relation().expect("indexed heap");
    // Like rewrite's RLS_NONE_ENV handling, a bypass decision depends on role.
    // The auxiliary permission RTE is added after native policy rewrite.
    if (*heap.rd_rel).relrowsecurity {
        (*query).hasRowSecurity = true;
    }
    if pg_sys::check_enable_rls(heap.oid(), actor, true)
        == pg_sys::CheckEnableRlsResult::RLS_ENABLED as i32
    {
        pg_sys::ereport!(
            ERROR,
            PgSqlErrorCode::ERRCODE_INSUFFICIENT_PRIVILEGE,
            "opaque search field is not exposed by its row-security source"
        );
    }
    declare_relation_reads(query, &heap, actor, reads);
}

/// Declare native column requirements without introducing another row scan.
unsafe fn declare_relation_reads(
    query: *mut pg_sys::Query,
    relation: &PgSearchRelation,
    actor: pg_sys::Oid,
    reads: &ReadSet,
) {
    for rte in PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable).iter_ptr() {
        if let Some(info) = permission_info(query, rte)
            && (*info).relid == relation.oid()
            && (*info).checkAsUser == actor
        {
            add_read_set(query, rte, reads);
            return;
        }
    }
    let state = pg_sys::make_parsestate(std::ptr::null_mut());
    (*state).p_rtable = (*query).rtable;
    #[cfg(not(feature = "pg15"))]
    {
        (*state).p_rteperminfos = (*query).rteperminfos;
    }
    let ns = pg_sys::addRangeTableEntryForRelation(
        state,
        relation.as_ptr(),
        pg_sys::AccessShareLock as _,
        pg_sys::makeAlias(
            c"pg_search_read_dependency".as_ptr().cast_mut(),
            std::ptr::null_mut(),
        ),
        false,
        false,
    );
    let rte = (*ns).p_rte;
    #[cfg(not(feature = "pg15"))]
    {
        (*query).rteperminfos = (*state).p_rteperminfos;
    }
    (*query).rtable = (*state).p_rtable;
    let info = permission_info(query, rte).expect("new relation permission entry");
    (*info).checkAsUser = actor;
    add_read_set(query, rte, reads);
    pg_sys::free_parsestate(state);
}

unsafe fn mark_source(
    query: *mut pg_sys::Query,
    rte: *mut pg_sys::RangeTblEntry,
    index: &PgSearchRelation,
    reads: &ReadSet,
    parents: &[*mut pg_sys::Query],
    source: ColumnOrigin,
) {
    if pg_sys::bms_num_members(reads.attributes) == 0 {
        return;
    }
    let heap = index.heap_relation().expect("indexed heap");
    if query == source.query && rte == source.rte {
        add_read_set(query, rte, reads);
        return;
    }
    let Some((sub, chain)) = nested_source(query, rte, parents) else {
        extra_source_permission(query, index, pg_sys::Oid::INVALID, reads);
        return;
    };
    let mut exposed = ReadSet::default();
    let mut missing = ReadSet::default();
    // Expand whole-row permission bits to the derived row's output positions.
    // Dropped columns must not make a full projection appear incomplete.
    // Keep system-column dependencies explicit.
    let mut required = pg_sys::bms_copy(reads.attributes);
    let whole = -pg_sys::FirstLowInvalidHeapAttributeNumber;
    if pg_sys::bms_is_member(whole, required) {
        required = pg_sys::bms_del_member(required, whole);
        for attr in heap.tuple_desc().iter() {
            if !attr.attisdropped {
                required = pg_sys::bms_add_member(
                    required,
                    i32::from(attr.attnum) - pg_sys::FirstLowInvalidHeapAttributeNumber,
                );
            }
        }
    }
    let mut member = -1;
    loop {
        member = pg_sys::bms_next_member(required, member);
        if member < 0 {
            break;
        }
        let attno = member + pg_sys::FirstLowInvalidHeapAttributeNumber;
        let matched = PgList::<pg_sys::TargetEntry>::from_pg(query_output_list(sub))
            .iter_ptr()
            .filter(|tle| !(**tle).resjunk)
            .find(|tle| {
                (**tle)
                    .expr
                    .find_single_node::<pg_sys::Var>()
                    .and_then(|var| origin(sub, var, &chain, 0))
                    == Some(ColumnOrigin {
                        attno: attno as _,
                        ..source
                    })
                    && nodecast!(Var, T_Var, (**tle).expr).is_some()
            });
        if let Some(tle) = matched {
            exposed.attribute((*tle).resno);
        } else {
            missing.attribute(attno as _);
        }
    }
    pg_sys::bms_free(required);
    // PostgreSQL 16+ leaves the view's permission entry on the rewritten outer
    // RTE_SUBQUERY. It is not a base-table RTE inside the view's query.
    #[cfg(not(feature = "pg15"))]
    if (*rte).perminfoindex != 0 {
        add_read_set(query, rte, &exposed);
        if pg_sys::bms_num_members(missing.attributes) != 0 {
            extra_source_permission(query, index, rte_actor(query, rte), &missing);
        }
        return;
    }
    let mut view_actor = None;
    for old in PgList::<pg_sys::RangeTblEntry>::from_pg((*sub).rtable).iter_ptr() {
        if (*old).rtekind == pg_sys::RTEKind::RTE_RELATION
            && (*old).relkind == pg_sys::RELKIND_VIEW as core::ffi::c_char
            && rte_reads(sub, old)
        {
            view_actor = Some(rte_actor(sub, old));
            add_read_set(sub, old, &exposed);
        }
    }
    if pg_sys::bms_num_members(missing.attributes) != 0 {
        let actor = view_actor
            .or_else(|| {
                PgList::<pg_sys::RangeTblEntry>::from_pg((*sub).rtable)
                    .iter_ptr()
                    .find(|rte| {
                        (**rte).rtekind == pg_sys::RTEKind::RTE_RELATION
                            && (**rte).relid == heap.oid()
                            && rte_reads(sub, *rte)
                    })
                    .map(|rte| rte_actor(sub, rte))
            })
            .unwrap_or(pg_sys::Oid::INVALID);
        extra_source_permission(query, index, actor, &missing);
    }
}

fn aggregate_fields(
    value: &serde_json::Value,
    fields: &mut HashSet<String>,
    filters: &mut Vec<SearchQueryInput>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (kind, options) in map {
                if matches!(kind.as_str(), "aggs" | "aggregations") {
                    if let Some(children) = options.as_object() {
                        for definition in children.values() {
                            aggregate_fields(definition, fields, filters);
                        }
                    }
                } else if let Some(field) = options.get("field").and_then(serde_json::Value::as_str)
                {
                    fields.insert(field.to_owned());
                }
            }
            if let Some(filter) = map.get("filter").and_then(serde_json::Value::as_str) {
                filters.push(SearchQueryInput::Parse {
                    query_string: filter.to_owned(),
                    lenient: None,
                    conjunction_mode: None,
                });
            }
            if let Some(top) = map.get("top_hits") {
                if let Some(values) = top
                    .get("docvalue_fields")
                    .and_then(serde_json::Value::as_array)
                {
                    fields.extend(
                        values
                            .iter()
                            .filter_map(serde_json::Value::as_str)
                            .filter(|field| !field.contains('*'))
                            .map(str::to_owned),
                    );
                }
                if let Some(values) = top.get("sort").and_then(serde_json::Value::as_array) {
                    for value in values {
                        if let Some(entry) = value.as_object() {
                            fields
                                .extend(entry.keys().filter(|key| !key.starts_with('_')).cloned());
                        }
                    }
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                aggregate_fields(value, fields, filters);
            }
        }
        _ => {}
    }
}

unsafe fn register_search(
    query: *mut pg_sys::Query,
    opno: pg_sys::Oid,
    arguments: *mut pg_sys::List,
    parents: &[*mut pg_sys::Query],
    scalar_array: bool,
) {
    let args = PgList::<pg_sys::Node>::from_pg(arguments);
    let (Some(lhs), Some(rhs)) = (args.get_ptr(0), args.get_ptr(1)) else {
        untraceable_source();
    };
    let Some(var) = lhs.find_node::<pg_sys::Var>() else {
        // Calls without a SQL row dependency are outside this annotation pass.
        return;
    };
    let Some((target, rte, _, target_parents)) = var_source(query, var, parents) else {
        return;
    };
    let Some(source) = origin(query, var, parents, 0) else {
        return;
    };
    let Some((heap, index)) = rel_get_bm25_index((*source.rte).relid) else {
        return;
    };
    let normalized = pg_sys::eval_const_expressions(std::ptr::null_mut(), lhs);
    let field = if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
        field_name_from_node(VarContext::from_query(target), &heap, &index, normalized)
    } else {
        for var in normalized.collect_nodes::<pg_sys::Var>() {
            (*var).varattno = source.attno;
            (*var).varattnosyn = source.attno;
            (*var).varlevelsup = 0;
        }
        field_name_from_node(VarContext::from_exec(heap.oid()), &heap, &index, normalized)
    };
    let folded = pg_sys::eval_const_expressions(std::ptr::null_mut(), rhs);
    let mut reads = ReadSet::default();
    reads.search(&index, opno, lhs, field, folded, scalar_array);
    mark_source(target, rte, &index, &reads, target_parents, source);
}

unsafe fn register_aggregate(
    query: *mut pg_sys::Query,
    arguments: *mut pg_sys::List,
    parents: &[*mut pg_sys::Query],
) {
    let Some(arg) = PgList::<pg_sys::Node>::from_pg(arguments).get_ptr(0) else {
        return;
    };
    let arg = nodecast!(TargetEntry, T_TargetEntry, arg).map_or(arg, |tle| (*tle).expr.cast());
    let folded = pg_sys::eval_const_expressions(std::ptr::null_mut(), arg);
    let json = nodecast!(Const, T_Const, folded).and_then(|c| {
        ((*c).consttype == pg_sys::JSONBOID)
            .then(|| JsonB::from_datum((*c).constvalue, (*c).constisnull))
            .flatten()
    });
    let Some(json) = json else {
        return;
    };
    let mut names = HashSet::default();
    let mut filters = vec![];
    aggregate_fields(&json.0, &mut names, &mut filters);
    let sources: Vec<_> = PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable)
        .iter_ptr()
        .flat_map(|rte| {
            source_indexes(query, rte, parents, 0)
                .into_iter()
                .map(move |(source, index)| (rte, source, index))
        })
        .collect();
    for (_, _, index) in &sources {
        for filter in &filters {
            filter.extract_field_names(&index.schema().expect("indexed schema"), &mut names);
        }
    }
    for name in names {
        for (rte, source, index) in &sources {
            let alias = if (**rte).eref.is_null() {
                ""
            } else {
                std::ffi::CStr::from_ptr((*(**rte).eref).aliasname)
                    .to_str()
                    .expect("relation alias")
            };
            let field = name.strip_prefix(&format!("{alias}.")).unwrap_or(&name);
            if index
                .schema()
                .expect("indexed schema")
                .search_field(field)
                .is_some()
            {
                let mut set = ReadSet::default();
                set.field(index, field);
                mark_source(query, *rte, index, &set, parents, *source);
            }
        }
    }
}

pub(crate) unsafe fn annotate(query: *mut pg_sys::Query) -> bool {
    annotate_query(query, &[])
}

// Constant folding preserves SubLink Query pointers. Select surviving query
// levels without replacing current-level expressions or their dependencies.
unsafe fn live_subqueries(
    query: *mut pg_sys::Query,
    expr: *mut pg_sys::Node,
) -> HashSet<*mut pg_sys::Query> {
    if !(*query).hasSubLinks || expr.find_node::<pg_sys::Query>().is_none() {
        return HashSet::default();
    }
    pg_sys::eval_const_expressions(std::ptr::null_mut(), expr)
        .collect_nodes::<pg_sys::Query>()
        .into_iter()
        .collect()
}

unsafe fn annotate_query(query: *mut pg_sys::Query, parents: &[*mut pg_sys::Query]) -> bool {
    pg_sys::check_stack_depth();
    let agg_oids = crate::api::agg_funcoids();
    let helper_oids = crate::api::operator::search_with_query_input_exec_procoids();
    let mut chain = vec![query];
    chain.extend_from_slice(parents);
    let mut depends_on_role = false;
    // Snapshot before annotation can append permission-only RTEs. PostgreSQL
    // enumerates executable expressions; query-level recursion stays below.
    let rtes: Vec<_> = PgList::<pg_sys::RangeTblEntry>::from_pg((*query).rtable)
        .iter_ptr()
        .collect();
    let mut expressions = Vec::<*mut pg_sys::Node>::new();
    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn collect_expression(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if !node.is_null() {
            (&mut *context.cast::<Vec<*mut pg_sys::Node>>()).push(node);
        }
        false
    }
    let flags = pg_sys::QTW_IGNORE_RT_SUBQUERIES
        | pg_sys::QTW_IGNORE_CTE_SUBQUERIES
        | pg_sys::QTW_IGNORE_JOINALIASES;
    #[cfg(feature = "pg18")]
    let flags = flags | pg_sys::QTW_IGNORE_GROUPEXPRS;
    pg_sys::query_tree_walker(
        query,
        Some(collect_expression),
        std::ptr::from_mut(&mut expressions).cast(),
        flags as _,
    );
    for expr in expressions {
        let policy = expr == (*query).withCheckOptions.cast()
            || rtes.iter().any(|rte| expr == (**rte).securityQuals.cast());
        let live = live_subqueries(query, expr);
        let folded = (!policy && expr.any(|node| functions::is_scalar_sql_call(node)))
            .then(|| pg_sys::eval_const_expressions(std::ptr::null_mut(), expr));
        for (root, recurse) in std::iter::once((expr, true)).chain(folded.map(|expr| (expr, false)))
        {
            root.walk(|node| {
                if let Some(sub) = nodecast!(Query, T_Query, node) {
                    if recurse && live.contains(&sub) {
                        depends_on_role |= annotate_query(sub, &chain);
                    }
                    return WalkControl::SkipChildren;
                }
                if policy {
                    // Native policy columns are not caller SELECT dependencies.
                    return WalkControl::Continue;
                }
                if let Some(op) = nodecast!(OpExpr, T_OpExpr, node)
                    && (*op).opno.to_u32() >= pg_sys::FirstNormalObjectId
                    && is_paradedb_search_operator((*op).opno)
                {
                    register_search(query, (*op).opno, (*op).args, parents, false);
                }
                if let Some(op) = nodecast!(ScalarArrayOpExpr, T_ScalarArrayOpExpr, node)
                    && (*op).opno.to_u32() >= pg_sys::FirstNormalObjectId
                    && is_paradedb_search_operator((*op).opno)
                {
                    register_search(query, (*op).opno, (*op).args, parents, true);
                }
                if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, node)
                    && (helper_oids.contains(&(*func).funcid)
                        || (*func).funcid == crate::api::operator::anyelement_query_input_procoid())
                {
                    register_search(
                        query,
                        crate::api::operator::anyelement_query_input_opoid(),
                        (*func).args,
                        parents,
                        false,
                    );
                }
                if let Some(agg) = nodecast!(Aggref, T_Aggref, node)
                    && agg_oids.contains(&(*agg).aggfnoid.to_u32())
                {
                    register_aggregate(query, (*agg).args, parents);
                }
                if let Some(wf) = nodecast!(WindowFunc, T_WindowFunc, node)
                    && agg_oids.contains(&(*wf).winfnoid.to_u32())
                {
                    register_aggregate(query, (*wf).args, parents);
                }
                WalkControl::Continue
            });
        }
    }
    for rte in rtes {
        if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY {
            depends_on_role |= annotate_query((*rte).subquery, &chain);
        } else if (*rte).rtekind == pg_sys::RTEKind::RTE_FUNCTION {
            depends_on_role |= functions::annotate_srf(query, rte, &chain);
        }
    }
    for cte in PgList::<pg_sys::CommonTableExpr>::from_pg((*query).cteList).iter_ptr() {
        let sub = (*cte).ctequery.cast::<pg_sys::Query>();
        if (*cte).cterefcount != 0 || (*sub).commandType != pg_sys::CmdType::CMD_SELECT {
            depends_on_role |= annotate_query(sub, &chain);
        }
    }
    depends_on_role
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static CALLS: Cell<usize> = const { Cell::new(0) };
        static PREVIOUS: Cell<pg_sys::ExecutorCheckPerms_hook_type> = const { Cell::new(None) };
    }

    #[cfg(not(feature = "pg15"))]
    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn count_permissions(
        rtable: *mut pg_sys::List,
        infos: *mut pg_sys::List,
        report: bool,
    ) -> bool {
        CALLS.set(CALLS.get() + 1);
        PREVIOUS
            .get()
            .is_none_or(|hook| hook(rtable, infos, report))
    }

    #[cfg(feature = "pg15")]
    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn count_permissions(rtable: *mut pg_sys::List, report: bool) -> bool {
        CALLS.set(CALLS.get() + 1);
        PREVIOUS.get().is_none_or(|hook| hook(rtable, report))
    }

    struct RestoreHook(pg_sys::ExecutorCheckPerms_hook_type);
    // We intentionally do NOT use `impl_safe_drop!` here because the test hook
    // must be restored during unwinding; this assignment makes no PostgreSQL calls.
    impl Drop for RestoreHook {
        fn drop(&mut self) {
            unsafe { pg_sys::ExecutorCheckPerms_hook = self.0 };
        }
    }

    #[pgrx::pg_test]
    fn permission_hook_stays_in_executor() {
        pgrx::Spi::run(
            "CREATE TABLE permission_hook_docs(id integer, body text);
            INSERT INTO permission_hook_docs VALUES(1,'alpha');
            CREATE INDEX permission_hook_idx ON permission_hook_docs USING paradedb(id,body);
            CREATE FUNCTION permission_hook_search() RETURNS SETOF integer LANGUAGE sql STABLE AS $$
            SELECT id FROM permission_hook_docs WHERE id @@@ pdb.parse('body:alpha') $$",
        )
        .unwrap();
        unsafe {
            let _restore = RestoreHook(pg_sys::ExecutorCheckPerms_hook);
            PREVIOUS.set(pg_sys::ExecutorCheckPerms_hook);
            pg_sys::ExecutorCheckPerms_hook = Some(count_permissions);
            for sql in [
                c"SELECT id FROM permission_hook_docs WHERE body @@@ 'alpha'",
                c"SELECT * FROM permission_hook_search()",
            ] {
                CALLS.set(0);
                let raw = PgList::<pg_sys::RawStmt>::from_pg(pg_sys::pg_parse_query(sql.as_ptr()));
                let queries = pg_sys::pg_analyze_and_rewrite_fixedparams(
                    raw.get_ptr(0).expect("one SQL statement"),
                    sql.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                );
                let plans = pg_sys::pg_plan_queries(queries, sql.as_ptr(), 0, std::ptr::null_mut());
                assert!(!plans.is_null(), "query must actually be planned");
                assert_eq!(
                    CALLS.get(),
                    0,
                    "planning must not invoke executor permission hooks"
                );
                assert_eq!(
                    pgrx::Spi::get_one::<i32>(sql.to_str().unwrap()).unwrap(),
                    Some(1)
                );
                assert!(
                    CALLS.get() > 0,
                    "native ExecutorStart must still invoke the hook"
                );
            }
        }
    }

    #[cfg(not(feature = "pg15"))]
    #[pgrx::pg_test(error = "cannot establish table permissions for search expression")]
    fn missing_permission_entry_is_not_ignored() {
        unsafe {
            let mut query = pg_sys::Query::default();
            let mut rte = pg_sys::RangeTblEntry::default();
            let mut reads = ReadSet::default();
            reads.attribute(1);
            add_read_set(&mut query, &mut rte, &reads);
        }
    }
}
